use crate::chromium::{cookies_db, has_table, indexeddb_origin, local_storage_dir, sw_cache_dirs, CACHE_DIRS, DOWNLOAD_URL};
use crate::firefox;
use crate::localstorage;
use crate::model::Family;
use crate::site::{site_of_host, site_of_origin, site_of_url};
use crate::snapshot::dir_size;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const KEEP_SECS: u64 = 7 * 24 * 3600;
static NEXT: AtomicU32 = AtomicU32::new(0);
const ADDRESS_TABLES: &[&str] = &[
    "addresses",
    "address_type_tokens",
    "local_addresses",
    "local_addresses_type_tokens",
    "autofill_profiles",
    "autofill_profile_names",
    "autofill_profile_emails",
    "autofill_profile_phones",
    "autofill_profile_addresses",
];

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    #[serde(rename = "c")]
    Cookies,
    #[serde(rename = "h")]
    History,
    #[serde(rename = "d")]
    Downloads,
    #[serde(rename = "s")]
    Storage,
    #[serde(rename = "k")]
    SiteCache,
    #[serde(rename = "bc")]
    BrowserCache,
    #[serde(rename = "f")]
    Forms,
    #[serde(rename = "a")]
    Addresses,
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CleanRequest {
    pub sites: HashMap<String, Vec<Key>>,
    pub profile: Vec<Key>,
}

#[derive(Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub freed_bytes: u64,
    pub backup: Option<PathBuf>,
}

impl CleanRequest {
    fn domains(&self, key: Key) -> HashSet<&str> {
        self.sites.iter().filter(|(_, keys)| keys.contains(&key)).map(|(d, _)| d.as_str()).collect()
    }
}

struct Backup<'a> {
    profile: &'a Path,
    dir: PathBuf,
    used: bool,
}

impl Backup<'_> {
    fn target(&mut self, src: &Path) -> io::Result<PathBuf> {
        let rel = src.strip_prefix(self.profile).map_err(io::Error::other)?;
        let dst = self.dir.join(rel);
        fs::create_dir_all(dst.parent().unwrap_or(&self.dir))?;
        self.used = true;
        Ok(dst)
    }

    fn copy_db(&mut self, src: &Path) -> io::Result<()> {
        let dst = self.target(src)?;
        fs::copy(src, &dst)?;
        for ext in ["-wal", "-journal"] {
            let side = PathBuf::from(format!("{}{ext}", src.display()));
            if side.exists() {
                fs::copy(&side, format!("{}{ext}", dst.display()))?;
            }
        }
        Ok(())
    }

    fn copy_dir(&mut self, src: &Path) -> io::Result<()> {
        let dst = self.target(src)?;
        copy_tree(src, &dst)
    }

    fn take(&mut self, src: &Path) -> io::Result<u64> {
        let size = if src.is_dir() { dir_size(src) } else { src.metadata()?.len() };
        let dst = self.target(src)?;
        match fs::rename(src, &dst) {
            Err(e) if e.raw_os_error() == Some(17) => {
                copy_tree(src, &dst)?;
                if src.is_dir() { fs::remove_dir_all(src) } else { fs::remove_file(src) }?;
            }
            r => r?,
        }
        Ok(size)
    }
}

fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.is_dir() {
        return fs::copy(src, dst).map(|_| ());
    }
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)?.flatten() {
        copy_tree(&e.path(), &dst.join(e.file_name()))?;
    }
    Ok(())
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CleanError {
    pub message: String,
    pub touched: bool,
}

impl CleanError {
    pub fn before(message: impl Into<String>) -> Self {
        Self { message: message.into(), touched: false }
    }
}

pub fn backups_root() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Cache Out").join("backups")
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn sweep_backups(root: &Path, now: u64) {
    for e in fs::read_dir(root).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let born = name.split('-').next().and_then(|s| s.parse::<u64>().ok());
        if born.is_some_and(|t| now.saturating_sub(t) > KEEP_SECS) {
            let _ = fs::remove_dir_all(e.path());
        }
    }
}

fn has_column(db: &Connection, table: &str, column: &str) -> bool {
    db.query_row("SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2", [table, column], |_| Ok(())).is_ok()
}

fn matching(db: &Connection, sql: &str, sites: &HashSet<&str>, site: fn(&str) -> Option<String>) -> rusqlite::Result<Vec<i64>> {
    let mut q = db.prepare(sql)?;
    let rows = q.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)))?;
    Ok(rows
        .flatten()
        .filter(|(_, v)| v.as_deref().and_then(site).is_some_and(|d| sites.contains(d.as_str())))
        .map(|(id, _)| id)
        .collect())
}

fn ids(db: &Connection, sql: &str) -> rusqlite::Result<Vec<i64>> {
    let mut q = db.prepare(sql)?;
    let rows = q.query_map([], |r| r.get(0))?;
    rows.collect()
}

fn mark(db: &Connection, set: &str, ids: &[i64]) -> rusqlite::Result<()> {
    db.execute_batch(&format!("DROP TABLE IF EXISTS temp.{set}; CREATE TEMP TABLE {set}(id INTEGER PRIMARY KEY);"))?;
    let mut q = db.prepare(&format!("INSERT OR IGNORE INTO temp.{set} VALUES (?1)"))?;
    for id in ids {
        q.execute([id])?;
    }
    Ok(())
}

