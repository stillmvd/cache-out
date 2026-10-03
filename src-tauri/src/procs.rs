use std::process::Command;

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

pub fn running() -> Vec<String> {
    let mut cmd = Command::new("tasklist");
    cmd.args(["/FO", "CSV", "/NH"]);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    cmd.output().map(|o| parse_tasklist(&String::from_utf8_lossy(&o.stdout))).unwrap_or_default()
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
