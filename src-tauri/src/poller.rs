use crate::extra_peeks::ExtraPeeksMonitor;
use crate::state::SharedState;
use crate::window_manager::{set_hwnd_click_through, CONTROL_HWND, TEXT_INPUT_HWND};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO},
    Media::Audio::{
        eCapture, eConsole, eRender,
        Endpoints::{IAudioEndpointVolume, IAudioMeterInformation},
        IMMDeviceEnumerator, MMDeviceEnumerator,
    },
    System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
    UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE},
};

#[derive(Clone, Serialize)]
struct PeekPayload {
    kind: String,
    label: String,
    active: bool,
    percent: u32,
}

#[derive(Clone, Serialize)]
struct VolumeSyncPayload {
    volume: u32,
    muted: bool,
}

#[derive(Default)]
struct MonitorPollState {
    is_hovered: bool,
    outside_ticks: u32,
    is_obstructed: bool,
    is_fullscreen_hidden: bool,
    hotkey_pinned_open: bool,
}

#[derive(Clone, Copy, Debug)]
struct MonBounds {
    index: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    work_w: i32,
    work_h: i32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WinPoint {
    x: i32,
    y: i32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WinRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(windows)]
#[derive(Clone, Copy)]
struct VisibleAppWin {
    hwnd: isize,
    rect: WinRect,
    is_zoomed: bool,
    has_caption: bool,
    has_thickframe: bool,
}

#[cfg(windows)]
struct EnumCtx<'a> {
    ctrl_raw: isize,
    notch_hwnds: &'a [isize],
    out: Vec<VisibleAppWin>,
}

#[cfg(windows)]
#[repr(C)]
struct PowerStatus {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

#[cfg(windows)]
#[repr(C)]
struct DisplayBrightness {
    display_policy: u8,
    ac_brightness: u8,
    dc_brightness: u8,
}

#[cfg(windows)]
#[repr(C)]
struct BtFindRadioParams {
    dw_size: u32,
}

#[cfg(windows)]
#[repr(C)]
struct BtDeviceSearchParams {
    dw_size: u32,
    f_return_authenticated: i32,
    f_return_remembered: i32,
    f_return_unknown: i32,
    f_return_connected: i32,
    f_issue_inquiry: i32,
    c_timeout_multiplier: u8,
    h_radio: isize,
}

#[cfg(windows)]
#[repr(C)]
struct BtSystemTime {
    w: [u16; 8],
}

#[cfg(windows)]
#[repr(C)]
struct BtDeviceInfo {
    dw_size: u32,
    address: u64,
    ul_class_of_device: u32,
    f_connected: i32,
    f_remembered: i32,
    f_authenticated: i32,
    st_last_seen: BtSystemTime,
    st_last_used: BtSystemTime,
    sz_name: [u16; 248],
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn GetCursorPos(lp_point: *mut WinPoint) -> i32;
    fn GetWindowRect(hwnd: isize, lp_rect: *mut WinRect) -> i32;
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn IsIconic(hwnd: isize) -> i32;
    fn IsZoomed(hwnd: isize) -> i32;
    fn GetWindowLongW(hwnd: isize, n_index: i32) -> i32;
    fn EnumWindows(
        lp_enum_func: unsafe extern "system" fn(isize, isize) -> i32,
        lparam: isize,
    ) -> i32;
    fn SendMessageTimeoutW(
        hwnd: isize,
        msg: u32,
        wparam: usize,
        lparam: isize,
        fu_flags: u32,
        u_timeout: u32,
        lpdw_result: *mut usize,
    ) -> isize;
    fn GetClassNameW(hwnd: isize, lp_class_name: *mut u16, n_max_count: i32) -> i32;
    fn GetAsyncKeyState(v_key: i32) -> i16;
    fn GetKeyState(n_virt_key: i32) -> i16;
    fn GetDpiForWindow(hwnd: isize) -> u32;
}

#[cfg(windows)]
#[link(name = "dwmapi")]
extern "system" {
    fn DwmGetWindowAttribute(
        hwnd: isize,
        dw_attribute: u32,
        pv_attribute: *mut std::ffi::c_void,
        cb_attribute: u32,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetSystemPowerStatus(lp_status: *mut PowerStatus) -> i32;
    fn CreateFileW(
        lp_file_name: *const u16,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *mut std::ffi::c_void,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: isize,
    ) -> isize;
    fn DeviceIoControl(
        h_device: isize,
        dw_io_control_code: u32,
        lp_in_buffer: *mut std::ffi::c_void,
        n_in_buffer_size: u32,
        lp_out_buffer: *mut std::ffi::c_void,
        n_out_buffer_size: u32,
        lp_bytes_returned: *mut u32,
        lp_overlapped: *mut std::ffi::c_void,
    ) -> i32;
    fn CloseHandle(h_object: isize) -> i32;
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> isize;
    fn GetProcAddress(h_module: isize, lp_proc_name: *const u8) -> *const std::ffi::c_void;
}

fn enumerate_monitor_bounds() -> Vec<MonBounds> {
    let mut monitors: Vec<MonBounds> = Vec::new();
    #[cfg(windows)]
    unsafe {
        unsafe extern "system" fn cb(
            hmonitor: HMONITOR,
            _hdc: HDC,
            _lprc: *mut RECT,
            dwdata: LPARAM,
        ) -> BOOL {
            let list = &mut *(dwdata.0 as *mut Vec<MonBounds>);
            let mut mi = MONITORINFO::default();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
                let r = mi.rcMonitor;
                let w = mi.rcWork;
                let idx = list.len();
                list.push(MonBounds {
                    index: idx,
                    x: r.left,
                    y: r.top,
                    width: r.right - r.left,
                    height: r.bottom - r.top,
                    work_w: w.right - w.left,
                    work_h: w.bottom - w.top,
                });
            }
            BOOL(1)
        }
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(cb),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    monitors
}

#[cfg(windows)]
unsafe fn get_window_class(hwnd: isize) -> String {
    let mut buf = [0u16; 64];
    let len = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        String::new()
    }
}

#[cfg(windows)]
unsafe extern "system" fn enum_visible_windows_cb(hwnd: isize, lparam: isize) -> i32 {
    const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;
    const DWMWA_CLOAKED: u32 = 14;
    const WS_CAPTION: u32 = 0x00C0_0000;
    const WS_THICKFRAME: u32 = 0x0004_0000;
    const WS_EX_TRANSPARENT: u32 = 0x0000_0020;
    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
    const WS_EX_APPWINDOW: u32 = 0x0004_0000;

    let ctx = &mut *(lparam as *mut EnumCtx<'_>);
    if hwnd == 0 || hwnd == ctx.ctrl_raw || ctx.notch_hwnds.contains(&hwnd) {
        return 1;
    }
    if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
        return 1;
    }

    // Abaikan window UWP/virtual desktop yang ter-cloak oleh DWM
    let mut cloaked: u32 = 0;
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut _ as *mut std::ffi::c_void,
        std::mem::size_of::<u32>() as u32,
    ) == 0
        && cloaked != 0
    {
        return 1;
    }

