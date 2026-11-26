use crate::config::{get_config_dir, ShelfItem};
use crate::window_manager::get_hwnd;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[cfg(windows)]
use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        Foundation::HWND,
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, IBindCtx, IDataObject, CLSCTX_ALL,
                COINIT_MULTITHREADED,
            },
            Ole::{IDropSource, OleInitialize, DROPEFFECT_COPY, DROPEFFECT_LINK},
        },
        UI::{
            Input::KeyboardAndMouse::ReleaseCapture,
            Shell::{
                BHID_DataObject, FileOpenDialog, IFileOpenDialog, IShellItemArray, SHDoDragDrop,
                ShellExecuteW, FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM,
                SIGDN_FILESYSPATH,
            },
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
};

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn OpenClipboard(h_wnd_new_owner: isize) -> i32;
    fn CloseClipboard() -> i32;
    fn IsClipboardFormatAvailable(format: u32) -> i32;
    fn GetClipboardData(u_format: u32) -> isize;
    fn RegisterClipboardFormatW(lpsz_format: *const u16) -> u32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GlobalLock(h_mem: isize) -> *const u8;
    fn GlobalUnlock(h_mem: isize) -> i32;
    fn GlobalSize(h_mem: isize) -> usize;
}

#[cfg(windows)]
#[link(name = "shell32")]
extern "system" {
    fn DragQueryFileW(hdrop: isize, i_file: u32, lpsz_file: *mut u16, cch: u32) -> u32;
    fn ILCreateFromPathW(psz_path: *const u16) -> *mut std::ffi::c_void;
    fn ILFree(pidl: *mut std::ffi::c_void);
    fn SHCreateShellItemArrayFromIDLists(
        cidl: u32,
        rgpidl: *const *const std::ffi::c_void,
        ppsia: *mut *mut std::ffi::c_void,
    ) -> i32;
}

pub fn unblock_file_motw(path: &str) {
    let ads_path = format!("{}:Zone.Identifier", path);
    let _ = fs::remove_file(ads_path);
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", (bytes as f64) / 1024.0)
    } else {
        format!("{:.1} MB", (bytes as f64) / (1024.0 * 1024.0))
    }
}

/// Konversi Windows CF_DIB (24/32-bit) menjadi file .png valid tanpa crate tambahan.
#[cfg(windows)]
fn dib_to_png(dib: &[u8]) -> Option<Vec<u8>> {
    if dib.len() < 40 {
        return None;
    }
    let bi_size = u32::from_le_bytes([dib[0], dib[1], dib[2], dib[3]]) as usize;
    let width = i32::from_le_bytes([dib[4], dib[5], dib[6], dib[7]]);
    let raw_height = i32::from_le_bytes([dib[8], dib[9], dib[10], dib[11]]);
    let bpp = u16::from_le_bytes([dib[14], dib[15]]);
    let compression = u32::from_le_bytes([dib[16], dib[17], dib[18], dib[19]]);

    if width <= 0 || raw_height == 0 || (bpp != 24 && bpp != 32) {
        return None;
    }
    let w = width as usize;
    let h = raw_height.unsigned_abs() as usize;
    let top_down = raw_height < 0;
    let mask_bytes = if compression == 3 && bi_size == 40 { 12 } else { 0 };
    let pixel_offset = bi_size + mask_bytes;
    let stride = ((w * (bpp as usize) + 31) / 32) * 4;
    if pixel_offset + stride * h > dib.len() {
        return None;
    }

    let mut raw = Vec::with_capacity(h * (1 + w * 4));
    for y in 0..h {
        let src_y = if top_down { y } else { h - 1 - y };
        let row_start = pixel_offset + src_y * stride;
        raw.push(0u8); // PNG filter type: None
        for x in 0..w {
            let px = row_start + x * (bpp as usize / 8);
            let b = dib[px];
            let g = dib[px + 1];
            let r = dib[px + 2];
            let a = if bpp == 32 {
                let av = dib[px + 3];
                if av == 0 { 255 } else { av }
            } else {
                255
            };
            raw.extend_from_slice(&[r, g, b, a]);
        }
    }

    // Zlib stream (uncompressed stored blocks) + Adler-32
    let mut zlib = Vec::with_capacity(raw.len() + 64);
    zlib.extend_from_slice(&[0x78, 0x01]);
    let mut pos = 0;
    while pos < raw.len() {
        let chunk_len = (raw.len() - pos).min(65535);
        let is_last = pos + chunk_len >= raw.len();
        zlib.push(if is_last { 0x01 } else { 0x00 });
        let len16 = chunk_len as u16;
        zlib.extend_from_slice(&len16.to_le_bytes());
        zlib.extend_from_slice(&(!len16).to_le_bytes());
        zlib.extend_from_slice(&raw[pos..pos + chunk_len]);
        pos += chunk_len;
    }
    let (mut s1, mut s2) = (1u32, 0u32);
    for &byte in &raw {
        s1 = (s1 + byte as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    zlib.extend_from_slice(&((s2 << 16) | s1).to_be_bytes());

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = if (crc & 1) != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(tag);
        out.extend_from_slice(payload);
        let c = crc32(&out[start..]);
        out.extend_from_slice(&c.to_be_bytes());
    }

    let mut png = Vec::with_capacity(zlib.len() + 64);
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA
    push_chunk(&mut png, b"IHDR", &ihdr);
    push_chunk(&mut png, b"IDAT", &zlib);
    push_chunk(&mut png, b"IEND", &[]);
    Some(png)
}

pub fn enrich_shelf_item(full_path: &str) -> Option<ShelfItem> {
    let p = PathBuf::from(full_path);
    if !p.exists() {
        return None;
    }
    unblock_file_motw(full_path);
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| full_path.to_string());
    let ext = p
        .extension()
        .map(|s| s.to_string_lossy().to_uppercase())
        .unwrap_or_else(|| "FILE".to_string());
    let meta = fs::metadata(&p).ok();
    let file_len = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let size_str = if meta.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
        "FOLDER".to_string()
    } else {
        format_file_size(file_len)
    };

    let mut thumb_b64 = String::new();
    let ext_low = ext.to_lowercase();
    if matches!(
        ext_low.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "ico"
    ) && file_len > 0
        && file_len <= 2_500_000
    {
        if let Ok(bytes) = fs::read(&p) {
            let mime = match ext_low.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "webp" => "image/webp",
                "gif" => "image/gif",
                "bmp" => "image/bmp",
                "ico" => "image/x-icon",
                _ => "image/png",
            };
            thumb_b64 = format!("data:{};base64,{}", mime, BASE64.encode(&bytes));
        }
    }

    Some(ShelfItem {
        name,
        path: full_path.to_string(),
        ext: ext.chars().take(5).collect(),
        size_str,
        thumb_b64,
    })
}

