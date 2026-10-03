use crate::chromium::{bump, has_table, open, Sites};
use crate::model::{FormField, ProfileScan};
use crate::site::{site_of_host, site_of_url};
use crate::snapshot::{dir_size, Snapshot};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

pub const CACHE_DIRS: &[&str] = &["cache2", "startupCache", "jumpListCache"];
pub const STORAGE_REPOS: &[&str] = &["default", "permanent", "temporary"];
pub const DOWNLOAD_ATTR: &str = "downloads/destinationFileURI";

fn micros_to_ms(t: i64) -> Option<i64> {
    (t > 0).then_some(t / 1000)
}

pub fn local_dir(profile: &Path) -> PathBuf {
    let (Some(roaming), Some(local)) = (std::env::var_os("APPDATA"), std::env::var_os("LOCALAPPDATA")) else {
        return profile.to_path_buf();
    };
    match profile.strip_prefix(&roaming) {
        Ok(rel) => PathBuf::from(local).join(rel),
        Err(_) => profile.to_path_buf(),
    }
}

pub fn cache_dirs(profile: &Path) -> Vec<PathBuf> {
    let local = local_dir(profile);
    CACHE_DIRS.iter().map(|d| local.join(d)).collect()
}

pub fn storage_origin(dir_name: &str) -> Option<String> {
    let base = dir_name.split('^').next()?;
    let (scheme, rest) = base.split_once("+++")?;
    let host = match rest.rsplit_once('+') {
        Some((h, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => format!("{h}:{port}"),
        _ => rest.to_string(),
    };
    Some(format!("{scheme}://{host}"))
}

pub fn storage_dirs(profile: &Path) -> Vec<(String, PathBuf)> {
    STORAGE_REPOS
        .iter()
        .flat_map(|r| fs::read_dir(profile.join("storage").join(r)).into_iter().flatten().flatten())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            Some((storage_origin(&name).and_then(|o| site_of_url(&o))?, e.path()))
        })
        .collect()
}

fn read_cookies(db: &Connection, sites: &mut Sites) -> rusqlite::Result<()> {
    let mut q = db.prepare("SELECT host, COUNT(*), MAX(lastAccessed) FROM moz_cookies GROUP BY host")?;
    let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, Option<i64>>(2)?)))?;
    for (host, n, last) in rows.flatten() {
        if let Some(d) = site_of_host(&host) {
            let s = sites.at(d);
            s.cookies += n;
            bump(&mut s.last_cookie_access, last.and_then(micros_to_ms));
        }
    }
    Ok(())
}

pub(crate) fn read_places(db: &Connection, sites: &mut Sites) -> rusqlite::Result<()> {
    let mut q = db.prepare("SELECT url, visit_count, last_visit_date FROM moz_places WHERE visit_count > 0")?;
    let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, Option<i64>>(2)?)))?;
    for (url, visits, last) in rows.flatten() {
        if let Some(d) = site_of_url(&url) {
            let s = sites.at(d);
            s.history_urls += 1;
            s.visits += visits;
            bump(&mut s.last_visit, last.and_then(micros_to_ms));
        }
    }
    if has_table(db, "moz_annos") {
        let mut q = db.prepare(
            "SELECT p.url FROM moz_annos n JOIN moz_anno_attributes a ON a.id = n.anno_attribute_id JOIN moz_places p ON p.id = n.place_id WHERE a.name = ?1",
        )?;
        let rows = q.query_map([DOWNLOAD_ATTR], |r| r.get::<_, String>(0))?;
        for url in rows.flatten() {
            if let Some(d) = site_of_url(&url) {
                sites.at(d).downloads += 1;
            }
        }
    }
    Ok(())
}

pub fn read_forms(db: &Connection) -> rusqlite::Result<Vec<FormField>> {
    let mut q = db.prepare("SELECT fieldname, COUNT(*), MAX(lastUsed) FROM moz_formhistory GROUP BY fieldname ORDER BY COUNT(*) DESC")?;
    let rows = q.query_map([], |r| {
        Ok(FormField { name: r.get(0)?, entries: r.get(1)?, last_used: r.get::<_, Option<i64>>(2)?.and_then(micros_to_ms) })
    })?;
    Ok(rows.flatten().collect())
}

