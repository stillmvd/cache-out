use crate::model::Family;
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

const EXTS: &[&str] = &["png", "svg", "ico"];

fn ext_of(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"\x89PNG") {
        Some("png")
    } else if data.starts_with(b"<svg") || data.starts_with(b"<?xml") {
        Some("svg")
    } else if data.starts_with(b"\x00\x00\x01\x00") {
        Some("ico")
    } else {
        None
    }
}

fn file_of(domain: &str, ext: &str) -> String {
    format!("{}.{ext}", domain.replace(':', "!"))
}

fn domain_of(file: &str) -> Option<String> {
    let (name, ext) = file.rsplit_once('.')?;
    EXTS.contains(&ext).then(|| name.replace('!', ":"))
}

fn queries(family: Family) -> (&'static str, &'static str, &'static str) {
    match family {
        Family::Chromium => (
            "Favicons",
            "SELECT m.page_url, m.icon_id, COALESCE((SELECT MAX(width) FROM favicon_bitmaps b WHERE b.icon_id = m.icon_id), 0)
             FROM icon_mapping m WHERE m.icon_id IS NOT NULL",
            "SELECT image_data FROM favicon_bitmaps WHERE icon_id = ?1 AND length(image_data) > 0 ORDER BY width = 32 DESC, width DESC LIMIT 1",
        ),
        Family::Firefox => (
            "favicons.sqlite",
            "SELECT p.page_url, i.id, i.width FROM moz_icons_to_pages ip
             JOIN moz_pages_w_icons p ON p.id = ip.page_id JOIN moz_icons i ON i.id = ip.icon_id",
            "SELECT data FROM moz_icons WHERE id = ?1 AND length(data) > 0",
        ),
    }
}

pub fn pick_icons(db: &Connection, family: Family) -> rusqlite::Result<HashMap<String, Vec<u8>>> {
    let (_, pages, data) = queries(family);
    let mut counts: HashMap<(String, i64), (u32, i64)> = HashMap::new();
    let mut q = db.prepare(pages)?;
    let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?;
    for (url, id, width) in rows.flatten() {
        if let Some(d) = site_of_url(&url) {
            let e = counts.entry((d, id)).or_insert((0, width));
            e.0 += 1;
        }
    }
    let rank = |id: i64, (n, w): (u32, i64)| (n, w == 32, w, -id);
    let mut best: HashMap<String, (i64, (u32, i64))> = HashMap::new();
    for ((d, id), score) in counts {
        let e = best.entry(d).or_insert((id, score));
        if rank(id, score) > rank(e.0, e.1) {
            *e = (id, score);
        }
    }
    let mut q = db.prepare(data)?;
    let mut out = HashMap::new();
    for (d, (id, _)) in best {
        if let Some(bytes) = q.query_row([id], |r| r.get::<_, Vec<u8>>(0)).optional()? {
            out.insert(d, bytes);
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
        if let Some(ext) = ext_of(data) {
            fs::write(dir.join(file_of(d, ext)), data)?;
        }
    }
    fs::write(dir.join(STAMP), stamp)
}

pub fn site_icons(profile: &Path, family: Family, dir: &Path) -> io::Result<HashMap<String, PathBuf>> {
    let src = profile.join(queries(family).0);
    let Some(stamp) = stamp_of(&src) else { return Ok(HashMap::new()) };
    if fs::read_to_string(dir.join(STAMP)).is_ok_and(|s| s == stamp) {
        return Ok(listed(dir));
    }
    let snap = Snapshot::new()?;
    let db = Connection::open_with_flags(snap.copy_db(&src)?, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(io::Error::other)?;
    let icons = pick_icons(&db, family).map_err(io::Error::other)?;
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
        let icons = pick_icons(&favicons_db(), Family::Chromium).unwrap();
        assert_eq!(icons.len(), 2);
        assert_eq!(icons["vk.ru"], vec![0x11]);
        assert_eq!(icons["ya.ru"], vec![0x30]);
    }

    #[test]
    fn picks_firefox_icon_sizes_preferring_32px() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE moz_icons (id INTEGER PRIMARY KEY, icon_url TEXT, width INTEGER, data BLOB);
             CREATE TABLE moz_pages_w_icons (id INTEGER PRIMARY KEY, page_url TEXT);
             CREATE TABLE moz_icons_to_pages (page_id INTEGER, icon_id INTEGER);
             INSERT INTO moz_icons VALUES (1,'a',16,x'16'), (2,'a',32,x'32'), (3,'a',192,x'c0'), (4,'b',16,x'aa');
             INSERT INTO moz_pages_w_icons VALUES (1,'https://2ch.org/a'), (2,'https://2ch.org/b'), (3,'https://vk.com/');
             INSERT INTO moz_icons_to_pages VALUES (1,1), (1,2), (1,3), (2,1), (2,2), (2,3), (3,4);",
        )
        .unwrap();
        let icons = pick_icons(&db, Family::Firefox).unwrap();
        assert_eq!(icons["2ch.org"], vec![0x32]);
        assert_eq!(icons["vk.com"], vec![0xaa]);
        assert_eq!(ext_of(b"<svg xmlns"), Some("svg"));
        assert_eq!(ext_of(b"GIF8"), None);
    }

    #[test]
    fn caches_by_stamp_and_sweeps_unknown_profiles() {
        let root = std::env::temp_dir().join(format!("cache-out-icons-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = cache_dir(&root, "chrome", "Profile 1");
        assert_eq!(dir, root.join("chrome").join("Profile_1"));
        let png = b"\x89PNG1".to_vec();
        let icons = HashMap::from([("vk.ru".to_string(), png.clone()), ("::1".to_string(), b"<svg/>".to_vec()), ("x.org".to_string(), vec![7u8])]);
        write_icons(&dir, &icons, "s1").unwrap();
        let got = listed(&dir);
        assert_eq!(got.len(), 2);
        assert_eq!(fs::read(&got["::1"]).unwrap(), b"<svg/>");
        assert!(got["vk.ru"].ends_with("vk.ru.png"));
        write_icons(&dir, &HashMap::from([("ya.ru".to_string(), png)]), "s2").unwrap();
        assert_eq!(listed(&dir).keys().collect::<Vec<_>>(), ["ya.ru"]);
        let stale = cache_dir(&root, "brave", "Default");
        fs::create_dir_all(&stale).unwrap();
        sweep(&root, &HashSet::from([dir.clone()]));
        assert!(dir.exists());
        assert!(!root.join("brave").exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