fn drop_where(db: &Connection, table: &str, column: &str, set: &str) -> rusqlite::Result<()> {
    if has_table(db, table) && has_column(db, table, column) {
        db.execute(&format!("DELETE FROM {table} WHERE {column} IN (SELECT id FROM temp.{set})"), [])?;
    }
    Ok(())
}

fn open(path: &Path) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.busy_timeout(std::time::Duration::from_secs(2))?;
    db.execute_batch("PRAGMA secure_delete = ON;")?;
    Ok(db)
}

fn in_tx(db: &mut Connection, work: impl FnOnce(&Connection) -> rusqlite::Result<()>) -> rusqlite::Result<()> {
    let tx = db.transaction()?;
    work(&tx)?;
    tx.commit()
}

pub fn clean_cookies(db: &Connection, sites: &HashSet<&str>) -> rusqlite::Result<()> {
    mark(db, "gone_c", &matching(db, "SELECT rowid, host_key FROM cookies", sites, site_of_host)?)?;
    db.execute("DELETE FROM cookies WHERE rowid IN (SELECT id FROM temp.gone_c)", []).map(|_| ())
}

pub fn clean_history(db: &Connection, sites: &HashSet<&str>) -> rusqlite::Result<()> {
    mark(db, "gone_u", &matching(db, "SELECT id, url FROM urls", sites, site_of_url)?)?;
    mark(db, "gone_v", &ids(db, "SELECT id FROM visits WHERE url IN (SELECT id FROM temp.gone_u)")?)?;
    for (table, column) in [
        ("visit_source", "id"),
        ("content_annotations", "visit_id"),
        ("context_annotations", "visit_id"),
        ("clusters_and_visits", "visit_id"),
        ("cluster_visit_duplicates", "visit_id"),
        ("cluster_visit_duplicates", "duplicate_visit_id"),
        ("visits", "id"),
    ] {
        drop_where(db, table, column, "gone_v")?;
    }
    drop_where(db, "keyword_search_terms", "url_id", "gone_u")?;
    drop_where(db, "visited_links", "link_url_id", "gone_u")?;
    if has_table(db, "segments") {
        mark(db, "gone_s", &ids(db, "SELECT id FROM segments WHERE url_id IN (SELECT id FROM temp.gone_u)")?)?;
        drop_where(db, "segment_usage", "segment_id", "gone_s")?;
        drop_where(db, "segments", "id", "gone_s")?;
    }
    drop_where(db, "urls", "id", "gone_u")
}

pub fn clean_shortcuts(db: &Connection, sites: &HashSet<&str>) -> rusqlite::Result<()> {
    if !has_table(db, "omni_box_shortcuts") {
        return Ok(());
    }
    mark(db, "gone_sc", &matching(db, "SELECT rowid, url FROM omni_box_shortcuts", sites, site_of_url)?)?;
    db.execute("DELETE FROM omni_box_shortcuts WHERE rowid IN (SELECT id FROM temp.gone_sc)", []).map(|_| ())
}

pub fn clean_top_sites(db: &Connection, sites: &HashSet<&str>) -> rusqlite::Result<()> {
    if !has_table(db, "top_sites") {
        return Ok(());
    }
    mark(db, "gone_ts", &matching(db, "SELECT rowid, url FROM top_sites", sites, site_of_url)?)?;
    db.execute_batch(
        "DELETE FROM top_sites WHERE rowid IN (SELECT id FROM temp.gone_ts);
         UPDATE top_sites SET url_rank = (SELECT COUNT(*) FROM top_sites t WHERE t.url_rank < top_sites.url_rank);",
    )
}

pub fn clean_favicons(db: &Connection, sites: &HashSet<&str>) -> rusqlite::Result<()> {
    if !has_table(db, "icon_mapping") {
        return Ok(());
    }
    mark(db, "gone_im", &matching(db, "SELECT id, page_url FROM icon_mapping", sites, site_of_url)?)?;
    db.execute_batch(
        "DELETE FROM icon_mapping WHERE id IN (SELECT id FROM temp.gone_im);
         DELETE FROM favicon_bitmaps WHERE icon_id NOT IN (SELECT icon_id FROM icon_mapping WHERE icon_id IS NOT NULL);
         DELETE FROM favicons WHERE id NOT IN (SELECT icon_id FROM icon_mapping WHERE icon_id IS NOT NULL);",
    )
}

pub fn clean_downloads(db: &Connection, sites: &HashSet<&str>) -> rusqlite::Result<()> {
    if !has_table(db, "downloads") {
        return Ok(());
    }
    mark(db, "gone_d", &matching(db, &format!("SELECT id, {DOWNLOAD_URL} FROM downloads"), sites, site_of_url)?)?;
    drop_where(db, "downloads_url_chains", "id", "gone_d")?;
    drop_where(db, "downloads_slices", "download_id", "gone_d")?;
    drop_where(db, "downloads", "id", "gone_d")
}

fn clean_web_data(db: &Connection, forms: bool, addresses: bool) -> rusqlite::Result<()> {
    let tables = forms.then_some("autofill").into_iter().chain(ADDRESS_TABLES.iter().copied().filter(|_| addresses));
    for t in tables {
        if has_table(db, t) {
            db.execute(&format!("DELETE FROM {t}"), [])?;
        }
    }
    Ok(())
}

