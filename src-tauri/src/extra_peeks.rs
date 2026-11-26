use crate::state::RuntimeConfig;
use serde::Serialize;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
struct ExtraPeekPayload {
    kind: String,
    label: String,
    active: bool,
    percent: u32,
}

#[cfg(windows)]
#[repr(C)]
struct SystemPowerStatus {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn GetKeyState(n_virt_key: i32) -> i16;
    fn GetForegroundWindow() -> isize;
    fn GetWindowThreadProcessId(hwnd: isize, lpdw_process_id: *mut u32) -> u32;
    fn GetKeyboardLayout(id_thread: u32) -> isize;
    fn GetClipboardSequenceNumber() -> u32;
    fn IsClipboardFormatAvailable(format: u32) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetLogicalDrives() -> u32;
    fn GetDriveTypeW(lp_root_path_name: *const u16) -> u32;
    fn GetSystemPowerStatus(lp_system_power_status: *mut SystemPowerStatus) -> i32;
    fn LCIDToLocaleName(locale: u32, lp_name: *mut u16, cch_name: i32, dw_flags: u32) -> i32;
}

#[cfg(windows)]
#[link(name = "wininet")]
extern "system" {
    fn InternetGetConnectedState(lpdw_flags: *mut u32, dw_reserved: u32) -> i32;
}

#[cfg(windows)]
unsafe fn current_layout_id() -> u16 {
    let fg = GetForegroundWindow();
    let tid = if fg != 0 {
        GetWindowThreadProcessId(fg, std::ptr::null_mut())
    } else {
        0
    };
    let hkl = GetKeyboardLayout(tid);
    (hkl as usize & 0xFFFF) as u16
}

#[cfg(windows)]
unsafe fn lang_id_to_tag(lang_id: u16) -> String {
    let mut buf = [0u16; 32];
    let len = LCIDToLocaleName(lang_id as u32, buf.as_mut_ptr(), buf.len() as i32, 0);
    if len > 1 {
        return String::from_utf16_lossy(&buf[..(len as usize - 1)]).to_uppercase();
    }
    format!("0x{:04X}", lang_id)
}

#[cfg(windows)]
unsafe fn check_network() -> (bool, &'static str) {
    let mut flags: u32 = 0;
    let ok = InternetGetConnectedState(&mut flags, 0) != 0;
    if !ok {
        return (false, "Disconnected");
    }
    if (flags & 0x02) != 0 {
        (true, "Wi-Fi / Ethernet")
    } else if (flags & 0x01) != 0 {
        (true, "Cellular / Modem")
    } else {
        (true, "Connected")
    }
}

pub struct ExtraPeeksMonitor {
    tick_count: u32,
    last_numlock: bool,
    last_layout_id: u16,
    last_clip_seq: u32,
    last_clip_emit: Instant,
    last_drives: u32,
    last_net_online: bool,
    warned_20: bool,
    warned_10: bool,
}

impl ExtraPeeksMonitor {
    pub fn new() -> Self {
        #[cfg(windows)]
        let (numlock, layout_id, clip_seq, drives, net_online) = unsafe {
            let nl = (GetKeyState(0x90) & 1) != 0;
            let lid = current_layout_id();
            let cs = GetClipboardSequenceNumber();
            let dr = GetLogicalDrives();
            let (online, _) = check_network();
            (nl, lid, cs, dr, online)
        };
        #[cfg(not(windows))]
        let (numlock, layout_id, clip_seq, drives, net_online) = (false, 0, 0, 0, true);

        Self {
            tick_count: 0,
            last_numlock: numlock,
            last_layout_id: layout_id,
            last_clip_seq: clip_seq,
            last_clip_emit: Instant::now(),
            last_drives: drives,
            last_net_online: net_online,
            warned_20: false,
            warned_10: false,
        }
    }

