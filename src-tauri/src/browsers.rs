use crate::model::{Browser, Family, Profile};
use crate::snapshot::Snapshot;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rusqlite::{Connection, OpenFlags};
use std::fs;
use std::path::{Component, Path, PathBuf};

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
            let info = names.as_ref().and_then(|c| c.get(&id));
            let name = info.and_then(|i| i.get("name")?.as_str().map(String::from)).unwrap_or_else(|| id.clone());
            let avatar_src = info.and_then(|i| i.get("gaia_picture_file_name")?.as_str()).and_then(|f| avatar_in(&e.path(), f));
            Profile { id, name, path: e.path(), avatar_src, avatar: None }
        })
        .collect();
    if out.is_empty() && is_chromium_profile(root) {
        out.push(Profile { id: "Default".into(), name: "Основной".into(), path: root.to_path_buf(), avatar_src: None, avatar: None });
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
                out.push(Profile { id: id.clone(), name: name.take().unwrap_or(id), path: full, avatar_src: None, avatar: None });
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

fn avatar_in(dir: &Path, file: &str) -> Option<PathBuf> {
    let mut parts = Path::new(file).components();
    let only = matches!((parts.next(), parts.next()), (Some(Component::Normal(_)), None));
    Some(dir.join(file)).filter(|p| only && p.is_file())
}

pub fn avatar_data(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|b| format!("data:image/png;base64,{}", STANDARD.encode(b)))
}

fn ini_store_id(text: &str) -> Option<String> {
    text.lines().find_map(|l| l.trim().strip_prefix("StoreID=")).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn read_profile_group(db: &Connection, root: &Path) -> rusqlite::Result<Vec<Profile>> {
    let avatars = root.join("Profile Groups").join("avatars");
    let mut q = db.prepare("SELECT path, name, avatar FROM Profiles ORDER BY id")?;
    let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)))?;
    Ok(rows
        .flatten()
        .map(|(path, name, avatar)| {
            let p = Path::new(&path);
            let full = if p.is_absolute() { p.to_path_buf() } else { root.join(path.replace('/', "\\")) };
            let id = full.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            let avatar_src = avatar.and_then(|a| avatar_in(&avatars, &a));
            Profile { id, name, path: full, avatar_src, avatar: None }
        })
        .collect())
}

fn firefox_group(root: &Path, store_id: &str) -> Option<Vec<Profile>> {
    let src = root.join("Profile Groups").join(format!("{store_id}.sqlite"));
    if !src.is_file() {
        return None;
    }
    let snap = Snapshot::new().ok()?;
    let db = Connection::open_with_flags(snap.copy_db(&src).ok()?, OpenFlags::SQLITE_OPEN_READ_WRITE).ok()?;
    read_profile_group(&db, root).ok()
}

fn firefox_profiles(root: &Path) -> Vec<Profile> {
    let Ok(text) = fs::read_to_string(root.join("profiles.ini")) else { return vec![] };
    let group = ini_store_id(&text).and_then(|id| firefox_group(root, &id)).filter(|g| !g.is_empty());
    group.unwrap_or_else(|| parse_profiles_ini(&text, root)).into_iter().filter(|p| p.path.join("prefs.js").is_file()).collect()
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
        assert_eq!(ini_store_id(ini), None);
        assert_eq!(ini_store_id("[Profile0]\nPath=Profiles/x\nStoreID=98c5fa9c\n").as_deref(), Some("98c5fa9c"));
    }

    #[test]
    fn reads_firefox_profile_group_with_avatars() {
        let root = std::env::temp_dir().join(format!("cache-out-ffgroup-{}", std::process::id()));
        let avatars = root.join("Profile Groups").join("avatars");
        fs::create_dir_all(&avatars).unwrap();
        fs::write(avatars.join("87099f90"), b"png").unwrap();
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            r"CREATE TABLE Profiles (id INTEGER PRIMARY KEY, path TEXT, name TEXT, avatar TEXT, themeId TEXT, themeFg TEXT, themeBg TEXT);
              INSERT INTO Profiles VALUES (2, 'Profiles\b.work', 'Work','..\..\secret', '', '', '');
              INSERT INTO Profiles VALUES (1, 'Profiles\a.default-release', 'Dark', '87099f90', '', '', '');
              INSERT INTO Profiles VALUES (3, 'Profiles\c.x', 'Цветок', 'flower', '', '', '');",
        )
        .unwrap();
        let p = read_profile_group(&db, &root).unwrap();
        assert_eq!(p.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Dark", "Work", "Цветок"]);
        assert_eq!(p[0].id, "a.default-release");
        assert_eq!(p[0].path, root.join(r"Profiles\a.default-release"));
        assert_eq!(p[0].avatar_src, Some(avatars.join("87099f90")));
        assert_eq!(p[1].avatar_src, None);
        assert_eq!(p[2].avatar_src, None);
        assert!(avatar_data(&avatars.join("87099f90")).unwrap().starts_with("data:image/png;base64,cG5n"));
        fs::remove_dir_all(&root).unwrap();
    }
}
