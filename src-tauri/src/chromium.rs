use crate::model::{FormField, ProfileScan, Site};
use crate::site::{chrome_time_ms, site_of_host, site_of_origin, site_of_url};
use crate::snapshot::{dir_size, Snapshot};
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const CACHE_DIRS: &[&str] = &["Cache", "Code Cache", "GPUCache", "DawnGraphiteCache", "DawnWebGPUCache", r"Service Worker\ScriptCache"];
pub const DOWNLOAD_URL: &str = "COALESCE(NULLIF(tab_url, ''), NULLIF(site_url, ''), referrer)";

pub fn cookies_db(profile: &Path) -> Option<PathBuf> {
    [profile.join("Network").join("Cookies"), profile.join("Cookies")].into_iter().find(|p| p.is_file())
}

#[derive(Default)]
struct Sites(HashMap<String, Site>);

impl Sites {
    fn at(&mut self, domain: String) -> &mut Site {
        self.0.entry(domain.clone()).or_insert_with(|| Site { domain, ..Default::default() })
    }
}

fn bump(slot: &mut Option<i64>, t: Option<i64>) {
    if t > *slot {
        *slot = t;
    }
}

fn open(snap: &Snapshot, src: &Path, label: &str, locked: &mut Vec<String>) -> Option<Connection> {
    if !src.is_file() {
        return None;
    }
    match snap.copy_db(src).and_then(|p| Connection::open_with_flags(p, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(std::io::Error::other)) {
        Ok(c) => Some(c),
        Err(_) => {
            locked.push(label.to_string());
            None
        }
    }
}

pub fn has_table(db: &Connection, name: &str) -> bool {
    db.query_row("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1", [name], |_| Ok(())).is_ok()
}

fn read_cookies(db: &Connection, sites: &mut Sites) -> rusqlite::Result<()> {
    let mut q = db.prepare("SELECT host_key, COUNT(*), MAX(last_access_utc) FROM cookies GROUP BY host_key")?;
    let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, Option<i64>>(2)?)))?;
    for (host, n, last) in rows.flatten() {
        if let Some(d) = site_of_host(&host) {
            let s = sites.at(d);
            s.cookies += n;
            bump(&mut s.last_cookie_access, last.and_then(chrome_time_ms));
        }
    }
    Ok(())
}

fn read_history(db: &Connection, sites: &mut Sites) -> rusqlite::Result<()> {
    let mut q = db.prepare("SELECT url, visit_count, last_visit_time FROM urls")?;
    let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, Option<i64>>(2)?)))?;
    for (url, visits, last) in rows.flatten() {
        if let Some(d) = site_of_url(&url) {
            let s = sites.at(d);
            s.history_urls += 1;
            s.visits += visits;
            bump(&mut s.last_visit, last.and_then(chrome_time_ms));
        }
    }
    if has_table(db, "downloads") {
        let mut q = db.prepare(&format!("SELECT {DOWNLOAD_URL} FROM downloads"))?;
        let rows = q.query_map([], |r| r.get::<_, Option<String>>(0))?;
        for url in rows.flatten().flatten() {
            if let Some(d) = site_of_url(&url) {
                sites.at(d).downloads += 1;
            }
        }
    }
    Ok(())
}

pub fn indexeddb_origin(dir_name: &str) -> Option<String> {
    let base = dir_name.strip_suffix(".indexeddb.leveldb").or_else(|| dir_name.strip_suffix(".indexeddb.blob"))?;
    let (scheme, rest) = base.split_once('_')?;
    let (host, port) = rest.rsplit_once('_')?;
    let port = if port == "0" { String::new() } else { format!(":{port}") };
    Some(format!("{scheme}://{host}{port}"))
}

pub fn cache_storage_origin(index: &[u8]) -> Option<String> {
    let start = [b"https://".as_slice(), b"http://".as_slice()]
        .iter()
        .filter_map(|p| index.windows(p.len()).position(|w| w == *p))
        .min()?;
    let end = index[start..].iter().position(|b| !(0x21..=0x7e).contains(b)).map_or(index.len(), |n| start + n);
    String::from_utf8(index[start..end].to_vec()).ok()
}

pub fn sw_cache_dirs(profile: &Path) -> Vec<(String, PathBuf)> {
    fs::read_dir(profile.join("Service Worker").join("CacheStorage"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let index = fs::read(e.path().join("index.txt")).ok()?;
            Some((cache_storage_origin(&index).and_then(|o| site_of_origin(&o))?, e.path()))
        })
        .collect()
}

fn read_storage(profile: &Path, snap: &Snapshot, sites: &mut Sites, locked: &mut Vec<String>) {
    for (d, dir) in sw_cache_dirs(profile) {
        let size = dir_size(&dir);
        if size > 0 {
            sites.at(d).site_cache_bytes += size;
        }
    }
    for e in fs::read_dir(profile.join("IndexedDB")).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if let Some(d) = indexeddb_origin(&name).and_then(|o| site_of_url(&o)) {
            sites.at(d).storage_bytes += dir_size(&e.path());
        }
    }
    let web = profile.join("WebStorage");
    if let Some(db) = open(snap, &web.join("QuotaManager"), "Хранилище сайтов", locked) {
        if let Ok(mut q) = db.prepare("SELECT id, storage_key FROM buckets") {
            if let Ok(rows) = q.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))) {
                for (id, key) in rows.flatten() {
                    if let Some(d) = site_of_origin(&key) {
                        let dir = web.join(id.to_string());
                        let total = dir_size(&dir);
                        if total == 0 {
                            continue;
                        }
                        let cache = dir_size(&dir.join("CacheStorage"));
                        let s = sites.at(d);
                        s.site_cache_bytes += cache;
                        s.storage_bytes += total.saturating_sub(cache);
                    }
                }
            }
        }
    }
}