    pub fn tick(&mut self, app: &AppHandle, rt: &RuntimeConfig) {
        self.tick_count = self.tick_count.wrapping_add(1);

        #[cfg(windows)]
        unsafe {
            // 1. Num Lock (cek setiap tick)
            let nl = (GetKeyState(0x90) & 1) != 0;
            if nl != self.last_numlock {
                self.last_numlock = nl;
                if rt.enable_peeks && rt.peek_numlock {
                    let _ = app.emit(
                        "system-peek",
                        ExtraPeekPayload {
                            kind: "numlock".to_string(),
                            label: if nl { "Numpad Active" } else { "Numpad Off" }.to_string(),
                            active: nl,
                            percent: 0,
                        },
                    );
                }
            }

            // 2. Keyboard Input Language / Layout (cek setiap 4 tick ~160ms)
            if self.tick_count % 4 == 0 {
                let lid = current_layout_id();
                if lid != 0 && lid != self.last_layout_id {
                    self.last_layout_id = lid;
                    if rt.enable_peeks && rt.peek_keyboard_layout {
                        let tag = lang_id_to_tag(lid);
                        let _ = app.emit(
                            "system-peek",
                            ExtraPeekPayload {
                                kind: "kb_layout".to_string(),
                                label: tag,
                                active: true,
                                percent: 0,
                            },
                        );
                    }
                }
            }

            // 3. Clipboard Copy (cek setiap 5 tick ~200ms, lock-free!)
            if self.tick_count % 5 == 0 {
                let seq = GetClipboardSequenceNumber();
                if seq != self.last_clip_seq {
                    self.last_clip_seq = seq;
                    if rt.enable_peeks
                        && rt.peek_clipboard
                        && self.last_clip_emit.elapsed().as_millis() > 900
                    {
                        self.last_clip_emit = Instant::now();
                        let kind_label = if IsClipboardFormatAvailable(15) != 0 {
                            "Files Copied"
                        } else if IsClipboardFormatAvailable(8) != 0 {
                            "Image Copied"
                        } else if IsClipboardFormatAvailable(13) != 0 {
                            "Text Copied"
                        } else {
                            "Copied to Clipboard"
                        };
                        let _ = app.emit(
                            "system-peek",
                            ExtraPeekPayload {
                                kind: "clipboard".to_string(),
                                label: kind_label.to_string(),
                                active: true,
                                percent: 0,
                            },
                        );
                    }
                }
            }

            // 4. USB / Removable Drive Mount & Eject (cek setiap 15 tick ~600ms)
            if self.tick_count % 15 == 0 {
                let drives = GetLogicalDrives();
                if drives != self.last_drives {
                    let added = drives & !self.last_drives;
                    let removed = self.last_drives & !drives;
                    self.last_drives = drives;

                    if rt.enable_peeks && rt.peek_usb {
                        for bit in 0..26u32 {
                            let mask = 1u32 << bit;
                            let letter = (b'A' + bit as u8) as char;
                            if (added & mask) != 0 {
                                let root: [u16; 4] = [letter as u16, ':' as u16, '\\' as u16, 0];
                                let desc = match GetDriveTypeW(root.as_ptr()) {
                                    2 => "USB Removable",
                                    3 => "External Drive",
                                    5 => "Optical Drive",
                                    _ => "Storage Drive",
                                };
                                let _ = app.emit(
                                    "system-peek",
                                    ExtraPeekPayload {
                                        kind: "usb".to_string(),
                                        label: format!("Drive {}:\\ · {}", letter, desc),
                                        active: true,
                                        percent: 0,
                                    },
                                );
                            } else if (removed & mask) != 0 {
                                let _ = app.emit(
                                    "system-peek",
                                    ExtraPeekPayload {
                                        kind: "usb".to_string(),
                                        label: format!("Drive {}:\\ Ejected", letter),
                                        active: false,
                                        percent: 0,
                                    },
                                );
                            }
                        }
                    }
                }
            }

            // 5. Network Online / Offline (cek setiap 25 tick ~1 detik)
            if self.tick_count % 25 == 0 {
                let (online, desc) = check_network();
                if online != self.last_net_online {
                    self.last_net_online = online;
                    if rt.enable_peeks && rt.peek_network {
                        let _ = app.emit(
                            "system-peek",
                            ExtraPeekPayload {
                                kind: "network".to_string(),
                                label: desc.to_string(),
                                active: online,
                                percent: 0,
                            },
                        );
                    }
                }
            }

            // 6. Low Battery Warning 20% & 10% (cek setiap 60 tick ~2.4 detik)
            if self.tick_count % 60 == 0 {
                let mut sps = SystemPowerStatus {
                    ac_line_status: 255,
                    battery_flag: 0,
                    battery_life_percent: 255,
                    system_status_flag: 0,
                    battery_life_time: 0,
                    battery_full_life_time: 0,
                };
                if GetSystemPowerStatus(&mut sps) != 0 {
                    let on_battery = sps.ac_line_status == 0;
                    let pct = sps.battery_life_percent;
                    if !on_battery || pct > 24 {
                        self.warned_20 = false;
                        self.warned_10 = false;
                    } else if on_battery && pct <= 100 {
                        if pct <= 10 && !self.warned_10 {
                            self.warned_10 = true;
                            self.warned_20 = true;
                            if rt.enable_peeks && rt.peek_battery_low {
                                let _ = app.emit(
                                    "system-peek",
                                    ExtraPeekPayload {
                                        kind: "battery_low".to_string(),
                                        label: "Critical · Plug in charger!".to_string(),
                                        active: false,
                                        percent: pct as u32,
                                    },
                                );
                            }
                        } else if pct <= 20 && !self.warned_20 {
                            self.warned_20 = true;
                            if rt.enable_peeks && rt.peek_battery_low {
                                let _ = app.emit(
                                    "system-peek",
                                    ExtraPeekPayload {
                                        kind: "battery_low".to_string(),
                                        label: "Battery running low".to_string(),
                                        active: true,
                                        percent: pct as u32,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}