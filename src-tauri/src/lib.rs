use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicIsize, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent,
};

#[cfg(windows)]
use windows::{
    core::PCWSTR,
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
    Storage::Streams::DataReader,
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, BOOL, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, POINT,
            RECT, WPARAM,
        },
        Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO,
            MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
        },
        Media::Audio::{
            eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
            MMDeviceEnumerator,
        },
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, IBindCtx, IDataObject, CLSCTX_ALL,
                COINIT_MULTITHREADED,
            },
            Ole::{IDropSource, OleInitialize, DROPEFFECT_COPY, DROPEFFECT_LINK},
            Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
            Threading::CreateMutexW,
        },
        UI::{
            Input::KeyboardAndMouse::{GetKeyState, ReleaseCapture},
            Shell::{
                BHID_DataObject, IShellItem, SHCreateItemFromParsingName, SHDoDragDrop,
                ShellExecuteW,
            },
            WindowsAndMessaging::{
                GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
                IsIconic, SendMessageTimeoutW, SetForegroundWindow, SetWindowLongPtrW,
                SetWindowPos, SetWindowTextW, ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST,
                SMTO_ABORTIFHUNG, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
                SW_SHOWNOACTIVATE, SW_SHOWNORMAL, WINDOW_LONG_PTR_INDEX, WM_NCHITTEST,
                WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
            },
        },
    },
};

const WINDOW_WIDTH: i32 = 1100;
const HKEY_CURRENT_USER: isize = -2147483647i32 as isize;
const REG_SZ: u32 = 1;
const WAIT_OBJECT_0: u32 = 0;
const ASFW_ANY: u32 = 0xFFFFFFFF;
const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
const DWMWA_CLOAK: u32 = 13;
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
const DWMWCP_DONOTROUND: u32 = 1;
const DWMWA_BORDER_COLOR: u32 = 34;
const DWMWA_COLOR_NONE: u32 = 0xFFFFFFFE;
const NOTCH_REVEAL_DELAY_MS: u64 = 250;