fn remove(path: &Path) -> io::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let size = dir_size(path);
    fs::remove_dir_all(path)?;
    Ok(size)
}

fn step<T, E: std::fmt::Display>(label: &str, r: Result<T, E>) -> Result<T, String> {
    r.map_err(|e| format!("{label}: {e}"))
}

pub fn clean_profile(profile: &Path, family: Family, req: &CleanRequest, backup_dir: PathBuf) -> Result<CleanReport, CleanError> {
    let mut backup = Backup { profile, dir: backup_dir, used: false };
    let done = match family {
        Family::Chromium => run(profile, req, &mut backup),
        Family::Firefox => run_firefox(profile, req, &mut backup),
    };
    match done {
        Ok(freed) => Ok(CleanReport { freed_bytes: freed, backup: backup.used.then_some(backup.dir) }),
        Err(message) => Err(CleanError {
            message: if backup.used { format!("{message}. Что успело удалиться — в копии {}", backup.dir.display()) } else { message },
            touched: true,
        }),
    }
}

fn run(profile: &Path, req: &CleanRequest, backup: &mut Backup) -> Result<u64, String> {
    let mut freed = 0u64;
    let cookies = req.domains(Key::Cookies);
    let history = req.domains(Key::History);
    let downloads = req.domains(Key::Downloads);
    let storage = req.domains(Key::Storage);
    let site_cache = req.domains(Key::SiteCache);

    if let Some(path) = cookies_db(profile).filter(|_| !cookies.is_empty()) {
        step("Копия куки", backup.copy_db(&path))?;
        let mut db = step("Куки", open(&path))?;
        step("Куки", in_tx(&mut db, |db| clean_cookies(db, &cookies)))?;
    }

    let path = profile.join("History");
    if path.is_file() && !(history.is_empty() && downloads.is_empty()) {
        step("Копия истории", backup.copy_db(&path))?;
        let mut db = step("История", open(&path))?;
        step("История", in_tx(&mut db, |db| clean_history(db, &history).and_then(|_| clean_downloads(db, &downloads))))?;
    }

    if !history.is_empty() {
        let traces: [(&str, fn(&Connection, &HashSet<&str>) -> rusqlite::Result<()>); 3] =
            [("Shortcuts", clean_shortcuts), ("Top Sites", clean_top_sites), ("Favicons", clean_favicons)];
        for (file, work) in traces {
            let path = profile.join(file);
            if path.is_file() {
                step("Копия истории", backup.copy_db(&path))?;
                let mut db = step("История", open(&path))?;
                step("История", in_tx(&mut db, |db| work(db, &history)))?;
            }
        }
    }

    let forms = req.profile.contains(&Key::Forms);
    let addresses = req.profile.contains(&Key::Addresses);
    let path = profile.join("Web Data");
    if path.is_file() && (forms || addresses) {
        step("Копия форм", backup.copy_db(&path))?;
        let mut db = step("Формы", open(&path))?;
        step("Формы", in_tx(&mut db, |db| clean_web_data(db, forms, addresses)))?;
    }

    if !storage.is_empty() {
        for e in fs::read_dir(profile.join("IndexedDB")).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if indexeddb_origin(&name).and_then(|o| site_of_url(&o)).is_some_and(|d| storage.contains(d.as_str())) {
                freed += step("Хранилище", backup.take(&e.path()))?;
            }
        }
    }

    let ls = local_storage_dir(profile);
    if !storage.is_empty() && ls.is_dir() {
        step("Local Storage", localstorage::delete_sites(&ls, &storage, || backup.copy_dir(&ls)))?;
    }

    let web = profile.join("WebStorage");
    let quota = web.join("QuotaManager");
    if quota.is_file() && !(storage.is_empty() && site_cache.is_empty()) {
        let buckets: Vec<(i64, String)> = {
            let db = step("Хранилище", open(&quota))?;
            let mut q = step("Хранилище", db.prepare("SELECT id, storage_key FROM buckets"))?;
            let rows = step("Хранилище", q.query_map([], |r| Ok((r.get(0)?, r.get(1)?))))?;
            let found = rows.flatten().filter_map(|(id, key): (i64, String)| site_of_origin(&key).map(|d| (id, d))).collect();
            found
        };
        let mut gone = vec![];
        for (id, domain) in buckets {
            let dir = web.join(id.to_string());
            if site_cache.contains(domain.as_str()) {
                freed += step("Кеш сайта", remove(&dir.join("CacheStorage")))?;
            }
            if !storage.contains(domain.as_str()) {
                continue;
            }
            if dir.join("CacheStorage").exists() {
                for e in step("Хранилище", fs::read_dir(&dir))?.flatten() {
                    if e.file_name() != "CacheStorage" {
                        freed += step("Хранилище", backup.take(&e.path()))?;
                    }
                }
            } else {
                if dir.exists() {
                    freed += step("Хранилище", backup.take(&dir))?;
                }
                gone.push(id);
            }
        }
        if !gone.is_empty() {
            step("Копия хранилища", backup.copy_db(&quota))?;
            let mut db = step("Хранилище", open(&quota))?;
            step(
                "Хранилище",
                in_tx(&mut db, |db| {
                    mark(db, "gone_b", &gone)?;
                    drop_where(db, "buckets", "id", "gone_b")
                }),
            )?;
        }
    }

    if !site_cache.is_empty() {
        for (domain, dir) in sw_cache_dirs(profile) {
            if site_cache.contains(domain.as_str()) {
                freed += step("Кеш сайта", remove(&dir))?;
            }
        }
    }

    if req.profile.contains(&Key::BrowserCache) {
        for d in CACHE_DIRS {
            freed += step("Кеш браузера", remove(&profile.join(d)))?;
        }
    }

    Ok(freed)
}

