use crate::model::{Browser, Family, Profile};
use crate::procs::hidden;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Command;

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

pub fn command_line(exe: &Path, args: &[String]) -> String {
    std::iter::once(exe.display().to_string())
        .chain(args.iter().cloned())
        .map(|a| if a.is_empty() || a.contains(' ') { format!("\"{a}\"") } else { a })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(windows)]
fn spawn_with_shell_token(exe: &Path, args: &[String]) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        DuplicateTokenEx, SecurityImpersonation, TokenPrimary, TOKEN_ADJUST_DEFAULT, TOKEN_ADJUST_SESSIONID, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{
        CreateProcessWithTokenW, OpenProcess, OpenProcessToken, PROCESS_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, STARTUPINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId};
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let app = wide(exe.as_os_str());
    let mut line = wide(command_line(exe, args).as_ref());
    let dir = exe.parent().map(|d| wide(d.as_os_str()));
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(GetShellWindow(), &mut pid);
        if pid == 0 {
            return Err(io::Error::other("не найден рабочий стол Windows"));
        }
        let shell = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if shell.is_null() {
            return Err(io::Error::last_os_error());
        }
        let mut token: HANDLE = std::ptr::null_mut();
        let opened = OpenProcessToken(shell, TOKEN_DUPLICATE, &mut token);
        CloseHandle(shell);
        if opened == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut primary: HANDLE = std::ptr::null_mut();
        let access = TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT | TOKEN_ADJUST_SESSIONID;
        let duplicated = DuplicateTokenEx(token, access, std::ptr::null(), SecurityImpersonation, TokenPrimary, &mut primary);
        CloseHandle(token);
        if duplicated == 0 {
            return Err(io::Error::last_os_error());
        }
        let si = STARTUPINFOW { cb: std::mem::size_of::<STARTUPINFOW>() as u32, ..std::mem::zeroed() };
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
        let created = CreateProcessWithTokenW(
            primary,
            0,
            app.as_ptr(),
            line.as_mut_ptr(),
            0,
            std::ptr::null(),
            dir.as_ref().map_or(std::ptr::null(), |d| d.as_ptr()),
            &si,
            &mut pi,
        );
        let error = io::Error::last_os_error();
        CloseHandle(primary);
        if created == 0 {
            return Err(error);
        }
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
    }
    Ok(())
}

#[cfg(windows)]
pub fn launch_as_user(exe: &Path, args: &[String]) -> io::Result<()> {
    if !is_elevated() {
        return Command::new(exe).args(args).spawn().map(|_| ());
    }
    spawn_with_shell_token(exe, args)
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

    #[test]
    fn quotes_args_with_spaces() {
        let exe = Path::new(r"C:\Program Files\Google\Chrome\Application\chrome.exe");
        let args = ["--profile-directory=Profile 1".to_string(), "https://ya.ru/".to_string()];
        assert_eq!(command_line(exe, &args), r#""C:\Program Files\Google\Chrome\Application\chrome.exe" "--profile-directory=Profile 1" https://ya.ru/"#);
    }
}
