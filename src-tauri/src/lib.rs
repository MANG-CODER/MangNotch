mod config;
mod extra_peeks;
mod media;
mod poller;
mod shelf;
mod state;
mod window_manager;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use config::{
    apply_autostart, get_config_dir, load_config_from_disk, resolve_language, save_config_to_disk,
    AppConfig, MonitorItem, ShelfItem, TodoItem,
};
use media::MediaPayload;
use serde::Serialize;
use state::{LyricsSyncState, SharedState};
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, DragDropEvent, Emitter, Manager, State, WebviewWindow, WindowEvent,
};
use window_manager::{
    enumerate_monitors, reconcile_notch_windows, set_text_input_for_window,
    show_control_center_safe, NOTCH_REVEAL_DELAY_MS,
};

#[cfg(windows)]
use config::{is_packaged_app, query_packaged_startup, wide};
#[cfg(windows)]
use window_manager::{apply_control_backdrop, get_hwnd, position_notch_on_monitor};
#[cfg(windows)]
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HWND,
        Media::Audio::{
            eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
            MMDeviceEnumerator,
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    },
};

#[derive(Clone, Serialize)]
struct VolumeResponse {
    volume: u32,
    muted: bool,
}

#[derive(Clone, Serialize)]
struct HttpResponse {
    status: u32,
    body: String,
}

#[cfg(windows)]
#[link(name = "wininet")]
extern "system" {
    fn InternetOpenW(
        lpsz_agent: *const u16,
        dw_access_type: u32,
        lpsz_proxy: *const u16,
        lpsz_proxy_bypass: *const u16,
        dw_flags: u32,
    ) -> isize;
    fn InternetOpenUrlW(
        h_internet: isize,
        lpsz_url: *const u16,
        lpsz_headers: *const u16,
        dw_headers_length: u32,
        dw_flags: u32,
        dw_context: usize,
    ) -> isize;
    fn HttpQueryInfoW(
        h_request: isize,
        dw_info_level: u32,
        lpv_buffer: *mut std::ffi::c_void,
        lpdw_buffer_length: *mut u32,
        lpdw_index: *mut u32,
    ) -> i32;
    fn InternetReadFile(
        h_file: isize,
        lp_buffer: *mut u8,
        dw_number_of_bytes_to_read: u32,
        lpdw_number_of_bytes_read: *mut u32,
    ) -> i32;
    fn InternetCloseHandle(h_internet: isize) -> i32;
}

