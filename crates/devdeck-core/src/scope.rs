//! Global / project scope: attribution of paths to projects, grouping and totals.

use crate::project::{Project, ProjectId, ProjectRef};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Scope {
    Global,
    Project { id: ProjectId },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub count: u64,
    pub ram_bytes: u64,
    pub disk_bytes: u64,
}

impl Totals {
    fn add(&mut self, ram: u64, disk: u64) {
        self.count += 1;
        self.ram_bytes += ram;
        self.disk_bytes += disk;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group<T> {
    /// `None` = "No project" (label rendered by the frontend).
    pub project: Option<ProjectRef>,
    pub items: Vec<T>,
    pub totals: Totals,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Grouped<T> {
    pub groups: Vec<Group<T>>,
    pub total: Totals,
}

/// Anything that can be attributed to a project by path and counted in totals.
pub trait Attributable {
    fn path(&self) -> Option<&Path>;
    fn ram(&self) -> u64 {
        0
    }
    fn disk(&self) -> u64 {
        0
    }
}

/// Normalizes a path into `/`-separated form: strips the `\\?\` prefix, converts separators,
/// lowercases on Windows and drops a trailing separator.
pub fn normalize(path: &Path) -> String {
    normalize_with(path, cfg!(windows))
}

pub(crate) fn normalize_with(path: &Path, case_insensitive: bool) -> String {
    let raw = path.to_string_lossy();
    let raw = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
    let mut s = raw.replace('\\', "/");
    if case_insensitive {
        s = s.to_lowercase();
    }
    while s.len() > 1 && s.ends_with('/') {
        s.pop();
    }
    s
}

fn components(s: &str) -> Vec<&str> {
    s.split('/').filter(|c| !c.is_empty()).collect()
}

/// True if `root` is a component-wise prefix of `path` (`/a/foo` does not own `/a/foobar`).
fn is_prefix(root: &str, path: &str) -> bool {
    let (r, p) = (components(root), components(path));
    !r.is_empty() && r.len() <= p.len() && r.iter().zip(p.iter()).all(|(a, b)| a == b)
}

pub(crate) fn attribute_with(
    path: &Path,
    projects: &[Project],
    case_insensitive: bool,
) -> Option<ProjectId> {
    let target = normalize_with(path, case_insensitive);
    let mut best: Option<(usize, &ProjectId)> = None;
    for p in projects {
        for root in p.paths() {
            let r = normalize_with(root, case_insensitive);
            if is_prefix(&r, &target) {
                let depth = components(&r).len();
                if best.is_none_or(|(d, _)| depth > d) {
                    best = Some((depth, &p.id));
                }
            }
        }
    }
    best.map(|(_, id)| id.clone())
}

/// Returns the project owning `path` (longest component-wise prefix wins), if any.
pub fn attribute(path: &Path, projects: &[Project]) -> Option<ProjectId> {
    attribute_with(path, projects, cfg!(windows))
}

/// Filters by scope and groups by project. In `Global` there is one group per project with items
/// (in `projects` order) plus a trailing "No project" group; in `Project(id)` only that project.
pub fn group_by_project<T: Attributable>(
    items: Vec<T>,
    scope: &Scope,
    projects: &[Project],
) -> Grouped<T> {
    let mut buckets: Vec<Group<T>> = projects
        .iter()
        .map(|p| Group {
            project: Some(p.to_ref()),
            items: Vec::new(),
            totals: Totals::default(),
        })
        .collect();
    let mut none = Group {
        project: None,
        items: Vec::new(),
        totals: Totals::default(),
    };
    for item in items {
        let owner = item.path().and_then(|p| attribute(p, projects));
        let idx = owner
            .as_ref()
            .and_then(|id| projects.iter().position(|p| &p.id == id));
        let (ram, disk) = (item.ram(), item.disk());
        let bucket = match idx {
            Some(i) => &mut buckets[i],
            None => &mut none,
        };
        bucket.totals.add(ram, disk);
        bucket.items.push(item);
    }
    buckets.push(none);
    let mut groups: Vec<Group<T>> = match scope {
        Scope::Global => buckets,
        Scope::Project { id } => buckets
            .into_iter()
            .filter(|g| g.project.as_ref().is_some_and(|p| &p.id == id))
            .collect(),
    };
    groups.retain(|g| !g.items.is_empty() || matches!(scope, Scope::Project { .. }));
    let mut total = Totals::default();
    for g in &groups {
        total.count += g.totals.count;
        total.ram_bytes += g.totals.ram_bytes;
        total.disk_bytes += g.totals.disk_bytes;
    }
    Grouped { groups, total }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn project(name: &str, main: &str, wts: &[&str]) -> Project {
        Project {
            id: ProjectId::from_path(Path::new(main)),
            name: name.into(),
            main_path: main.into(),
            worktree_paths: wts.iter().map(PathBuf::from).collect(),
        }
    }

    struct Item(Option<PathBuf>, u64, u64);
    impl Attributable for Item {
        fn path(&self) -> Option<&Path> {
            self.0.as_deref()
        }
        fn ram(&self) -> u64 {
            self.1
        }
        fn disk(&self) -> u64 {
            self.2
        }
    }

    fn item(p: &str, ram: u64, disk: u64) -> Item {
        Item(Some(PathBuf::from(p)), ram, disk)
    }

    #[test]
    fn prefix_is_component_wise() {
        let ps = [project("foo", "/a/foo", &[])];
        assert_eq!(
            attribute_with(Path::new("/a/foo/src"), &ps, false),
            Some(ps[0].id.clone())
        );
        assert_eq!(
            attribute_with(Path::new("/a/foo"), &ps, false),
            Some(ps[0].id.clone())
        );
        assert_eq!(attribute_with(Path::new("/a/foobar"), &ps, false), None);
        assert_eq!(attribute_with(Path::new("/a"), &ps, false), None);
    }

    #[test]
    fn longest_prefix_wins() {
        let ps = [
            project("outer", "/a/outer", &[]),
            project("inner", "/a/outer/nested", &[]),
        ];
        assert_eq!(
            attribute_with(Path::new("/a/outer/nested/x"), &ps, false),
            Some(ps[1].id.clone())
        );
        assert_eq!(
            attribute_with(Path::new("/a/outer/x"), &ps, false),
            Some(ps[0].id.clone())
        );
    }

    #[test]
    fn worktree_paths_belong_to_project() {
        let ps = [project("p", "/r/p", &["/w/p-feat"])];
        assert_eq!(
            attribute_with(Path::new("/w/p-feat/node_modules"), &ps, false),
            Some(ps[0].id.clone())
        );
    }

    #[test]
    fn case_insensitive_when_requested() {
        let ps = [project("p", "C:/Repos/P", &[])];
        assert_eq!(
            attribute_with(Path::new("c:/repos/p/src"), &ps, true),
            Some(ps[0].id.clone())
        );
        assert_eq!(
            attribute_with(Path::new("c:/repos/p/src"), &ps, false),
            None
        );
    }

    #[test]
    fn strips_verbatim_prefix_and_backslashes() {
        assert_eq!(
            normalize_with(Path::new(r"\\?\C:\Repos\P\"), true),
            "c:/repos/p"
        );
        let ps = [project("p", "C:/Repos/P", &[])];
        assert_eq!(
            attribute_with(Path::new(r"\\?\C:\Repos\P\src"), &ps, true),
            Some(ps[0].id.clone())
        );
    }

    #[test]
    fn global_groups_with_no_project_and_totals() {
        let ps = [project("a", "/r/a", &[]), project("b", "/r/b", &[])];
        let items = vec![
            item("/r/b/x", 10, 100),
            item("/r/a/y", 1, 2),
            item("/elsewhere", 5, 5),
            Item(None, 7, 0),
        ];
        let g = group_by_project(items, &Scope::Global, &ps);
        assert_eq!(g.groups.len(), 3);
        assert_eq!(g.groups[0].project.as_ref().unwrap().name, "a");
        assert_eq!(g.groups[1].project.as_ref().unwrap().name, "b");
        assert!(g.groups[2].project.is_none());
        assert_eq!(g.groups[2].items.len(), 2);
        assert_eq!(g.groups[2].totals.ram_bytes, 12);
        assert_eq!(
            g.total,
            Totals {
                count: 4,
                ram_bytes: 23,
                disk_bytes: 107
            }
        );
    }

    #[test]
    fn project_scope_keeps_only_that_project() {
        let ps = [project("a", "/r/a", &[]), project("b", "/r/b", &[])];
        let items = vec![
            item("/r/b/x", 10, 100),
            item("/r/a/y", 1, 2),
            item("/z", 1, 1),
        ];
        let g = group_by_project(
            items,
            &Scope::Project {
                id: ps[1].id.clone(),
            },
            &ps,
        );
        assert_eq!(g.groups.len(), 1);
        assert_eq!(g.groups[0].items.len(), 1);
        assert_eq!(g.total.count, 1);
        assert_eq!(g.total.disk_bytes, 100);
    }

    #[test]
    fn scope_serializes_as_tagged() {
        let v = serde_json::to_value(Scope::Global).unwrap();
        assert_eq!(v, serde_json::json!({"kind": "global"}));
        let s: Scope = serde_json::from_str(r#"{"kind":"project","id":"abc"}"#).unwrap();
        assert_eq!(
            s,
            Scope::Project {
                id: ProjectId("abc".into())
            }
        );
    }

    #[cfg(windows)]
    #[test]
    fn attribute_is_case_insensitive_on_windows() {
        let ps = [project("p", r"C:\Repos\P", &[])];
        assert_eq!(
            attribute(Path::new(r"c:\repos\p\src"), &ps),
            Some(ps[0].id.clone())
        );
    }
}
