use crate::model::{Browser, Family, Profile};
use crate::procs::hidden;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn site_url(domain: &str) -> String {
    let local = domain.parse::<IpAddr>().is_ok() || !domain.contains('.');
    format!("{}://{domain}/", if local { "http" } else { "https" })
}

pub fn parse_reg_default(out: &str) -> Option<PathBuf> {
    let line = out.lines().find(|l| l.contains("REG_SZ") || l.contains("REG_EXPAND_SZ"))?;
    let value = line.split_once("_SZ")?.1.trim().trim_matches('"');
    (!value.is_empty()).then(|| PathBuf::from(value))
}

fn app_path(exe: &str) -> Option<PathBuf> {
    ["HKCU", "HKLM"].iter().find_map(|root| {
        let key = format!(r"{root}\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{exe}");
        let out = hidden("reg", &["query", &key, "/ve"]).output().ok()?;
        parse_reg_default(&String::from_utf8_lossy(&out.stdout)).filter(|p| p.is_file())
    })
}

pub fn browser_exe(browser: &Browser, profile: &Profile) -> Option<PathBuf> {
    app_path(&browser.process).or_else(|| match browser.family {
        Family::Chromium => profile
            .path
            .ancestors()
            .find(|p| p.join("Local State").is_file())?
            .parent()
            .map(|p| p.join("Application").join(&browser.process))
            .filter(|p| p.is_file()),
        Family::Firefox => None,
    })
}

pub fn launch_args(browser: &Browser, profile: &Profile, url: &str) -> Vec<String> {
    match browser.family {
        Family::Chromium => vec![format!("--profile-directory={}", profile.id), url.to_string()],
        Family::Firefox => vec!["--profile".into(), profile.path.display().to_string(), url.to_string()],
    }
}

#[cfg(windows)]
fn is_elevated() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut e = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let ok = GetTokenInformation(token, TokenElevation, &mut e as *mut _ as *mut _, std::mem::size_of::<TOKEN_ELEVATION>() as u32, &mut len);
        CloseHandle(token);
        ok != 0 && e.TokenIsElevated != 0
    }
}

#[cfg(windows)]
fn short_path(p: &str) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
    if !p.contains(' ') {
        return Some(p.to_string());
    }
    let wide: Vec<u16> = std::ffi::OsStr::new(p).encode_wide().chain(Some(0)).collect();
    let mut buf = vec![0u16; 1024];
    let n = unsafe { GetShortPathNameW(wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    Some(String::from_utf16_lossy(&buf[..n])).filter(|s| n > 0 && n < buf.len() && !s.contains(' '))
}

#[cfg(windows)]
pub fn launch_as_user(exe: &Path, args: &[String]) -> io::Result<()> {
    if !is_elevated() {
        return Command::new(exe).args(args).spawn().map(|_| ());
    }
    let parts: Option<Vec<String>> = std::iter::once(exe.display().to_string()).chain(args.iter().cloned()).map(|a| short_path(&a)).collect();
    let line = parts.ok_or_else(|| io::Error::other("путь с пробелами без короткого имени"))?.join(" ");
    hidden("runas", &[]).raw_arg("/trustlevel:0x20000").raw_arg(format!("\"{line}\"")).spawn().map(|_| ())
}

#[cfg(not(windows))]
pub fn launch_as_user(exe: &Path, args: &[String]) -> io::Result<()> {
    Command::new(exe).args(args).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_site_urls() {
        assert_eq!(site_url("yandex.ru"), "https://yandex.ru/");
        assert_eq!(site_url("localhost"), "http://localhost/");
        assert_eq!(site_url("172.31.7.227"), "http://172.31.7.227/");
    }

    #[test]
    fn parses_reg_query_output() {
        let out = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\chrome.exe\r\n    (Default)    REG_SZ    C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\r\n";
        assert_eq!(parse_reg_default(out), Some(PathBuf::from(r"C:\Program Files\Google\Chrome\Application\chrome.exe")));
        assert_eq!(parse_reg_default("ERROR"), None);
    }
}