#[cfg(windows)]
fn shell_open(target: &str) {
    unsafe {
        let op = wide("open");
        let file = wide(target);
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

fn tray_labels(lang: &str) -> (&'static str, &'static str) {
    match resolve_language(lang).as_str() {
        "id" => ("Buka Control Center", "Keluar MangNotch"),
        "ja" => ("コントロールセンターを開く", "MangNotch を終了"),
        "ko" => ("제어 센터 열기", "MangNotch 종료"),
        "zh-CN" => ("打开控制中心", "退出 MangNotch"),
        "zh-TW" => ("開啟控制中心", "結束 MangNotch"),
        "ms" => ("Buka Pusat Kawalan", "Keluar MangNotch"),
        "es" => ("Abrir Centro de control", "Salir de MangNotch"),
        "de" => ("Kontrollzentrum öffnen", "MangNotch beenden"),
        "nl" => ("Bedieningspaneel openen", "MangNotch afsluiten"),
        "fr" => ("Ouvrir le Centre de contrôle", "Quitter MangNotch"),
        "ru" => ("Открыть Центр управления", "Выход из MangNotch"),
        _ => ("Open Control Center", "Quit MangNotch"),
    }
}

fn build_tray_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let (lbl_open, lbl_quit) = tray_labels(lang);
    let open_item = MenuItem::with_id(app, "open_ctrl", lbl_open, true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", lbl_quit, true, None::<&str>)?;
    Menu::with_items(app, &[&open_item, &sep, &quit_item])
}

fn apply_tray_language(app: &AppHandle, lang: &str) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Ok(menu) = build_tray_menu(app, lang) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

// === CONFIG & SYSTEM COMMANDS ===

#[tauri::command]
fn get_config(state: State<'_, SharedState>) -> AppConfig {
    state.0.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_full_config(app: AppHandle, state: State<'_, SharedState>, mut new_cfg: AppConfig) {
    new_cfg.sanitize();
    let (prev_autostart, prev_mon_mode, prev_mon_list, prev_backdrop) = {
        let mut cfg = state.0.config.lock().unwrap();
        let calendar_json = std::mem::take(&mut cfg.calendar_json);
        let prev_auto = cfg.start_with_windows;
        let prev_mode = cfg.monitor_mode.clone();
        let prev_list = cfg.selected_monitors.clone();
        let prev_bd = cfg.bg_backdrop.clone();

        *cfg = new_cfg;
        cfg.calendar_json = calendar_json;
        save_config_to_disk(&cfg);
        (prev_auto, prev_mode, prev_list, prev_bd)
    };

    let snapshot = state.0.config.lock().unwrap().clone();
    state.sync_runtime(&snapshot);

    if snapshot.start_with_windows != prev_autostart {
        apply_autostart(snapshot.start_with_windows);
    }
    apply_tray_language(&app, &snapshot.language);

    if snapshot.bg_backdrop != prev_backdrop {
        let app_c = app.clone();
        let mode = snapshot.bg_backdrop.clone();
        let _ = app.run_on_main_thread(move || {
            #[cfg(windows)]
            if let Some(ctrl) = app_c.get_webview_window("control") {
                if let Some(hwnd) = get_hwnd(&ctrl) {
                    apply_control_backdrop(hwnd, &mode);
                }
            }
        });
    }

    if snapshot.monitor_mode != prev_mon_mode || snapshot.selected_monitors != prev_mon_list {
        let app_r = app.clone();
        let st_r = state.inner().clone();
        let _ = app.run_on_main_thread(move || {
            reconcile_notch_windows(&app_r, &st_r);
        });
    }

    let _ = app.emit("config-updated", snapshot);
}

#[tauri::command]
fn get_monitors() -> Vec<MonitorItem> {
    enumerate_monitors()
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_app_logo() -> String {
    let bytes = include_bytes!("../icons/128x128.png");
    format!("data:image/png;base64,{}", BASE64.encode(bytes))
}

#[tauri::command]
fn get_install_channel() -> String {
    #[cfg(windows)]
    {
        if is_packaged_app() {
            return "msix".to_string();
        }
    }
    "exe".to_string()
}

#[tauri::command]
fn open_external_url(url: String) {
    #[cfg(windows)]
    shell_open(&url);
    #[cfg(not(windows))]
    let _ = url;
}

#[tauri::command]
fn open_config_folder() {
    let dir = get_config_dir();
    #[cfg(windows)]
    shell_open(&dir.to_string_lossy());
    #[cfg(not(windows))]
    let _ = dir;
}

#[tauri::command]
fn launch_update_and_exit(app: AppHandle, url: String) {
    #[cfg(windows)]
    shell_open(&url);
    #[cfg(not(windows))]
    let _ = url;
    app.exit(0);
}

#[tauri::command]
fn trigger_update_window(app: AppHandle, state: State<'_, SharedState>, is_forced: bool) {
    show_control_center_safe(&app, state.inner());
    if is_forced {
        if let Some(w) = app.get_webview_window("control") {
            let _ = w.set_always_on_top(true);
        }
    }
}

// === NOTCH WINDOW & WIDGET SYNC COMMANDS ===

#[tauri::command]
fn notch_ready(app: AppHandle, window: WebviewWindow, state: State<'_, SharedState>) {
    let label = window.label().to_string();
    let mon_index = {
        let Ok(mut map) = state.0.notches.lock() else {
            return;
        };
        let Some(inst) = map.get_mut(&label) else {
            return;
        };
        if inst.ready_called {
            return;
        }
        inst.ready_called = true;
        inst.monitor_index
    };

    let app_c = app.clone();
    let st_c = state.inner().clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(NOTCH_REVEAL_DELAY_MS));
        let app_m = app_c.clone();
        let _ = app_c.run_on_main_thread(move || {
            let monitors = enumerate_monitors();
            let mon_opt = monitors
                .iter()
                .find(|m| m.index == mon_index)
                .or_else(|| monitors.first());
            if let (Some(mon), Some(win)) = (mon_opt, app_m.get_webview_window(&label)) {
                #[cfg(windows)]
                if let Some(hwnd) = get_hwnd(&win) {
                    position_notch_on_monitor(hwnd, mon);
                }
                if let Ok(mut map) = st_c.0.notches.lock() {
                    if let Some(inst) = map.get_mut(&label) {
                        inst.positioned = true;
                    }
                }
                let _ = app_m.emit_to(&label, "notch-revealed", ());
            }
        });
    });
}

#[tauri::command]
fn update_notch_hitbox(
    window: WebviewWindow,
    state: State<'_, SharedState>,
    width: i32,
    height: i32,
    resting_width: i32,
) {
    let label = window.label();
    if let Ok(mut map) = state.0.notches.lock() {
        if let Some(inst) = map.get_mut(label) {
            inst.hitbox.active_width = width;
            inst.hitbox.active_height = height;
            inst.hitbox.resting_width = resting_width;
        }
    }
}

#[tauri::command]
fn set_notch_text_input(app: AppHandle, window: WebviewWindow, active: bool) {
    set_text_input_for_window(&app, window.label().to_string(), active);
}

#[tauri::command]
fn sync_widget_data(
    app: AppHandle,
    state: State<'_, SharedState>,
    todos: Vec<TodoItem>,
    water_current_ml: u32,
    last_water_date: String,
    shelf: Vec<ShelfItem>,
) {
    let snapshot = {
        let mut cfg = state.0.config.lock().unwrap();
        cfg.todos = todos;
        cfg.water_current_ml = water_current_ml;
        cfg.last_water_date = last_water_date;
        cfg.shelf = shelf;
        save_config_to_disk(&cfg);
        cfg.clone()
    };
    let _ = app.emit("config-updated", snapshot);
}

#[tauri::command]
fn set_lyrics_pinned(app: AppHandle, state: State<'_, SharedState>, pinned: bool) {
    let snapshot = {
        let mut cfg = state.0.config.lock().unwrap();
        cfg.lyrics_pinned_open = pinned;
        save_config_to_disk(&cfg);
        cfg.clone()
    };
    let _ = app.emit("config-updated", snapshot);
}

#[tauri::command]
fn set_calendar_data(app: AppHandle, state: State<'_, SharedState>, json: String) {
    {
        let mut cfg = state.0.config.lock().unwrap();
        cfg.calendar_json = json.clone();
        save_config_to_disk(&cfg);
    }
    let _ = app.emit("calendar-updated", json);
}

// === MEDIA & VOLUME COMMANDS ===

#[tauri::command]
fn get_current_media(state: State<'_, SharedState>) -> MediaPayload {
    state.0.last_media.lock().unwrap().clone()
}

#[tauri::command]
fn media_action(action: String) {
    media::perform_media_action(&action);
}

#[tauri::command]
fn media_seek(app: AppHandle, position_sec: f64) {
    media::perform_media_seek(position_sec);
    let _ = app.emit("media-seek-sync", position_sec);
}

#[tauri::command]
fn get_system_volume() -> VolumeResponse {
    #[cfg(windows)]
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            if let Ok(dev) = en.GetDefaultAudioEndpoint(eRender, eConsole) {
                if let Ok(vol) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    let v = vol.GetMasterVolumeLevelScalar().unwrap_or(0.5);
                    let m = vol.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                    return VolumeResponse {
                        volume: (v * 100.0).round() as u32,
                        muted: m,
                    };
                }
            }
        }
    }
    VolumeResponse {
        volume: 50,
        muted: false,
    }
}