static NOTCH_READY_CALLED: AtomicBool = AtomicBool::new(false);
static NOTCH_POSITIONED: AtomicBool = AtomicBool::new(false);
static ORIG_NOTCH_WNDPROC: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
#[repr(C)]
struct StyleStruct {
    style_old: u32,
    style_new: u32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct DisplayBrightness {
    uc_display_policy: u8,
    uc_ac_brightness: u8,
    uc_dc_brightness: u8,
}

#[cfg(windows)]
#[repr(C)]
struct BluetoothFindRadioParams {
    dw_size: u32,
}

#[cfg(windows)]
#[repr(C)]
struct BluetoothDeviceSearchParams {
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
struct SystemTime {
    w_year: u16,
    w_month: u16,
    w_day_of_week: u16,
    w_day: u16,
    w_hour: u16,
    w_minute: u16,
    w_second: u16,
    w_milliseconds: u16,
}

#[cfg(windows)]
#[repr(C)]
struct BluetoothDeviceInfo {
    dw_size: u32,
    address: u64,
    ul_class_of_device: u32,
    f_connected: i32,
    f_remembered: i32,
    f_authenticated: i32,
    st_last_seen: SystemTime,
    st_last_used: SystemTime,
    sz_name: [u16; 248],
}

#[cfg(windows)]
extern "system" {
    fn CreateFileW(
        lp_file_name: *const u16,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *const std::ffi::c_void,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: isize,
    ) -> isize;
    fn DeviceIoControl(
        h_device: isize,
        dw_io_control_code: u32,
        lp_in_buffer: *const std::ffi::c_void,
        n_in_buffer_size: u32,
        lp_out_buffer: *mut std::ffi::c_void,
        n_out_buffer_size: u32,
        lp_bytes_returned: *mut u32,
        lp_overlapped: *mut std::ffi::c_void,
    ) -> i32;
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> isize;
    fn GetProcAddress(h_module: isize, lp_proc_name: *const u8) -> *const std::ffi::c_void;

    fn CreateEventW(
        lp_event_attributes: *const std::ffi::c_void,
        b_manual_reset: i32,
        b_initial_state: i32,
        lp_name: *const u16,
    ) -> isize;
    fn SetEvent(h_event: isize) -> i32;
    fn ResetEvent(h_event: isize) -> i32;
    fn WaitForSingleObject(h_handle: isize, dw_milliseconds: u32) -> u32;
    fn AllowSetForegroundWindow(dw_process_id: u32) -> i32;
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn GetDpiForWindow(hwnd: isize) -> u32;
    fn CallWindowProcW(
        lp_prev_wnd_func: isize,
        hwnd: isize,
        msg: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize;
}

#[cfg(windows)]
#[link(name = "advapi32")]
extern "system" {
    fn RegSetKeyValueW(
        hkey: isize,
        lp_sub_key: *const u16,
        lp_value_name: *const u16,
        dw_type: u32,
        lp_data: *const std::ffi::c_void,
        cb_data: u32,
    ) -> i32;
    fn RegDeleteKeyValueW(
        hkey: isize,
        lp_sub_key: *const u16,
        lp_value_name: *const u16,
    ) -> i32;
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
}

#[cfg(windows)]
unsafe extern "system" fn notch_wnd_proc(
    hwnd: isize,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    const WM_NCCALCSIZE: u32 = 0x0083;
    const WM_NCPAINT: u32 = 0x0085;
    const WM_NCACTIVATE: u32 = 0x0086;
    const WM_MOUSEACTIVATE: u32 = 0x0021;
    const WM_STYLECHANGING: u32 = 0x007C;
    const MA_NOACTIVATE: isize = 3;
    const WM_QUERYENDSESSION: u32 = 0x0011;
    const WM_ENDSESSION: u32 = 0x0016;

    match msg {
        // Izinkan Microsoft Store / Windows Restart Manager mematikan app saat proses update
        WM_QUERYENDSESSION => return 1,
        WM_ENDSESSION => {
            if wparam != 0 {
                std::process::exit(0);
            }
            return 0;
        }
        // Paksa non-client area = 0px secara mutlak
        WM_NCCALCSIZE => return 0,
        // Cegah Windows menggambar frame atau bar putih non-client
        WM_NCPAINT => return 0,
        // Cegah DefWindowProcW menggambar title bar putih saat pindah fokus / klik tray
        WM_NCACTIVATE => return 1,
        // Klik di dalam notch tidak boleh mencuri fokus dari aplikasi lain
        WM_MOUSEACTIVATE => return MA_NOACTIVATE,
        // Cegah tao/Windows mengembalikan WS_CAPTION / WS_THICKFRAME secara diam-diam
        WM_STYLECHANGING => {
            if lparam != 0 {
                let ss = &mut *(lparam as *mut StyleStruct);
                if wparam as i32 == -16 {
                    ss.style_new &=
                        !(0x00C00000 | 0x00040000 | 0x00800000 | 0x00400000 | 0x00080000);
                } else if wparam as i32 == -20 {
                    ss.style_new |= 0x08000000 | 0x00000080;
                }
            }
            return 0;
        }
        _ => {}
    }

    let prev = ORIG_NOTCH_WNDPROC.load(Ordering::SeqCst);
    if prev != 0 {
        CallWindowProcW(prev, hwnd, msg, wparam, lparam)
    } else {
        0
    }
}

#[cfg(windows)]
fn install_notch_wndproc(hwnd: HWND) {
    unsafe {
        let prev = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(-4));
        let hook_fn = notch_wnd_proc as *const () as usize as isize;
        if prev != 0 && prev != hook_fn {
            ORIG_NOTCH_WNDPROC.store(prev, Ordering::SeqCst);
            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(-4), hook_fn);
        }
    }
}

#[cfg(windows)]
fn disable_window_transitions(hwnd: HWND) {
    unsafe {
        let val: u32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &val as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
        let no_border: u32 = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_BORDER_COLOR,
            &no_border as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
        let corner: u32 = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

#[cfg(windows)]
fn set_window_cloak(hwnd: HWND, cloak: bool) {
    unsafe {
        let val: u32 = if cloak { 1 } else { 0 };
        let _ = DwmSetWindowAttribute(
            hwnd.0 as isize,
            DWMWA_CLOAK,
            &val as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

#[cfg(windows)]
fn strip_window_borders(hwnd: HWND) {
    unsafe {
        install_notch_wndproc(hwnd);

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
fn set_hwnd_click_through(hwnd: HWND, ignore: bool) {
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let base_ex = ex_style
            | (WS_EX_LAYERED.0 as isize)
            | (WS_EX_NOACTIVATE.0 as isize)
            | (WS_EX_TOOLWINDOW.0 as isize);
        let new_ex = if ignore {
            base_ex | (WS_EX_TRANSPARENT.0 as isize)
        } else {
            base_ex & !(WS_EX_TRANSPARENT.0 as isize)
        };
        if new_ex != ex_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex);
        }
    }
}

#[cfg(windows)]
fn read_screen_brightness() -> Option<u8> {
    unsafe {
        let path: Vec<u16> = "\\\\.\\LCD\0".encode_utf16().collect();
        let h_dev = CreateFileW(
            path.as_ptr(),
            0x80000000 | 0x40000000,
            3,
            std::ptr::null(),
            3,
            0,
            0,
        );
        if h_dev == -1 || h_dev == 0 {
            return None;
        }
        let mut brightness = DisplayBrightness::default();
        let mut bytes_ret: u32 = 0;
        let ok = DeviceIoControl(
            h_dev,
            0x00230498,
            std::ptr::null(),
            0,
            &mut brightness as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<DisplayBrightness>() as u32,
            &mut bytes_ret,
            std::ptr::null_mut(),
        );
        let _ = CloseHandle(HANDLE(h_dev as *mut _));
        if ok != 0 {
            let val = if brightness.uc_display_policy == 1 {
                brightness.uc_ac_brightness
            } else {
                brightness.uc_dc_brightness
            };
            Some(val.min(100))
        } else {
            None
        }
    }
}

#[cfg(windows)]
struct BtState {
    radio_on: bool,
    connected_count: u32,
    device_name: String,
}

#[cfg(windows)]
fn read_bluetooth_state(bth_mod: isize) -> Option<BtState> {
    if bth_mod == 0 {
        return None;
    }
    unsafe {
        type FnFindFirstRadio =
            unsafe extern "system" fn(*const BluetoothFindRadioParams, *mut isize) -> isize;
        type FnFindRadioClose = unsafe extern "system" fn(isize) -> i32;
        type FnFindFirstDevice = unsafe extern "system" fn(
            *const BluetoothDeviceSearchParams,
            *mut BluetoothDeviceInfo,
        ) -> isize;
        type FnFindNextDevice =
            unsafe extern "system" fn(isize, *mut BluetoothDeviceInfo) -> i32;
        type FnFindDeviceClose = unsafe extern "system" fn(isize) -> i32;

        let p_first_radio = GetProcAddress(bth_mod, b"BluetoothFindFirstRadio\0".as_ptr());
        let p_close_radio = GetProcAddress(bth_mod, b"BluetoothFindRadioClose\0".as_ptr());
        let p_first_dev = GetProcAddress(bth_mod, b"BluetoothFindFirstDevice\0".as_ptr());
        let p_next_dev = GetProcAddress(bth_mod, b"BluetoothFindNextDevice\0".as_ptr());
        let p_close_dev = GetProcAddress(bth_mod, b"BluetoothFindDeviceClose\0".as_ptr());

        if p_first_radio.is_null() || p_close_radio.is_null() {
            return None;
        }

        let find_first_radio: FnFindFirstRadio = std::mem::transmute(p_first_radio);
        let find_radio_close: FnFindRadioClose = std::mem::transmute(p_close_radio);

        let radio_params = BluetoothFindRadioParams {
            dw_size: std::mem::size_of::<BluetoothFindRadioParams>() as u32,
        };
        let mut h_radio: isize = 0;
        let h_find_radio = find_first_radio(&radio_params, &mut h_radio);

        if h_find_radio == 0 || h_radio == 0 {
            return Some(BtState {
                radio_on: false,
                connected_count: 0,
                device_name: String::new(),
            });
        }

        let mut connected_count = 0;
        let mut first_dev_name = String::new();

        if !p_first_dev.is_null() && !p_next_dev.is_null() && !p_close_dev.is_null() {
            let find_first_dev: FnFindFirstDevice = std::mem::transmute(p_first_dev);
            let find_next_dev: FnFindNextDevice = std::mem::transmute(p_next_dev);
            let find_dev_close: FnFindDeviceClose = std::mem::transmute(p_close_dev);

            let search_params = BluetoothDeviceSearchParams {
                dw_size: std::mem::size_of::<BluetoothDeviceSearchParams>() as u32,
                f_return_authenticated: 1,
                f_return_remembered: 1,
                f_return_unknown: 0,
                f_return_connected: 1,
                f_issue_inquiry: 0,
                c_timeout_multiplier: 0,
                h_radio,
            };

            let mut dev_info: BluetoothDeviceInfo = std::mem::zeroed();
            dev_info.dw_size = std::mem::size_of::<BluetoothDeviceInfo>() as u32;

            let h_find_dev = find_first_dev(&search_params, &mut dev_info);
            if h_find_dev != 0 {
                loop {
                    if dev_info.f_connected != 0 {
                        connected_count += 1;
                        if first_dev_name.is_empty() {
                            let len = dev_info
                                .sz_name
                                .iter()
                                .position(|&c| c == 0)
                                .unwrap_or(dev_info.sz_name.len());
                            first_dev_name = String::from_utf16_lossy(&dev_info.sz_name[..len])
                                .trim()
                                .to_string();
                        }
                    }
                    if find_next_dev(h_find_dev, &mut dev_info) == 0 {
                        break;
                    }
                }
                let _ = find_dev_close(h_find_dev);
            }
        }

        let _ = CloseHandle(HANDLE(h_radio as *mut _));
        let _ = find_radio_close(h_find_radio);

        Some(BtState {
            radio_on: true,
            connected_count,
            device_name: first_dev_name,
        })
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MonitorItem {
    pub index: usize,
    pub name: String,
    pub is_primary: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TodoItem {
    pub text: String,
    pub done: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ShelfItem {
    pub name: String,
    pub path: String,
    pub ext: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VolumePayload {
    pub volume: u32,
    pub muted: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AppConfig {
    pub use_primary_monitor: bool,
    pub selected_monitor_index: usize,
    pub hide_on_fullscreen: bool,
    pub smart_proximity: bool,
    pub start_with_windows: bool,
    pub interaction_mode: String,
    pub todo_style: String,
    pub resting_mode: String,
    pub mascot_style: String,
    pub theme_preset: String,
    pub enable_peeks: bool,
    pub enable_sound: bool,
    pub water_reminder_enabled: bool,
    pub water_tap_sound: bool,
    pub water_reminder_sound: bool,
    pub water_goal_ml: u32,
    pub water_cup_ml: u32,
    pub water_current_ml: u32,
    pub water_interval_min: u32,
    pub last_water_date: String,
    pub pomo_minutes: u32,
    pub pomo_break_minutes: u32,
    pub todos: Vec<TodoItem>,
    pub shelf: Vec<ShelfItem>,
    pub active_width: i32,
    pub active_height: i32,
    pub resting_width: i32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            use_primary_monitor: true,
            selected_monitor_index: 0,
            hide_on_fullscreen: true,
            smart_proximity: true,
            start_with_windows: false,
            interaction_mode: "hover".to_string(),
            todo_style: "horizontal".to_string(),
            resting_mode: "pet".to_string(),
            mascot_style: "neko".to_string(),
            theme_preset: "dusk".to_string(),
            enable_peeks: true,
            enable_sound: true,
            water_reminder_enabled: true,
            water_tap_sound: true,
            water_reminder_sound: true,
            water_goal_ml: 1500,
            water_cup_ml: 250,
            water_current_ml: 0,
            water_interval_min: 45,
            last_water_date: String::new(),
            pomo_minutes: 25,
            pomo_break_minutes: 5,
            todos: Vec::new(),
            shelf: Vec::new(),
            active_width: 155,
            active_height: 32,
            resting_width: 155,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct MediaPayload {
    pub has_media: bool,
    pub is_playing: bool,
    pub title: String,
    pub artist: String,
    pub position_sec: f64,
    pub duration_sec: f64,
    pub thumbnail_b64: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SystemPeekPayload {
    pub kind: String,
    pub active: bool,
    pub percent: u8,
    pub label: String,
}

pub struct SharedState(pub Arc<Mutex<AppConfig>>);

fn get_config_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(base).join("MangNotch");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

fn load_config_from_disk() -> AppConfig {
    let path = get_config_path();
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&data) {
            cfg.active_width = 155;
            cfg.active_height = 32;
            cfg.resting_width = 155;
            if cfg.interaction_mode != "click" && cfg.interaction_mode != "hover" {
                cfg.interaction_mode = "hover".to_string();
            }
            if cfg.todo_style != "horizontal" && cfg.todo_style != "vertical" {
                cfg.todo_style = "horizontal".to_string();
            }
            return cfg;
        }
    }
    AppConfig::default()
}

fn save_config_to_disk(cfg: &AppConfig) {
    let path = get_config_path();
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(path, json);
    }
}

fn unblock_file_motw(path: &str) {
    let ads_path = format!("{}:Zone.Identifier", path);
    let _ = fs::remove_file(ads_path);
}

fn apply_autostart(enable: bool) {
    #[cfg(windows)]
    unsafe {
        let sub_key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
            .encode_utf16()
            .collect();
        let val_name: Vec<u16> = "MangNotch\0".encode_utf16().collect();

        if enable {
            if let Ok(exe_path) = std::env::current_exe() {
                let exe_str = exe_path.to_string_lossy().to_string();
                let launch_cmd = if exe_str.contains("WindowsApps") {
                    "explorer.exe shell:AppsFolder\\vestiapani.MangNotch_33rhz9prkc4x8!mangNotch"
                        .to_string()
                } else {
                    format!("\"{}\"", exe_str)
                };
                let data_wide: Vec<u16> = launch_cmd
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    sub_key.as_ptr(),
                    val_name.as_ptr(),
                    REG_SZ,
                    data_wide.as_ptr() as *const std::ffi::c_void,
                    (data_wide.len() * 2) as u32,
                );
            }
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, sub_key.as_ptr(), val_name.as_ptr());
        }
    }
}

fn show_control_center_safe(app: &AppHandle) {
    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(notch_win) = app_clone.get_webview_window("notch") {
            #[cfg(windows)]
            if let Some(hwnd) = get_hwnd(&notch_win) {
                set_hwnd_click_through(hwnd, true);
            }
        }
        let _ = app_clone.emit("notch-hover", false);

        if let Some(ctrl_win) = app_clone.get_webview_window("control") {
            let _ = ctrl_win.show();
            let _ = ctrl_win.unminimize();
            let _ = ctrl_win.set_always_on_top(true);
            let _ = ctrl_win.set_focus();
            let _ = ctrl_win.set_always_on_top(false);

            #[cfg(windows)]
            if let Some(hwnd) = get_hwnd(&ctrl_win) {
                unsafe {
                    let _ = SetForegroundWindow(hwnd);
                }
            }
        }
    });
}

#[cfg(windows)]
fn read_master_volume() -> VolumePayload {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(enumerator) =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
        {
            if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                if let Ok(endpoint) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    let level = endpoint.GetMasterVolumeLevelScalar().unwrap_or(0.5);
                    let muted = endpoint.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                    return VolumePayload {
                        volume: (level * 100.0).round().clamp(0.0, 100.0) as u32,
                        muted,
                    };
                }
            }
        }
    }
    VolumePayload {
        volume: 50,
        muted: false,
    }
}

#[cfg(windows)]
fn write_master_volume(volume: Option<u32>, mute: Option<bool>) -> VolumePayload {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(enumerator) =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
        {
            if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                if let Ok(endpoint) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    if let Some(v) = volume {
                        let clamped = v.min(100);
                        let scalar = (clamped as f32) / 100.0;
                        let _ = endpoint.SetMasterVolumeLevelScalar(scalar, std::ptr::null());
                        // Jika digeser sampai 0, wajib panggil SetMute(1) agar benar-benar mati di semua audio driver
                        if clamped == 0 {
                            let _ = endpoint.SetMute(BOOL(1), std::ptr::null());
                        } else {
                            let _ = endpoint.SetMute(BOOL(0), std::ptr::null());
                        }
                    }
                    if let Some(m) = mute {
                        let _ = endpoint.SetMute(BOOL(if m { 1 } else { 0 }), std::ptr::null());
                    }
                }
            }
        }
    }
    read_master_volume()
}

