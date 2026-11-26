use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[cfg(windows)]
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

const HKEY_CURRENT_USER: isize = -2147483647i32 as isize;
const REG_SZ: u32 = 1;
const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE: &str = "MangNotch";
const STARTUP_TASK_ID: &str = "MangNotchStartup";

pub const SUPPORTED_LANGS: [&str; 17] = [
    "auto", "id", "en", "ja", "ko", "es", "de", "nl", "fr", "ru", "zh-CN", "zh-TW", "ms", "fil",
    "th", "vi", "hi",
];

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
#[link(name = "kernel32")]
extern "system" {
    fn GetUserDefaultLocaleName(lp_locale_name: *mut u16, cch_locale_name: i32) -> i32;
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

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct ShelfItem {
    pub name: String,
    pub path: String,
    pub ext: String,
    pub size_str: String,
    pub thumb_b64: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AppConfig {
    pub language: String,

    pub monitor_mode: String,
    pub selected_monitors: Vec<usize>,
    pub use_primary_monitor: bool,
    pub selected_monitor_index: usize,

    pub hide_on_fullscreen: bool,
    pub smart_proximity: bool,
    pub start_with_windows: bool,
    pub interaction_mode: String,
    pub hover_close_speed: String,
    pub enable_hotkey: bool,

    pub resting_mode: String,
    pub mascot_style: String,
    pub theme_preset: String,
    pub custom_c1: String,
    pub custom_c2: String,
    pub custom_c3: String,
    pub dynamic_island: bool,
    pub album_art_glow: bool,
    pub lyrics_pinned_open: bool,

    // HyperOS 4 Soft Light Glass Customizer
    pub bg_backdrop: String,
    pub bg_opacity: f32,
    pub notch_bg_opacity: f32,
    pub comp_opacity: f32,
    pub comp_blur: f32,
    pub btn_opacity: f32,
    pub btn_blur: f32,
    pub glass_rim: f32,

    pub enable_audio_visualizer: bool,
    pub ambient_volume: u32,
    pub enable_peeks: bool,
    pub peek_duration_ms: u32,
    pub peek_volume: bool,
    pub peek_brightness: bool,
    pub peek_mic: bool,
    pub peek_charging: bool,
    pub peek_bluetooth: bool,
    pub peek_capslock: bool,
    pub peek_numlock: bool,
    pub peek_keyboard_layout: bool,
    pub peek_clipboard: bool,
    pub peek_usb: bool,
    pub peek_network: bool,
    pub peek_battery_low: bool,

    pub enable_sound: bool,
    pub pomo_auto_break: bool,
    pub pomo_minutes: u32,
    pub pomo_break_minutes: u32,

    pub water_reminder_enabled: bool,
    pub water_tap_sound: bool,
    pub water_reminder_sound: bool,
    pub water_goal_ml: u32,
    pub water_cup_ml: u32,
    pub water_current_ml: u32,
    pub water_interval_min: u32,
    pub last_water_date: String,

    pub todo_style: String,
    pub calendar_holiday_region: String,
    pub calendar_reminder_enabled: bool,
    pub calendar_reminder_lead_min: u32,
    pub calendar_reminder_sound: bool,
    pub calendar_json: String,

    pub todos: Vec<TodoItem>,
    pub shelf: Vec<ShelfItem>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            monitor_mode: "primary".to_string(),
            selected_monitors: vec![0],
            use_primary_monitor: true,
            selected_monitor_index: 0,

            hide_on_fullscreen: true,
            smart_proximity: true,
            start_with_windows: false,
            interaction_mode: "hover".to_string(),
            hover_close_speed: "normal".to_string(),
            enable_hotkey: true,

            resting_mode: "pet".to_string(),
            mascot_style: "neko".to_string(),
            theme_preset: "dusk".to_string(),
            custom_c1: "#FFB7C5".to_string(),
            custom_c2: "#E2D1F9".to_string(),
            custom_c3: "#A2E4F5".to_string(),
            dynamic_island: false,
            album_art_glow: true,
            lyrics_pinned_open: false,

            bg_backdrop: "acrylic".to_string(),
            bg_opacity: 0.42,
            notch_bg_opacity: 0.94,
            comp_opacity: 0.06,
            comp_blur: 14.0,
            btn_opacity: 0.09,
            btn_blur: 12.0,
            glass_rim: 0.16,

            enable_audio_visualizer: true,
            ambient_volume: 55,
            enable_peeks: true,
            peek_duration_ms: 2400,
            peek_volume: true,
            peek_brightness: true,
            peek_mic: true,
            peek_charging: true,
            peek_bluetooth: true,
            peek_capslock: true,
            peek_numlock: true,
            peek_keyboard_layout: true,
            peek_clipboard: true,
            peek_usb: true,
            peek_network: true,
            peek_battery_low: true,

            enable_sound: true,
            pomo_auto_break: false,
            pomo_minutes: 25,
            pomo_break_minutes: 5,

            water_reminder_enabled: true,
            water_tap_sound: true,
            water_reminder_sound: true,
            water_goal_ml: 1500,
            water_cup_ml: 250,
            water_current_ml: 0,
            water_interval_min: 45,
            last_water_date: String::new(),

            todo_style: "horizontal".to_string(),
            calendar_holiday_region: "auto".to_string(),
            calendar_reminder_enabled: true,
            calendar_reminder_lead_min: 10,
            calendar_reminder_sound: true,
            calendar_json: String::new(),

            todos: Vec::new(),
            shelf: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn sanitize(&mut self) {
        if !SUPPORTED_LANGS.contains(&self.language.as_str()) {
            self.language = "auto".to_string();
        }
        if !matches!(self.monitor_mode.as_str(), "primary" | "all" | "custom") {
            self.monitor_mode = "primary".to_string();
        }
        if self.selected_monitors.is_empty() {
            self.selected_monitors.push(0);
        }
        if self.interaction_mode != "click" && self.interaction_mode != "hover" {
            self.interaction_mode = "hover".to_string();
        }
        if !matches!(self.hover_close_speed.as_str(), "fast" | "normal" | "relaxed") {
            self.hover_close_speed = "normal".to_string();
        }
        if self.todo_style != "horizontal" && self.todo_style != "vertical" {
            self.todo_style = "horizontal".to_string();
        }
        if self.mascot_style == "visor" {
            self.mascot_style = "mochi".to_string();
        }
        if !matches!(
            self.bg_backdrop.as_str(),
            "acrylic" | "mica" | "mica_alt" | "none"
        ) {
            self.bg_backdrop = "acrylic".to_string();
        }
        let valid_hex = |s: &str| {
            s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
        };
        if !valid_hex(&self.custom_c1) {
            self.custom_c1 = "#FFB7C5".to_string();
        }
        if !valid_hex(&self.custom_c2) {
            self.custom_c2 = "#E2D1F9".to_string();
        }
        if !valid_hex(&self.custom_c3) {
            self.custom_c3 = "#A2E4F5".to_string();
        }

        self.bg_opacity = self.bg_opacity.clamp(0.0, 0.96);
        self.notch_bg_opacity = self.notch_bg_opacity.clamp(0.35, 1.0);
        self.comp_opacity = self.comp_opacity.clamp(0.0, 0.40);
        self.comp_blur = self.comp_blur.clamp(0.0, 28.0);
        self.btn_opacity = self.btn_opacity.clamp(0.0, 0.40);
        self.btn_blur = self.btn_blur.clamp(0.0, 28.0);
        self.glass_rim = self.glass_rim.clamp(0.0, 0.45);
        self.pomo_minutes = self.pomo_minutes.clamp(1, 180);
        self.pomo_break_minutes = self.pomo_break_minutes.clamp(1, 60);
        self.water_goal_ml = self.water_goal_ml.max(250);
        self.water_cup_ml = self.water_cup_ml.max(50);
        self.water_interval_min = self.water_interval_min.max(1);
    }
}

pub fn get_config_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(base).join("MangNotch");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn debug_log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_config_dir().join("debug.log"))
    {
        let _ = writeln!(f, "{}", msg);
    }
}

