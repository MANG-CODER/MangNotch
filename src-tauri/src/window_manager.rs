use crate::config::{debug_log, AppConfig, MonitorItem};
use crate::state::{NotchHitbox, NotchInstance, SharedState};
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, AtomicIsize, Ordering},
    Mutex, OnceLock,
};
use tauri::{AppHandle, Emitter, Manager, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[cfg(windows)]
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, RECT},
        Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW},
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowLongPtrW, SetForegroundWindow, SetWindowLongPtrW,
            SetWindowPos, SetWindowTextW, ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST,
            SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
            SW_HIDE, SW_SHOWNOACTIVATE, WINDOW_LONG_PTR_INDEX, WS_EX_LAYERED, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
        },
    },
};

pub const WINDOW_WIDTH: i32 = 1100;
pub const WINDOW_HEIGHT: i32 = 180;
pub const NOTCH_REVEAL_DELAY_MS: u64 = 220;

const ASFW_ANY: u32 = 0xFFFFFFFF;
const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
const DWMWA_CLOAK: u32 = 13;
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
const DWMWCP_DONOTROUND: u32 = 1;
const DWMWCP_ROUND: u32 = 2;
const DWMWA_BORDER_COLOR: u32 = 34;
const DWMWA_COLOR_NONE: u32 = 0xFFFFFFFE;
const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
const DWMSBT_NONE: u32 = 1;
const DWMSBT_MAINWINDOW: u32 = 2; // Mica
const DWMSBT_TRANSIENTWINDOW: u32 = 3; // Acrylic
const DWMSBT_TABBEDWINDOW: u32 = 4; // Mica Alt

pub static TEXT_INPUT_HWND: AtomicIsize = AtomicIsize::new(0);
pub static NOTCH_PREV_FG: AtomicIsize = AtomicIsize::new(0);
pub static CONTROL_HWND: AtomicIsize = AtomicIsize::new(0);

static RECONCILE_LOCK: Mutex<()> = Mutex::new(());
static CONTROL_CREATING: AtomicBool = AtomicBool::new(false);
static ORIG_WNDPROCS: OnceLock<Mutex<HashMap<isize, isize>>> = OnceLock::new();

fn orig_wndprocs() -> &'static Mutex<HashMap<isize, isize>> {
    ORIG_WNDPROCS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(windows)]
#[repr(C)]
struct StyleStruct {
    style_old: u32,
    style_new: u32,
}

#[cfg(windows)]
#[repr(C)]
struct DwmMargins {
    cx_left_width: i32,
    cx_right_width: i32,
    cy_top_height: i32,
    cy_bottom_height: i32,
}

#[cfg(windows)]
extern "system" {
    pub fn AllowSetForegroundWindow(dw_process_id: u32) -> i32;
    fn GetDpiForWindow(hwnd: isize) -> u32;
    fn ChangeWindowMessageFilterEx(
        hwnd: isize,
        message: u32,
        action: u32,
        p_change_filter_struct: *mut std::ffi::c_void,
    ) -> i32;
    fn CallWindowProcW(
        lp_prev_wnd_func: isize,
        hwnd: isize,
        msg: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize;
    fn DefWindowProcW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
}

#[cfg(windows)]
#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: isize,
        dw_attribute: u32,
        pv_attribute: *const std::ffi::c_void,
        cb_attribute: u32,
    ) -> i32;
    fn DwmExtendFrameIntoClientArea(hwnd: isize, p_mar_inset: *const DwmMargins) -> i32;
}

#[cfg(windows)]
pub fn get_hwnd(win: &WebviewWindow) -> Option<HWND> {
    win.hwnd().ok().map(|h| HWND(h.0 as *mut _))
}

#[cfg(windows)]
pub fn get_window_dpi_scale(hwnd: HWND) -> f64 {
    let dpi = unsafe { GetDpiForWindow(hwnd.0 as isize) };
    if dpi > 0 {
        (dpi as f64) / 96.0
    } else {
        1.0
    }
}

