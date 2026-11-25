//! Regenerable folders (`node_modules`, `target`, `.venv`, `dist`, package-manager caches):
//! search, sizing and the guarded permanent delete.
//!
//! Safety model (the only destructive code in Reclaim): a folder may be deleted only if, after
//! canonicalization, it is an allow-listed regenerable folder inside the configured root, or
//! exactly one of the known global caches. Links/junctions, the root itself, drive roots and the
//! home directory (or any ancestor of it) are always rejected.

use crate::error::{Error, Result};
use crate::fsutil::{dir_size, is_link_or_reparse};
use crate::project::{Project, ProjectRef};
use crate::scope::{self, Attributable};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use walkdir::WalkDir;

/// How deep below the root the search looks for regenerable folders.
pub const SEARCH_DEPTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FolderKind {
    #[serde(rename = "node_modules")]
    NodeModules,
    #[serde(rename = "target")]
    Target,
    #[serde(rename = ".venv")]
    Venv,
    #[serde(rename = "dist")]
    Dist,
    #[serde(rename = "npmCache")]
    NpmCache,
    #[serde(rename = "pnpmStore")]
    PnpmStore,
    #[serde(rename = "cargoRegistry")]
    CargoRegistry,
    #[serde(rename = "cargoGit")]
    CargoGit,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeavyFolder {
    pub path: PathBuf,
    pub kind: FolderKind,
    /// `None` until sized.
    pub disk_bytes: Option<u64>,
    /// Last modification of the folder (UTC ISO 8601), as an approximation of "last used".
    pub last_used_iso: Option<String>,
    pub project: Option<ProjectRef>,
}

impl Attributable for HeavyFolder {
    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }
    fn disk(&self) -> u64 {
        self.disk_bytes.unwrap_or(0)
    }
}

/// Known global cache roots (the folders themselves are the deletable unit).
#[derive(Debug, Clone, Default)]
pub struct GlobalCaches {
    pub entries: Vec<(FolderKind, PathBuf)>,
}

impl GlobalCaches {
    pub fn new(entries: Vec<(FolderKind, PathBuf)>) -> Self {
        Self { entries }
    }

    /// Default locations of the npm cache, pnpm store and cargo registry/git caches.
    pub fn from_env() -> Self {
        let var = |k: &str| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let mut entries = Vec::new();
        #[cfg(windows)]
        {
            if let Some(local) = var("LOCALAPPDATA") {
                entries.push((FolderKind::NpmCache, local.join("npm-cache")));
                entries.push((FolderKind::PnpmStore, local.join("pnpm").join("store")));
            }
        }
        #[cfg(not(windows))]
        {
            if let Some(home) = var("HOME") {
                entries.push((FolderKind::NpmCache, home.join(".npm")));
                entries.push((
                    FolderKind::PnpmStore,
                    home.join(".local").join("share").join("pnpm").join("store"),
                ));
            }
        }
        let cargo_home = var("CARGO_HOME").or_else(|| {
            var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(|h| h.join(".cargo"))
        });
        if let Some(c) = cargo_home {
            entries.push((FolderKind::CargoRegistry, c.join("registry")));
            entries.push((FolderKind::CargoGit, c.join("git")));
        }
        Self { entries }
    }

    fn kind_of(&self, canonical: &Path) -> Option<FolderKind> {
        let target = scope::normalize(canonical);
        self.entries.iter().find_map(|(k, p)| {
            let c = std::fs::canonicalize(p).ok()?;
            (scope::normalize(&c) == target).then_some(*k)
        })
    }
}