pub fn clean_ff_cookies(db: &Connection, sites: &HashSet<&str>) -> rusqlite::Result<()> {
    mark(db, "gone_c", &matching(db, "SELECT id, host FROM moz_cookies", sites, site_of_host)?)?;
    drop_where(db, "moz_cookies", "id", "gone_c")
}

fn prune_places(db: &Connection, set: &str) -> rusqlite::Result<()> {
    db.execute_batch(&format!(
        "DELETE FROM moz_places WHERE id IN (SELECT id FROM temp.{set}) AND visit_count = 0 AND foreign_count = 0
           AND id NOT IN (SELECT place_id FROM moz_annos);"
    ))?;
    for (table, column) in [("moz_places_extra", "place_id"), ("moz_inputhistory", "place_id"), ("moz_places_metadata", "place_id"), ("moz_places_metadata", "referrer_place_id")] {
        if has_table(db, table) && has_column(db, table, column) {
            db.execute(&format!("DELETE FROM {table} WHERE {column} IN (SELECT id FROM temp.{set}) AND {column} NOT IN (SELECT id FROM moz_places)"), [])?;
        }
    }
    db.execute_batch("DELETE FROM moz_origins WHERE id NOT IN (SELECT origin_id FROM moz_places WHERE origin_id IS NOT NULL);")
}

pub fn clean_ff_places(db: &Connection, history: &HashSet<&str>, downloads: &HashSet<&str>) -> rusqlite::Result<()> {
    let dl = "SELECT n.id, p.url FROM moz_annos n JOIN moz_places p ON p.id = n.place_id
              WHERE n.anno_attribute_id IN (SELECT id FROM moz_anno_attributes WHERE name LIKE 'downloads/%')";
    mark(db, "gone_a", &matching(db, dl, downloads, site_of_url)?)?;
    mark(db, "touched", &ids(db, "SELECT DISTINCT place_id FROM moz_annos WHERE id IN (SELECT id FROM temp.gone_a)")?)?;
    drop_where(db, "moz_annos", "id", "gone_a")?;
    mark(db, "gone_p", &matching(db, "SELECT id, url FROM moz_places", history, site_of_url)?)?;
    mark(db, "gone_v", &ids(db, "SELECT id FROM moz_historyvisits WHERE place_id IN (SELECT id FROM temp.gone_p)")?)?;
    drop_where(db, "moz_historyvisits_extra", "visit_id", "gone_v")?;
    drop_where(db, "moz_historyvisits", "id", "gone_v")?;
    drop_where(db, "moz_inputhistory", "place_id", "gone_p")?;
    drop_where(db, "moz_places_metadata", "place_id", "gone_p")?;
    drop_where(db, "moz_places_metadata", "referrer_place_id", "gone_p")?;
    db.execute_batch(
        "UPDATE moz_places SET visit_count = 0, last_visit_date = NULL WHERE id IN (SELECT id FROM temp.gone_p);
         INSERT OR IGNORE INTO temp.touched SELECT id FROM temp.gone_p;",
    )?;
    for (column, value) in [("frecency", "0"), ("recalc_frecency", "1")] {
        if has_column(db, "moz_places", column) {
            db.execute(&format!("UPDATE moz_places SET {column} = {value} WHERE id IN (SELECT id FROM temp.gone_p)"), [])?;
        }
    }
    prune_places(db, "touched")
}