pub fn read_forms(db: &Connection) -> rusqlite::Result<(Vec<FormField>, u32)> {
    let mut forms = vec![];
    if has_table(db, "autofill") {
        let mut q = db.prepare("SELECT name, COUNT(*), MAX(date_last_used) FROM autofill GROUP BY name ORDER BY COUNT(*) DESC")?;
        let rows = q.query_map([], |r| {
            Ok(FormField {
                name: r.get(0)?,
                entries: r.get(1)?,
                last_used: r.get::<_, Option<i64>>(2)?.filter(|t| *t > 0).map(|t| t * 1000),
            })
        })?;
        forms = rows.flatten().collect();
    }
    let addresses = ["addresses", "local_addresses", "autofill_profiles"]
        .iter()
        .find(|t| has_table(db, t))
        .map(|t| db.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0)))
        .transpose()?
        .unwrap_or(0);
    Ok((forms, addresses))
}

pub fn scan(profile: &Path) -> std::io::Result<ProfileScan> {
    let snap = Snapshot::new()?;
    let mut sites = Sites::default();
    let mut out = ProfileScan::default();

    if let Some(db) = cookies_db(profile).and_then(|p| open(&snap, &p, "Куки", &mut out.locked)) {
        let _ = read_cookies(&db, &mut sites);
    }
    if let Some(db) = open(&snap, &profile.join("History"), "История", &mut out.locked) {
        let _ = read_history(&db, &mut sites);
    }
    read_storage(profile, &snap, &mut sites, &mut out.locked);
    if let Some(db) = open(&snap, &profile.join("Web Data"), "Формы", &mut out.locked) {
        if let Ok((forms, addresses)) = read_forms(&db) {
            out.forms = forms;
            out.addresses = addresses;
        }
    }
    out.from_shadow = snap.used_shadow();
    out.cache_bytes = CACHE_DIRS.iter().map(|d| dir_size(&profile.join(d))).sum();
    out.sites = sites.0.into_values().collect();
    out.sites.sort_by(|a, b| b.last_visit.max(b.last_cookie_access).cmp(&a.last_visit.max(a.last_cookie_access)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_indexeddb_dir_names() {
        assert_eq!(indexeddb_origin("https_www.youtube.com_0.indexeddb.leveldb").as_deref(), Some("https://www.youtube.com"));
        assert_eq!(indexeddb_origin("http_localhost_5173.indexeddb.blob").as_deref(), Some("http://localhost:5173"));
        assert_eq!(indexeddb_origin("chrome-extension_abc_0.indexeddb.leveldb").as_deref(), Some("chrome-extension://abc"));
        assert_eq!(indexeddb_origin("LOCK"), None);
    }

    #[test]
    fn reads_origin_from_cache_storage_index() {
        let index = b"\x0a\x47\x0a\x07cachify\x12$b6da\x1a\x02\x28\x00\x3a\x00https://lolz.live/\x12\x12https://lolz.live/ u(";
        assert_eq!(cache_storage_origin(index).as_deref(), Some("https://lolz.live/"));
        assert_eq!(cache_storage_origin(b"\x00\x01http://localhost:5173/\x00").as_deref(), Some("http://localhost:5173/"));
        assert_eq!(cache_storage_origin(b"no origin here"), None);
    }

    #[test]
    fn aggregates_cookies_by_site_without_values() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE cookies (host_key TEXT, name TEXT, value TEXT, encrypted_value BLOB, last_access_utc INTEGER);
             INSERT INTO cookies VALUES ('.google.com','a','',x'00',13000000000000000);
             INSERT INTO cookies VALUES ('accounts.google.com','b','',x'00',13000000001000000);
             INSERT INTO cookies VALUES ('vk.com','c','',x'00',0);",
        )
        .unwrap();
        let mut sites = Sites::default();
        read_cookies(&db, &mut sites).unwrap();
        let g = &sites.0["google.com"];
        assert_eq!(g.cookies, 2);
        assert_eq!(g.last_cookie_access, Some(1_355_526_401_000));
        assert_eq!(sites.0["vk.com"].last_cookie_access, None);
    }

    #[test]
    fn counts_forms_and_addresses_without_values() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE autofill (name TEXT, value TEXT, count INTEGER, date_last_used INTEGER);
             INSERT INTO autofill VALUES ('email','x',3,1700000000);
             INSERT INTO autofill VALUES ('email','y',1,1600000000);
             INSERT INTO autofill VALUES ('q','z',1,0);
             CREATE TABLE addresses (guid TEXT);
             INSERT INTO addresses VALUES ('1');",
        )
        .unwrap();
        let (forms, addresses) = read_forms(&db).unwrap();
        assert_eq!(forms[0], FormField { name: "email".into(), entries: 2, last_used: Some(1_700_000_000_000) });
        assert_eq!(forms[1].last_used, None);
        assert_eq!(addresses, 1);
    }
}