pub fn read_clipboard_to_shelf_paths() -> Vec<String> {
    let mut paths = Vec::new();
    #[cfg(windows)]
    unsafe {
        if OpenClipboard(0) == 0 {
            return paths;
        }

        let png_name: Vec<u16> = "PNG\0".encode_utf16().collect();
        let cf_png = RegisterClipboardFormatW(png_name.as_ptr());

        // 1. File asli yang di-copy dari File Explorer (CF_HDROP = 15)
        if IsClipboardFormatAvailable(15) != 0 {
            let hdrop = GetClipboardData(15);
            if hdrop != 0 {
                let count = DragQueryFileW(hdrop, 0xFFFFFFFF, std::ptr::null_mut(), 0);
                for i in 0..count {
                    let len = DragQueryFileW(hdrop, i, std::ptr::null_mut(), 0);
                    if len > 0 {
                        let mut buf = vec![0u16; (len + 1) as usize];
                        DragQueryFileW(hdrop, i, buf.as_mut_ptr(), len + 1);
                        let path_str = String::from_utf16_lossy(&buf[..len as usize]);
                        if !path_str.is_empty() {
                            paths.push(path_str);
                        }
                    }
                }
            }
        }
        // 2. Gambar PNG langsung dari Clipboard (Snipping Tool / Browser / Discord)
        else if cf_png != 0 && IsClipboardFormatAvailable(cf_png) != 0 {
            let hpng = GetClipboardData(cf_png);
            if hpng != 0 {
                let ptr = GlobalLock(hpng);
                let size = GlobalSize(hpng);
                if !ptr.is_null() && size > 8 {
                    let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
                    let _ = GlobalUnlock(hpng);
                    let drops_dir = get_config_dir().join("ShelfDrops");
                    let _ = fs::create_dir_all(&drops_dir);
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0);
                    let file_path = drops_dir.join(format!("screenshot_{}.png", ts));
                    if fs::write(&file_path, bytes).is_ok() {
                        paths.push(file_path.to_string_lossy().to_string());
                    }
                } else if !ptr.is_null() {
                    let _ = GlobalUnlock(hpng);
                }
            }
        }
        // 3. Fallback CF_DIB (8) -> konversi otomatis ke .png (bukan .bmp lagi!)
        else if IsClipboardFormatAvailable(8) != 0 {
            let hdib = GetClipboardData(8);
            if hdib != 0 {
                let ptr = GlobalLock(hdib);
                let size = GlobalSize(hdib);
                if !ptr.is_null() && size > 40 {
                    let dib_slice = std::slice::from_raw_parts(ptr, size);
                    let png_opt = dib_to_png(dib_slice);
                    let _ = GlobalUnlock(hdib);
                    if let Some(png_bytes) = png_opt {
                        let drops_dir = get_config_dir().join("ShelfDrops");
                        let _ = fs::create_dir_all(&drops_dir);
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0);
                        let file_path = drops_dir.join(format!("screenshot_{}.png", ts));
                        if fs::write(&file_path, png_bytes).is_ok() {
                            paths.push(file_path.to_string_lossy().to_string());
                        }
                    }
                } else if !ptr.is_null() {
                    let _ = GlobalUnlock(hdib);
                }
            }
        }
        // 4. Teks dari Clipboard (CF_UNICODETEXT = 13)
        else if IsClipboardFormatAvailable(13) != 0 {
            let htext = GetClipboardData(13);
            if htext != 0 {
                let ptr = GlobalLock(htext) as *const u16;
                if !ptr.is_null() {
                    let mut len = 0usize;
                    while *ptr.add(len) != 0 && len < 100_000 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(ptr, len);
                    let text_str = String::from_utf16_lossy(slice).trim().to_string();
                    let _ = GlobalUnlock(htext);

                    if !text_str.is_empty() {
                        let drops_dir = get_config_dir().join("ShelfDrops");
                        let _ = fs::create_dir_all(&drops_dir);
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0);
                        let file_path = drops_dir.join(format!("note_{}.txt", ts));
                        if fs::write(&file_path, text_str.as_bytes()).is_ok() {
                            paths.push(file_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        let _ = CloseClipboard();
    }
    paths
}

pub fn pick_files() -> Vec<ShelfItem> {
    let mut out = Vec::new();
    #[cfg(windows)]
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(dialog) =
            CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_ALL)
        {
            if let Ok(opts) = dialog.GetOptions() {
                let _ = dialog.SetOptions(
                    opts | FOS_ALLOWMULTISELECT | FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM,
                );
            }
            if dialog.Show(HWND::default()).is_ok() {
                if let Ok(results) = dialog.GetResults() {
                    let count = results.GetCount().unwrap_or(0);
                    for i in 0..count {
                        if let Ok(item) = results.GetItemAt(i) {
                            if let Ok(pwstr) = item.GetDisplayName(SIGDN_FILESYSPATH) {
                                let path_str = pwstr.to_string().unwrap_or_default();
                                CoTaskMemFree(Some(pwstr.0 as *const _));
                                if let Some(enriched) = enrich_shelf_item(&path_str) {
                                    out.push(enriched);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

pub fn save_clipboard(image_b64: Option<String>, text_content: Option<String>) -> Option<ShelfItem> {
    let drops_dir = get_config_dir().join("ShelfDrops");
    let _ = fs::create_dir_all(&drops_dir);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Some(b64_raw) = image_b64 {
        let clean = b64_raw.split(',').nth(1).unwrap_or(&b64_raw);
        if let Ok(bytes) = BASE64.decode(clean) {
            let file_path = drops_dir.join(format!("clip_{}.png", ts));
            if fs::write(&file_path, bytes).is_ok() {
                return enrich_shelf_item(&file_path.to_string_lossy());
            }
        }
    }

    if let Some(txt) = text_content {
        let trimmed = txt.trim();
        if !trimmed.is_empty() {
            let file_path = drops_dir.join(format!("note_{}.txt", ts));
            if fs::write(&file_path, trimmed.as_bytes()).is_ok() {
                return enrich_shelf_item(&file_path.to_string_lossy());
            }
        }
    }
    None
}

pub fn open_file(path: &str) {
    unblock_file_motw(path);
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

pub fn reveal_file(path: &str) {
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

pub fn start_drag_out(app: AppHandle, window_label: String, paths: Vec<String>) {
    for p in &paths {
        unblock_file_motw(p);
    }
    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        #[cfg(windows)]
        unsafe {
            let _ = OleInitialize(None);
            let _ = ReleaseCapture();

            let hwnd = app_clone
                .get_webview_window(&window_label)
                .and_then(|w| get_hwnd(&w))
                .unwrap_or_default();

            let mut pidls: Vec<*const std::ffi::c_void> = Vec::new();
            for p in &paths {
                let wide_path: Vec<u16> = p.encode_utf16().chain(std::iter::once(0)).collect();
                let pidl = ILCreateFromPathW(wide_path.as_ptr());
                if !pidl.is_null() {
                    pidls.push(pidl as *const _);
                }
            }

            if !pidls.is_empty() {
                let mut raw_array: *mut std::ffi::c_void = std::ptr::null_mut();
                let hr = SHCreateShellItemArrayFromIDLists(
                    pidls.len() as u32,
                    pidls.as_ptr(),
                    &mut raw_array,
                );
                if hr >= 0 && !raw_array.is_null() {
                    let shell_array = IShellItemArray::from_raw(raw_array);
                    if let Ok(data_obj) = shell_array
                        .BindToHandler::<_, IDataObject>(None::<&IBindCtx>, &BHID_DataObject)
                    {
                        let _ = SHDoDragDrop(
                            hwnd,
                            &data_obj,
                            None::<&IDropSource>,
                            DROPEFFECT_COPY | DROPEFFECT_LINK,
                        );
                    }
                }
                for &pidl in &pidls {
                    ILFree(pidl as *mut _);
                }
            }
        }
    });
}