pub fn load_config_from_disk() -> AppConfig {
    let path = get_config_path();
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&data) {
            if !data.contains("\"monitor_mode\"") && !cfg.use_primary_monitor {
                cfg.monitor_mode = "custom".to_string();
                cfg.selected_monitors = vec![cfg.selected_monitor_index];
            }
            cfg.sanitize();
            return cfg;
        }
    }
    AppConfig::default()
}

pub fn save_config_to_disk(cfg: &AppConfig) {
    let path = get_config_path();
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(path, json);
    }
}

pub fn detect_system_language() -> String {
    #[cfg(windows)]
    unsafe {
        let mut buf = [0u16; 96];
        let len = GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32);
        if len > 1 {
            let raw = String::from_utf16_lossy(&buf[..(len as usize - 1)]).to_lowercase();
            if raw.starts_with("zh-tw")
                || raw.starts_with("zh-hk")
                || raw.starts_with("zh-mo")
                || raw.contains("hant")
            {
                return "zh-TW".to_string();
            }
            if raw.starts_with("zh") {
                return "zh-CN".to_string();
            }
            let primary = raw.split('-').next().unwrap_or("en");
            return match primary {
                "id" => "id",
                "ja" => "ja",
                "ko" => "ko",
                "es" => "es",
                "de" => "de",
                "nl" => "nl",
                "fr" => "fr",
                "ru" => "ru",
                "ms" => "ms",
                "fil" | "tl" => "fil",
                "th" => "th",
                "vi" => "vi",
                "hi" => "hi",
                _ => "en",
            }
            .to_string();
        }
    }
    "en".to_string()
}