#[cfg(windows)]
fn get_hwnd(win: &WebviewWindow) -> Option<HWND> {
    win.hwnd().ok().map(|h| HWND(h.0 as *mut _))
}

#[cfg(windows)]
fn enumerate_monitors() -> Vec<MonitorItem> {
    let mut monitors: Vec<MonitorItem> = Vec::new();
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

#[cfg(windows)]
fn get_scaled_notch_size(notch_hwnd: HWND) -> (i32, i32) {
    let dpi = unsafe { GetDpiForWindow(notch_hwnd.0 as isize) };
    let scale = if dpi > 0 { (dpi as f64) / 96.0 } else { 1.0 };
    let phys_w = ((WINDOW_WIDTH as f64) * scale).round() as i32;
    let phys_h = (180.0 * scale).round() as i32;
    (phys_w, phys_h)
}

#[cfg(windows)]
fn place_notch_boot_peeking(notch_hwnd: HWND, cfg: &AppConfig) {
    let monitors = enumerate_monitors();
    let target = if cfg.use_primary_monitor {
        monitors.iter().find(|m| m.is_primary).or(monitors.first())
    } else {
        monitors.get(cfg.selected_monitor_index).or(monitors.first())
    };

    if let Some(mon) = target {
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
        }
    }
}

#[cfg(windows)]
fn position_notch_window(notch_hwnd: HWND, cfg: &AppConfig) {
    let monitors = enumerate_monitors();
    let target = if cfg.use_primary_monitor {
        monitors.iter().find(|m| m.is_primary).or(monitors.first())
    } else {
        monitors.get(cfg.selected_monitor_index).or(monitors.first())
    };

    if let Some(mon) = target {
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
        }
    }
}

