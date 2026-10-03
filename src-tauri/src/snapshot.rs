use crate::vss::{self, Shadow};
use std::sync::Arc;
use std::cell::OnceCell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT: AtomicU32 = AtomicU32::new(0);

pub struct Snapshot {
    dir: PathBuf,
    shadow: OnceCell<Option<Arc<Shadow>>>,
}

fn is_locked(e: &io::Error) -> bool {
    matches!(e.raw_os_error(), Some(32 | 33))
}

impl Snapshot {
    pub fn new() -> io::Result<Self> {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join("cache-out").join(format!("{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir)?;
        Ok(Self { dir, shadow: OnceCell::new() })
    }

    pub fn copy_db(&self, src: &Path) -> io::Result<PathBuf> {
        self.copy_into(src, &self.dir)
    }

    pub fn copy_dir(&self, src: &Path, name: &str) -> io::Result<PathBuf> {
        let dst = self.dir.join(name);
        fs::create_dir_all(&dst)?;
        for e in fs::read_dir(src)?.flatten() {
            if e.file_type()?.is_file() && e.file_name() != "LOCK" {
                self.copy_into(&e.path(), &dst)?;
            }
        }
        Ok(dst)
    }

    fn copy_into(&self, src: &Path, dir: &Path) -> io::Result<PathBuf> {
        match self.copy_from(src, src, dir) {
            Err(e) if is_locked(&e) => {
                let shadow = self.shadow.get_or_init(|| vss::volume_of(src).and_then(|v| vss::shared(&v)));
                let from = shadow.as_ref().filter(|s| s.covers(src)).and_then(|s| s.path(src)).ok_or(e)?;
                self.copy_from(&from, src, dir)
            }
            r => r,
        }
    }

    pub fn used_shadow(&self) -> bool {
        matches!(self.shadow.get(), Some(Some(_)))
    }

    fn copy_from(&self, from: &Path, original: &Path, dir: &Path) -> io::Result<PathBuf> {
        let name = original.file_name().ok_or_else(|| io::Error::other("bad path"))?;
        let dst = dir.join(name);
        fs::copy(from, &dst)?;
        for ext in ["-wal", "-journal"] {
            let side = PathBuf::from(format!("{}{}", from.display(), ext));
            if side.exists() {
                fs::copy(&side, format!("{}{}", dst.display(), ext))?;
            }
        }
        Ok(dst)
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

pub fn sweep_stale() {
    let root = std::env::temp_dir().join("cache-out");
    let me = format!("{}-", std::process::id());
    if let Ok(entries) = fs::read_dir(&root) {
        for e in entries.flatten() {
            if !e.file_name().to_string_lossy().starts_with(&me) {
                let _ = fs::remove_dir_all(e.path());
            }
        }
    }
}

pub fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else { return 0 };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(_) => e.metadata().map(|m| m.len()).unwrap_or(0),
            Err(_) => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_with_sidecars_and_cleans_up() {
        let src_dir = std::env::temp_dir().join(format!("cache-out-test-{}", std::process::id()));
        fs::create_dir_all(src_dir.join("sub")).unwrap();
        fs::write(src_dir.join("History"), b"main").unwrap();
        fs::write(src_dir.join("History-wal"), b"wal").unwrap();
        fs::write(src_dir.join("sub").join("x"), b"12345").unwrap();
        let snap = Snapshot::new().unwrap();
        let copy = snap.copy_db(&src_dir.join("History")).unwrap();
        assert_eq!(fs::read(&copy).unwrap(), b"main");
        assert!(PathBuf::from(format!("{}-wal", copy.display())).exists());
        assert_eq!(dir_size(&src_dir), 12);
        let dir = snap.dir.clone();
        drop(snap);
        assert!(!dir.exists());
        fs::remove_dir_all(&src_dir).unwrap();
    }
}