pub fn resolve_language(lang: &str) -> String {
    if lang == "auto" {
        detect_system_language()
    } else if SUPPORTED_LANGS.contains(&lang) {
        lang.to_string()
    } else {
        "en".to_string()
    }
}

#[cfg(windows)]
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub fn is_packaged_app() -> bool {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().contains("WindowsApps"))
        .unwrap_or(false)
}

#[cfg(windows)]
fn run_key_delete() {
    let (k, v) = (wide(RUN_SUBKEY), wide(RUN_VALUE));
    unsafe {
        let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), v.as_ptr());
    }
}

#[cfg(windows)]
fn run_key_set() {
    let Ok(exe_path) = std::env::current_exe() else {
        return;
    };
    let cmd = wide(&format!("\"{}\"", exe_path.to_string_lossy()));
    let (k, v) = (wide(RUN_SUBKEY), wide(RUN_VALUE));
    unsafe {
        let _ = RegSetKeyValueW(
            HKEY_CURRENT_USER,
            k.as_ptr(),
            v.as_ptr(),
            REG_SZ,
            cmd.as_ptr() as *const std::ffi::c_void,
            (cmd.len() * 2) as u32,
        );
    }
}

#[cfg(windows)]
fn set_packaged_startup(enable: bool) {
    std::thread::spawn(move || unsafe {
        use windows::core::HSTRING;
        use windows::ApplicationModel::{StartupTask, StartupTaskState};
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(task) = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))
            .and_then(|op| op.get())
        else {
            return;
        };
        let state = task.State().ok();
        if enable {
            if state == Some(StartupTaskState::Disabled) {
                let _ = task.RequestEnableAsync().and_then(|op| op.get());
            }
        } else if matches!(
            state,
            Some(StartupTaskState::Enabled) | Some(StartupTaskState::EnabledByPolicy)
        ) {
            let _ = task.Disable();
        }
    });
}

#[cfg(windows)]
pub fn query_packaged_startup() -> Option<bool> {
    std::thread::spawn(|| unsafe {
        use windows::core::HSTRING;
        use windows::ApplicationModel::{StartupTask, StartupTaskState};
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let task = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))
            .and_then(|op| op.get())
            .ok()?;
        let st = task.State().ok()?;
        Some(st == StartupTaskState::Enabled || st == StartupTaskState::EnabledByPolicy)
    })
    .join()
    .ok()
    .flatten()
}

pub fn apply_autostart(enable: bool) {
    #[cfg(windows)]
    {
        if is_packaged_app() {
            run_key_delete();
            set_packaged_startup(enable);
        } else if enable {
            if !cfg!(debug_assertions) {
                run_key_set();
            }
        } else {
            run_key_delete();
        }
    }
    #[cfg(not(windows))]
    let _ = enable;
}