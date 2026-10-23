use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
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
        Foundation::{BOOL, HWND, LPARAM, POINT, RECT, WPARAM},
        Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO,
            MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
        },
        System::{
            Com::{CoInitializeEx, IBindCtx, IDataObject, COINIT_MULTITHREADED},
            Ole::{IDropSource, OleInitialize, DROPEFFECT_COPY, DROPEFFECT_LINK},
            Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
        },
        UI::{
            Input::KeyboardAndMouse::{GetKeyState, ReleaseCapture},
            Shell::{
                BHID_DataObject, IShellItem, SHCreateItemFromParsingName, SHDoDragDrop,
                ShellExecuteW,
            },
            WindowsAndMessaging::{
                GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
                IsIconic, SendMessageTimeoutW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
                GWL_EXSTYLE, HWND_TOPMOST, SMTO_ABORTIFHUNG, SWP_NOACTIVATE, SWP_NOSIZE,
                SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, SW_SHOWNORMAL, WM_NCHITTEST,
                WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            },
        },
    },
};

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
#[serde(default)]
pub struct AppConfig {
    pub use_primary_monitor: bool,
    pub selected_monitor_index: usize,
    pub hide_on_fullscreen: bool,
    pub smart_proximity: bool,
    pub start_with_windows: bool,
    pub resting_mode: String,
    pub mascot_style: String,
    pub theme_preset: String,
    pub enable_peeks: bool,
    pub enable_sound: bool,
    pub water_tap_sound: bool,
    pub water_reminder_sound: bool,
    pub water_goal_ml: u32,
    pub water_cup_ml: u32,
    pub water_current_ml: u32,
    pub water_interval_min: u32,
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
            resting_mode: "pet".to_string(),
            mascot_style: "neko".to_string(),
            theme_preset: "dusk".to_string(),
            enable_peeks: true,
            enable_sound: true,
            water_tap_sound: true,
            water_reminder_sound: true,
            water_goal_ml: 1500,
            water_cup_ml: 250,
            water_current_ml: 0,
            water_interval_min: 45,
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
}

pub struct SharedState(pub Arc<Mutex<AppConfig>>);