    // Abaikan overlay click-through & tool window kecil
    let ex_style = GetWindowLongW(hwnd, -20) as u32;
    if (ex_style & WS_EX_TRANSPARENT) != 0 {
        return 1;
    }
    if (ex_style & WS_EX_TOOLWINDOW) != 0 && (ex_style & WS_EX_APPWINDOW) == 0 {
        return 1;
    }

    let cls = get_window_class(hwnd);
    if matches!(
        cls.as_str(),
        "Progman"
            | "WorkerW"
            | "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "Shell_InputSwitchTopLevelWindow"
            | "NotifyIconOverflowWindow"
            | "TopLevelWindowForOverflowXamlIsland"
            | "EdgeUiInputTopWndClass"
            | "ApplicationManager_ImmersiveShellWindow"
            | "Windows.UI.Core.CoreWindow"
    ) {
        return 1;
    }

    let mut rect = WinRect::default();
    let got_dwm = DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        &mut rect as *mut _ as *mut std::ffi::c_void,
        std::mem::size_of::<WinRect>() as u32,
    ) == 0;
    if !got_dwm && GetWindowRect(hwnd, &mut rect) == 0 {
        return 1;
    }
    if rect.right - rect.left < 80 || rect.bottom - rect.top < 40 {
        return 1;
    }

    let style = GetWindowLongW(hwnd, -16) as u32;
    let is_zoomed = IsZoomed(hwnd) != 0;
    let has_caption = (style & WS_CAPTION) == WS_CAPTION;
    let has_thickframe = (style & WS_THICKFRAME) != 0;

    ctx.out.push(VisibleAppWin {
        hwnd,
        rect,
        is_zoomed,
        has_caption,
        has_thickframe,
    });
    1
}

#[cfg(windows)]
unsafe fn collect_visible_app_windows(ctrl_raw: isize, notch_hwnds: &[isize]) -> Vec<VisibleAppWin> {
    let mut ctx = EnumCtx {
        ctrl_raw,
        notch_hwnds,
        out: Vec::with_capacity(24),
    };
    let _ = EnumWindows(enum_visible_windows_cb, &mut ctx as *mut _ as isize);
    ctx.out
}