pub fn addresses_file(profile: &Path) -> PathBuf {
    profile.join("autofill-profiles.json")
}

fn read_addresses(profile: &Path) -> u32 {
    fs::read_to_string(addresses_file(profile))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("addresses")?.as_array().map(|a| a.len() as u32))
        .unwrap_or(0)
}

pub fn sync_enabled(profile: &Path) -> bool {
    fs::read_to_string(profile.join("prefs.js")).is_ok_and(|s| s.lines().any(|l| l.starts_with("user_pref(\"services.sync.username\"")))
}

pub fn scan(profile: &Path) -> std::io::Result<ProfileScan> {
    let snap = Snapshot::new()?;
    let mut sites = Sites::default();
    let mut out = ProfileScan::default();
    if let Some(db) = open(&snap, &profile.join("cookies.sqlite"), "Куки", &mut out.locked) {
        let _ = read_cookies(&db, &mut sites);
    }
    if let Some(db) = open(&snap, &profile.join("places.sqlite"), "История", &mut out.locked) {
        let _ = read_places(&db, &mut sites);
    }
    for (d, dir) in storage_dirs(profile) {
        let total = dir_size(&dir);
        if total == 0 {
            continue;
        }
        let cache = dir_size(&dir.join("cache"));
        let s = sites.at(d);
        s.site_cache_bytes += cache;
        s.storage_bytes += total - cache;
    }
    if let Some(db) = open(&snap, &profile.join("formhistory.sqlite"), "Формы", &mut out.locked) {
        out.forms = read_forms(&db).unwrap_or_default();
    }
    out.addresses = read_addresses(profile);
    out.from_shadow = snap.used_shadow();
    out.sync = sync_enabled(profile);
    out.cache_bytes = cache_dirs(profile).iter().map(|d| dir_size(d)).sum();
    out.sites = sites.0.into_values().collect();
    out.sites.sort_by(|a, b| b.last_visit.max(b.last_cookie_access).cmp(&a.last_visit.max(a.last_cookie_access)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_storage_dir_names() {
        assert_eq!(storage_origin("https+++2ch.org").as_deref(), Some("https://2ch.org"));
        assert_eq!(storage_origin("http+++localhost+3000").as_deref(), Some("http://localhost:3000"));
        assert_eq!(storage_origin("https+++www.youtube.com^partitionKey=%28https%2Cexample.com%29").as_deref(), Some("https://www.youtube.com"));
        assert_eq!(storage_origin("moz-extension+++0b1c-uuid^userContextId=1").as_deref(), Some("moz-extension://0b1c-uuid"));
        assert_eq!(storage_origin("chrome"), None);
    }

    #[test]
    fn reads_places_visits_and_downloads() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT, visit_count INTEGER, last_visit_date INTEGER);
             CREATE TABLE moz_anno_attributes (id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE moz_annos (id INTEGER PRIMARY KEY, place_id INTEGER, anno_attribute_id INTEGER);
             INSERT INTO moz_places VALUES (1,'https://www.2ch.org/b/',3,1700000000000000), (2,'https://2ch.org/b/x.mp4',0,NULL), (3,'https://vk.com/',1,NULL);
             INSERT INTO moz_anno_attributes VALUES (1,'downloads/destinationFileURI'), (2,'downloads/metaData');
             INSERT INTO moz_annos VALUES (1,2,1), (2,2,2);",
        )
        .unwrap();
        let mut sites = Sites::default();
        read_places(&db, &mut sites).unwrap();
        let s = &sites.0["2ch.org"];
        assert_eq!((s.history_urls, s.visits, s.downloads, s.last_visit), (1, 3, 1, Some(1_700_000_000_000)));
        assert_eq!(sites.0["vk.com"].downloads, 0);
    }
}