#[cfg(windows)]
fn allow_drop_messages(hwnd: HWND) {
    const MSGFLT_ALLOW: u32 = 1;
    const WM_DROPFILES: u32 = 0x0233;
    const WM_COPYDATA: u32 = 0x004A;
    const WM_COPYGLOBALDATA: u32 = 0x0049;
    unsafe {
        let h = hwnd.0 as isize;
        let _ = ChangeWindowMessageFilterEx(h, WM_DROPFILES, MSGFLT_ALLOW, std::ptr::null_mut());
        let _ = ChangeWindowMessageFilterEx(h, WM_COPYDATA, MSGFLT_ALLOW, std::ptr::null_mut());
        let _ = ChangeWindowMessageFilterEx(
            h,
            WM_COPYGLOBALDATA,
            MSGFLT_ALLOW,
            std::ptr::null_mut(),
        );
    }
}

#[cfg(windows)]
unsafe extern "system" fn notch_wnd_proc(
    hwnd: isize,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    const WM_NCDESTROY: u32 = 0x0082;
    const WM_NCCALCSIZE: u32 = 0x0083;
    const WM_NCPAINT: u32 = 0x0085;
    const WM_NCACTIVATE: u32 = 0x0086;
    const WM_MOUSEACTIVATE: u32 = 0x0021;
    const WM_STYLECHANGING: u32 = 0x007C;
    const MA_NOACTIVATE: isize = 3;
    const WM_QUERYENDSESSION: u32 = 0x0011;
    const WM_ENDSESSION: u32 = 0x0016;

    // Saat jendela benar-benar dihancurkan, lepaskan hook dan teruskan ke WndProc asli
    if msg == WM_NCDESTROY {
        let prev = orig_wndprocs()
            .lock()
            .ok()
            .and_then(|mut m| m.remove(&hwnd))
            .unwrap_or(0);
        if prev != 0 {
            SetWindowLongPtrW(HWND(hwnd as *mut _), WINDOW_LONG_PTR_INDEX(-4), prev);
            return CallWindowProcW(prev, hwnd, msg, wparam, lparam);
        }
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let is_typing_here = TEXT_INPUT_HWND.load(Ordering::SeqCst) == hwnd;

    match msg {
        WM_QUERYENDSESSION => return 1,
        WM_ENDSESSION => {
            if wparam != 0 {
                std::process::exit(0);
            }
            return 0;
        }
        WM_NCCALCSIZE => return 0,
        WM_NCPAINT => return 0,
        WM_NCACTIVATE => return 1,
        WM_MOUSEACTIVATE => {
            if !is_typing_here {
                return MA_NOACTIVATE;
            }
        }
        WM_STYLECHANGING => {
            if lparam != 0 {
                let ss = &mut *(lparam as *mut StyleStruct);
                if wparam as i32 == -16 {
                    ss.style_new &=
                        !(0x00C00000 | 0x00040000 | 0x00800000 | 0x00400000 | 0x00080000);
                } else if wparam as i32 == -20 {
                    ss.style_new |= 0x00000080;
                    if is_typing_here {
                        ss.style_new &= !0x08000000;
                    } else {
                        ss.style_new |= 0x08000000;
                    }
                }
            }
            return 0;
        }
        _ => {}
    }

    let prev = orig_wndprocs()
        .lock()
        .ok()
        .and_then(|m| m.get(&hwnd).copied())
        .unwrap_or(0);
    if prev != 0 {
        CallWindowProcW(prev, hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

#[cfg(windows)]
pub fn install_notch_wndproc(hwnd: HWND) {
    unsafe {
        let key = hwnd.0 as isize;
        let prev = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(-4));
        let hook_fn = notch_wnd_proc as *const () as usize as isize;
        if prev != 0 && prev != hook_fn {
            if let Ok(mut map) = orig_wndprocs().lock() {
                map.insert(key, prev);
            }
            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(-4), hook_fn);
        }
    }
}

#[cfg(windows)]
pub fn restore_notch_wndproc(hwnd_raw: isize) {
    if hwnd_raw == 0 {
        return;
    }
    let prev = orig_wndprocs()
        .lock()
        .ok()
        .and_then(|mut map| map.remove(&hwnd_raw));
    if let Some(prev_fn) = prev {
        if prev_fn != 0 {
            unsafe {
                SetWindowLongPtrW(
                    HWND(hwnd_raw as *mut _),
                    WINDOW_LONG_PTR_INDEX(-4),
                    prev_fn,
                );
            }
        }
    }
}

#[cfg(windows)]
pub fn restore_all_notch_wndprocs() {
    let entries: Vec<(isize, isize)> = orig_wndprocs()
        .lock()
        .ok()
        .map(|mut map| map.drain().collect())
        .unwrap_or_default();
    for (hwnd_raw, prev_fn) in entries {
        if hwnd_raw != 0 && prev_fn != 0 {
            unsafe {
                SetWindowLongPtrW(
                    HWND(hwnd_raw as *mut _),
                    WINDOW_LONG_PTR_INDEX(-4),
                    prev_fn,
                );
            }
        }
    }
}

#[cfg(windows)]
pub fn disable_window_transitions(hwnd: HWND) {
    unsafe {
        let val: u32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &val as *const _ as *const std::ffi::c_void,
            4,
        );
        let no_border: u32 = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_BORDER_COLOR,
            &no_border as *const _ as *const std::ffi::c_void,
            4,
        );
        let corner: u32 = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner as *const _ as *const std::ffi::c_void,
            4,
        );
    }
}