#[tauri::command]
fn set_system_volume(volume: Option<u32>, mute: Option<bool>) -> VolumeResponse {
    #[cfg(windows)]
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            if let Ok(dev) = en.GetDefaultAudioEndpoint(eRender, eConsole) {
                if let Ok(vol) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    if let Some(v) = volume {
                        let clamped = (v.min(100) as f32) / 100.0;
                        let _ = vol.SetMasterVolumeLevelScalar(clamped, std::ptr::null());
                    }
                    if let Some(m) = mute {
                        let _ = vol.SetMute(m, std::ptr::null());
                    }
                    let cur_v = vol.GetMasterVolumeLevelScalar().unwrap_or(0.5);
                    let cur_m = vol.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                    return VolumeResponse {
                        volume: (cur_v * 100.0).round() as u32,
                        muted: cur_m,
                    };
                }
            }
        }
    }
    #[cfg(not(windows))]
    let _ = (volume, mute);
    VolumeResponse {
        volume: 50,
        muted: false,
    }
}

// === LYRICS HTTP & MULTI-MONITOR SHARED SYNC COMMANDS ===

#[tauri::command]
async fn lyrics_http_get(url: String) -> HttpResponse {
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        unsafe {
            const INTERNET_OPEN_TYPE_PRECONFIG: u32 = 0;
            const INTERNET_FLAG_RELOAD: u32 = 0x8000_0000;
            const INTERNET_FLAG_NO_CACHE_WRITE: u32 = 0x0400_0000;
            const INTERNET_FLAG_SECURE: u32 = 0x0080_0000;
            const HTTP_QUERY_STATUS_CODE: u32 = 19;
            const HTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;

            let agent = wide("MangNotch/2.0 (Windows)");
            let h_net = InternetOpenW(
                agent.as_ptr(),
                INTERNET_OPEN_TYPE_PRECONFIG,
                std::ptr::null(),
                std::ptr::null(),
                0,
            );
            if h_net == 0 {
                return HttpResponse {
                    status: 0,
                    body: String::new(),
                };
            }

            let url_w = wide(&url);
            let hdr_w = wide("Accept: application/json\r\n");
            let mut flags = INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE;
            if url.starts_with("https://") {
                flags |= INTERNET_FLAG_SECURE;
            }
            let h_url = InternetOpenUrlW(
                h_net,
                url_w.as_ptr(),
                hdr_w.as_ptr(),
                (hdr_w.len().saturating_sub(1)) as u32,
                flags,
                0,
            );
            if h_url == 0 {
                let _ = InternetCloseHandle(h_net);
                return HttpResponse {
                    status: 0,
                    body: String::new(),
                };
            }

            let mut status: u32 = 0;
            let mut status_len: u32 = std::mem::size_of::<u32>() as u32;
            let _ = HttpQueryInfoW(
                h_url,
                HTTP_QUERY_STATUS_CODE | HTTP_QUERY_FLAG_NUMBER,
                &mut status as *mut _ as *mut std::ffi::c_void,
                &mut status_len,
                std::ptr::null_mut(),
            );

            let mut data = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let mut read: u32 = 0;
                if InternetReadFile(h_url, buf.as_mut_ptr(), buf.len() as u32, &mut read) == 0
                    || read == 0
                {
                    break;
                }
                data.extend_from_slice(&buf[..read as usize]);
                if data.len() > 2_000_000 {
                    break;
                }
            }

            let _ = InternetCloseHandle(h_url);
            let _ = InternetCloseHandle(h_net);

            HttpResponse {
                status,
                body: String::from_utf8_lossy(&data).to_string(),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = url;
            HttpResponse {
                status: 0,
                body: String::new(),
            }
        }
    })
    .await
    .unwrap_or(HttpResponse {
        status: 0,
        body: String::new(),
    })
}