/// Home directory of the current user (used to protect it from deletion).
pub fn home_dir() -> Option<PathBuf> {
    let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn is_file(p: &Path) -> bool {
    std::fs::metadata(p).is_ok_and(|m| m.is_file())
}

/// Kind of an in-project regenerable folder, by name plus a marker that proves it is one:
/// `node_modules` always; `target` next to `Cargo.toml`; `dist` next to `package.json`;
/// `.venv` containing `pyvenv.cfg`.
pub fn classify_dir(path: &Path) -> Option<FolderKind> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    let parent = path.parent();
    match name.as_str() {
        "node_modules" => Some(FolderKind::NodeModules),
        "target" if parent.is_some_and(|p| is_file(&p.join("Cargo.toml"))) => {
            Some(FolderKind::Target)
        }
        "dist" if parent.is_some_and(|p| is_file(&p.join("package.json"))) => {
            Some(FolderKind::Dist)
        }
        ".venv" if is_file(&path.join("pyvenv.cfg")) => Some(FolderKind::Venv),
        _ => None,
    }
}

fn last_used(path: &Path) -> Option<String> {
    let secs = std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(super::iso_from_unix(secs as i64))
}

fn folder(path: PathBuf, kind: FolderKind) -> HeavyFolder {
    HeavyFolder {
        last_used_iso: last_used(&path),
        path,
        kind,
        disk_bytes: None,
        project: None,
    }
}

/// Finds regenerable folders under `root` without following links and without descending into
/// matches (nor into `.git`). Checks `cancel` between entries; `progress` receives the number of
/// folders found so far. Global caches that exist are appended. Sizes are not computed here.
pub fn find_folders(
    root: &Path,
    caches: &GlobalCaches,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<Vec<HeavyFolder>> {
    let mut found = Vec::new();
    let mut walker = WalkDir::new(root)
        .follow_links(false)
        .max_depth(SEARCH_DEPTH)
        .into_iter();
    while let Some(entry) = walker.next() {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::cancelled());
        }
        let Ok(entry) = entry else { continue };
        if entry.depth() == 0 || !entry.file_type().is_dir() {
            continue;
        }
        if is_link_or_reparse(entry.path()) {
            walker.skip_current_dir();
            continue;
        }
        if entry.file_name().eq_ignore_ascii_case(".git") {
            walker.skip_current_dir();
            continue;
        }
        if let Some(kind) = classify_dir(entry.path()) {
            found.push(folder(entry.path().to_path_buf(), kind));
            progress(found.len() as u64);
            walker.skip_current_dir();
        }
    }
    let base = found.len() as u64;
    found.extend(find_caches(caches, &mut |n| progress(base + n)));
    Ok(found)
}

/// The known global caches that exist on disk (not links), unsized. `progress` gets 1.. as found.
pub fn find_caches(caches: &GlobalCaches, progress: &mut dyn FnMut(u64)) -> Vec<HeavyFolder> {
    let mut found = Vec::new();
    for (kind, path) in &caches.entries {
        if path.is_dir() && !is_link_or_reparse(path) {
            found.push(folder(path.clone(), *kind));
            progress(found.len() as u64);
        }
    }
    found
}

/// Fills `project` for every folder from the known projects.
pub fn attach_projects(folders: &mut [HeavyFolder], projects: &[Project]) {
    for f in folders {
        f.project = scope::attribute(&f.path, projects)
            .and_then(|id| projects.iter().find(|p| p.id == id))
            .map(Project::to_ref);
    }
}

/// Sizes every folder with `fsutil::dir_size` (cancelable). `progress(done, total)` is called
/// after each folder. A folder that cannot be sized keeps `disk_bytes = None`.
pub fn size_folders(
    folders: &mut [HeavyFolder],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<()> {
    let total = folders.len() as u64;
    for (i, f) in folders.iter_mut().enumerate() {
        f.disk_bytes = match dir_size(&f.path, cancel, &mut |_| {}) {
            Ok(n) => Some(n),
            Err(e) if e.kind == crate::error::ErrorKind::Cancelled => return Err(e),
            Err(_) => None,
        };
        progress(i as u64 + 1, total);
    }
    Ok(())
}

fn reject(reason: &str) -> Error {
    Error::not_allowed("folder_not_allowed").with_param("reason", reason)
}

/// A folder that passed validation: its canonical path and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deletable {
    pub path: PathBuf,
    pub kind: FolderKind,
}

