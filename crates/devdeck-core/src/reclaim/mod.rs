//! Reclaim module (phase 2D): what can be recovered on this machine, read-only.
//! Dev processes (RAM), forgotten worktrees and regenerable folders (disk), a summary grouped by
//! project, and the one destructive core operation: deleting an allow-listed regenerable folder.
//! Removing a worktree and terminating a process reuse `worktrees::remove` / `process_info::kill`.

pub mod folders;
pub mod forgotten;
pub mod processes;
pub mod summary;

use crate::scope::normalize;
use std::path::Path;

/// True when `path` is `root` itself or lives under it (component-wise, so `/a/foo` does not
/// contain `/a/foobar`). Case-insensitive on Windows, like the rest of the scope logic.
pub(crate) fn is_within(root: &Path, path: &Path) -> bool {
    let (r, p) = (normalize(root), normalize(path));
    p == r || p.starts_with(&format!("{}/", r.trim_end_matches('/')))
}

/// Seconds since the Unix epoch (0 if the clock is before it).
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `YYYY-MM-DDTHH:MM:SSZ` for a Unix timestamp (UTC, civil-from-days algorithm).
pub fn iso_from_unix(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_is_component_wise_and_includes_itself() {
        assert!(is_within(Path::new("/a/foo"), Path::new("/a/foo")));
        assert!(is_within(Path::new("/a/foo"), Path::new("/a/foo/b/c")));
        assert!(!is_within(Path::new("/a/foo"), Path::new("/a/foobar")));
        assert!(!is_within(Path::new("/a/foo"), Path::new("/a")));
    }

    #[test]
    fn iso_from_unix_known_values() {
        assert_eq!(iso_from_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_from_unix(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(iso_from_unix(951_782_400), "2000-02-29T00:00:00Z");
    }
}
