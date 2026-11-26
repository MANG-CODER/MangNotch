use serde::{Deserialize, Serialize};

#[cfg(windows)]
use windows::Win32::{
    Foundation::BOOL,
    Media::Audio::{
        eCapture, eConsole, eRender,
        Endpoints::{IAudioEndpointVolume, IAudioMeterInformation},
        IMMDeviceEnumerator, MMDeviceEnumerator,
    },
    System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VolumePayload {
    pub volume: u32,
    pub muted: bool,
}

#[cfg(windows)]
pub struct AudioMonitor {
    enumerator: Option<IMMDeviceEnumerator>,
    render_vol: Option<IAudioEndpointVolume>,
    render_meter: Option<IAudioMeterInformation>,
    capture_vol: Option<IAudioEndpointVolume>,
}

#[cfg(windows)]
impl AudioMonitor {
    pub fn new() -> Self {
        let mut m = Self {
            enumerator: None,
            render_vol: None,
            render_meter: None,
            capture_vol: None,
        };
        m.refresh_devices();
        m
    }

    pub fn refresh_devices(&mut self) {
        unsafe {
            if self.enumerator.is_none() {
                self.enumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok();
            }
            if let Some(ref enumerator) = self.enumerator {
                if let Ok(dev) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                    self.render_vol = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok();
                    self.render_meter =
                        dev.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None).ok();
                } else {
                    self.render_vol = None;
                    self.render_meter = None;
                }

                if let Ok(cap) = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole) {
                    self.capture_vol = cap.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok();
                } else {
                    self.capture_vol = None;
                }
            }
        }
    }

    pub fn read_volume(&mut self) -> VolumePayload {
        unsafe {
            if let Some(ref ep) = self.render_vol {
                if let (Ok(level), Ok(muted)) =
                    (ep.GetMasterVolumeLevelScalar(), ep.GetMute())
                {
                    return VolumePayload {
                        volume: (level * 100.0).round().clamp(0.0, 100.0) as u32,
                        muted: muted.as_bool(),
                    };
                }
                self.refresh_devices();
            }
        }
        VolumePayload {
            volume: 50,
            muted: false,
        }
    }

    pub fn read_peak(&mut self) -> f32 {
        unsafe {
            if let Some(ref meter) = self.render_meter {
                if let Ok(peak) = meter.GetPeakValue() {
                    return peak.clamp(0.0, 1.0);
                }
                self.refresh_devices();
            }
        }
        0.0
    }

    pub fn read_mic_muted(&mut self) -> Option<bool> {
        unsafe {
            if let Some(ref ep) = self.capture_vol {
                if let Ok(muted) = ep.GetMute() {
                    return Some(muted.as_bool());
                }
                self.refresh_devices();
            }
        }
        None
    }
}

pub fn read_master_volume() -> VolumePayload {
    #[cfg(windows)]
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

pub fn write_master_volume(volume: Option<u32>, mute: Option<bool>) -> VolumePayload {
    #[cfg(windows)]
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
                        let _ = endpoint.SetMute(
                            BOOL(if clamped == 0 { 1 } else { 0 }),
                            std::ptr::null(),
                        );
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

pub fn read_mic_muted() -> Option<bool> {
    #[cfg(windows)]
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(enumerator) =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
        {
            if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole) {
                if let Ok(endpoint) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    if let Ok(muted) = endpoint.GetMute() {
                        return Some(muted.as_bool());
                    }
                }
            }
        }
    }
    None
}

pub fn toggle_mic_muted_state() -> Option<bool> {
    #[cfg(windows)]
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(enumerator) =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
        {
            if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole) {
                if let Ok(endpoint) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    let cur = endpoint.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                    let next = !cur;
                    let _ = endpoint.SetMute(BOOL(if next { 1 } else { 0 }), std::ptr::null());
                    return Some(next);
                }
            }
        }
    }
    None
}