#[cfg(windows)]
pub fn set_window_cloak(hwnd: HWND, cloak: bool) {
    unsafe {
        let val: u32 = if cloak { 1 } else { 0 };
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_CLOAK,
            &val as *const _ as *const std::ffi::c_void,
            4,
        );
    }
}

#[cfg(windows)]
pub fn strip_window_borders(hwnd: HWND) {
    unsafe {
        install_notch_wndproc(hwnd);
        allow_drop_messages(hwnd);

        let empty_title: [u16; 1] = [0];
        let _ = SetWindowTextW(hwnd, PCWSTR(empty_title.as_ptr()));

        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let clean_style = style
            & !(0x00C00000isize
                | 0x00040000isize
                | 0x00800000isize
                | 0x00400000isize
                | 0x00080000isize
                | 0x00020000isize
                | 0x00010000isize);
        SetWindowLongPtrW(hwnd, GWL_STYLE, clean_style);

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            ex_style
                | (WS_EX_LAYERED.0 as isize)
                | (WS_EX_NOACTIVATE.0 as isize)
                | (WS_EX_TOOLWINDOW.0 as isize),
        );

        disable_window_transitions(hwnd);
    }
}

#[cfg(windows)]
pub fn set_hwnd_click_through(hwnd: HWND, ignore: bool) {
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let typing = TEXT_INPUT_HWND.load(Ordering::SeqCst) == hwnd.0 as isize;
        let mut base_ex = ex_style | (WS_EX_LAYERED.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize);
        if typing {
            base_ex &= !(WS_EX_NOACTIVATE.0 as isize);
        } else {
            base_ex |= WS_EX_NOACTIVATE.0 as isize;
        }
        let ignore = ignore && !typing;
        let new_ex = if ignore {
            base_ex | (WS_EX_TRANSPARENT.0 as isize)
        } else {
            base_ex & !(WS_EX_TRANSPARENT.0 as isize)
        };
        if new_ex != ex_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex);
            let _ = SetWindowPos(
                hwnd,
                HWND::default(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

#[cfg(windows)]
pub fn apply_control_backdrop(hwnd: HWND, mode: &str) {
    unsafe {
        let margins = DwmMargins {
            cx_left_width: -1,
            cx_right_width: -1,
            cy_top_height: -1,
            cy_bottom_height: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd.0 as isize, &margins);

        let dark: u32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const _ as *const std::ffi::c_void,
            4,
        );

        let round: u32 = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &round as *const _ as *const std::ffi::c_void,
            4,
        );

        let backdrop: u32 = match mode {
            "mica" => DWMSBT_MAINWINDOW,
            "mica_alt" => DWMSBT_TABBEDWINDOW,
            "none" => DWMSBT_NONE,
            _ => DWMSBT_TRANSIENTWINDOW,
        };
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const std::ffi::c_void,
            4,
        );
    }
}