/// Cek apakah area tengah atas tepat di belakang Notch berisi elemen UI aktif
/// (tab Chrome yang penuh, Command Center VS Code, dll.) atau ruang kosong titlebar (HTCAPTION).
#[cfg(windows)]
unsafe fn is_notch_area_occupied_by_ui(
    hwnd: isize,
    center_x: i32,
    top_y: i32,
    scale_px: &dyn Fn(i32) -> i32,
) -> bool {
    const WM_NCHITTEST: u32 = 0x0084;
    const SMTO_ABORTIFHUNG: u32 = 0x0002;
    const HTNOWHERE: usize = 0;
    const HTCAPTION: usize = 2;
    const HTTRANSPARENT: usize = usize::MAX;

    let probe_y = top_y + scale_px(18);
    let offsets = [-scale_px(64), 0, scale_px(64)];

    for dx in offsets {
        let px = center_x + dx;
        let packed = (((probe_y as i16 as u16) as usize) << 16) | ((px as i16 as u16) as usize);
        let mut res: usize = 0;
        let ok = SendMessageTimeoutW(
            hwnd,
            WM_NCHITTEST,
            0,
            packed as isize,
            SMTO_ABORTIFHUNG,
            20,
            &mut res,
        );
        if ok == 0 {
            // App elevated (UIPI) atau sedang sibuk -> fallback aman ke Minimalist
            return true;
        }
        if res != HTCAPTION && res != HTNOWHERE && res != HTTRANSPARENT {
            return true;
        }
    }
    false
}

#[cfg(windows)]
unsafe fn read_lcd_brightness() -> Option<u32> {
    const GENERIC_READ: u32 = 0x8000_0000;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    const OPEN_EXISTING: u32 = 3;
    const IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS: u32 = 0x0023_0498;

    let path: Vec<u16> = "\\\\.\\LCD\0".encode_utf16().collect();
    let h = CreateFileW(
        path.as_ptr(),
        GENERIC_READ,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        std::ptr::null_mut(),
        OPEN_EXISTING,
        0,
        0,
    );
    if h == -1 || h == 0 {
        return None;
    }
    let mut out = DisplayBrightness {
        display_policy: 0,
        ac_brightness: 0,
        dc_brightness: 0,
    };
    let mut ret: u32 = 0;
    let ok = DeviceIoControl(
        h,
        IOCTL_VIDEO_QUERY_DISPLAY_BRIGHTNESS,
        std::ptr::null_mut(),
        0,
        &mut out as *mut _ as *mut std::ffi::c_void,
        std::mem::size_of::<DisplayBrightness>() as u32,
        &mut ret,
        std::ptr::null_mut(),
    );
    let _ = CloseHandle(h);
    if ok == 0 {
        return None;
    }
    let val = if out.display_policy == 2 {
        out.dc_brightness
    } else {
        out.ac_brightness
    };
    Some((val as u32).min(100))
}

#[cfg(windows)]
struct BtApi {
    find_first_radio: unsafe extern "system" fn(*const BtFindRadioParams, *mut isize) -> isize,
    find_radio_close: unsafe extern "system" fn(isize) -> i32,
    find_first_device:
        unsafe extern "system" fn(*const BtDeviceSearchParams, *mut BtDeviceInfo) -> isize,
    find_next_device: unsafe extern "system" fn(isize, *mut BtDeviceInfo) -> i32,
    find_device_close: unsafe extern "system" fn(isize) -> i32,
}

#[cfg(windows)]
unsafe fn load_bt_api() -> Option<BtApi> {
    let dll: Vec<u16> = "bthprops.cpl\0".encode_utf16().collect();
    let hmod = LoadLibraryW(dll.as_ptr());
    if hmod == 0 {
        return None;
    }
    let p1 = GetProcAddress(hmod, b"BluetoothFindFirstRadio\0".as_ptr());
    let p2 = GetProcAddress(hmod, b"BluetoothFindRadioClose\0".as_ptr());
    let p3 = GetProcAddress(hmod, b"BluetoothFindFirstDevice\0".as_ptr());
    let p4 = GetProcAddress(hmod, b"BluetoothFindNextDevice\0".as_ptr());
    let p5 = GetProcAddress(hmod, b"BluetoothFindDeviceClose\0".as_ptr());
    if p1.is_null() || p2.is_null() || p3.is_null() || p4.is_null() || p5.is_null() {
        return None;
    }
    Some(BtApi {
        find_first_radio: std::mem::transmute(p1),
        find_radio_close: std::mem::transmute(p2),
        find_first_device: std::mem::transmute(p3),
        find_next_device: std::mem::transmute(p4),
        find_device_close: std::mem::transmute(p5),
    })
}

