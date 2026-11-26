use crate::state::SharedState;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
    Storage::Streams::DataReader,
    Win32::{
        System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
        UI::Input::KeyboardAndMouse::{
            keybd_event, KEYEVENTF_KEYUP, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE,
            VK_MEDIA_PREV_TRACK,
        },
    },
};

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct MediaPayload {
    pub has_media: bool,
    pub is_playing: bool,
    pub title: String,
    pub artist: String,
    pub position_sec: f64,
    pub duration_sec: f64,
    pub thumbnail_b64: String,
    pub thumb_changed: bool,
}

#[cfg(windows)]
fn fetch_windows_media(last_key: &mut String, last_thumb: &mut String) -> (MediaPayload, bool) {
    let Ok(manager) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .and_then(|op| op.get())
    else {
        *last_key = String::new();
        *last_thumb = String::new();
        return (MediaPayload::default(), false);
    };

    let Ok(session) = manager.GetCurrentSession() else {
        *last_key = String::new();
        *last_thumb = String::new();
        return (MediaPayload::default(), false);
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
        return (MediaPayload::default(), false);
    }

    let (position_sec, duration_sec) = match session.GetTimelineProperties() {
        Ok(tl) => {
            let pos = tl
                .Position()
                .map(|t| t.Duration as f64 / 10_000_000.0)
                .unwrap_or(0.0);
            let end = tl
                .EndTime()
                .map(|t| t.Duration as f64 / 10_000_000.0)
                .unwrap_or(0.0);
            (pos.max(0.0), end.max(0.0))
        }
        Err(_) => (0.0, 0.0),
    };

    let current_key = format!("{}::{}", title, artist);
    let mut thumb_changed = false;

    if *last_key != current_key {
        *last_key = current_key;
        *last_thumb = String::new();
        thumb_changed = true;

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

    let full = MediaPayload {
        has_media: true,
        is_playing,
        title,
        artist,
        position_sec,
        duration_sec,
        thumbnail_b64: last_thumb.clone(),
        thumb_changed: true,
    };

    (full, thumb_changed)
}

pub fn spawn_media_poller(app: AppHandle, state: SharedState) {
    thread::spawn(move || {
        #[cfg(windows)]
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        let mut last_key = String::new();
        let mut last_thumb = String::new();
        let mut prev_has = false;
        let mut prev_playing = false;
        let mut prev_pos: f64 = -1.0;

        loop {
            thread::sleep(Duration::from_millis(600));
            #[cfg(windows)]
            {
                let (full_payload, thumb_changed) =
                    fetch_windows_media(&mut last_key, &mut last_thumb);

                let changed = thumb_changed
                    || full_payload.has_media != prev_has
                    || full_payload.is_playing != prev_playing
                    || (full_payload.is_playing
                        && (full_payload.position_sec - prev_pos).abs() > 0.35);

                if changed {
                    prev_has = full_payload.has_media;
                    prev_playing = full_payload.is_playing;
                    prev_pos = full_payload.position_sec;

                    // Simpan payload lengkap (termasuk thumbnail) di state untuk notch baru yang baru spawn
                    if let Ok(mut cached) = state.0.last_media.lock() {
                        *cached = full_payload.clone();
                    }

                    // Kirim payload ringan: kosongkan thumbnail_b64 jika lagunya belum berganti
                    let mut wire = full_payload;
                    if !thumb_changed {
                        wire.thumbnail_b64.clear();
                        wire.thumb_changed = false;
                    }
                    let _ = app.emit("media-update", wire);
                }
            }
        }
    });
}

pub fn perform_media_action(action: &str) {
    #[cfg(windows)]
    {
        let mut handled = false;
        if let Ok(manager) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .and_then(|op| op.get())
        {
            if let Ok(session) = manager.GetCurrentSession() {
                let res = match action {
                    "play_pause" => session.TryTogglePlayPauseAsync().and_then(|op| op.get()),
                    "next" => session.TrySkipNextAsync().and_then(|op| op.get()),
                    "prev" => session.TrySkipPreviousAsync().and_then(|op| op.get()),
                    _ => Ok(true),
                };
                handled = matches!(res, Ok(true));
            }
        }
        if !handled {
            let vk = match action {
                "play_pause" => VK_MEDIA_PLAY_PAUSE,
                "next" => VK_MEDIA_NEXT_TRACK,
                "prev" => VK_MEDIA_PREV_TRACK,
                _ => return,
            };
            unsafe {
                keybd_event(vk.0 as u8, 0, Default::default(), 0);
                keybd_event(vk.0 as u8, 0, KEYEVENTF_KEYUP, 0);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = action;
}

pub fn perform_media_seek(position_sec: f64) {
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
    #[cfg(not(windows))]
    let _ = position_sec;
}