#[tauri::command]
fn get_shared_lyrics(state: State<'_, SharedState>) -> LyricsSyncState {
    state.0.lyrics.lock().unwrap().clone()
}

#[tauri::command]
fn claim_lyrics_fetch(state: State<'_, SharedState>, track_key: String) -> String {
    let Ok(mut lyr) = state.0.lyrics.lock() else {
        return "fetch".to_string();
    };
    if lyr.track_key == track_key {
        if lyr.payload.is_some() {
            return "cached".to_string();
        }
        if lyr.fetching {
            return "wait".to_string();
        }
    }
    lyr.track_key = track_key;
    lyr.fetching = true;
    lyr.payload = None;
    "fetch".to_string()
}

#[tauri::command]
fn publish_lyrics_data(
    app: AppHandle,
    state: State<'_, SharedState>,
    track_key: String,
    payload: Option<serde_json::Value>,
    done: bool,
) {
    let (emit_val, emit_empty) = {
        let Ok(mut lyr) = state.0.lyrics.lock() else {
            return;
        };
        if lyr.track_key != track_key {
            return;
        }
        if done {
            lyr.fetching = false;
        }
        if let Some(p) = payload {
            lyr.payload = Some(p.clone());
            (Some(p), false)
        } else {
            (None, done && lyr.payload.is_none())
        }
    };
    if let Some(p) = emit_val {
        let _ = app.emit("lyrics-data-sync", p);
    } else if emit_empty {
        let _ = app.emit(
            "lyrics-data-sync",
            serde_json::json!({ "track_key": track_key, "empty": true }),
        );
    }
}

