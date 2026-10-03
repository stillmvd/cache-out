use crate::site::site_of_url;
use crate::snapshot::Snapshot;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const STAMP: &str = ".stamp";

pub fn icons_root() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Cache Out").join("favicons")
}

pub fn safe(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '.' { c } else { '_' }).collect()
}

pub fn cache_dir(root: &Path, browser_id: &str, profile_id: &str) -> PathBuf {
    root.join(safe(browser_id)).join(safe(profile_id))
}

fn file_of(domain: &str) -> String {
    format!("{}.png", domain.replace(':', "!"))
}

fn domain_of(file: &str) -> Option<String> {
    file.strip_suffix(".png").map(|d| d.replace('!', ":"))
}

pub fn pick_icons(db: &Connection) -> rusqlite::Result<HashMap<String, Vec<u8>>> {
    let mut counts: HashMap<(String, i64), u32> = HashMap::new();
    let mut q = db.prepare("SELECT page_url, icon_id FROM icon_mapping WHERE icon_id IS NOT NULL")?;
    let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    for (url, id) in rows.flatten() {
        if let Some(d) = site_of_url(&url) {
            *counts.entry((d, id)).or_default() += 1;
        }
    }
    let mut best: HashMap<String, (i64, u32)> = HashMap::new();
    for ((d, id), n) in counts {
        let e = best.entry(d).or_insert((id, 0));
        if n > e.1 || (n == e.1 && id < e.0) {
            *e = (id, n);
        }
    }
    let mut q = db.prepare(
        "SELECT image_data FROM favicon_bitmaps WHERE icon_id = ?1 AND length(image_data) > 0 ORDER BY width = 32 DESC, width DESC LIMIT 1",
    )?;
    let mut out = HashMap::new();
    for (d, (id, _)) in best {
        if let Some(data) = q.query_row([id], |r| r.get::<_, Vec<u8>>(0)).optional()? {
            out.insert(d, data);
        }
    }
    Ok(out)
}

fn stamp_of(src: &Path) -> Option<String> {
    let m = src.metadata().ok()?;
    let t = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis();
    Some(format!("{t}-{}", m.len()))
}

fn listed(dir: &Path) -> HashMap<String, PathBuf> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| Some((domain_of(&e.file_name().to_string_lossy())?, e.path())))
        .collect()
}

pub fn write_icons(dir: &Path, icons: &HashMap<String, Vec<u8>>, stamp: &str) -> io::Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    fs::create_dir_all(dir)?;
    for (d, data) in icons {
        fs::write(dir.join(file_of(d)), data)?;
    }
    fs::write(dir.join(STAMP), stamp)
}

pub fn site_icons(profile: &Path, dir: &Path) -> io::Result<HashMap<String, PathBuf>> {
    let src = profile.join("Favicons");
    let Some(stamp) = stamp_of(&src) else { return Ok(HashMap::new()) };
    if fs::read_to_string(dir.join(STAMP)).is_ok_and(|s| s == stamp) {
        return Ok(listed(dir));
    }
    let snap = Snapshot::new()?;
    let db = Connection::open_with_flags(snap.copy_db(&src)?, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(io::Error::other)?;
    let icons = pick_icons(&db).map_err(io::Error::other)?;
    write_icons(dir, &icons, &stamp)?;
    Ok(listed(dir))
}

pub fn sweep(root: &Path, keep: &HashSet<PathBuf>) {
    for b in fs::read_dir(root).into_iter().flatten().flatten() {
        for p in fs::read_dir(b.path()).into_iter().flatten().flatten() {
            if !keep.contains(&p.path()) {
                let _ = fs::remove_dir_all(p.path());
            }
        }
        let _ = fs::remove_dir(b.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn favicons_db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE icon_mapping (id INTEGER PRIMARY KEY, page_url TEXT, icon_id INTEGER);
             CREATE TABLE favicons (id INTEGER PRIMARY KEY, url TEXT, icon_type INTEGER);
             CREATE TABLE favicon_bitmaps (id INTEGER PRIMARY KEY, icon_id INTEGER, image_data BLOB, width INTEGER);
             INSERT INTO icon_mapping (page_url, icon_id) VALUES
               ('https://vk.ru/feed', 1), ('https://m.vk.ru/im', 1), ('https://login.vk.ru/?x', 2),
               ('https://ya.ru/', 3), ('chrome://settings', 1), ('https://none.org/', 9);
             INSERT INTO favicon_bitmaps (icon_id, image_data, width) VALUES
               (1, x'10', 16), (1, x'11', 32), (1, x'12', 64), (2, x'20', 32), (3, x'30', 16), (3, x'', 32);",
        )
        .unwrap();
        db
    }

    #[test]
    fn picks_most_used_icon_per_site_preferring_32px() {
        let icons = pick_icons(&favicons_db()).unwrap();
        assert_eq!(icons.len(), 2);
        assert_eq!(icons["vk.ru"], vec![0x11]);
        assert_eq!(icons["ya.ru"], vec![0x30]);
    }

    #[test]
    fn caches_by_stamp_and_sweeps_unknown_profiles() {
        let root = std::env::temp_dir().join(format!("cache-out-icons-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = cache_dir(&root, "chrome", "Profile 1");
        assert_eq!(dir, root.join("chrome").join("Profile_1"));
        let icons = HashMap::from([("vk.ru".to_string(), vec![1u8]), ("::1".to_string(), vec![2u8])]);
        write_icons(&dir, &icons, "s1").unwrap();
        let got = listed(&dir);
        assert_eq!(got.len(), 2);
        assert_eq!(fs::read(&got["::1"]).unwrap(), vec![2u8]);
        write_icons(&dir, &HashMap::from([("ya.ru".to_string(), vec![3u8])]), "s2").unwrap();
        assert_eq!(listed(&dir).keys().collect::<Vec<_>>(), ["ya.ru"]);
        let stale = cache_dir(&root, "brave", "Default");
        fs::create_dir_all(&stale).unwrap();
        sweep(&root, &HashSet::from([dir.clone()]));
        assert!(dir.exists());
        assert!(!root.join("brave").exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
