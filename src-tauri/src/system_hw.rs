#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE};

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
}

pub struct BtState {
    pub radio_on: bool,
    pub connected_count: u32,
    pub device_name: String,
}

pub fn load_bluetooth_dll() -> isize {
    #[cfg(windows)]
    unsafe {
        let bth_dll: Vec<u16> = "BluetoothAPIs.dll\0".encode_utf16().collect();
        LoadLibraryW(bth_dll.as_ptr())
    }
    #[cfg(not(windows))]
    0
}

pub fn read_screen_brightness() -> Option<u8> {
    #[cfg(windows)]
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
            return Some(val.min(100));
        }
    }
    None
}

pub fn read_bluetooth_state(bth_mod: isize) -> Option<BtState> {
    #[cfg(windows)]
    {
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

            return Some(BtState {
                radio_on: true,
                connected_count,
                device_name: first_dev_name,
            });
        }
    }
    #[cfg(not(windows))]
    {
        let _ = bth_mod;
        None
    }
}