#[cfg(windows)]
unsafe fn query_bluetooth(api: &BtApi) -> (bool, u32, String) {
    let params = BtFindRadioParams {
        dw_size: std::mem::size_of::<BtFindRadioParams>() as u32,
    };
    let mut radio: isize = 0;
    let hfind = (api.find_first_radio)(&params, &mut radio);
    if hfind == 0 {
        return (false, 0, String::new());
    }
    let _ = (api.find_radio_close)(hfind);

    let mut count = 0u32;
    let mut last_name = String::new();
    let dparams = BtDeviceSearchParams {
        dw_size: std::mem::size_of::<BtDeviceSearchParams>() as u32,
        f_return_authenticated: 0,
        f_return_remembered: 0,
        f_return_unknown: 0,
        f_return_connected: 1,
        f_issue_inquiry: 0,
        c_timeout_multiplier: 0,
        h_radio: radio,
    };
    let mut info: BtDeviceInfo = std::mem::zeroed();
    info.dw_size = std::mem::size_of::<BtDeviceInfo>() as u32;
    let hdev = (api.find_first_device)(&dparams, &mut info);
    if hdev != 0 {
        loop {
            if info.f_connected != 0 {
                count += 1;
                let end = info
                    .sz_name
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(info.sz_name.len());
                let nm = String::from_utf16_lossy(&info.sz_name[..end])
                    .trim()
                    .to_string();
                if !nm.is_empty() {
                    last_name = nm;
                }
            }
            if (api.find_next_device)(hdev, &mut info) == 0 {
                break;
            }
        }
        let _ = (api.find_device_close)(hdev);
    }
    if radio != 0 {
        let _ = CloseHandle(radio);
    }
    (true, count, last_name)
}

#[cfg(windows)]
unsafe fn read_audio_render() -> Option<(u32, bool, f32)> {
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
    let vol = device
        .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
        .ok()?;
    let scalar = vol.GetMasterVolumeLevelScalar().ok()?;
    let muted = vol.GetMute().ok().map(|b| b.as_bool()).unwrap_or(false);
    let peak = device
        .Activate::<IAudioMeterInformation>(CLSCTX_ALL, None)
        .ok()
        .and_then(|m| m.GetPeakValue().ok())
        .unwrap_or(0.0);
    Some(((scalar * 100.0).round() as u32, muted, peak))
}

#[cfg(windows)]
unsafe fn read_mic_muted() -> Option<bool> {
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
    let device = enumerator
        .GetDefaultAudioEndpoint(eCapture, eConsole)
        .ok()?;
    let vol = device
        .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
        .ok()?;
    vol.GetMute().ok().map(|b| b.as_bool())
}