fn run_firefox(profile: &Path, req: &CleanRequest, backup: &mut Backup) -> Result<u64, String> {
    let mut freed = 0u64;
    let cookies = req.domains(Key::Cookies);
    let history = req.domains(Key::History);
    let downloads = req.domains(Key::Downloads);
    let storage = req.domains(Key::Storage);
    let site_cache = req.domains(Key::SiteCache);

    let path = profile.join("cookies.sqlite");
    if path.is_file() && !cookies.is_empty() {
        step("Копия куки", backup.copy_db(&path))?;
        let mut db = step("Куки", open(&path))?;
        step("Куки", in_tx(&mut db, |db| clean_ff_cookies(db, &cookies)))?;
    }

    let path = profile.join("places.sqlite");
    if path.is_file() && !(history.is_empty() && downloads.is_empty()) {
        step("Копия истории", backup.copy_db(&path))?;
        let mut db = step("История", open(&path))?;
        step("История", in_tx(&mut db, |db| clean_ff_places(db, &history, &downloads)))?;
    }

    for (domain, dir) in firefox::storage_dirs(profile) {
        let cache = dir.join("cache");
        if site_cache.contains(domain.as_str()) {
            freed += step("Кеш сайта", remove(&cache))?;
        }
        if !storage.contains(domain.as_str()) {
            continue;
        }
        if cache.exists() {
            for e in step("Хранилище", fs::read_dir(&dir))?.flatten() {
                let name = e.file_name();
                if name != "cache" && name != ".metadata-v2" {
                    freed += step("Хранилище", backup.take(&e.path()))?;
                }
            }
        } else {
            freed += step("Хранилище", backup.take(&dir))?;
        }
    }

    if req.profile.contains(&Key::Forms) {
        let path = profile.join("formhistory.sqlite");
        if path.is_file() {
            step("Копия форм", backup.copy_db(&path))?;
            let mut db = step("Формы", open(&path))?;
            step(
                "Формы",
                in_tx(&mut db, |db| {
                    for t in ["moz_history_to_sources", "moz_formhistory"] {
                        if has_table(db, t) {
                            db.execute(&format!("DELETE FROM {t}"), [])?;
                        }
                    }
                    Ok(())
                }),
            )?;
        }
    }

    let path = firefox::addresses_file(profile);
    if req.profile.contains(&Key::Addresses) && path.is_file() {
        step("Копия адресов", backup.copy_db(&path))?;
        let text = step("Адреса", fs::read_to_string(&path))?;
        let mut json: serde_json::Value = step("Адреса", serde_json::from_str(&text))?;
        if let Some(a) = json.get_mut("addresses") {
            *a = serde_json::Value::Array(vec![]);
        }
        step("Адреса", fs::write(&path, step("Адреса", serde_json::to_string(&json))?))?;
    }

    if req.profile.contains(&Key::BrowserCache) {
        for d in firefox::cache_dirs(profile) {
            freed += step("Кеш браузера", remove(&d))?;
        }
    }
    Ok(freed)
}

pub fn backup_dir(browser_id: &str, profile_id: &str) -> PathBuf {
    let safe: String = profile_id.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    backups_root().join(format!("{}-{n}-{browser_id}-{safe}", now_secs()))
}