/// Decides whether `path` may be permanently deleted (see the module docs). Rejections are
/// `folder_not_allowed` with a `reason` param: `link`, `not_a_directory`, `drive_root`, `home`,
/// `root`, `outside_root`, `not_allowlisted`. A missing path is `not_found`.
pub fn validate_deletable(
    path: &Path,
    root: Option<&Path>,
    caches: &GlobalCaches,
    home: Option<&Path>,
) -> Result<Deletable> {
    let md = std::fs::symlink_metadata(path)?;
    if is_link_or_reparse(path) {
        return Err(reject("link"));
    }
    if !md.is_dir() {
        return Err(reject("not_a_directory"));
    }
    let canon = std::fs::canonicalize(path)?;
    if canon.parent().is_none() {
        return Err(reject("drive_root"));
    }
    if is_link_or_reparse(&canon) {
        return Err(reject("link"));
    }
    if let Some(h) = home {
        let h = std::fs::canonicalize(h).unwrap_or_else(|_| h.to_path_buf());
        // The home itself and every ancestor of it (`C:\Users`, `C:\`).
        if super::is_within(&canon, &h) {
            return Err(reject("home"));
        }
    }
    let canon_root = root.and_then(|r| std::fs::canonicalize(r).ok());
    if let Some(r) = &canon_root {
        if scope::normalize(r) == scope::normalize(&canon) {
            return Err(reject("root"));
        }
    }
    if let Some(kind) = caches.kind_of(&canon) {
        return Ok(Deletable { path: canon, kind });
    }
    match &canon_root {
        Some(r) if super::is_within(r, &canon) => {}
        _ => return Err(reject("outside_root")),
    }
    match classify_dir(&canon) {
        Some(kind) => Ok(Deletable { path: canon, kind }),
        None => Err(reject("not_allowlisted")),
    }
}

