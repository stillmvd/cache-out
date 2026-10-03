use std::process::Command;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn parse_tasklist(csv: &str) -> Vec<String> {
    let mut out: Vec<String> = csv
        .lines()
        .filter_map(|l| l.strip_prefix('"')?.split('"').next())
        .map(|n| n.to_ascii_lowercase())
        .collect();
    out.sort();
    out.dedup();
    out
}

pub(crate) fn hidden(exe: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    cmd
}

pub fn running() -> Vec<String> {
    hidden("tasklist", &["/FO", "CSV", "/NH"]).output().map(|o| parse_tasklist(&String::from_utf8_lossy(&o.stdout))).unwrap_or_default()
}

pub fn is_running(process: &str) -> bool {
    running().contains(&process.to_ascii_lowercase())
}

pub fn close(process: &str, name: &str) -> Result<(), String> {
    let _ = hidden("taskkill", &["/IM", process]).output();
    for _ in 0..60 {
        std::thread::sleep(Duration::from_millis(250));
        if !is_running(process) {
            return Ok(());
        }
    }
    Err(format!("{name} не закрылся за 15 секунд — закрой его сам и нажми «Очистить» ещё раз"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tasklist_csv() {
        let csv = "\"chrome.exe\",\"1\",\"Console\",\"1\",\"10 K\"\r\n\"Chrome.exe\",\"2\",\"Console\",\"1\",\"10 K\"\r\n\"firefox.exe\",\"3\",\"Console\",\"1\",\"5 K\"\r\n";
        assert_eq!(parse_tasklist(csv), vec!["chrome.exe", "firefox.exe"]);
    }
}