pub fn sweep_stale() {
    sweep_backups(&backups_root(), now_secs());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cache-out-clean-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn count(db: &Path, sql: &str) -> i64 {
        Connection::open(db).unwrap().query_row(sql, [], |r| r.get(0)).unwrap()
    }

    fn fixture(p: &Path) {
        fs::create_dir_all(p.join("Network")).unwrap();
        Connection::open(p.join("Network").join("Cookies"))
            .unwrap()
            .execute_batch(
                "CREATE TABLE cookies (host_key TEXT, name TEXT, value TEXT);
                 INSERT INTO cookies VALUES ('.google.com','a',''), ('accounts.google.com','b',''), ('vk.com','c','');",
            )
            .unwrap();
        Connection::open(p.join("History"))
            .unwrap()
            .execute_batch(
                "CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, visit_count INTEGER, last_visit_time INTEGER);
                 CREATE TABLE visits (id INTEGER PRIMARY KEY, url INTEGER);
                 CREATE TABLE visit_source (id INTEGER PRIMARY KEY, source INTEGER);
                 CREATE TABLE keyword_search_terms (keyword_id INTEGER, url_id INTEGER, term TEXT);
                 CREATE TABLE segments (id INTEGER PRIMARY KEY, name TEXT, url_id INTEGER);
                 CREATE TABLE segment_usage (id INTEGER PRIMARY KEY, segment_id INTEGER);
                 CREATE TABLE downloads (id INTEGER PRIMARY KEY, tab_url TEXT, site_url TEXT, referrer TEXT);
                 CREATE TABLE downloads_url_chains (id INTEGER, chain_index INTEGER, url TEXT);
                 INSERT INTO urls VALUES (1,'https://www.google.com/search?q=x',2,0), (2,'https://vk.com/feed',1,0);
                 INSERT INTO visits VALUES (10,1), (11,1), (12,2);
                 INSERT INTO visit_source VALUES (10,0), (12,0);
                 INSERT INTO keyword_search_terms VALUES (1,1,'x');
                 INSERT INTO segments VALUES (5,'google',1), (6,'vk',2);
                 INSERT INTO segment_usage VALUES (50,5), (60,6);
                 INSERT INTO downloads VALUES (7,'','https://drive.google.com/f',''), (8,'https://vk.com/doc','','');
                 INSERT INTO downloads_url_chains VALUES (7,0,'https://x'), (8,0,'https://y');",
            )
            .unwrap();
        Connection::open(p.join("Web Data"))
            .unwrap()
            .execute_batch(
                "CREATE TABLE autofill (name TEXT, value TEXT);
                 CREATE TABLE addresses (guid TEXT);
                 CREATE TABLE address_type_tokens (guid TEXT, type INTEGER, value TEXT);
                 INSERT INTO autofill VALUES ('email','x');
                 INSERT INTO addresses VALUES ('1');
                 INSERT INTO address_type_tokens VALUES ('1',1,'x');",
            )
            .unwrap();
        let idb = p.join("IndexedDB");
        fs::create_dir_all(idb.join("https_mail.google.com_0.indexeddb.leveldb")).unwrap();
        fs::write(idb.join("https_mail.google.com_0.indexeddb.leveldb").join("000.log"), b"12345").unwrap();
        fs::create_dir_all(idb.join("https_vk.com_0.indexeddb.leveldb")).unwrap();
        let web = p.join("WebStorage");
        fs::create_dir_all(web.join("1").join("CacheStorage")).unwrap();
        fs::write(web.join("1").join("CacheStorage").join("c"), b"123").unwrap();
        fs::create_dir_all(web.join("1").join("IndexedDB")).unwrap();
        fs::write(web.join("1").join("IndexedDB").join("i"), b"1234").unwrap();
        fs::create_dir_all(web.join("2").join("IndexedDB")).unwrap();
        Connection::open(web.join("QuotaManager"))
            .unwrap()
            .execute_batch(
                "CREATE TABLE buckets (id INTEGER PRIMARY KEY, storage_key TEXT);
                 INSERT INTO buckets VALUES (1,'https://www.google.com/'), (2,'https://vk.com/'), (3,'https://mail.google.com/');",
            )
            .unwrap();
        let mut ls = rusty_leveldb::DB::open(local_storage_dir(p), rusty_leveldb::Options::default()).unwrap();
        ls.put(b"META:https://www.google.com", b"m").unwrap();
        ls.put(b"_https://www.google.com\x00\x01k", b"g").unwrap();
        ls.put(b"_https://vk.com\x00\x01k", b"v").unwrap();
        ls.close().unwrap();
        let sw = p.join("Service Worker").join("CacheStorage");
        fs::create_dir_all(sw.join("aa").join("c1")).unwrap();
        fs::write(sw.join("aa").join("index.txt"), b"\x0a\x00https://www.google.com/\x12").unwrap();
        fs::write(sw.join("aa").join("c1").join("d"), b"12").unwrap();
        fs::create_dir_all(sw.join("bb")).unwrap();
        fs::write(sw.join("bb").join("index.txt"), b"\x00https://vk.com/\x00").unwrap();
        fs::create_dir_all(p.join("Cache").join("Cache_Data")).unwrap();
        fs::write(p.join("Cache").join("Cache_Data").join("f"), b"1234567").unwrap();
    }

    fn req(sites: &[(&str, &[Key])], profile: &[Key]) -> CleanRequest {
        CleanRequest { sites: sites.iter().map(|(d, k)| (d.to_string(), k.to_vec())).collect(), profile: profile.to_vec() }
    }

    #[test]
    fn cleans_only_chosen_site_and_backs_up() {
        let root = temp("site");
        let p = root.join("Default");
        fixture(&p);
        let all = [Key::Cookies, Key::History, Key::Downloads, Key::Storage, Key::SiteCache];
        let report = clean_profile(&p, Family::Chromium, &req(&[("google.com", &all)], &[]), root.join("bk")).unwrap();

        assert_eq!(count(&p.join("Network").join("Cookies"), "SELECT COUNT(*) FROM cookies"), 1);
        let h = p.join("History");
        assert_eq!(count(&h, "SELECT COUNT(*) FROM urls"), 1);
        assert_eq!(count(&h, "SELECT COUNT(*) FROM visits"), 1);
        assert_eq!(count(&h, "SELECT COUNT(*) FROM visit_source"), 1);
        assert_eq!(count(&h, "SELECT COUNT(*) FROM keyword_search_terms"), 0);
        assert_eq!(count(&h, "SELECT COUNT(*) FROM segments"), 1);
        assert_eq!(count(&h, "SELECT COUNT(*) FROM segment_usage"), 1);
        assert_eq!(count(&h, "SELECT COUNT(*) FROM downloads"), 1);
        assert_eq!(count(&h, "SELECT COUNT(*) FROM downloads_url_chains"), 1);
        assert!(!p.join("IndexedDB").join("https_mail.google.com_0.indexeddb.leveldb").exists());
        assert!(p.join("IndexedDB").join("https_vk.com_0.indexeddb.leveldb").exists());
        assert!(!p.join("WebStorage").join("1").exists());
        assert!(p.join("WebStorage").join("2").exists());
        assert_eq!(count(&p.join("WebStorage").join("QuotaManager"), "SELECT COUNT(*) FROM buckets"), 1);
        assert_eq!(count(&p.join("Web Data"), "SELECT COUNT(*) FROM autofill"), 1);
        assert!(p.join("Cache").exists());
        assert!(!p.join("Service Worker").join("CacheStorage").join("aa").exists());
        assert!(p.join("Service Worker").join("CacheStorage").join("bb").exists());
        assert_eq!(report.freed_bytes, 12 + 28);
        let ls = localstorage::site_sizes(&local_storage_dir(&p)).unwrap();
        assert!(!ls.contains_key("google.com"));
        assert!(ls.contains_key("vk.com"));

        let bk = report.backup.unwrap();
        assert_eq!(count(&bk.join("Network").join("Cookies"), "SELECT COUNT(*) FROM cookies"), 3);
        assert_eq!(count(&bk.join("History"), "SELECT COUNT(*) FROM urls"), 2);
        assert!(bk.join("IndexedDB").join("https_mail.google.com_0.indexeddb.leveldb").join("000.log").exists());
        assert!(bk.join("WebStorage").join("1").join("IndexedDB").join("i").exists());
        assert!(!bk.join("WebStorage").join("1").join("CacheStorage").exists());
        assert!(localstorage::site_sizes(&local_storage_dir(&bk)).unwrap().contains_key("google.com"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn storage_without_site_cache_keeps_bucket() {
        let root = temp("storage");
        let p = root.join("Default");
        fixture(&p);
        clean_profile(&p, Family::Chromium, &req(&[("google.com", &[Key::Storage])], &[]), root.join("bk")).unwrap();
        let b1 = p.join("WebStorage").join("1");
        assert!(b1.join("CacheStorage").join("c").exists());
        assert!(p.join("Service Worker").join("CacheStorage").join("aa").exists());
        assert!(!b1.join("IndexedDB").exists());
        assert_eq!(count(&p.join("WebStorage").join("QuotaManager"), "SELECT SUM(id) FROM buckets"), 3);
        assert_eq!(count(&p.join("History"), "SELECT COUNT(*) FROM urls"), 2);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cleans_profile_wide_items() {
        let root = temp("profile");
        let p = root.join("Default");
        fixture(&p);
        let report = clean_profile(&p, Family::Chromium, &req(&[], &[Key::BrowserCache, Key::Forms, Key::Addresses]), root.join("bk")).unwrap();
        let w = p.join("Web Data");
        assert_eq!(count(&w, "SELECT COUNT(*) FROM autofill"), 0);
        assert_eq!(count(&w, "SELECT COUNT(*) FROM addresses"), 0);
        assert_eq!(count(&w, "SELECT COUNT(*) FROM address_type_tokens"), 0);
        assert!(!p.join("Cache").exists());
        assert_eq!(report.freed_bytes, 7);
        assert!(report.backup.unwrap().join("Web Data").exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn nothing_chosen_touches_nothing() {
        let root = temp("none");
        let p = root.join("Default");
        fixture(&p);
        let report = clean_profile(&p, Family::Chromium, &CleanRequest::default(), root.join("bk")).unwrap();
        assert_eq!(report.freed_bytes, 0);
        assert!(report.backup.is_none());
        assert!(!root.join("bk").exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn failure_midway_reports_backup() {
        let root = temp("fail");
        let p = root.join("Default");
        fixture(&p);
        fs::write(p.join("WebStorage").join("QuotaManager"), b"not a database at all, just junk bytes").unwrap();
        let err = clean_profile(&p, Family::Chromium, &req(&[("google.com", &[Key::Cookies, Key::SiteCache])], &[]), root.join("bk")).unwrap_err();
        assert!(err.touched);
        assert!(err.message.contains(&root.join("bk").display().to_string()));
        assert_eq!(count(&root.join("bk").join("Network").join("Cookies"), "SELECT COUNT(*) FROM cookies"), 3);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn firefox_history_keeps_bookmarks_and_downloads_until_asked() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE moz_origins (id INTEGER PRIMARY KEY);
             CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT, visit_count INTEGER, last_visit_date INTEGER, foreign_count INTEGER, origin_id INTEGER, frecency INTEGER, recalc_frecency INTEGER);
             CREATE TABLE moz_historyvisits (id INTEGER PRIMARY KEY, place_id INTEGER);
             CREATE TABLE moz_anno_attributes (id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE moz_annos (id INTEGER PRIMARY KEY, place_id INTEGER, anno_attribute_id INTEGER);
             CREATE TABLE moz_inputhistory (place_id INTEGER, input TEXT);
             INSERT INTO moz_origins VALUES (1), (2);
             INSERT INTO moz_places VALUES (1,'https://2ch.org/a',2,1,0,1,10,0), (2,'https://2ch.org/bm',1,1,1,1,10,0),
               (3,'https://2ch.org/f.mp4',0,NULL,0,1,0,0), (4,'https://vk.com/',1,1,0,2,10,0);
             INSERT INTO moz_historyvisits VALUES (10,1), (11,2), (12,4);
             INSERT INTO moz_anno_attributes VALUES (1,'downloads/destinationFileURI');
             INSERT INTO moz_annos VALUES (1,3,1);
             INSERT INTO moz_inputhistory VALUES (1,'2ch');
             CREATE TABLE moz_places_metadata (id INTEGER PRIMARY KEY, place_id INTEGER, referrer_place_id INTEGER);
             INSERT INTO moz_places_metadata VALUES (1,2,NULL), (2,4,1), (3,4,NULL);",
        )
        .unwrap();
        let q = |sql: &str| -> String { db.query_row(sql, [], |r| r.get(0)).unwrap() };
        clean_ff_places(&db, &HashSet::from(["2ch.org"]), &HashSet::new()).unwrap();
        assert_eq!(q("SELECT group_concat(id) FROM moz_places"), "2,3,4");
        assert_eq!(q("SELECT group_concat(id) FROM moz_historyvisits"), "12");
        assert_eq!(q("SELECT visit_count || '/' || ifnull(last_visit_date, '-') FROM moz_places WHERE id = 2"), "0/-");
        assert_eq!(q("SELECT count(*) || '' FROM moz_inputhistory"), "0");
        assert_eq!(q("SELECT group_concat(id) FROM moz_places_metadata"), "3");
        assert_eq!(q("SELECT group_concat(id) FROM moz_origins"), "1,2");
        clean_ff_places(&db, &HashSet::new(), &HashSet::from(["2ch.org"])).unwrap();
        assert_eq!(q("SELECT group_concat(id) FROM moz_places"), "2,4");
        assert_eq!(q("SELECT count(*) || '' FROM moz_annos"), "0");
        clean_ff_places(&db, &HashSet::from(["vk.com"]), &HashSet::new()).unwrap();
        assert_eq!(q("SELECT group_concat(id) FROM moz_origins"), "1");
    }

    #[test]
    fn firefox_storage_and_profile_items() {
        let root = temp("ff");
        let p = root.join("prof");
        let site = p.join("storage").join("default").join("https+++2ch.org");
        fs::create_dir_all(site.join("idb")).unwrap();
        fs::create_dir_all(site.join("cache")).unwrap();
        fs::write(site.join(".metadata-v2"), b"m").unwrap();
        fs::write(site.join("idb").join("x"), b"123").unwrap();
        fs::write(site.join("cache").join("y"), b"45").unwrap();
        let other = p.join("storage").join("permanent").join("https+++vk.com");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("z"), b"1").unwrap();
        Connection::open(p.join("formhistory.sqlite")).unwrap().execute_batch("CREATE TABLE moz_formhistory (fieldname TEXT, value TEXT); INSERT INTO moz_formhistory VALUES ('q','x');").unwrap();
        fs::write(firefox::addresses_file(&p), r#"{"version":1,"addresses":[{"guid":"a"}],"creditCards":[{"guid":"c"}]}"#).unwrap();

        let r = clean_profile(&p, Family::Firefox, &req(&[("2ch.org", &[Key::Storage])], &[Key::Forms, Key::Addresses]), root.join("bk")).unwrap();
        assert_eq!(r.freed_bytes, 3);
        assert!(!site.join("idb").exists());
        assert!(site.join("cache").join("y").exists());
        assert!(site.join(".metadata-v2").exists());
        assert!(other.exists());
        assert_eq!(count(&p.join("formhistory.sqlite"), "SELECT COUNT(*) FROM moz_formhistory"), 0);
        let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(firefox::addresses_file(&p)).unwrap()).unwrap();
        assert_eq!(json["addresses"].as_array().unwrap().len(), 0);
        assert_eq!(json["creditCards"].as_array().unwrap().len(), 1);
        let bk = r.backup.unwrap();
        assert!(bk.join("storage").join("default").join("https+++2ch.org").join("idb").join("x").exists());
        assert!(bk.join("autofill-profiles.json").exists());

        let r = clean_profile(&p, Family::Firefox, &req(&[("2ch.org", &[Key::Storage, Key::SiteCache])], &[]), root.join("bk2")).unwrap();
        assert_eq!(r.freed_bytes, 2 + 1);
        assert!(!site.exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn history_traces_are_removed_per_site() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE omni_box_shortcuts (id VARCHAR PRIMARY KEY, text VARCHAR, url VARCHAR);
             INSERT INTO omni_box_shortcuts VALUES ('a','jut','https://jutsu.love/x'), ('b','vk','https://vk.com/');
             CREATE TABLE top_sites (url TEXT PRIMARY KEY, url_rank INTEGER, title TEXT);
             INSERT INTO top_sites VALUES ('https://vk.com/',0,''), ('https://www.jutsu.love/',1,''), ('https://ya.ru/',2,'');
             CREATE TABLE icon_mapping (id INTEGER PRIMARY KEY, page_url TEXT, icon_id INTEGER);
             CREATE TABLE favicons (id INTEGER PRIMARY KEY, url TEXT);
             CREATE TABLE favicon_bitmaps (id INTEGER PRIMARY KEY, icon_id INTEGER);
             INSERT INTO icon_mapping VALUES (1,'https://jutsu.love/a',10), (2,'https://vk.com/',20), (3,'https://jutsu.love/b',30), (4,'https://ya.ru/',30);
             INSERT INTO favicons VALUES (10,'j'), (20,'v'), (30,'shared');
             INSERT INTO favicon_bitmaps VALUES (1,10), (2,20), (3,30);",
        )
        .unwrap();
        let sites = HashSet::from(["jutsu.love"]);
        clean_shortcuts(&db, &sites).unwrap();
        clean_top_sites(&db, &sites).unwrap();
        clean_favicons(&db, &sites).unwrap();
        let q = |sql: &str| -> String { db.query_row(sql, [], |r| r.get(0)).unwrap() };
        assert_eq!(q("SELECT group_concat(id) FROM omni_box_shortcuts"), "b");
        assert_eq!(q("SELECT group_concat(url || url_rank, ' ') FROM (SELECT * FROM top_sites ORDER BY url_rank)"), "https://vk.com/0 https://ya.ru/1");
        assert_eq!(q("SELECT group_concat(id) FROM icon_mapping"), "2,4");
        assert_eq!(q("SELECT group_concat(id) FROM favicons"), "20,30");
        assert_eq!(q("SELECT group_concat(icon_id) FROM favicon_bitmaps"), "20,30");
    }

    #[test]
    fn backup_dirs_are_unique() {
        assert_ne!(backup_dir("chrome", "Default"), backup_dir("chrome", "Default"));
    }

    #[test]
    fn sweeps_backups_older_than_week() {
        let root = temp("sweep");
        fs::create_dir_all(root.join("100-chrome-Default")).unwrap();
        fs::create_dir_all(root.join(format!("{}-chrome-Default", 100 + KEEP_SECS))).unwrap();
        fs::create_dir_all(root.join("notes")).unwrap();
        sweep_backups(&root, 101 + KEEP_SECS);
        assert!(!root.join("100-chrome-Default").exists());
        assert!(root.join(format!("{}-chrome-Default", 100 + KEEP_SECS)).exists());
        assert!(root.join("notes").exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