pub fn enumerate_monitors() -> Vec<MonitorItem> {
    let mut monitors: Vec<MonitorItem> = Vec::new();
    #[cfg(windows)]
    unsafe {
        unsafe extern "system" fn callback(
            hmonitor: HMONITOR,
            _hdc: HDC,
            _lprc: *mut RECT,
            dwdata: LPARAM,
        ) -> BOOL {
            let list = &mut *(dwdata.0 as *mut Vec<MonitorItem>);
            let mut mi = MONITORINFOEXW::default();
            mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

            if GetMonitorInfoW(hmonitor, &mut mi.monitorInfo).as_bool() {
                let r = mi.monitorInfo.rcMonitor;
                let is_primary = (mi.monitorInfo.dwFlags & 1) != 0;
                let raw_name = String::from_utf16_lossy(&mi.szDevice)
                    .trim_matches(char::from(0))
                    .to_string();
                let idx = list.len();
                list.push(MonitorItem {
                    index: idx,
                    name: if is_primary {
                        format!("Monitor {} ({}) - Primary", idx + 1, raw_name)
                    } else {
                        format!("Monitor {} ({})", idx + 1, raw_name)
                    },
                    is_primary,
                    x: r.left,
                    y: r.top,
                    width: r.right - r.left,
                    height: r.bottom - r.top,
                });
            }
            BOOL(1)
        }

        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(callback),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    monitors
}

pub fn resolve_target_monitors(cfg: &AppConfig, monitors: &[MonitorItem]) -> Vec<MonitorItem> {
    if monitors.is_empty() {
        return Vec::new();
    }
    match cfg.monitor_mode.as_str() {
        "all" => monitors.to_vec(),
        "custom" => {
            let mut chosen: Vec<MonitorItem> = cfg
                .selected_monitors
                .iter()
                .filter_map(|&idx| monitors.get(idx).cloned())
                .collect();
            if chosen.is_empty() {
                if let Some(p) = monitors.iter().find(|m| m.is_primary).or(monitors.first()) {
                    chosen.push(p.clone());
                }
            }
            chosen
        }
        _ => {
            if let Some(p) = monitors.iter().find(|m| m.is_primary).or(monitors.first()) {
                vec![p.clone()]
            } else {
                Vec::new()
            }
        }
    }
}

#[cfg(windows)]
fn get_scaled_notch_size(notch_hwnd: HWND) -> (i32, i32) {
    let scale = get_window_dpi_scale(notch_hwnd);
    let phys_w = ((WINDOW_WIDTH as f64) * scale).round() as i32;
    let phys_h = ((WINDOW_HEIGHT as f64) * scale).round() as i32;
    (phys_w, phys_h)
}

#[cfg(windows)]
pub fn place_notch_boot_peeking(notch_hwnd: HWND, mon: &MonitorItem) {
    strip_window_borders(notch_hwnd);
    let (phys_w, phys_h) = get_scaled_notch_size(notch_hwnd);
    let x = mon.x + (mon.width - phys_w) / 2;
    let y = mon.y - phys_h + 2;
    unsafe {
        let _ = SetWindowPos(
            notch_hwnd,
            HWND_TOPMOST,
            x,
            y,
            phys_w,
            phys_h,
            SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        // Cek ulang DPI setelah berpindah ke monitor target (multi-monitor beda skala DPI)
        let (phys_w2, phys_h2) = get_scaled_notch_size(notch_hwnd);
        if phys_w2 != phys_w || phys_h2 != phys_h {
            let x2 = mon.x + (mon.width - phys_w2) / 2;
            let y2 = mon.y - phys_h2 + 2;
            let _ = SetWindowPos(
                notch_hwnd,
                HWND_TOPMOST,
                x2,
                y2,
                phys_w2,
                phys_h2,
                SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(windows)]
pub fn position_notch_on_monitor(notch_hwnd: HWND, mon: &MonitorItem) {
    strip_window_borders(notch_hwnd);
    let (phys_w, phys_h) = get_scaled_notch_size(notch_hwnd);
    let x = mon.x + (mon.width - phys_w) / 2;
    let y = mon.y;
    unsafe {
        let _ = ShowWindow(notch_hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            notch_hwnd,
            HWND_TOPMOST,
            x,
            y,
            phys_w,
            phys_h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        // Recenter ulang jika DPI monitor target berbeda dari monitor asal
        let (phys_w2, phys_h2) = get_scaled_notch_size(notch_hwnd);
        if phys_w2 != phys_w || phys_h2 != phys_h {
            let x2 = mon.x + (mon.width - phys_w2) / 2;
            let _ = SetWindowPos(
                notch_hwnd,
                HWND_TOPMOST,
                x2,
                y,
                phys_w2,
                phys_h2,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }
}

pub fn reconcile_notch_windows(app: &AppHandle, state: &SharedState) {
    let Ok(_guard) = RECONCILE_LOCK.lock() else {
        return;
    };

    let cfg = state.0.config.lock().unwrap().clone();
    let monitors = enumerate_monitors();
    let targets = resolve_target_monitors(&cfg, &monitors);
    let target_labels: HashSet<String> = targets.iter().map(|m| format!("notch-{}", m.index)).collect();

    // 1. Pastikan semua target monitor memiliki jendela Notch aktif
    for mon in &targets {
        let label = format!("notch-{}", mon.index);
        if let Some(existing) = app.get_webview_window(&label) {
            #[cfg(windows)]
            if let Some(hwnd) = get_hwnd(&existing) {
                let is_positioned = state
                    .0
                    .notches
                    .lock()
                    .ok()
                    .and_then(|m| m.get(&label).map(|n| n.positioned))
                    .unwrap_or(false);
                if is_positioned {
                    position_notch_on_monitor(hwnd, mon);
                    let scale = get_window_dpi_scale(hwnd);
                    if let Ok(mut map) = state.0.notches.lock() {
                        if let Some(inst) = map.get_mut(&label) {
                            inst.fallback_dpi = scale;
                        }
                    }
                }
            }
            #[cfg(not(windows))]
            let _ = existing;
            continue;
        }

        let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("notch.html".into()))
            .title("")
            .inner_size(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(true);
        match builder.build() {
            Ok(win) => {
                #[cfg(windows)]
                if let Some(hwnd) = get_hwnd(&win) {
                    install_notch_wndproc(hwnd);
                    disable_window_transitions(hwnd);
                    set_window_cloak(hwnd, true);
                    strip_window_borders(hwnd);
                    place_notch_boot_peeking(hwnd, mon);

                    let _ = win.show();

                    place_notch_boot_peeking(hwnd, mon);
                    set_hwnd_click_through(hwnd, true);
                    set_window_cloak(hwnd, false);

                    let fallback_dpi = get_window_dpi_scale(hwnd);
                    if let Ok(mut map) = state.0.notches.lock() {
                        map.insert(
                            label.clone(),
                            NotchInstance {
                                label: label.clone(),
                                monitor_index: mon.index,
                                hwnd: hwnd.0 as isize,
                                fallback_dpi,
                                ready_called: false,
                                positioned: false,
                                hitbox: NotchHitbox::default(),
                            },
                        );
                    }
                }
            }
            Err(e) => {
                debug_log(&format!("[window] gagal spawn {}: {:?}", label, e));
            }
        }
    }

    // 2. Hapus seluruh jendela notch-* yang tidak lagi menjadi target (baik di state maupun registry Tauri)
    let mut to_remove_labels: HashSet<String> = HashSet::new();
    if let Ok(map) = state.0.notches.lock() {
        for lbl in map.keys() {
            if !target_labels.contains(lbl) {
                to_remove_labels.insert(lbl.clone());
            }
        }
    }
    for (lbl, _) in app.webview_windows() {
        if lbl.starts_with("notch-") && !target_labels.contains(&lbl) {
            to_remove_labels.insert(lbl);
        }
    }

    for label in to_remove_labels {
        let state_hwnd = state
            .0
            .notches
            .lock()
            .ok()
            .and_then(|mut map| map.remove(&label).map(|n| n.hwnd))
            .unwrap_or(0);

        let app_c = app.clone();
        let label_c = label.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(w) = app_c.get_webview_window(&label_c) {
                #[cfg(windows)]
                if let Some(hwnd) = get_hwnd(&w) {
                    set_window_cloak(hwnd, true);
                    unsafe {
                        let _ = ShowWindow(hwnd, SW_HIDE);
                    }
                    // Pulihkan WndProc asli Tauri SEBELUM destroy agar WM_DESTROY/WM_NCDESTROY berjalan bersih
                    restore_notch_wndproc(hwnd.0 as isize);
                }
                let _ = w.hide();
                let _ = w.destroy();
            } else if state_hwnd != 0 {
                #[cfg(windows)]
                restore_notch_wndproc(state_hwnd);
            }
        });
    }
}

pub fn show_control_center_safe(app: &AppHandle, state: &SharedState) {
    // 1. Jika jendela Control Center sudah ada, tampilkan & fokus langsung di main thread
    if app.get_webview_window("control").is_some() {
        let app_clone = app.clone();
        let state_clone = state.clone();
        let _ = app.run_on_main_thread(move || {
            if let Ok(notches) = state_clone.0.notches.lock() {
                for inst in notches.values() {
                    #[cfg(windows)]
                    set_hwnd_click_through(HWND(inst.hwnd as *mut _), true);
                    let _ = app_clone.emit_to(&inst.label, "notch-hover", false);
                }
            }

            let backdrop_mode = state_clone
                .0
                .config
                .lock()
                .map(|c| c.bg_backdrop.clone())
                .unwrap_or_else(|_| "acrylic".to_string());

            if let Some(win) = app_clone.get_webview_window("control") {
                #[cfg(windows)]
                if let Some(hwnd) = get_hwnd(&win) {
                    CONTROL_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
                    apply_control_backdrop(hwnd, &backdrop_mode);
                }
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_always_on_top(true);
                let _ = win.set_focus();
                let _ = win.set_always_on_top(false);
                #[cfg(windows)]
                if let Some(hwnd) = get_hwnd(&win) {
                    unsafe {
                        let _ = SetForegroundWindow(hwnd);
                    }
                }
            }
        });
        return;
    }

    // 2. Jika belum ada (sudah ditutup/di-destroy), build di background thread agar tidak deadlock WebView2
    if CONTROL_CREATING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    let app_clone = app.clone();
    let state_clone = state.clone();
    std::thread::spawn(move || {
        let backdrop_mode = state_clone
            .0
            .config
            .lock()
            .map(|c| c.bg_backdrop.clone())
            .unwrap_or_else(|_| "acrylic".to_string());

        let built = WebviewWindowBuilder::new(
            &app_clone,
            "control",
            WebviewUrl::App("control.html".into()),
        )
        .title("MangNotch Control Center")
        .inner_size(740.0, 580.0)
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .center()
        .theme(Some(Theme::Dark))
        .visible(false)
        .build()
        .ok();

        if let Some(win) = built {
            let app_m = app_clone.clone();
            let st_m = state_clone.clone();
            let _ = app_clone.run_on_main_thread(move || {
                if let Ok(notches) = st_m.0.notches.lock() {
                    for inst in notches.values() {
                        #[cfg(windows)]
                        set_hwnd_click_through(HWND(inst.hwnd as *mut _), true);
                        let _ = app_m.emit_to(&inst.label, "notch-hover", false);
                    }
                }
                #[cfg(windows)]
                if let Some(hwnd) = get_hwnd(&win) {
                    CONTROL_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
                    apply_control_backdrop(hwnd, &backdrop_mode);
                }
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_always_on_top(true);
                let _ = win.set_focus();
                let _ = win.set_always_on_top(false);
                #[cfg(windows)]
                if let Some(hwnd) = get_hwnd(&win) {
                    unsafe {
                        let _ = SetForegroundWindow(hwnd);
                    }
                }
            });
        }

        CONTROL_CREATING.store(false, Ordering::SeqCst);
    });
}

pub fn set_text_input_for_window(app: &AppHandle, label: String, active: bool) {
    #[cfg(windows)]
    {
        let app2 = app.clone();
        let _ = app.run_on_main_thread(move || {
            let Some(w) = app2.get_webview_window(&label) else {
                return;
            };
            let Some(hwnd) = get_hwnd(&w) else {
                return;
            };
            let raw = hwnd.0 as isize;
            unsafe {
                let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                if active {
                    TEXT_INPUT_HWND.store(raw, Ordering::SeqCst);
                    let prev = GetForegroundWindow();
                    if prev.0 as isize != raw {
                        NOTCH_PREV_FG.store(prev.0 as isize, Ordering::SeqCst);
                    }
                    SetWindowLongPtrW(
                        hwnd,
                        GWL_EXSTYLE,
                        ex & !(WS_EX_NOACTIVATE.0 as isize) & !(WS_EX_TRANSPARENT.0 as isize),
                    );
                    let _ = AllowSetForegroundWindow(ASFW_ANY);
                    let _ = SetForegroundWindow(hwnd);
                    let _ = w.set_focus();
                } else {
                    if TEXT_INPUT_HWND.load(Ordering::SeqCst) == raw {
                        TEXT_INPUT_HWND.store(0, Ordering::SeqCst);
                    }
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | (WS_EX_NOACTIVATE.0 as isize));
                    let prev = NOTCH_PREV_FG.swap(0, Ordering::SeqCst);
                    if prev != 0 && GetForegroundWindow().0 as isize == raw {
                        let _ = SetForegroundWindow(HWND(prev as *mut _));
                    }
                }
            }
        });
    }
    #[cfg(not(windows))]
    {
        let _ = (app, label, active);
    }
}