pub fn start_poller(app: AppHandle, state: SharedState) {
    std::thread::spawn(move || {
        #[cfg(windows)]
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        let mut per_mon: HashMap<String, MonitorPollState> = HashMap::new();
        let mut extra_peeks = ExtraPeeksMonitor::new();
        let mut drag_start_pt: Option<WinPoint> = None;
        let mut prev_hotkey = false;
        let mut tick: u32 = 0;

        #[cfg(windows)]
        let bt_api = unsafe { load_bt_api() };

        #[cfg(windows)]
        let mut visible_wins: Vec<VisibleAppWin> = Vec::new();

        let mut last_vol: Option<(u32, bool)> = None;
        let mut last_mic_muted: Option<bool> = None;
        let mut last_bright: Option<u32> = None;
        let mut last_caps: Option<bool> = None;
        let mut last_ac: Option<bool> = None;
        let mut last_bat_pct: u32 = 255;
        let mut last_bt: Option<(bool, u32)> = None;
        let mut last_peak_emit = Instant::now();
        let mut last_peak_zero = true;
        let mut cached_monitors = enumerate_monitor_bounds();

        loop {
            std::thread::sleep(Duration::from_millis(40));
            tick = tick.wrapping_add(1);

            let rt = match state.0.runtime.lock() {
                Ok(g) => g.clone(),
                Err(_) => continue,
            };

            if tick % 50 == 0 {
                cached_monitors = enumerate_monitor_bounds();
            }

            let notches_snap = match state.0.notches.lock() {
                Ok(g) => g.values().cloned().collect::<Vec<_>>(),
                Err(_) => continue,
            };
            let shelf_drag_label = state
                .0
                .shelf_drag_label
                .lock()
                .ok()
                .and_then(|g| g.clone());

            #[cfg(windows)]
            unsafe {
                let mut pt = WinPoint::default();
                let has_cursor = GetCursorPos(&mut pt) != 0;

                // Track mouse drag start
                let lbutton_down = (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0;
                if lbutton_down {
                    if drag_start_pt.is_none() && has_cursor {
                        drag_start_pt = Some(pt);
                    }
                } else {
                    drag_start_pt = None;
                }

                // Global Hotkey: Alt + N
                let alt_down = (GetAsyncKeyState(0x12) as u16 & 0x8000) != 0;
                let n_down = (GetAsyncKeyState(0x4E) as u16 & 0x8000) != 0;
                let hotkey_now = rt.enable_hotkey && alt_down && n_down;
                let hotkey_pressed = hotkey_now && !prev_hotkey;
                prev_hotkey = hotkey_now;

                // Control Center rect check
                let ctrl_raw = CONTROL_HWND.load(Ordering::SeqCst);
                let mut ctrl_rect = WinRect::default();
                let ctrl_visible = ctrl_raw != 0
                    && IsWindowVisible(ctrl_raw) != 0
                    && GetWindowRect(ctrl_raw, &mut ctrl_rect) != 0;

                // Scan semua window aktif berurutan Z-order (~160ms)
                let scan_windows = tick == 1 || tick % 4 == 0;
                if scan_windows && (rt.hide_on_fullscreen || rt.smart_proximity) {
                    let notch_hwnds: Vec<isize> = notches_snap.iter().map(|n| n.hwnd).collect();
                    visible_wins = collect_visible_app_windows(ctrl_raw, &notch_hwnds);
                }

                // === PER-MONITOR LOOP ===
                for inst in &notches_snap {
                    if !inst.positioned || inst.hwnd == 0 {
                        continue;
                    }
                    let hwnd = HWND(inst.hwnd as *mut _);
                    let mut win_rect = WinRect::default();
                    if GetWindowRect(inst.hwnd, &mut win_rect) == 0 {
                        continue;
                    }

                    let dpi = GetDpiForWindow(inst.hwnd);
                    let dpi_scale = if dpi > 0 {
                        (dpi as f64) / 96.0
                    } else {
                        inst.fallback_dpi.max(1.0)
                    };
                    let scale_px = |v: i32| ((v as f64) * dpi_scale).round() as i32;
                    let notch_center_x = (win_rect.left + win_rect.right) / 2;

                    let poll = per_mon.entry(inst.label.clone()).or_default();
                    let mon_opt = cached_monitors.iter().find(|m| m.index == inst.monitor_index);

                    // 1 & 2. Evaluasi Fullscreen & Smart Proximity INDEPENDEN per monitor
                    if scan_windows {
                        let mut mon_fs = false;
                        let mut mon_obstructed = false;

                        if let Some(mon) = mon_opt {
                            let taskbar_margin =
                                (mon.width - mon.work_w).max(mon.height - mon.work_h);
                            let has_visible_taskbar = taskbar_margin >= 16;

                            if rt.hide_on_fullscreen || rt.smart_proximity {
                                for w in &visible_wins {
                                    let cx = (w.rect.left + w.rect.right) / 2;
                                    let cy = (w.rect.top + w.rect.bottom) / 2;
                                    let center_on_mon = cx >= mon.x
                                        && cx < mon.x + mon.width
                                        && cy >= mon.y
                                        && cy < mon.y + mon.height;

                                    let covers_full_mon = w.rect.left <= mon.x + 4
                                        && w.rect.top <= mon.y + 4
                                        && w.rect.right >= mon.x + mon.width - 4
                                        && w.rect.bottom >= mon.y + mon.height - 4;

                                    // Cek True Fullscreen DULU (Chrome F11, YouTube FS, Game)
                                    if covers_full_mon && center_on_mon {
                                        let is_fs = if has_visible_taskbar {
                                            // Di monitor bertaskbar, menutupi taskbar = 100% True Fullscreen
                                            true
                                        } else {
                                            // Di monitor tanpa taskbar, bedakan dari maximize ber-frame biasa
                                            !w.is_zoomed || !w.has_caption || !w.has_thickframe
                                        };
                                        if is_fs {
                                            mon_fs = true;
                                            mon_obstructed = true;
                                            break;
                                        }
                                    }

                                    // Window Maximize: cek WM_NCHITTEST di belakang Notch
                                    // Chrome 1 tab (titlebar kosong) -> HTCAPTION -> tetap Mascot!
                                    // Chrome banyak tab / VS Code Command Center -> HTCLIENT -> Minimalist!
                                    if w.is_zoomed {
                                        if center_on_mon {
                                            if rt.smart_proximity {
                                                mon_obstructed = is_notch_area_occupied_by_ui(
                                                    w.hwnd,
                                                    notch_center_x,
                                                    mon.y,
                                                    &scale_px,
                                                );
                                            }
                                            break;
                                        }
                                        continue;
                                    }

                                    // Window floating biasa yang menutupi area Notch monitor ini
                                    let touches_notch = w.rect.top <= mon.y + scale_px(38)
                                        && w.rect.bottom > mon.y + scale_px(6)
                                        && w.rect.left < notch_center_x + scale_px(82)
                                        && w.rect.right > notch_center_x - scale_px(82);

                                    if touches_notch {
                                        if rt.smart_proximity {
                                            mon_obstructed = is_notch_area_occupied_by_ui(
                                                w.hwnd,
                                                notch_center_x,
                                                mon.y,
                                                &scale_px,
                                            );
                                        }
                                        break;
                                    }
                                }
                            }
                        }

                        let should_hide_fs = rt.hide_on_fullscreen && mon_fs;
                        if should_hide_fs != poll.is_fullscreen_hidden {
                            poll.is_fullscreen_hidden = should_hide_fs;
                            if should_hide_fs {
                                poll.is_hovered = false;
                                poll.hotkey_pinned_open = false;
                                set_hwnd_click_through(hwnd, true);
                                let _ = ShowWindow(hwnd, SW_HIDE);
                            } else {
                                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                            }
                        }

                        let should_obstruct = rt.smart_proximity && mon_obstructed;
                        if should_obstruct != poll.is_obstructed {
                            poll.is_obstructed = should_obstruct;
                            let _ = app.emit_to(&inst.label, "notch-proximity", should_obstruct);
                        }
                    }

                    if poll.is_fullscreen_hidden {
                        continue;
                    }

                    // 3. Global Hotkey Alt+N (toggle notch pada monitor tempat kursor berada)
                    if hotkey_pressed && has_cursor {
                        let cursor_on_mon = mon_opt.map_or(false, |m| {
                            pt.x >= m.x
                                && pt.x < m.x + m.width
                                && pt.y >= m.y
                                && pt.y < m.y + m.height
                        });
                        if cursor_on_mon {
                            poll.hotkey_pinned_open = !poll.hotkey_pinned_open;
                            poll.is_hovered = poll.hotkey_pinned_open;
                            poll.outside_ticks = 0;
                            set_hwnd_click_through(hwnd, !poll.hotkey_pinned_open);
                            let _ = app.emit_to(
                                &inst.label,
                                "notch-hotkey-toggle",
                                poll.hotkey_pinned_open,
                            );
                        }
                    }

                    // 4. Smart Hover independen per monitor
                    if has_cursor {
                        let is_dragging_shelf =
                            shelf_drag_label.as_deref() == Some(inst.label.as_str());

                        if is_dragging_shelf {
                            let local_x = ((pt.x - win_rect.left) as f64 / dpi_scale).round();
                            let local_y = ((pt.y - win_rect.top) as f64 / dpi_scale).round();
                            let _ = app.emit_to(
                                &inst.label,
                                "shelf-drag-position",
                                serde_json::json!({"x": local_x, "y": local_y}),
                            );
                        }

                        let is_mouse_dragging = is_dragging_shelf
                            || drag_start_pt.map_or(false, |start| {
                                start.y > win_rect.top + scale_px(60)
                                    && ((pt.x - start.x).abs() + (pt.y - start.y).abs())
                                        > scale_px(20)
                            });

                        let effective_w = if is_dragging_shelf
                            || (poll.is_hovered
                                && (!rt.interaction_mode_click
                                    || is_mouse_dragging
                                    || poll.hotkey_pinned_open))
                        {
                            inst.hitbox.active_width.max(420)
                        } else {
                            inst.hitbox.active_width
                        };
                        let effective_h = if is_dragging_shelf
                            || (poll.is_hovered
                                && (!rt.interaction_mode_click
                                    || is_mouse_dragging
                                    || poll.hotkey_pinned_open))
                        {
                            inst.hitbox.active_height.max(140)
                        } else {
                            inst.hitbox.active_height
                        };

                        let is_expanded = effective_h >= 100;
                        let is_minimal = poll.is_obstructed || effective_h <= 12;

                        let (half_w, hit_bottom) = if !poll.is_hovered && is_minimal {
                            let top_zone = if is_mouse_dragging { 24 } else { 6 };
                            (
                                scale_px(if is_mouse_dragging { 140 } else { 65 }),
                                win_rect.top + scale_px(top_zone),
                            )
                        } else if is_expanded {
                            (
                                scale_px(effective_w / 2 + 55),
                                win_rect.top + scale_px(effective_h + 40),
                            )
                        } else {
                            (
                                scale_px(
                                    effective_w / 2 + if is_mouse_dragging { 70 } else { 14 },
                                ),
                                win_rect.top
                                    + scale_px(
                                        effective_h.max(32)
                                            + if is_mouse_dragging { 28 } else { 8 },
                                    ),
                            )
                        };

                        let approaching_drag = is_mouse_dragging
                            && pt.x >= notch_center_x - scale_px(260)
                            && pt.x <= notch_center_x + scale_px(260)
                            && pt.y >= win_rect.top - scale_px(10)
                            && pt.y <= win_rect.top + scale_px(120);

                        let in_exact_notch = pt.x >= notch_center_x - scale_px(effective_w / 2)
                            && pt.x <= notch_center_x + scale_px(effective_w / 2)
                            && pt.y >= win_rect.top
                            && pt.y <= win_rect.top + scale_px(effective_h);

                        let mut inside = is_dragging_shelf
                            || approaching_drag
                            || (pt.x >= notch_center_x - half_w
                                && pt.x <= notch_center_x + half_w
                                && pt.y >= win_rect.top - scale_px(10)
                                && pt.y <= hit_bottom);

                        if ctrl_visible && !is_dragging_shelf && !in_exact_notch {
                            let in_ctrl = pt.x >= ctrl_rect.left
                                && pt.x <= ctrl_rect.right
                                && pt.y >= ctrl_rect.top
                                && pt.y <= ctrl_rect.bottom;
                            if in_ctrl && pt.y > win_rect.top + scale_px(34) {
                                inside = false;
                            }
                        }

                        let typing_here = TEXT_INPUT_HWND.load(Ordering::SeqCst) == inst.hwnd;

                        if inside {
                            poll.outside_ticks = 0;
                            if !poll.is_hovered {
                                poll.is_hovered = true;
                                if !rt.interaction_mode_click || is_mouse_dragging {
                                    if let Ok(mut map) = state.0.notches.lock() {
                                        if let Some(entry) = map.get_mut(&inst.label) {
                                            entry.hitbox.active_width =
                                                entry.hitbox.active_width.max(420);
                                            entry.hitbox.active_height =
                                                entry.hitbox.active_height.max(140);
                                        }
                                    }
                                }
                                set_hwnd_click_through(hwnd, false);
                                let _ = app.emit_to(&inst.label, "notch-hover", true);
                            }
                        } else if poll.is_hovered
                            && !poll.hotkey_pinned_open
                            && !typing_here
                            && !is_dragging_shelf
                        {
                            poll.outside_ticks += 1;
                            let required_ticks = if is_expanded {
                                rt.hover_close_ticks
                            } else {
                                3
                            };
                            if poll.outside_ticks >= required_ticks {
                                poll.is_hovered = false;
                                poll.outside_ticks = 0;
                                if let Ok(mut map) = state.0.notches.lock() {
                                    if let Some(entry) = map.get_mut(&inst.label) {
                                        entry.hitbox.active_width =
                                            entry.hitbox.resting_width.max(155);
                                        entry.hitbox.active_height = 32;
                                    }
                                }
                                set_hwnd_click_through(hwnd, true);
                                let _ = app.emit_to(&inst.label, "notch-hover", false);
                            }
                        }
                    }
                }

                // === GLOBAL SYSTEM PEEKS ===

                // A. Volume + Audio Beat Peak (~80ms)
                if tick % 2 == 0 {
                    if let Some((vol, muted, peak)) = read_audio_render() {
                        if rt.enable_audio_visualizer && last_peak_emit.elapsed().as_millis() >= 70
                        {
                            if peak > 0.015 {
                                last_peak_zero = false;
                                last_peak_emit = Instant::now();
                                let _ = app.emit("audio-peak", peak);
                            } else if !last_peak_zero {
                                last_peak_zero = true;
                                last_peak_emit = Instant::now();
                                let _ = app.emit("audio-peak", 0.0f32);
                            }
                        }

                        if let Some((pv, pm)) = last_vol {
                            if pv != vol || pm != muted {
                                last_vol = Some((vol, muted));
                                let _ = app.emit(
                                    "volume-sync",
                                    VolumeSyncPayload { volume: vol, muted },
                                );
                                if rt.enable_peeks && rt.peek_volume {
                                    let _ = app.emit(
                                        "system-peek",
                                        PeekPayload {
                                            kind: "volume".to_string(),
                                            label: if muted {
                                                "Audio Muted"
                                            } else {
                                                "System Master Audio"
                                            }
                                            .to_string(),
                                            active: !muted && vol > 0,
                                            percent: if muted { 0 } else { vol },
                                        },
                                    );
                                }
                            }
                        } else {
                            last_vol = Some((vol, muted));
                        }
                    }
                }

                // B. Caps Lock (tiap tick)
                let caps = (GetKeyState(0x14) & 1) != 0;
                if let Some(prev) = last_caps {
                    if prev != caps {
                        last_caps = Some(caps);
                        if rt.enable_peeks && rt.peek_capslock {
                            let _ = app.emit(
                                "system-peek",
                                PeekPayload {
                                    kind: "capslock".to_string(),
                                    label: if caps {
                                        "Uppercase Active"
                                    } else {
                                        "Lowercase"
                                    }
                                    .to_string(),
                                    active: caps,
                                    percent: 0,
                                },
                            );
                        }
                    }
                } else {
                    last_caps = Some(caps);
                }

                // C. Brightness (~240ms)
                if tick % 6 == 0 {
                    if let Some(br) = read_lcd_brightness() {
                        if let Some(prev) = last_bright {
                            if prev != br {
                                last_bright = Some(br);
                                if rt.enable_peeks && rt.peek_brightness {
                                    let _ = app.emit(
                                        "system-peek",
                                        PeekPayload {
                                            kind: "brightness".to_string(),
                                            label: "Display Backlight".to_string(),
                                            active: true,
                                            percent: br,
                                        },
                                    );
                                }
                            }
                        } else {
                            last_bright = Some(br);
                        }
                    }
                }

                // D. Microphone Mute (~320ms)
                if tick % 8 == 0 {
                    if let Some(mm) = read_mic_muted() {
                        if let Some(prev) = last_mic_muted {
                            if prev != mm {
                                last_mic_muted = Some(mm);
                                if rt.enable_peeks && rt.peek_mic {
                                    let _ = app.emit(
                                        "system-peek",
                                        PeekPayload {
                                            kind: "mic".to_string(),
                                            label: if mm {
                                                "Microphone Muted"
                                            } else {
                                                "Microphone Live"
                                            }
                                            .to_string(),
                                            active: !mm,
                                            percent: 0,
                                        },
                                    );
                                }
                            }
                        } else {
                            last_mic_muted = Some(mm);
                        }
                    }
                }

                // E. Battery & Charging (~1 detik)
                if tick % 25 == 0 {
                    let mut sps = PowerStatus {
                        ac_line_status: 255,
                        battery_flag: 0,
                        battery_life_percent: 255,
                        system_status_flag: 0,
                        battery_life_time: 0,
                        battery_full_life_time: 0,
                    };
                    if GetSystemPowerStatus(&mut sps) != 0 {
                        let pct = if sps.battery_life_percent <= 100 {
                            sps.battery_life_percent as u32
                        } else {
                            100
                        };
                        if pct != last_bat_pct {
                            last_bat_pct = pct;
                            let _ = app.emit(
                                "system-peek",
                                PeekPayload {
                                    kind: "battery_sync".to_string(),
                                    label: String::new(),
                                    active: sps.ac_line_status == 1,
                                    percent: pct,
                                },
                            );
                        }
                        if sps.ac_line_status == 0 || sps.ac_line_status == 1 {
                            let ac = sps.ac_line_status == 1;
                            if let Some(prev) = last_ac {
                                if prev != ac {
                                    last_ac = Some(ac);
                                    if rt.enable_peeks && rt.peek_charging {
                                        let _ = app.emit(
                                            "system-peek",
                                            PeekPayload {
                                                kind: "charging".to_string(),
                                                label: if ac {
                                                    "Power Connected"
                                                } else {
                                                    "Power Unplugged"
                                                }
                                                .to_string(),
                                                active: ac,
                                                percent: pct,
                                            },
                                        );
                                    }
                                }
                            } else {
                                last_ac = Some(ac);
                            }
                        }
                    }
                }

                // F. Bluetooth (~1.6 detik)
                if tick % 40 == 0 {
                    if let Some(ref api) = bt_api {
                        let (bt_on, dev_cnt, dev_name) = query_bluetooth(api);
                        if let Some((prev_on, prev_cnt)) = last_bt {
                            if prev_on != bt_on || prev_cnt != dev_cnt {
                                last_bt = Some((bt_on, dev_cnt));
                                if rt.enable_peeks && rt.peek_bluetooth {
                                    let lbl = if !bt_on {
                                        "Radio Disabled".to_string()
                                    } else if dev_cnt > 0 && !dev_name.is_empty() {
                                        dev_name
                                    } else if dev_cnt > 0 {
                                        format!("{} Device Connected", dev_cnt)
                                    } else {
                                        "Ready to Pair".to_string()
                                    };
                                    let _ = app.emit(
                                        "system-peek",
                                        PeekPayload {
                                            kind: "bluetooth".to_string(),
                                            label: lbl,
                                            active: bt_on,
                                            percent: dev_cnt,
                                        },
                                    );
                                }
                            }
                        } else {
                            last_bt = Some((bt_on, dev_cnt));
                        }
                    }
                }

                // G. 6 Extra System Peeks
                extra_peeks.tick(&app, &rt);
            }
        }
    });
}

#[allow(dead_code)]
pub fn spawn_poller(app: AppHandle, state: SharedState) {
    start_poller(app, state);
}