#[tauri::command]
fn sync_lyrics_ui(
    app: AppHandle,
    state: State<'_, SharedState>,
    is_open: Option<bool>,
    offset_sec: Option<f64>,
) {
    let snapshot = {
        let Ok(mut lyr) = state.0.lyrics.lock() else {
            return;
        };
        if let Some(open) = is_open {
            lyr.is_open = open;
        }
        if let Some(off) = offset_sec {
            lyr.offset_sec = off;
        }
        serde_json::json!({
            "is_open": lyr.is_open,
            "offset_sec": lyr.offset_sec,
        })
    };
    let _ = app.emit("lyrics-ui-sync", snapshot);
}

// === FILE SHELF COMMANDS ===

#[tauri::command]
fn prepare_shelf_files(paths: Vec<String>) -> Vec<ShelfItem> {
    paths
        .iter()
        .filter_map(|p| shelf::enrich_shelf_item(p))
        .collect()
}

#[tauri::command]
fn pick_shelf_files() -> Vec<ShelfItem> {
    shelf::pick_files()
}

#[tauri::command]
fn paste_clipboard_to_shelf() -> Vec<ShelfItem> {
    shelf::read_clipboard_to_shelf_paths()
        .iter()
        .filter_map(|p| shelf::enrich_shelf_item(p))
        .collect()
}

#[tauri::command]
fn save_clipboard_to_shelf(
    image_b64: Option<String>,
    text_content: Option<String>,
) -> Option<ShelfItem> {
    shelf::save_clipboard(image_b64, text_content)
}

#[tauri::command]
fn open_shelf_file(path: String) {
    shelf::open_file(&path);
}

#[tauri::command]
fn reveal_shelf_file(path: String) {
    shelf::reveal_file(&path);
}

#[tauri::command]
fn start_file_drag(app: AppHandle, window: WebviewWindow, paths: Vec<String>) {
    shelf::start_drag_out(app, window.label().to_string(), paths);
}

