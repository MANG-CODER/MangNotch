use crate::config::AppConfig;
use crate::media::MediaPayload;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Snapshot ringan khusus loop polling 40ms (0 String besar, 0 Vec base64).
#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub hide_on_fullscreen: bool,
    pub smart_proximity: bool,
    pub interaction_mode_click: bool,
    pub hover_close_ticks: u32,
    pub enable_hotkey: bool,
    pub enable_audio_visualizer: bool,
    pub enable_peeks: bool,
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
}

impl From<&AppConfig> for RuntimeConfig {
    fn from(c: &AppConfig) -> Self {
        let hover_close_ticks = match c.hover_close_speed.as_str() {
            "fast" => 4,
            "relaxed" => 13,
            _ => 7,
        };
        Self {
            hide_on_fullscreen: c.hide_on_fullscreen,
            smart_proximity: c.smart_proximity,
            interaction_mode_click: c.interaction_mode == "click",
            hover_close_ticks,
            enable_hotkey: c.enable_hotkey,
            enable_audio_visualizer: c.enable_audio_visualizer,
            enable_peeks: c.enable_peeks,
            peek_volume: c.peek_volume,
            peek_brightness: c.peek_brightness,
            peek_mic: c.peek_mic,
            peek_charging: c.peek_charging,
            peek_bluetooth: c.peek_bluetooth,
            peek_capslock: c.peek_capslock,
            peek_numlock: c.peek_numlock,
            peek_keyboard_layout: c.peek_keyboard_layout,
            peek_clipboard: c.peek_clipboard,
            peek_usb: c.peek_usb,
            peek_network: c.peek_network,
            peek_battery_low: c.peek_battery_low,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NotchHitbox {
    pub active_width: i32,
    pub active_height: i32,
    pub resting_width: i32,
}

impl Default for NotchHitbox {
    fn default() -> Self {
        Self {
            active_width: 155,
            active_height: 32,
            resting_width: 155,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NotchInstance {
    pub label: String,
    pub monitor_index: usize,
    pub hwnd: isize,
    pub fallback_dpi: f64,
    pub ready_called: bool,
    pub positioned: bool,
    pub hitbox: NotchHitbox,
}

/// State lirik bersama agar multi-monitor hanya 1x fetch & 100% sinkron.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct LyricsSyncState {
    pub track_key: String,
    pub fetching: bool,
    pub is_open: bool,
    pub offset_sec: f64,
    pub payload: Option<serde_json::Value>,
}

pub struct AppStateInner {
    pub config: Mutex<AppConfig>,
    pub runtime: Mutex<RuntimeConfig>,
    pub notches: Mutex<HashMap<String, NotchInstance>>,
    pub shelf_drag_label: Mutex<Option<String>>,
    pub last_media: Mutex<MediaPayload>,
    pub lyrics: Mutex<LyricsSyncState>,
}

#[derive(Clone)]
pub struct SharedState(pub Arc<AppStateInner>);

impl SharedState {
    pub fn new(initial: AppConfig) -> Self {
        let rt = RuntimeConfig::from(&initial);
        Self(Arc::new(AppStateInner {
            config: Mutex::new(initial),
            runtime: Mutex::new(rt),
            notches: Mutex::new(HashMap::new()),
            shelf_drag_label: Mutex::new(None),
            last_media: Mutex::new(MediaPayload::default()),
            lyrics: Mutex::new(LyricsSyncState::default()),
        }))
    }

    pub fn sync_runtime(&self, cfg: &AppConfig) {
        if let Ok(mut rt) = self.0.runtime.lock() {
            *rt = RuntimeConfig::from(cfg);
        }
    }
}