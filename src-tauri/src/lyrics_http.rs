use serde::Serialize;

#[cfg(windows)]
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

#[derive(Serialize)]
pub struct LyricsHttpResp {
    pub status: u16,
    pub body: String,
}

pub async fn http_get(url: String) -> Result<LyricsHttpResp, String> {
    const ALLOWED: [&str; 9] = [
        "lrclib.net",
        "lyricsplus.binimum.org",
        "lyricsplus.prjktla.my.id",
        "lyrics.paxsenix.org",
        "lyrics-api.boidu.dev",
        "api-lyrics.simpmusic.org",
        "lyrics.kugou.com",
        "music.163.com",
        "mobileservice.kugou.com",
    ];
    let host = url
        .strip_prefix("https://")
        .and_then(|rest| rest.split(&['/', '?', '#'][..]).next())
        .ok_or_else(|| "hanya https yang diizinkan".to_string())?;
    if !ALLOWED.contains(&host) {
        return Err(format!("host tidak diizinkan: {host}"));
    }

    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        unsafe {
            use windows::{core::HSTRING, Foundation::Uri, Web::Http::HttpClient};

            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let uri = Uri::CreateUri(&HSTRING::from(url.as_str())).map_err(|e| e.to_string())?;
            let client = HttpClient::new().map_err(|e| e.to_string())?;

            if let Ok(headers) = client.DefaultRequestHeaders() {
                if let Ok(ua) = headers.UserAgent() {
                    let _ = ua.TryParseAdd(&HSTRING::from("MangNotch/2.1 (lyrics)"));
                }
                if let Ok(acc) = headers.Accept() {
                    let _ = acc.TryParseAdd(&HSTRING::from("application/json"));
                }
            }

            let resp = client
                .GetAsync(&uri)
                .map_err(|e| e.to_string())?
                .get()
                .map_err(|e| e.to_string())?;

            let status = resp.StatusCode().map(|s| s.0 as u16).unwrap_or(500);
            let body = match resp.Content() {
                Ok(content) => content
                    .ReadAsStringAsync()
                    .and_then(|op| op.get())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                Err(_) => String::new(),
            };

            Ok(LyricsHttpResp { status, body })
        }
        #[cfg(not(windows))]
        {
            Err("hanya didukung di Windows".to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn http_post(url: String, body: String) -> Result<LyricsHttpResp, String> {
    const ALLOWED: [&str; 1] = ["u.y.qq.com"];
    let host = url
        .strip_prefix("https://")
        .and_then(|rest| rest.split(&['/', '?', '#'][..]).next())
        .ok_or_else(|| "hanya https yang diizinkan".to_string())?;
    if !ALLOWED.contains(&host) {
        return Err(format!("host tidak diizinkan: {host}"));
    }

    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        unsafe {
            use windows::{
                core::HSTRING,
                Foundation::Uri,
                Storage::Streams::UnicodeEncoding,
                Web::Http::{HttpClient, HttpStringContent},
            };

            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let uri = Uri::CreateUri(&HSTRING::from(url.as_str())).map_err(|e| e.to_string())?;
            let client = HttpClient::new().map_err(|e| e.to_string())?;

            if let Ok(headers) = client.DefaultRequestHeaders() {
                if let Ok(ua) = headers.UserAgent() {
                    let _ = ua.TryParseAdd(&HSTRING::from("MangNotch/2.1 (lyrics)"));
                }
                if let Ok(acc) = headers.Accept() {
                    let _ = acc.TryParseAdd(&HSTRING::from("application/json"));
                }
            }

            let content = HttpStringContent::CreateFromStringWithEncodingAndMediaType(
                &HSTRING::from(body.as_str()),
                UnicodeEncoding::Utf8,
                &HSTRING::from("application/json"),
            )
            .map_err(|e| e.to_string())?;

            let resp = client
                .PostAsync(&uri, &content)
                .map_err(|e| e.to_string())?
                .get()
                .map_err(|e| e.to_string())?;

            let status = resp.StatusCode().map(|s| s.0 as u16).unwrap_or(500);
            let body = match resp.Content() {
                Ok(content) => content
                    .ReadAsStringAsync()
                    .and_then(|op| op.get())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                Err(_) => String::new(),
            };

            Ok(LyricsHttpResp { status, body })
        }
        #[cfg(not(windows))]
        {
            Err("hanya didukung di Windows".to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}