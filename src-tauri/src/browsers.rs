use crate::model::{Browser, Family, Profile};
use std::fs;
use std::path::{Path, PathBuf};

struct Known {
    id: &'static str,
    name: &'static str,
    family: Family,
    base: &'static str,
    dir: &'static str,
    process: &'static str,
}

const KNOWN: &[Known] = &[
    Known { id: "chrome", name: "Google Chrome", family: Family::Chromium, base: "LOCALAPPDATA", dir: r"Google\Chrome\User Data", process: "chrome.exe" },
    Known { id: "edge", name: "Microsoft Edge", family: Family::Chromium, base: "LOCALAPPDATA", dir: r"Microsoft\Edge\User Data", process: "msedge.exe" },
    Known { id: "brave", name: "Brave", family: Family::Chromium, base: "LOCALAPPDATA", dir: r"BraveSoftware\Brave-Browser\User Data", process: "brave.exe" },
    Known { id: "yandex", name: "Яндекс Браузер", family: Family::Chromium, base: "LOCALAPPDATA", dir: r"Yandex\YandexBrowser\User Data", process: "browser.exe" },
    Known { id: "opera", name: "Opera", family: Family::Chromium, base: "APPDATA", dir: r"Opera Software\Opera Stable", process: "opera.exe" },
    Known { id: "opera-gx", name: "Opera GX", family: Family::Chromium, base: "APPDATA", dir: r"Opera Software\Opera GX Stable", process: "opera.exe" },
    Known { id: "vivaldi", name: "Vivaldi", family: Family::Chromium, base: "LOCALAPPDATA", dir: r"Vivaldi\User Data", process: "vivaldi.exe" },
    Known { id: "chromium", name: "Chromium", family: Family::Chromium, base: "LOCALAPPDATA", dir: r"Chromium\User Data", process: "chrome.exe" },
    Known { id: "thorium", name: "Thorium", family: Family::Chromium, base: "LOCALAPPDATA", dir: r"Thorium\User Data", process: "thorium.exe" },
    Known { id: "firefox", name: "Firefox", family: Family::Firefox, base: "APPDATA", dir: r"Mozilla\Firefox", process: "firefox.exe" },
    Known { id: "zen", name: "Zen", family: Family::Firefox, base: "APPDATA", dir: "zen", process: "zen.exe" },
    Known { id: "floorp", name: "Floorp", family: Family::Firefox, base: "APPDATA", dir: "Floorp", process: "floorp.exe" },
    Known { id: "librewolf", name: "LibreWolf", family: Family::Firefox, base: "APPDATA", dir: "librewolf", process: "librewolf.exe" },
];

pub fn detect() -> Vec<Browser> {
    KNOWN
        .iter()
        .filter_map(|k| {
            let root = PathBuf::from(std::env::var_os(k.base)?).join(k.dir);
            let profiles = match k.family {
                Family::Chromium => chromium_profiles(&root),
                Family::Firefox => firefox_profiles(&root),
            };
            (!profiles.is_empty()).then(|| Browser {
                id: k.id.into(),
                name: k.name.into(),
                family: k.family,
                process: k.process.into(),
                profiles,
            })
        })
        .collect()
}

pub fn find(browser_id: &str, profile_id: &str) -> Option<(Browser, Profile)> {
    let b = detect().into_iter().find(|b| b.id == browser_id)?;
    let p = b.profiles.iter().find(|p| p.id == profile_id)?.clone();
    Some((b, p))
}

fn is_chromium_profile(dir: &Path) -> bool {
    dir.join("Preferences").is_file()
}

fn chromium_profiles(root: &Path) -> Vec<Profile> {
    if !root.is_dir() {
        return vec![];
    }
    let names = fs::read_to_string(root.join("Local State"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.pointer("/profile/info_cache").cloned());
    let mut out: Vec<Profile> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            (n == "Default" || n.starts_with("Profile ")) && is_chromium_profile(&e.path())
        })
        .map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            let name = names
                .as_ref()
                .and_then(|c| c.get(&id)?.get("name")?.as_str().map(String::from))
                .unwrap_or_else(|| id.clone());
            Profile { id, name, path: e.path() }
        })
        .collect();
    if out.is_empty() && is_chromium_profile(root) {
        out.push(Profile { id: "Default".into(), name: "Основной".into(), path: root.to_path_buf() });
    }
    out.sort_by(|a, b| (a.id != "Default").cmp(&(b.id != "Default")).then(a.id.cmp(&b.id)));
    out
}

pub fn parse_profiles_ini(text: &str, root: &Path) -> Vec<Profile> {
    let mut out = vec![];
    let mut section = String::new();
    let (mut name, mut path, mut relative) = (None::<String>, None::<String>, true);
    let mut flush = |section: &str, name: &mut Option<String>, path: &mut Option<String>, relative: bool| {
        if section.starts_with("Profile") {
            if let Some(p) = path.take() {
                let full = if relative { root.join(p.replace('/', "\\")) } else { PathBuf::from(&p) };
                let id = full.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                out.push(Profile { id: id.clone(), name: name.take().unwrap_or(id), path: full });
            }
        }
        *name = None;
        *path = None;
    };
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            flush(&section, &mut name, &mut path, relative);
            section = line[1..line.len() - 1].to_string();
            relative = true;
        } else if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "Name" => name = Some(v.trim().to_string()),
                "Path" => path = Some(v.trim().to_string()),
                "IsRelative" => relative = v.trim() == "1",
                _ => {}
            }
        }
    }
    flush(&section, &mut name, &mut path, relative);
    out
}

fn firefox_profiles(root: &Path) -> Vec<Profile> {
    let Ok(text) = fs::read_to_string(root.join("profiles.ini")) else { return vec![] };
    parse_profiles_ini(&text, root).into_iter().filter(|p| p.path.join("prefs.js").is_file()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_firefox_profiles_ini() {
        let ini = "[Install308046B0AF4A39CB]\nDefault=Profiles/abc.default-release\n\n[Profile1]\nName=default\nIsRelative=1\nPath=Profiles/z9.default\n\n[Profile0]\nName=default-release\nIsRelative=1\nPath=Profiles/abc.default-release\nDefault=1\n\n[Profile2]\nName=Work\nIsRelative=0\nPath=D:\\FF\\work\n\n[General]\nVersion=2\n";
        let p = parse_profiles_ini(ini, Path::new(r"C:\R"));
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].id, "z9.default");
        assert_eq!(p[0].path, PathBuf::from(r"C:\R\Profiles\z9.default"));
        assert_eq!(p[1].name, "default-release");
        assert_eq!(p[2].path, PathBuf::from(r"D:\FF\work"));
    }
}
