use crate::site::site_of_origin;
use rusty_leveldb::{LdbIterator, Options, WriteBatch, DB};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;

pub fn origin_of_key(key: &[u8]) -> Option<&[u8]> {
    if let Some(rest) = key.strip_prefix(b"_") {
        return Some(&rest[..rest.iter().position(|b| *b == 0)?]);
    }
    key.strip_prefix(b"META:").or_else(|| key.strip_prefix(b"METAACCESS:"))
}

fn site_of_key(key: &[u8]) -> Option<String> {
    site_of_origin(std::str::from_utf8(origin_of_key(key)?).ok()?)
}

fn open(dir: &Path) -> io::Result<DB> {
    DB::open(dir, Options { create_if_missing: false, ..Options::default() }).map_err(|e| io::Error::other(e.to_string()))
}

pub fn site_sizes(dir: &Path) -> io::Result<HashMap<String, u64>> {
    let mut db = open(dir)?;
    let mut it = db.new_iter().map_err(|e| io::Error::other(e.to_string()))?;
    let mut out: HashMap<String, u64> = HashMap::new();
    while let Some((k, v)) = it.next() {
        if k.starts_with(b"_") {
            if let Some(d) = site_of_key(&k) {
                *out.entry(d).or_default() += (k.len() + v.len()) as u64;
            }
        }
    }
    Ok(out)
}

pub fn delete_sites(dir: &Path, sites: &HashSet<&str>, before_write: impl FnOnce() -> io::Result<()>) -> io::Result<usize> {
    let mut db = open(dir)?;
    let mut keys = vec![];
    {
        let mut it = db.new_iter().map_err(|e| io::Error::other(e.to_string()))?;
        while let Some((k, _)) = it.next() {
            if site_of_key(&k).is_some_and(|d| sites.contains(d.as_str())) {
                keys.push(k);
            }
        }
    }
    if !keys.is_empty() {
        before_write()?;
        let mut batch = WriteBatch::default();
        for k in &keys {
            batch.delete(k);
        }
        db.write(batch, true).map_err(|e| io::Error::other(e.to_string()))?;
    }
    db.close().map_err(|e| io::Error::other(e.to_string()))?;
    Ok(keys.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_origin_from_keys() {
        assert_eq!(origin_of_key(b"_https://vk.com\x00\x01token"), Some(b"https://vk.com".as_slice()));
        assert_eq!(origin_of_key(b"META:https://vk.com"), Some(b"https://vk.com".as_slice()));
        assert_eq!(origin_of_key(b"METAACCESS:http://a.org/^0http://b.org"), Some(b"http://a.org/^0http://b.org".as_slice()));
        assert_eq!(origin_of_key(b"VERSION"), None);
        assert_eq!(site_of_key(b"METAACCESS:http://m.a.org/^0http://b.org").as_deref(), Some("a.org"));
    }

    #[test]
    fn sizes_and_deletes_by_site() {
        let dir = std::env::temp_dir().join(format!("cache-out-ls-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        {
            let mut db = DB::open(&dir, Options::default()).unwrap();
            db.put(b"VERSION", b"1").unwrap();
            db.put(b"META:https://www.google.com", b"m").unwrap();
            db.put(b"METAACCESS:https://www.google.com", b"a").unwrap();
            db.put(b"_https://www.google.com\x00\x01k", b"12345").unwrap();
            db.put(b"_https://mail.google.com\x00\x01k", b"1").unwrap();
            db.put(b"_https://vk.com\x00\x01k", b"xyz").unwrap();
            db.close().unwrap();
        }
        let sizes = site_sizes(&dir).unwrap();
        assert_eq!(sizes["google.com"], (26 + 5) + (27 + 1));
        assert_eq!(sizes["vk.com"], 18 + 3);
        let mut called = 0;
        assert_eq!(delete_sites(&dir, &HashSet::from(["nothing.org"]), || {
            called += 1;
            Ok(())
        }).unwrap(), 0);
        assert_eq!(called, 0);
        assert_eq!(delete_sites(&dir, &HashSet::from(["google.com"]), || {
            called += 1;
            Ok(())
        }).unwrap(), 4);
        assert_eq!(called, 1);
        let mut db = open(&dir).unwrap();
        assert!(db.get(b"_https://www.google.com\x00\x01k").is_none());
        assert!(db.get(b"META:https://www.google.com").is_none());
        assert_eq!(db.get(b"_https://vk.com\x00\x01k").as_deref(), Some(b"xyz".as_slice()));
        assert_eq!(db.get(b"VERSION").as_deref(), Some(b"1".as_slice()));
        drop(db);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