#[cfg(windows)]
unsafe fn check_window_obstruction(
    fg: HWND,
    class_name: &str,
    fg_rect: &RECT,
    mon_rect: &RECT,
    notch_center_x: i32,
    resting_width: i32,
) -> bool {
    if IsIconic(fg).as_bool() {
        return false;
    }

    let notch_half_w = (resting_width.max(155) / 2) + 22;
    let n_left = notch_center_x - notch_half_w;
    let n_right = notch_center_x + notch_half_w;
    let n_bottom = mon_rect.top + 36;

    let horiz_overlap = fg_rect.left < n_right && fg_rect.right > n_left;
    let vert_overlap = fg_rect.top < n_bottom && fg_rect.bottom > mon_rect.top;
    if !horiz_overlap || !vert_overlap {
        return false;
    }

    let is_maximized_like = fg_rect.top <= mon_rect.top + 2
        && fg_rect.left <= mon_rect.left + 12
        && fg_rect.right >= mon_rect.right - 12;

    if !is_maximized_like {
        return true;
    }

    let is_browser = class_name.contains("Chrome_WidgetWin") || class_name.contains("Mozilla");
    if !is_browser {
        return false;
    }

    let sample_ys = [mon_rect.top + 14, mon_rect.top + 22, mon_rect.top + 28];
    let mut sx = n_left;
    while sx <= n_right {
        for &sy in &sample_ys {
            let lparam = LPARAM(((sy & 0xFFFF) << 16 | (sx & 0xFFFF)) as isize);
            let mut hit_res: usize = 0;
            let ok = SendMessageTimeoutW(
                fg,
                WM_NCHITTEST,
                WPARAM(0),
                lparam,
                SMTO_ABORTIFHUNG,
                25,
                Some(&mut hit_res as *mut usize),
            );
            if ok.0 != 0 && hit_res as u32 == 1 {
                return true;
            }
        }
        sx += 20;
    }

    false
}