/// Validates and permanently deletes the folder; returns the bytes freed (size before deleting).
pub fn delete_folder(
    path: &Path,
    root: Option<&Path>,
    caches: &GlobalCaches,
    home: Option<&Path>,
) -> Result<u64> {
    let target = validate_deletable(path, root, caches, home)?;
    let freed = dir_size(&target.path, &AtomicBool::new(false), &mut |_| {}).unwrap_or(0);
    std::fs::remove_dir_all(&target.path)?;
    Ok(freed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(p: &Path, bytes: usize) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, vec![0u8; bytes]).unwrap();
    }

    /// Creates a directory link (symlink on Unix, junction on Windows). False if not possible.
    fn make_link(link: &Path, target: &Path) -> bool {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
        #[cfg(windows)]
        {
            std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .output()
                .is_ok_and(|o| o.status.success())
        }
    }

    struct Env {
        _t: tempfile::TempDir,
        root: PathBuf,
        outside: PathBuf,
        home: PathBuf,
    }

    fn env() -> Env {
        let t = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(t.path()).unwrap();
        let (root, outside, home) = (base.join("root"), base.join("outside"), base.join("home"));
        for d in [&root, &outside, &home] {
            fs::create_dir_all(d).unwrap();
        }
        Env {
            _t: t,
            root,
            outside,
            home,
        }
    }

    fn none() -> GlobalCaches {
        GlobalCaches::default()
    }

    fn reason(e: &Error) -> String {
        assert_eq!(e.code, "folder_not_allowed", "{e:?}");
        e.params.as_ref().unwrap()["reason"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn validate(e: &Env, p: &Path) -> Result<Deletable> {
        validate_deletable(p, Some(&e.root), &none(), Some(&e.home))
    }

    #[test]
    fn classifies_by_name_and_marker() {
        let e = env();
        let nm = e.root.join("a/node_modules");
        write(&nm.join("x/index.js"), 1);
        assert_eq!(classify_dir(&nm), Some(FolderKind::NodeModules));

        let target = e.root.join("rs/target");
        write(&target.join("debug/x"), 1);
        assert_eq!(classify_dir(&target), None, "no Cargo.toml next to it");
        write(&e.root.join("rs/Cargo.toml"), 1);
        assert_eq!(classify_dir(&target), Some(FolderKind::Target));

        let dist = e.root.join("web/dist");
        write(&dist.join("index.html"), 1);
        assert_eq!(classify_dir(&dist), None, "no package.json next to it");
        write(&e.root.join("web/package.json"), 1);
        assert_eq!(classify_dir(&dist), Some(FolderKind::Dist));

        let venv = e.root.join("py/.venv");
        write(&venv.join("lib/x"), 1);
        assert_eq!(classify_dir(&venv), None, "no pyvenv.cfg inside");
        write(&venv.join("pyvenv.cfg"), 1);
        assert_eq!(classify_dir(&venv), Some(FolderKind::Venv));

        assert_eq!(classify_dir(&e.root.join("src")), None);
    }

    #[test]
    fn find_does_not_descend_into_matches_nor_git_and_skips_unproven_names() {
        let e = env();
        write(&e.root.join("app/node_modules/a/node_modules/deep.js"), 10);
        write(&e.root.join("app/node_modules/a.js"), 10);
        write(&e.root.join("app/package.json"), 1);
        write(&e.root.join("app/dist/out.js"), 1);
        write(&e.root.join("lib/target/debug/x"), 1);
        write(&e.root.join("lib/src/target/y"), 1);
        write(&e.root.join("lib/.git/node_modules/z"), 1);
        let mut seen = 0;
        let found =
            find_folders(&e.root, &none(), &AtomicBool::new(false), &mut |n| seen = n).unwrap();
        let mut names: Vec<String> = found
            .iter()
            .map(|f| {
                f.path
                    .strip_prefix(&e.root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        names.sort();
        assert_eq!(names, ["app/dist", "app/node_modules"]);
        assert_eq!(seen, 2);
        assert!(found.iter().all(|f| f.disk_bytes.is_none()));
        assert!(found[0].last_used_iso.as_ref().unwrap().ends_with('Z'));
    }

    #[test]
    fn find_does_not_follow_links_and_is_cancelable() {
        let e = env();
        write(&e.outside.join("node_modules/x"), 5);
        write(&e.root.join("keep/node_modules/y"), 5);
        if make_link(&e.root.join("linked"), &e.outside) {
            let found =
                find_folders(&e.root, &none(), &AtomicBool::new(false), &mut |_| {}).unwrap();
            assert_eq!(found.len(), 1);
            assert!(found[0].path.ends_with("node_modules"));
            assert!(found[0].path.starts_with(e.root.join("keep")));
        }
        let err = find_folders(&e.root, &none(), &AtomicBool::new(true), &mut |_| {}).unwrap_err();
        assert_eq!(err.code, "cancelled");
    }

    #[test]
    fn find_appends_existing_global_caches_only() {
        let e = env();
        write(&e.outside.join("npm-cache/_cacache/f"), 5);
        let caches = GlobalCaches::new(vec![
            (FolderKind::NpmCache, e.outside.join("npm-cache")),
            (FolderKind::PnpmStore, e.outside.join("missing-store")),
        ]);
        let found = find_folders(&e.root, &caches, &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, FolderKind::NpmCache);
    }

    #[test]
    fn sizes_with_dir_size_report_progress_and_cancel() {
        let e = env();
        write(&e.root.join("a/node_modules/x.bin"), 100);
        write(&e.root.join("a/node_modules/n/y.bin"), 50);
        write(&e.root.join("b/node_modules/z.bin"), 7);
        let mut found =
            find_folders(&e.root, &none(), &AtomicBool::new(false), &mut |_| {}).unwrap();
        let mut ticks = Vec::new();
        size_folders(&mut found, &AtomicBool::new(false), &mut |d, t| {
            ticks.push((d, t))
        })
        .unwrap();
        let mut sizes: Vec<u64> = found.iter().map(|f| f.disk_bytes.unwrap()).collect();
        sizes.sort();
        assert_eq!(sizes, [7, 150]);
        assert_eq!(ticks, [(1, 2), (2, 2)]);
        let err = size_folders(&mut found, &AtomicBool::new(true), &mut |_, _| {}).unwrap_err();
        assert_eq!(err.code, "cancelled");
    }

    #[test]
    fn attaches_projects_by_path() {
        let e = env();
        let main = e.root.join("app");
        write(&main.join("node_modules/x"), 1);
        let project = Project {
            id: crate::project::ProjectId::from_path(&main),
            name: "app".into(),
            main_path: main,
            worktree_paths: vec![],
        };
        let mut found =
            find_folders(&e.root, &none(), &AtomicBool::new(false), &mut |_| {}).unwrap();
        attach_projects(&mut found, &[project]);
        assert_eq!(found[0].project.as_ref().unwrap().name, "app");
    }

    // ---- validate_deletable: acceptance ----

    #[test]
    fn accepts_allowlisted_folders_inside_the_root() {
        let e = env();
        write(&e.root.join("a/node_modules/x"), 1);
        write(&e.root.join("a/Cargo.toml"), 1);
        write(&e.root.join("a/target/x"), 1);
        let d = validate(&e, &e.root.join("a/node_modules")).unwrap();
        assert_eq!(d.kind, FolderKind::NodeModules);
        assert_eq!(
            validate(&e, &e.root.join("a/target")).unwrap().kind,
            FolderKind::Target
        );
    }

    #[test]
    fn accepts_a_known_global_cache_exactly_even_outside_the_root_or_without_root() {
        let e = env();
        write(&e.outside.join("store/v3/f"), 1);
        let caches = GlobalCaches::new(vec![(FolderKind::PnpmStore, e.outside.join("store"))]);
        let d = validate_deletable(
            &e.outside.join("store"),
            Some(&e.root),
            &caches,
            Some(&e.home),
        )
        .unwrap();
        assert_eq!(d.kind, FolderKind::PnpmStore);
        let d = validate_deletable(&e.outside.join("store"), None, &caches, None).unwrap();
        assert_eq!(d.kind, FolderKind::PnpmStore);
        // a subfolder of a cache is not itself a cache
        let sub = validate_deletable(&e.outside.join("store/v3"), None, &caches, None).unwrap_err();
        assert_eq!(reason(&sub), "outside_root");
    }

    // ---- validate_deletable: rejections ----

    #[test]
    fn rejects_the_root_itself_even_if_it_is_allowlisted_by_name() {
        let e = env();
        assert_eq!(reason(&validate(&e, &e.root).unwrap_err()), "root");
        let nm_root = e.root.join("node_modules");
        fs::create_dir_all(&nm_root).unwrap();
        let err = validate_deletable(&nm_root, Some(&nm_root), &none(), Some(&e.home)).unwrap_err();
        assert_eq!(reason(&err), "root");
        // the root spelled with a trailing component trick
        let tricky = e.root.join("sub").join("..");
        fs::create_dir_all(e.root.join("sub")).unwrap();
        assert_eq!(reason(&validate(&e, &tricky).unwrap_err()), "root");
    }

    #[test]
    fn rejects_drive_roots() {
        let e = env();
        let drive_root = e.root.ancestors().last().unwrap().to_path_buf();
        let err = validate_deletable(&drive_root, Some(&drive_root), &none(), None).unwrap_err();
        assert_eq!(reason(&err), "drive_root");
        let err = validate_deletable(&drive_root, Some(&e.root), &none(), None).unwrap_err();
        assert_eq!(reason(&err), "drive_root");
    }

    #[test]
    fn rejects_the_home_dir_and_its_ancestors_even_when_named_like_the_allowlist() {
        let e = env();
        assert_eq!(reason(&validate(&e, &e.home).unwrap_err()), "home");
        // an ancestor of home
        let base = e.home.parent().unwrap();
        let err = validate_deletable(base, Some(&e.root), &none(), Some(&e.home)).unwrap_err();
        assert_eq!(reason(&err), "home");
        // home that is itself called node_modules and sits inside the root
        let weird_home = e.root.join("node_modules");
        fs::create_dir_all(&weird_home).unwrap();
        let err =
            validate_deletable(&weird_home, Some(&e.root), &none(), Some(&weird_home)).unwrap_err();
        assert_eq!(reason(&err), "home");
    }

    #[test]
    fn rejects_folders_that_are_not_allowlisted() {
        let e = env();
        for name in [
            "src",
            ".git",
            "build",
            "Documents",
            "node_modules_old",
            "out",
        ] {
            write(&e.root.join("app").join(name).join("f"), 1);
            let err = validate(&e, &e.root.join("app").join(name)).unwrap_err();
            assert_eq!(reason(&err), "not_allowlisted", "{name}");
        }
        // names that need a marker
        write(&e.root.join("rs/target/x"), 1);
        write(&e.root.join("web/dist/x"), 1);
        write(&e.root.join("py/.venv/x"), 1);
        for p in ["rs/target", "web/dist", "py/.venv"] {
            let err = validate(&e, &e.root.join(p)).unwrap_err();
            assert_eq!(reason(&err), "not_allowlisted", "{p}");
        }
        // `app` itself and its parent
        assert_eq!(
            reason(&validate(&e, &e.root.join("app")).unwrap_err()),
            "not_allowlisted"
        );
    }

    #[test]
    fn rejects_files_and_missing_paths() {
        let e = env();
        write(&e.root.join("a/node_modules"), 3);
        assert_eq!(
            reason(&validate(&e, &e.root.join("a/node_modules")).unwrap_err()),
            "not_a_directory"
        );
        let err = validate(&e, &e.root.join("nope/node_modules")).unwrap_err();
        assert_eq!(err.code, "not_found");
    }

    #[test]
    fn rejects_allowlisted_names_outside_the_root_and_without_a_root() {
        let e = env();
        write(&e.outside.join("node_modules/x"), 1);
        let err = validate(&e, &e.outside.join("node_modules")).unwrap_err();
        assert_eq!(reason(&err), "outside_root");
        write(&e.root.join("node_modules/x"), 1);
        let err =
            validate_deletable(&e.root.join("node_modules"), None, &none(), None).unwrap_err();
        assert_eq!(reason(&err), "outside_root");
        // a sibling that merely shares the root as a name prefix
        let sibling = e.root.parent().unwrap().join("root2/node_modules");
        write(&sibling.join("x"), 1);
        assert_eq!(reason(&validate(&e, &sibling).unwrap_err()), "outside_root");
    }

    #[test]
    fn rejects_dot_dot_traversal_out_of_the_root_or_onto_a_non_allowlisted_folder() {
        let e = env();
        write(&e.root.join("src/x"), 1);
        write(&e.root.join("node_modules/x"), 1);
        write(&e.outside.join("node_modules/x"), 1);
        let to_src = e.root.join("node_modules").join("..").join("src");
        assert_eq!(
            reason(&validate(&e, &to_src).unwrap_err()),
            "not_allowlisted"
        );
        let escape = e
            .root
            .join("src")
            .join("..")
            .join("..")
            .join("outside")
            .join("node_modules");
        assert_eq!(reason(&validate(&e, &escape).unwrap_err()), "outside_root");
        assert!(e.outside.join("node_modules/x").exists());
    }

    #[test]
    fn rejects_links_and_junctions_without_touching_their_target() {
        let e = env();
        write(&e.outside.join("keepme/node_modules/x"), 1);
        // the link itself is named like an allow-listed folder
        let link = e.root.join("node_modules");
        if !make_link(&link, &e.outside.join("keepme/node_modules")) {
            eprintln!("cannot create links here: skipping");
            return;
        }
        assert_eq!(reason(&validate(&e, &link).unwrap_err()), "link");
        let err = delete_folder(&link, Some(&e.root), &none(), Some(&e.home)).unwrap_err();
        assert_eq!(reason(&err), "link");
        assert!(e.outside.join("keepme/node_modules/x").exists());
    }

    #[test]
    fn rejects_a_path_that_goes_through_a_link_to_the_outside() {
        let e = env();
        write(&e.outside.join("node_modules/x"), 1);
        if !make_link(&e.root.join("via"), &e.outside) {
            eprintln!("cannot create links here: skipping");
            return;
        }
        let err = validate(&e, &e.root.join("via/node_modules")).unwrap_err();
        assert_eq!(reason(&err), "outside_root");
        assert!(e.outside.join("node_modules/x").exists());
    }

    // ---- delete_folder ----

    #[test]
    fn deletes_and_reports_freed_bytes() {
        let e = env();
        write(&e.root.join("a/node_modules/x.bin"), 100);
        write(&e.root.join("a/node_modules/n/y.bin"), 28);
        write(&e.root.join("a/keep.txt"), 1);
        let freed = delete_folder(
            &e.root.join("a/node_modules"),
            Some(&e.root),
            &none(),
            Some(&e.home),
        )
        .unwrap();
        assert_eq!(freed, 128);
        assert!(!e.root.join("a/node_modules").exists());
        assert!(e.root.join("a/keep.txt").exists());
    }

    #[test]
    fn a_rejected_delete_leaves_everything_in_place() {
        let e = env();
        write(&e.root.join("src/main.rs"), 1);
        let err =
            delete_folder(&e.root.join("src"), Some(&e.root), &none(), Some(&e.home)).unwrap_err();
        assert_eq!(reason(&err), "not_allowlisted");
        assert!(e.root.join("src/main.rs").exists());
    }

    #[test]
    fn deleting_a_folder_with_an_inner_link_does_not_follow_it() {
        let e = env();
        write(&e.outside.join("precious/data.bin"), 10);
        write(&e.root.join("a/node_modules/x"), 1);
        if !make_link(
            &e.root.join("a/node_modules/linked"),
            &e.outside.join("precious"),
        ) {
            eprintln!("cannot create links here: skipping");
            return;
        }
        delete_folder(
            &e.root.join("a/node_modules"),
            Some(&e.root),
            &none(),
            Some(&e.home),
        )
        .unwrap();
        assert!(!e.root.join("a/node_modules").exists());
        assert!(e.outside.join("precious/data.bin").exists());
    }

    #[test]
    fn deletes_a_global_cache() {
        let e = env();
        write(&e.outside.join("registry/crate/a"), 9);
        let caches = GlobalCaches::new(vec![(
            FolderKind::CargoRegistry,
            e.outside.join("registry"),
        )]);
        let freed = delete_folder(
            &e.outside.join("registry"),
            Some(&e.root),
            &caches,
            Some(&e.home),
        )
        .unwrap();
        assert_eq!(freed, 9);
        assert!(!e.outside.join("registry").exists());
    }

    #[test]
    fn folder_kinds_serialize_as_the_contract_names() {
        let names: Vec<_> = [
            FolderKind::NodeModules,
            FolderKind::Target,
            FolderKind::Venv,
            FolderKind::Dist,
            FolderKind::NpmCache,
            FolderKind::PnpmStore,
            FolderKind::CargoRegistry,
            FolderKind::CargoGit,
        ]
        .iter()
        .map(|k| serde_json::to_value(k).unwrap())
        .collect();
        assert_eq!(
            names,
            [
                "node_modules",
                "target",
                ".venv",
                "dist",
                "npmCache",
                "pnpmStore",
                "cargoRegistry",
                "cargoGit"
            ]
        );
    }
}
