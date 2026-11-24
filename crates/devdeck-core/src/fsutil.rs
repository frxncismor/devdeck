//! Filesystem helpers: link/junction detection and cancelable directory sizing.

use crate::error::{Error, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use walkdir::WalkDir;

/// True for symlinks and, on Windows, any reparse point (junctions, mount points...).
/// A path that cannot be inspected is reported as a link (fail closed).
pub fn is_link_or_reparse(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(md) => meta_is_link(&md),
        Err(_) => true,
    }
}

fn meta_is_link(md: &std::fs::Metadata) -> bool {
    if md.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if md.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Total size in bytes of the regular files under `path`, without following or counting links
/// (nor entering linked directories). `progress` receives the running total. Checks `cancel`
/// between entries and returns a `cancelled` error when set.
pub fn dir_size(path: &Path, cancel: &AtomicBool, progress: &mut dyn FnMut(u64)) -> Result<u64> {
    let mut total = 0u64;
    let mut walker = WalkDir::new(path).follow_links(false).into_iter();
    while let Some(entry) = walker.next() {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::cancelled());
        }
        let Ok(entry) = entry else { continue };
        let Ok(md) = entry.metadata() else { continue };
        if entry.depth() > 0 && meta_is_link(&md) {
            if md.is_dir() {
                walker.skip_current_dir();
            }
            continue;
        }
        if md.is_file() {
            total += md.len();
            progress(total);
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tree() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("a/b")).unwrap();
        fs::write(d.path().join("a/one.bin"), vec![0u8; 100]).unwrap();
        fs::write(d.path().join("a/b/two.bin"), vec![0u8; 250]).unwrap();
        fs::write(d.path().join("root.bin"), vec![0u8; 50]).unwrap();
        d
    }

    #[test]
    fn sums_regular_files_and_reports_progress() {
        let d = tree();
        let mut last = 0;
        let size = dir_size(d.path(), &AtomicBool::new(false), &mut |n| last = n).unwrap();
        assert_eq!(size, 400);
        assert_eq!(last, 400);
    }

    #[test]
    fn cancel_returns_cancelled_error() {
        let d = tree();
        let e = dir_size(d.path(), &AtomicBool::new(true), &mut |_| {}).unwrap_err();
        assert_eq!(e.code, "cancelled");
    }

    #[test]
    fn plain_dir_is_not_a_link() {
        let d = tree();
        assert!(!is_link_or_reparse(d.path()));
        assert!(!is_link_or_reparse(&d.path().join("root.bin")));
    }

    #[test]
    fn missing_path_fails_closed() {
        let d = tree();
        assert!(is_link_or_reparse(&d.path().join("nope")));
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed_or_counted() {
        let d = tree();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("big.bin"), vec![0u8; 10_000]).unwrap();
        std::os::unix::fs::symlink(outside.path(), d.path().join("link")).unwrap();
        assert!(is_link_or_reparse(&d.path().join("link")));
        let size = dir_size(d.path(), &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(size, 400);
    }

    #[cfg(windows)]
    #[test]
    fn junctions_are_not_followed_or_counted() {
        let d = tree();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("big.bin"), vec![0u8; 10_000]).unwrap();
        let link = d.path().join("junction");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .output()
            .unwrap();
        assert!(status.status.success(), "mklink failed");
        assert!(is_link_or_reparse(&link));
        let size = dir_size(d.path(), &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(size, 400);
    }
}