#[cfg(windows)]
fn fetch_windows_media(last_key: &mut String, last_thumb: &mut String) -> MediaPayload {
    let Ok(manager) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .and_then(|op| op.get())
    else {
        return MediaPayload::default();
    };

    let Ok(session) = manager.GetCurrentSession() else {
        return MediaPayload::default();
    };

    let is_playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|st| st == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .unwrap_or(false);

    let (title, artist, props_opt) =
        match session.TryGetMediaPropertiesAsync().and_then(|op| op.get()) {
            Ok(props) => {
                let t = props.Title().map(|s| s.to_string()).unwrap_or_default();
                let a = props.Artist().map(|s| s.to_string()).unwrap_or_default();
                (t, a, Some(props))
            }
            Err(_) => (String::new(), String::new(), None),
        };

    if title.is_empty() {
        return MediaPayload::default();
    }

    let (position_sec, duration_sec) = match session.GetTimelineProperties() {
        Ok(tl) => {
            let pos = tl.Position().map(|t| t.Duration as f64 / 10_000_000.0).unwrap_or(0.0);
            let end = tl.EndTime().map(|t| t.Duration as f64 / 10_000_000.0).unwrap_or(0.0);
            (pos.max(0.0), end.max(0.0))
        }
        Err(_) => (0.0, 0.0),
    };

    let current_key = format!("{}::{}", title, artist);
    if *last_key != current_key {
        *last_key = current_key;
        *last_thumb = String::new();

        if let Some(props) = props_opt {
            if let Ok(thumb_ref) = props.Thumbnail() {
                if let Ok(stream) = thumb_ref.OpenReadAsync().and_then(|op| op.get()) {
                    let size = stream.Size().unwrap_or(0) as u32;
                    if size > 0 && size < 4_000_000 {
                        if let Ok(reader) = DataReader::CreateDataReader(&stream) {
                            if reader.LoadAsync(size).and_then(|op| op.get()).is_ok() {
                                let mut buf = vec![0u8; size as usize];
                                if reader.ReadBytes(&mut buf).is_ok() {
                                    *last_thumb =
                                        format!("data:image/png;base64,{}", BASE64.encode(&buf));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    MediaPayload {
        has_media: true,
        is_playing,
        title,
        artist,
        position_sec,
        duration_sec,
        thumbnail_b64: last_thumb.clone(),
    }
}

#[tauri::command]
fn get_app_logo() -> String {
    let bytes = include_bytes!("../icons/logo.png");
    format!("data:image/png;base64,{}", BASE64.encode(bytes))
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_install_channel() -> String {
    if let Ok(exe_path) = std::env::current_exe() {
        if exe_path.to_string_lossy().contains("WindowsApps") {
            return "msix".to_string();
        }
    }
    "standalone".to_string()
}

#[tauri::command]
fn trigger_update_window(app: AppHandle, is_forced: bool) {
    if is_forced {
        let app_clone = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(notch_win) = app_clone.get_webview_window("notch") {
                let _ = notch_win.hide();
            }
        });
    }
    show_control_center_safe(&app);
}

#[tauri::command]
fn open_config_folder() {
    let dir = get_config_dir().to_string_lossy().to_string();
    #[cfg(windows)]
    unsafe {
        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = dir.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = ShellExecuteW(
            HWND::default(),
            PCWSTR(op.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[tauri::command]
fn open_external_url(url: String) {
    #[cfg(windows)]
    unsafe {
        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = ShellExecuteW(
            HWND::default(),
            PCWSTR(op.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[tauri::command]
fn launch_update_and_exit(app: AppHandle, url: String) {
    open_external_url(url);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(400));
        app.exit(0);
    });
}

#[tauri::command]
fn get_monitors() -> Vec<MonitorItem> {
    enumerate_monitors()
}

#[tauri::command]
fn get_config(state: State<'_, SharedState>) -> AppConfig {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
fn save_full_config(app: AppHandle, state: State<'_, SharedState>, new_cfg: AppConfig) {
    let mut cfg = state.0.lock().unwrap();
    let active_w = cfg.active_width;
    let active_h = cfg.active_height;
    let resting_w = cfg.resting_width;

    *cfg = new_cfg;
    cfg.active_width = active_w;
    cfg.active_height = active_h;
    cfg.resting_width = resting_w;

    apply_autostart(cfg.start_with_windows);
    save_config_to_disk(&cfg);

    if NOTCH_POSITIONED.load(Ordering::SeqCst) {
        if let Some(notch_win) = app.get_webview_window("notch") {
            if let Some(hwnd) = get_hwnd(&notch_win) {
                position_notch_window(hwnd, &cfg);
            }
        }
    }

    let _ = app.emit("config-updated", cfg.clone());
}

#[tauri::command]
fn sync_widget_data(
    app: AppHandle,
    state: State<'_, SharedState>,
    todos: Vec<TodoItem>,
    water_current_ml: u32,
    last_water_date: Option<String>,
    shelf: Vec<ShelfItem>,
) {
    for item in &shelf {
        unblock_file_motw(&item.path);
    }
    let mut cfg = state.0.lock().unwrap();
    cfg.todos = todos;
    cfg.water_current_ml = water_current_ml;
    if let Some(d) = last_water_date {
        if !d.is_empty() {
            cfg.last_water_date = d;
        }
    }
    cfg.shelf = shelf;
    save_config_to_disk(&cfg);
    let _ = app.emit("config-updated", cfg.clone());
}

#[tauri::command]
fn get_system_volume() -> VolumePayload {
    #[cfg(windows)]
    {
        read_master_volume()
    }
    #[cfg(not(windows))]
    {
        VolumePayload {
            volume: 50,
            muted: false,
        }
    }
}

#[tauri::command]
fn set_system_volume(volume: Option<u32>, mute: Option<bool>) -> VolumePayload {
    #[cfg(windows)]
    {
        write_master_volume(volume, mute)
    }
    #[cfg(not(windows))]
    {
        VolumePayload {
            volume: volume.unwrap_or(50),
            muted: mute.unwrap_or(false),
        }
    }
}

#[tauri::command]
fn open_shelf_file(path: String) {
    unblock_file_motw(&path);
    std::env::set_var("SEE_MASK_NOZONECHECKS", "1");

    #[cfg(windows)]
    unsafe {
        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let file: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = ShellExecuteW(
            HWND::default(),
            PCWSTR(op.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[tauri::command]
fn reveal_shelf_file(path: String) {
    #[cfg(windows)]
    unsafe {
        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let explorer: Vec<u16> = "explorer.exe\0".encode_utf16().collect();
        let params: Vec<u16> = format!("/select,\"{}\"", path)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let _ = ShellExecuteW(
            HWND::default(),
            PCWSTR(op.as_ptr()),
            PCWSTR(explorer.as_ptr()),
            PCWSTR(params.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[tauri::command]
fn start_file_drag(app: AppHandle, path: String) {
    unblock_file_motw(&path);
    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        #[cfg(windows)]
        unsafe {
            let _ = OleInitialize(None);
            let _ = ReleaseCapture();

            let hwnd = app_clone
                .get_webview_window("notch")
                .and_then(|w| get_hwnd(&w))
                .unwrap_or_default();

            let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            if let Ok(shell_item) = SHCreateItemFromParsingName::<_, _, IShellItem>(
                PCWSTR(wide_path.as_ptr()),
                None::<&IBindCtx>,
            ) {
                if let Ok(data_obj) =
                    shell_item.BindToHandler::<_, IDataObject>(None::<&IBindCtx>, &BHID_DataObject)
                {
                    let _ = SHDoDragDrop(
                        hwnd,
                        &data_obj,
                        None::<&IDropSource>,
                        DROPEFFECT_COPY | DROPEFFECT_LINK,
                    );
                }
            }
        }
    });
}

#[tauri::command]
fn update_notch_hitbox(
    state: State<'_, SharedState>,
    width: i32,
    height: i32,
    resting_width: i32,
) {
    let mut cfg = state.0.lock().unwrap();
    cfg.active_width = width;
    cfg.active_height = height;
    cfg.resting_width = resting_width;
}

#[tauri::command]
fn notch_ready(app: AppHandle, state: State<'_, SharedState>) {
    if NOTCH_READY_CALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    let cfg = state.0.lock().unwrap().clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(NOTCH_REVEAL_DELAY_MS));
        let app2 = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(w) = app2.get_webview_window("notch") {
                #[cfg(windows)]
                if let Some(hwnd) = get_hwnd(&w) {
                    position_notch_window(hwnd, &cfg);
                    set_hwnd_click_through(hwnd, true);
                }
                NOTCH_POSITIONED.store(true, Ordering::SeqCst);
                let _ = app2.emit("notch-revealed", ());
            }
        });
    });
}

#[tauri::command]
fn media_action(action: String) {
    #[cfg(windows)]
    {
        if let Ok(manager) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .and_then(|op| op.get())
        {
            if let Ok(session) = manager.GetCurrentSession() {
                match action.as_str() {
                    "play_pause" => {
                        let _ = session.TryTogglePlayPauseAsync();
                    }
                    "next" => {
                        let _ = session.TrySkipNextAsync();
                    }
                    "prev" => {
                        let _ = session.TrySkipPreviousAsync();
                    }
                    _ => {}
                }
            }
        }
    }
}

#[tauri::command]
fn media_seek(position_sec: f64) {
    #[cfg(windows)]
    {
        if let Ok(manager) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .and_then(|op| op.get())
        {
            if let Ok(session) = manager.GetCurrentSession() {
                let ticks = (position_sec * 10_000_000.0) as i64;
                let _ = session.TryChangePlaybackPositionAsync(ticks);
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut show_ctrl_event: isize = 0;
    #[cfg(windows)]
    unsafe {
        let event_name: Vec<u16> = "Local\\MangNotchShowControlEvent\0"
            .encode_utf16()
            .collect();
        show_ctrl_event = CreateEventW(std::ptr::null(), 1, 0, event_name.as_ptr());

        let mutex_name: Vec<u16> = "Local\\MangNotchSingleInstanceMutex\0"
            .encode_utf16()
            .collect();
        if let Ok(handle) = CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr())) {
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = AllowSetForegroundWindow(ASFW_ANY);
                if show_ctrl_event != 0 {
                    let _ = SetEvent(show_ctrl_event);
                }
                let _ = CloseHandle(handle);
                std::process::exit(0);
            }
            let _ = std::mem::ManuallyDrop::new(handle);
        }
    }

    std::env::set_var("SEE_MASK_NOZONECHECKS", "1");
    let initial_cfg = load_config_from_disk();
    if initial_cfg.start_with_windows {
        apply_autostart(true);
    }

    let shared_cfg = Arc::new(Mutex::new(initial_cfg));
    let bg_cfg = shared_cfg.clone();

    tauri::Builder::default()
        .manage(SharedState(shared_cfg))
        .on_window_event(|window, event| {
            if window.label() == "control" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            let show_item =
                MenuItem::with_id(app, "show", "Buka MangNotch Control", true, None::<&str>)?;
            let quit_item =
                MenuItem::with_id(app, "quit", "Keluar dari MangNotch", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("MangNotch - Klik untuk membuka Control Center")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        show_control_center_safe(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_control_center_safe(tray.app_handle());
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _ = tray_builder.build(app)?;

            let notch_win = app.get_webview_window("notch").unwrap();
            let notch_hwnd = get_hwnd(&notch_win).unwrap();

            let ctrl_hwnd = app
                .get_webview_window("control")
                .and_then(|w| get_hwnd(&w))
                .unwrap_or_default();

            let init_cfg_snapshot = bg_cfg.lock().unwrap().clone();

            // 1. Pasang WndProc hook + Cloak + matikan border DWM + parkir di atas layar
            install_notch_wndproc(notch_hwnd);
            disable_window_transitions(notch_hwnd);
            set_window_cloak(notch_hwnd, true);
            strip_window_borders(notch_hwnd);
            place_notch_boot_peeking(notch_hwnd, &init_cfg_snapshot);

            // 2. Panggil show() sekali saat cloaked agar state internal Tauri tercatat visible
            let _ = notch_win.show();
            let _ = notch_win.set_ignore_cursor_events(true);

            // 3. Kunci WndProc & click-through Win32, lalu uncloak agar WebView2 merender
            place_notch_boot_peeking(notch_hwnd, &init_cfg_snapshot);
            set_hwnd_click_through(notch_hwnd, true);
            set_window_cloak(notch_hwnd, false);

            let app_handle = app.handle().clone();
            let raw_notch_hwnd = notch_hwnd.0 as isize;
            let raw_ctrl_hwnd = ctrl_hwnd.0 as isize;
            let fallback_dpi = notch_win.scale_factor().unwrap_or(1.0);

            thread::spawn(move || {
                #[cfg(windows)]
                unsafe {
                    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                }

                let hwnd = HWND(raw_notch_hwnd as *mut _);
                let ctrl_win_hwnd = HWND(raw_ctrl_hwnd as *mut _);
                let mut is_hovered = false;
                let mut is_hidden_by_fs = false;
                let mut is_obstructed = false;
                let mut outside_ticks: u32 = 0;

                let mut last_caps = unsafe { (GetKeyState(0x14) & 1) != 0 };
                let mut last_ac: u8 = 255;

                let init_vol = read_master_volume();
                let mut last_vol = init_vol.volume;
                let mut last_muted = init_vol.muted;

                let mut last_brightness: Option<u8> = read_screen_brightness();

                let bth_dll: Vec<u16> = "BluetoothAPIs.dll\0".encode_utf16().collect();
                let bth_mod = unsafe { LoadLibraryW(bth_dll.as_ptr()) };
                let init_bt = read_bluetooth_state(bth_mod);
                let mut last_bt_radio = init_bt.as_ref().map(|b| b.radio_on);
                let mut last_bt_connected = init_bt.as_ref().map(|b| b.connected_count);

                let mut tick_counter: u32 = 0;

                loop {
                    thread::sleep(Duration::from_millis(40));
                    tick_counter = tick_counter.wrapping_add(1);

                    // Fallback pengaman (~3 detik) kalau frontend tidak mengirim notch_ready
                    if tick_counter == 75 && !NOTCH_POSITIONED.swap(true, Ordering::SeqCst) {
                        NOTCH_READY_CALLED.store(true, Ordering::SeqCst);
                        let cfg = bg_cfg.lock().unwrap().clone();
                        position_notch_window(hwnd, &cfg);
                        set_hwnd_click_through(hwnd, true);
                        let _ = app_handle.emit("notch-revealed", ());
                    }

                    #[cfg(windows)]
                    unsafe {
                        if show_ctrl_event != 0
                            && WaitForSingleObject(show_ctrl_event, 0) == WAIT_OBJECT_0
                        {
                            let _ = ResetEvent(show_ctrl_event);
                            is_hovered = false;
                            outside_ticks = 0;
                            show_control_center_safe(&app_handle);
                        }
                    }

                    if !NOTCH_POSITIONED.load(Ordering::SeqCst) {
                        continue;
                    }

                    let cfg = bg_cfg.lock().unwrap().clone();

                    unsafe {
                        let mut win_rect = RECT::default();
                        let _ = GetWindowRect(hwnd, &mut win_rect);
                        let notch_center_x = (win_rect.left + win_rect.right) / 2;

                        let measured_w = (win_rect.right - win_rect.left) as f64;
                        let dpi_scale = if measured_w > 100.0 {
                            measured_w / (WINDOW_WIDTH as f64)
                        } else {
                            fallback_dpi
                        };

                        let fg = GetForegroundWindow();
                        let mut fg_is_same_mon = false;
                        let mut is_desktop = false;
                        let mut class_name = String::new();
                        let mut fg_rect = RECT::default();
                        let mut mi = MONITORINFO {
                            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                            ..Default::default()
                        };

                        if !fg.0.is_null() && fg != hwnd {
                            let mut class_buf = [0u16; 128];
                            let len = GetClassNameW(fg, &mut class_buf);
                            class_name = String::from_utf16_lossy(&class_buf[..len as usize]);

                            is_desktop = matches!(
                                class_name.as_str(),
                                "Progman"
                                    | "WorkerW"
                                    | "Shell_TrayWnd"
                                    | "Shell_SecondaryTrayWnd"
                            );

                            let notch_mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                            let fg_mon = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);

                            if notch_mon == fg_mon
                                && GetWindowRect(fg, &mut fg_rect).is_ok()
                                && GetMonitorInfoW(fg_mon, &mut mi).as_bool()
                            {
                                fg_is_same_mon = true;
                            }
                        }

                        // 1. Deteksi Aplikasi Fullscreen Murni
                        if cfg.hide_on_fullscreen {
                            if fg_is_same_mon && !is_desktop && fg != ctrl_win_hwnd {
                                let m = mi.rcMonitor;
                                let fullscreen = fg_rect.left == m.left
                                    && fg_rect.top == m.top
                                    && fg_rect.right == m.right
                                    && fg_rect.bottom == m.bottom;

                                if fullscreen && !is_hidden_by_fs {
                                    let _ = ShowWindow(hwnd, SW_HIDE);
                                    is_hidden_by_fs = true;
                                } else if !fullscreen && is_hidden_by_fs {
                                    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                                    is_hidden_by_fs = false;
                                }
                            } else if is_hidden_by_fs {
                                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                                is_hidden_by_fs = false;
                            }
                        } else if is_hidden_by_fs {
                            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                            is_hidden_by_fs = false;
                        }

                        if is_hidden_by_fs {
                            continue;
                        }

                        // 2. Smart Proximity Auto-Shrink
                        if tick_counter % 3 == 0 && !is_hovered {
                            let now_obstructed = if cfg.smart_proximity
                                && fg_is_same_mon
                                && !is_desktop
                                && fg != ctrl_win_hwnd
                            {
                                check_window_obstruction(
                                    fg,
                                    &class_name,
                                    &fg_rect,
                                    &mi.rcMonitor,
                                    notch_center_x,
                                    cfg.resting_width,
                                )
                            } else {
                                false
                            };

                            if now_obstructed != is_obstructed {
                                is_obstructed = now_obstructed;
                                let _ = app_handle.emit("notch-proximity", is_obstructed);
                            }
                        }

                        // 3A. System Peek: Caps Lock
                        let caps_now = (GetKeyState(0x14) & 1) != 0;
                        if caps_now != last_caps {
                            last_caps = caps_now;
                            if cfg.enable_peeks {
                                let _ = app_handle.emit(
                                    "system-peek",
                                    SystemPeekPayload {
                                        kind: "capslock".to_string(),
                                        active: caps_now,
                                        percent: 0,
                                        label: if caps_now {
                                            "Uppercase active".to_string()
                                        } else {
                                            "Uppercase off".to_string()
                                        },
                                    },
                                );
                            }
                        }

                        // 3B. System Peek: Master Volume & Mute
                        if tick_counter % 3 == 0 {
                            let cur_vol = read_master_volume();
                            if cur_vol.volume != last_vol || cur_vol.muted != last_muted {
                                last_vol = cur_vol.volume;
                                last_muted = cur_vol.muted;
                                let _ = app_handle.emit("volume-sync", cur_vol.clone());
                                if cfg.enable_peeks {
                                    let _ = app_handle.emit(
                                        "system-peek",
                                        SystemPeekPayload {
                                            kind: "volume".to_string(),
                                            active: !cur_vol.muted && cur_vol.volume > 0,
                                            percent: if cur_vol.muted {
                                                0
                                            } else {
                                                cur_vol.volume as u8
                                            },
                                            label: if cur_vol.muted || cur_vol.volume == 0 {
                                                "Audio Muted".to_string()
                                            } else {
                                                "System Master Audio".to_string()
                                            },
                                        },
                                    );
                                }
                            }
                        }

                        // 3C. System Peek: Screen Brightness
                        if tick_counter % 5 == 0 {
                            if let Some(br) = read_screen_brightness() {
                                if let Some(prev_br) = last_brightness {
                                    if br != prev_br && cfg.enable_peeks {
                                        let _ = app_handle.emit(
                                            "system-peek",
                                            SystemPeekPayload {
                                                kind: "brightness".to_string(),
                                                active: true,
                                                percent: br,
                                                label: "Display Backlight".to_string(),
                                            },
                                        );
                                    }
                                }
                                last_brightness = Some(br);
                            }
                        }

                        // 3D. System Peek: Battery & Charging
                        if tick_counter % 20 == 0 {
                            let mut sps = SYSTEM_POWER_STATUS::default();
                            if GetSystemPowerStatus(&mut sps).is_ok() {
                                let pct = if sps.BatteryLifePercent <= 100 {
                                    sps.BatteryLifePercent
                                } else {
                                    100
                                };
                                let ac = sps.ACLineStatus;
                                if last_ac != 255 && ac != last_ac && cfg.enable_peeks {
                                    let _ = app_handle.emit(
                                        "system-peek",
                                        SystemPeekPayload {
                                            kind: "charging".to_string(),
                                            active: ac == 1,
                                            percent: pct,
                                            label: if ac == 1 {
                                                "Power connected".to_string()
                                            } else {
                                                "Power unplugged".to_string()
                                            },
                                        },
                                    );
                                }
                                last_ac = ac;
                                let _ = app_handle.emit(
                                    "system-peek",
                                    SystemPeekPayload {
                                        kind: "battery_sync".to_string(),
                                        active: ac == 1,
                                        percent: pct,
                                        label: String::new(),
                                    },
                                );
                            }
                        }

                        // 3E. System Peek: Bluetooth
                        if tick_counter % 30 == 0 {
                            if let Some(bt) = read_bluetooth_state(bth_mod) {
                                let radio_changed =
                                    last_bt_radio.map(|r| r != bt.radio_on).unwrap_or(false);
                                let conn_changed = last_bt_connected
                                    .map(|c| c != bt.connected_count)
                                    .unwrap_or(false);

                                if (radio_changed || conn_changed) && cfg.enable_peeks {
                                    let label = if !bt.radio_on {
                                        "Adapter Turned Off".to_string()
                                    } else if bt.connected_count > 0 && !bt.device_name.is_empty() {
                                        bt.device_name.clone()
                                    } else if bt.connected_count > 0 {
                                        format!("{} Device Connected", bt.connected_count)
                                    } else {
                                        "Bluetooth Ready".to_string()
                                    };

                                    let _ = app_handle.emit(
                                        "system-peek",
                                        SystemPeekPayload {
                                            kind: "bluetooth".to_string(),
                                            active: bt.radio_on,
                                            percent: bt.connected_count as u8,
                                            label,
                                        },
                                    );
                                }

                                last_bt_radio = Some(bt.radio_on);
                                last_bt_connected = Some(bt.connected_count);
                            }
                        }

                        // 4. Smart Hover, Safe-Zone Lebar & Proteksi Jendela Control Center
                        let mut pt = POINT::default();
                        if GetCursorPos(&mut pt).is_ok() {
                            let effective_w = if is_hovered && cfg.interaction_mode != "click" {
                                cfg.active_width.max(420)
                            } else {
                                cfg.active_width
                            };
                            let effective_h = if is_hovered && cfg.interaction_mode != "click" {
                                cfg.active_height.max(140)
                            } else {
                                cfg.active_height
                            };

                            let is_expanded = effective_h >= 100;
                            let is_minimal = is_obstructed || effective_h <= 12;

                            let scale_px =
                                |value: i32| (value as f64 * dpi_scale).round() as i32;

                            let (half_w, hit_bottom) = if !is_hovered && is_minimal {
                                (scale_px(65), win_rect.top + scale_px(6))
                            } else if is_expanded {
                                (
                                    scale_px(effective_w / 2 + 55),
                                    win_rect.top + scale_px(effective_h + 40),
                                )
                            } else {
                                (
                                    scale_px(effective_w / 2 + 14),
                                    win_rect.top + scale_px(effective_h.max(32) + 8),
                                )
                            };

                            let hit_left = notch_center_x - half_w;
                            let hit_right = notch_center_x + half_w;
                            let hit_top = win_rect.top - scale_px(10);

                            let mut inside = pt.x >= hit_left
                                && pt.x <= hit_right
                                && pt.y >= hit_top
                                && pt.y <= hit_bottom;

                            if !ctrl_win_hwnd.0.is_null()
                                && IsWindowVisible(ctrl_win_hwnd.0 as isize) != 0
                                && !IsIconic(ctrl_win_hwnd).as_bool()
                            {
                                let mut ctrl_rect = RECT::default();
                                if GetWindowRect(ctrl_win_hwnd, &mut ctrl_rect).is_ok() {
                                    let in_ctrl = pt.x >= ctrl_rect.left
                                        && pt.x <= ctrl_rect.right
                                        && pt.y >= ctrl_rect.top
                                        && pt.y <= ctrl_rect.bottom;
                                    if in_ctrl && pt.y > win_rect.top + scale_px(34) {
                                        inside = false;
                                    }
                                }
                            }

                            if inside {
                                outside_ticks = 0;
                                if !is_hovered {
                                    is_hovered = true;
                                    if cfg.interaction_mode != "click" {
                                        if let Ok(mut c) = bg_cfg.lock() {
                                            c.active_width = c.active_width.max(420);
                                            c.active_height = c.active_height.max(140);
                                        }
                                    }
                                    set_hwnd_click_through(hwnd, false);
                                    let _ = app_handle.emit("notch-hover", true);
                                }
                            } else if is_hovered {
                                outside_ticks += 1;
                                let required_ticks = if is_expanded { 7 } else { 3 };
                                if outside_ticks >= required_ticks {
                                    is_hovered = false;
                                    outside_ticks = 0;
                                    if let Ok(mut c) = bg_cfg.lock() {
                                        c.active_width = c.resting_width.max(155);
                                        c.active_height = 32;
                                    }
                                    set_hwnd_click_through(hwnd, true);
                                    let _ = app_handle.emit("notch-hover", false);
                                }
                            }
                        }
                    }
                }
            });

            let media_app_handle = app.handle().clone();
            thread::spawn(move || {
                #[cfg(windows)]
                unsafe {
                    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                }
                let mut last_key = String::new();
                let mut last_thumb = String::new();

                loop {
                    thread::sleep(Duration::from_millis(600));
                    #[cfg(windows)]
                    {
                        let payload = fetch_windows_media(&mut last_key, &mut last_thumb);
                        let _ = media_app_handle.emit("media-update", payload);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_logo,
            get_app_version,
            get_install_channel,
            trigger_update_window,
            open_config_folder,
            open_external_url,
            launch_update_and_exit,
            get_monitors,
            get_config,
            save_full_config,
            sync_widget_data,
            get_system_volume,
            set_system_volume,
            open_shelf_file,
            reveal_shelf_file,
            start_file_drag,
            update_notch_hitbox,
            notch_ready,
            media_action,
            media_seek
        ])
        .run(tauri::generate_context!())
        .expect("error while running MangNotch");
}