// === APP ENTRY POINT ===

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let mut cfg = load_config_from_disk();
            #[cfg(windows)]
            if is_packaged_app() {
                if let Some(enabled) = query_packaged_startup() {
                    cfg.start_with_windows = enabled;
                }
            }
            let lang = cfg.language.clone();
            let shared_state = SharedState::new(cfg);
            app.manage(shared_state.clone());

            // System Tray
            let menu = build_tray_menu(app.handle(), &lang)?;
            let mut tray_builder = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .tooltip("MangNotch")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open_ctrl" => {
                        let st = app.state::<SharedState>();
                        show_control_center_safe(app, st.inner());
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
                        let app = tray.app_handle();
                        let st = app.state::<SharedState>();
                        show_control_center_safe(app, st.inner());
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _ = tray_builder.build(app)?;

            reconcile_notch_windows(app.handle(), &shared_state);
            poller::start_poller(app.handle().clone(), shared_state.clone());
            media::spawn_media_poller(app.handle().clone(), shared_state.clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label().to_string();
            if !label.starts_with("notch-") {
                return;
            }
            if let WindowEvent::DragDrop(drag_ev) = event {
                #[cfg(debug_assertions)]
                eprintln!("[shelf-drag] {:?} -> {}", drag_ev, label);

                let app = window.app_handle();
                let state = app.state::<SharedState>();
                match drag_ev {
                    DragDropEvent::Enter { .. } => {
                        if let Ok(mut d) = state.0.shelf_drag_label.lock() {
                            *d = Some(label.clone());
                        }
                        if let Ok(mut map) = state.0.notches.lock() {
                            if let Some(inst) = map.get_mut(&label) {
                                inst.hitbox.active_width = inst.hitbox.active_width.max(420);
                                inst.hitbox.active_height = inst.hitbox.active_height.max(140);
                            }
                        }
                        let _ = app.emit_to(&label, "shelf-drag-enter", ());
                    }
                    DragDropEvent::Over { .. } => {
                        if let Ok(mut d) = state.0.shelf_drag_label.lock() {
                            *d = Some(label.clone());
                        }
                        if let Ok(mut map) = state.0.notches.lock() {
                            if let Some(inst) = map.get_mut(&label) {
                                inst.hitbox.active_width = inst.hitbox.active_width.max(420);
                                inst.hitbox.active_height = inst.hitbox.active_height.max(140);
                            }
                        }
                    }
                    DragDropEvent::Drop { paths, .. } => {
                        if let Ok(mut d) = state.0.shelf_drag_label.lock() {
                            *d = None;
                        }
                        let str_paths: Vec<String> = paths
                            .iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect();
                        let _ = app.emit_to(&label, "shelf-drag-drop", str_paths);
                    }
                    DragDropEvent::Leave => {
                        if let Ok(mut d) = state.0.shelf_drag_label.lock() {
                            if d.as_deref() == Some(label.as_str()) {
                                *d = None;
                            }
                        }
                        let _ = app.emit_to(&label, "shelf-drag-leave", ());
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_full_config,
            get_monitors,
            get_app_version,
            get_app_logo,
            get_install_channel,
            open_external_url,
            open_config_folder,
            launch_update_and_exit,
            trigger_update_window,
            notch_ready,
            update_notch_hitbox,
            set_notch_text_input,
            sync_widget_data,
            set_lyrics_pinned,
            set_calendar_data,
            get_current_media,
            media_action,
            media_seek,
            get_system_volume,
            set_system_volume,
            lyrics_http_get,
            get_shared_lyrics,
            claim_lyrics_fetch,
            publish_lyrics_data,
            sync_lyrics_ui,
            prepare_shelf_files,
            pick_shelf_files,
            paste_clipboard_to_shelf,
            save_clipboard_to_shelf,
            open_shelf_file,
            reveal_shelf_file,
            start_file_drag,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MangNotch");
}