const WINDOW_WIDTH: i32 = 600;

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
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_str = exe_path.to_string_lossy().to_string();
        if enable {
            let _ = Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "MangNotch",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &format!("\"{}\"", exe_str),
                    "/f",
                ])
                .creation_flags(0x08000000)
                .status();
        } else {
            let _ = Command::new("reg")
                .args([
                    "delete",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "MangNotch",
                    "/f",
                ])
                .creation_flags(0x08000000)
                .status();
        }
    }
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
fn position_notch_window(notch_hwnd: HWND, cfg: &AppConfig) {
    let monitors = enumerate_monitors();
    let target = if cfg.use_primary_monitor {
        monitors.iter().find(|m| m.is_primary).or(monitors.first())
    } else {
        monitors.get(cfg.selected_monitor_index).or(monitors.first())
    };

    if let Some(mon) = target {
        let x = mon.x + (mon.width - WINDOW_WIDTH) / 2;
        let y = mon.y;
        unsafe {
            let _ = ShowWindow(notch_hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                notch_hwnd,
                HWND_TOPMOST,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }
}

/// Mengecek apakah tab browser / tombol '+' (New Tab) / jendela aktif menyentuh area Notch
/// (Mendukung lebar dinamis baik saat mode Maskot 155px maupun mode Idle-Lyrics 340px)
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

    // Pindai 3 ketinggian Y sekaligus (14px, 22px, 28px) agar mengenai kotak Tab maupun lingkaran tombol '+' Chrome
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
            // HTCLIENT (1) berarti titik tersebut adalah Tab aktif atau tombol '+' (bukan ruang kosong titlebar)
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
fn open_config_folder() {
    let dir = get_config_dir();
    let _ = Command::new("explorer").arg(dir).spawn();
}

#[tauri::command]
fn open_external_url(url: String) {
    #[cfg(windows)]
    unsafe {
        let op: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
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
    let prev_autostart = cfg.start_with_windows;
    let active_w = cfg.active_width;
    let active_h = cfg.active_height;
    let resting_w = cfg.resting_width;

    *cfg = new_cfg;
    cfg.active_width = active_w;
    cfg.active_height = active_h;
    cfg.resting_width = resting_w;

    if prev_autostart != cfg.start_with_windows {
        apply_autostart(cfg.start_with_windows);
    }

    save_config_to_disk(&cfg);

    if let Some(notch_win) = app.get_webview_window("notch") {
        if let Some(hwnd) = get_hwnd(&notch_win) {
            position_notch_window(hwnd, &cfg);
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
    shelf: Vec<ShelfItem>,
) {
    for item in &shelf {
        unblock_file_motw(&item.path);
    }
    let mut cfg = state.0.lock().unwrap();
    cfg.todos = todos;
    cfg.water_current_ml = water_current_ml;
    cfg.shelf = shelf;
    save_config_to_disk(&cfg);
    let _ = app.emit("config-updated", cfg.clone());
}

#[tauri::command]
fn open_shelf_file(path: String) {
    unblock_file_motw(&path);
    std::env::set_var("SEE_MASK_NOZONECHECKS", "1");

    #[cfg(windows)]
    unsafe {
        let op: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
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
    let _ = Command::new("explorer")
        .arg(format!("/select,{}", path))
        .spawn();
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
    std::env::set_var("SEE_MASK_NOZONECHECKS", "1");
    let initial_cfg = load_config_from_disk();
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
                        if let Some(w) = app.get_webview_window("control") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
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
                        if let Some(w) = tray.app_handle().get_webview_window("control") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _ = tray_builder.build(app)?;

            let notch_win = app.get_webview_window("notch").unwrap();
            let _ = notch_win.show();
            let notch_hwnd = get_hwnd(&notch_win).unwrap();

            unsafe {
                let ex_style = GetWindowLongPtrW(notch_hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(
                    notch_hwnd,
                    GWL_EXSTYLE,
                    ex_style | (WS_EX_NOACTIVATE.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize),
                );
            }

            {
                let cfg = bg_cfg.lock().unwrap();
                position_notch_window(notch_hwnd, &cfg);
            }

            let app_handle = app.handle().clone();
            let raw_hwnd = notch_hwnd.0 as isize;

            thread::spawn(move || {
                let hwnd = HWND(raw_hwnd as *mut _);
                let mut is_hovered = false;
                let mut is_hidden_by_fs = false;
                let mut is_obstructed = false;
                let mut last_caps = unsafe { (GetKeyState(0x14) & 1) != 0 };
                let mut last_ac: u8 = 255;
                let mut tick_counter: u32 = 0;

                loop {
                    thread::sleep(Duration::from_millis(40));
                    tick_counter = tick_counter.wrapping_add(1);
                    let cfg = bg_cfg.lock().unwrap().clone();

                    unsafe {
                        let mut win_rect = RECT::default();
                        let _ = GetWindowRect(hwnd, &mut win_rect);
                        let notch_center_x = (win_rect.left + win_rect.right) / 2;

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
                            if fg_is_same_mon && !is_desktop {
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

                        // 2. Smart Proximity Auto-Shrink (memindai lebar penuh resting_width)
                        if tick_counter % 3 == 0 && !is_hovered {
                            let now_obstructed =
                                if cfg.smart_proximity && fg_is_same_mon && !is_desktop {
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

                        // 3. System Peeks
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
                                    },
                                );
                            }
                        }

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
                                    },
                                );
                            }
                        }

                        // 4. Smart Hover & Click-Through
                        // Saat Notch sedang mengecil ke mode minimal (is_obstructed / height <= 12),
                        // batasi sensor hover hanya di 4px paling atas layar agar tombol '+' & tab Chrome di bawahnya bebas diklik!
                        let mut pt = POINT::default();
                        if GetCursorPos(&mut pt).is_ok() {
                            let is_expanded = cfg.active_height >= 100;
                            let is_minimal = is_obstructed || cfg.active_height <= 12;

                            let (half_w, hit_bottom) = if !is_hovered && is_minimal {
                                (60, win_rect.top + 4)
                            } else if is_expanded {
                                (cfg.active_width / 2 + 12, win_rect.top + cfg.active_height + 10)
                            } else {
                                (cfg.active_width / 2, win_rect.top + cfg.active_height)
                            };

                            let hit_left = notch_center_x - half_w;
                            let hit_right = notch_center_x + half_w;
                            let hit_top = win_rect.top;

                            let inside = pt.x >= hit_left
                                && pt.x <= hit_right
                                && pt.y >= hit_top
                                && pt.y <= hit_bottom;

                            if let Some(w) = app_handle.get_webview_window("notch") {
                                if inside && !is_hovered {
                                    is_hovered = true;
                                    let _ = w.set_ignore_cursor_events(false);
                                    let _ = app_handle.emit("notch-hover", true);
                                } else if !inside && is_hovered {
                                    is_hovered = false;
                                    let _ = w.set_ignore_cursor_events(true);
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
            open_config_folder,
            open_external_url,
            get_monitors,
            get_config,
            save_full_config,
            sync_widget_data,
            open_shelf_file,
            reveal_shelf_file,
            start_file_drag,
            update_notch_hitbox,
            media_action,
            media_seek
        ])
        .run(tauri::generate_context!())
        .expect("error while running MangNotch");
}