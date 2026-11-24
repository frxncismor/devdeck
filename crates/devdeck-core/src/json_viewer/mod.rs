//! JSON viewer core (phase 2C): memory-mapped file, own tokenizer, container-only index with jump
//! points, offset reads, search and formatting. No value tree is ever built in memory.

pub mod format;
pub mod indexer;
pub mod reader;
pub mod search;

pub use indexer::{ContainerKind, JsonKind, LEAF_BIT};
pub use reader::JsonDoc;

use serde::Serialize;

/// A node as shown in the tree. Containers have an index node id; leaves a synthetic id
/// (`LEAF_BIT | member offset`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonNode {
    pub id: u64,
    /// Object member name (decoded).
    pub key: Option<String>,
    /// Position inside an array parent.
    pub index: Option<u64>,
    pub kind: JsonKind,
    pub child_count: Option<u64>,
    /// Raw JSON text of a leaf, truncated.
    pub preview: Option<String>,
    pub has_children: bool,
}

/// Opened document summary returned to the front.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub path: String,
    pub size_bytes: u64,
    pub root_id: u64,
    pub containers: u64,
}

// ---------------------------------------------------------------------------------------------
// Path `$.a.b[3]` (T052)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSeg {
    Key(String),
    Index(u64),
}

fn is_identifier(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// `$` + `.key` for identifier-like keys, `["any key"]` (JSON-escaped) otherwise, `[3]` for indexes.
pub fn format_path(segs: &[PathSeg]) -> String {
    let mut out = String::from("$");
    for seg in segs {
        match seg {
            PathSeg::Index(i) => out.push_str(&format!("[{i}]")),
            PathSeg::Key(k) if is_identifier(k) => {
                out.push('.');
                out.push_str(k);
            }
            PathSeg::Key(k) => {
                out.push('[');
                out.push_str(&serde_json::to_string(k).unwrap_or_else(|_| "\"\"".into()));
                out.push(']');
            }
        }
    }
    out
}

/// Decodes the escapes of a JSON string body (without quotes). Lone surrogates become U+FFFD.
pub fn unescape_inner(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    if !s.contains('\\') {
        return s.into_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    let hex4 = |it: &mut std::iter::Peekable<std::str::Chars>| -> Option<u32> {
        let mut v = 0u32;
        for _ in 0..4 {
            v = v * 16 + it.next()?.to_digit(16)?;
        }
        Some(v)
    };
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('u') => {
                let Some(hi) = hex4(&mut it) else {
                    out.push('\u{FFFD}');
                    continue;
                };
                if (0xD800..0xDC00).contains(&hi) {
                    // High surrogate: needs a following `\uDC00..DFFF`.
                    let mut look = it.clone();
                    let low = (look.next() == Some('\\') && look.next() == Some('u'))
                        .then(|| hex4(&mut look))
                        .flatten()
                        .filter(|l| (0xDC00..0xE000).contains(l));
                    match low {
                        Some(lo) => {
                            it = look;
                            let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                            out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                        }
                        None => out.push('\u{FFFD}'),
                    }
                } else {
                    out.push(char::from_u32(hi).unwrap_or('\u{FFFD}'));
                }
            }
            Some(other) => out.push(other), // `"` `\` `/`
            None => {}
        }
    }
    out
}

/// Decodes a quoted JSON string (with its quotes).
pub fn unescape_quoted(quoted: &[u8]) -> String {
    let inner = match quoted {
        [b'"', inner @ .., b'"'] => inner,
        other => other,
    };
    unescape_inner(inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexer::LEAF_BIT;
    use std::io::Write;
    use std::sync::atomic::AtomicBool;

    // ---- T052: paths -------------------------------------------------------------------------

    #[test]
    fn formats_paths_with_key_escaping() {
        use PathSeg::*;
        assert_eq!(format_path(&[]), "$");
        assert_eq!(
            format_path(&[Key("a".into()), Key("b_1".into()), Index(3)]),
            "$.a.b_1[3]"
        );
        assert_eq!(format_path(&[Key("a b".into())]), r#"$["a b"]"#);
        assert_eq!(format_path(&[Key("1st".into())]), r#"$["1st"]"#);
        assert_eq!(format_path(&[Key("".into())]), r#"$[""]"#);
        assert_eq!(format_path(&[Key("q\"uote\\".into())]), r#"$["q\"uote\\"]"#);
        assert_eq!(format_path(&[Key("ñ".into())]), r#"$["ñ"]"#);
        assert_eq!(
            format_path(&[Key("$id".into()), Index(0), Index(1)]),
            "$.$id[0][1]"
        );
    }

    #[test]
    fn unescapes_strings() {
        assert_eq!(
            unescape_quoted(br#""a\nb\t\"q\" \\ \/ \u00e9 \ud83d\ude00""#),
            "a\nb\t\"q\" \\ / é 😀"
        );
        assert_eq!(unescape_inner(br"\ud800x"), "\u{FFFD}x");
        assert_eq!(unescape_inner("日本".as_bytes()), "日本");
    }

    // ---- helpers -----------------------------------------------------------------------------

    fn doc_from(text: &str) -> (tempfile::TempDir, JsonDoc) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.json");
        std::fs::write(&p, text).unwrap();
        let d = JsonDoc::open(&p, &AtomicBool::new(false), &mut |_| {}).unwrap();
        (dir, d)
    }

    fn find_child(d: &JsonDoc, parent: u64, key: &str) -> JsonNode {
        let n = d.node(parent).unwrap().child_count.unwrap();
        d.children(parent, 0, n)
            .unwrap()
            .into_iter()
            .find(|c| c.key.as_deref() == Some(key))
            .unwrap()
    }

    // ---- T051: reader ------------------------------------------------------------------------

    #[test]
    fn reads_root_children_values_and_paths() {
        let (_g, d) = doc_from(r#"{"a":{"b":[10,"x",{"c d":true}]},"e":null,"s":"é\"q"}"#);
        let root = d.root().unwrap();
        assert_eq!(
            (root.kind, root.child_count, root.key.clone()),
            (JsonKind::Object, Some(3), None)
        );
        let kids = d.children(root.id, 0, 10).unwrap();
        let labels: Vec<_> = kids.iter().map(|k| k.key.clone().unwrap()).collect();
        assert_eq!(labels, ["a", "e", "s"]);
        assert_eq!(kids[1].kind, JsonKind::Null);
        assert_eq!(kids[1].preview.as_deref(), Some("null"));
        assert!(kids[0].has_children && !kids[1].has_children);
        assert_eq!(d.value(kids[2].id, 100).unwrap(), r#""é\"q""#);
        assert_eq!(d.value(kids[2].id, 2).unwrap(), "\"…"); // cut before the 2-byte char, never inside it

        let b = find_child(&d, kids[0].id, "b");
        let items = d.children(b.id, 0, 10).unwrap();
        assert_eq!(
            items.iter().map(|i| i.index).collect::<Vec<_>>(),
            [Some(0), Some(1), Some(2)]
        );
        assert_eq!(d.path_of(items[0].id).unwrap(), "$.a.b[0]");
        assert_eq!(d.path_of(items[1].id).unwrap(), "$.a.b[1]");
        let cd = find_child(&d, items[2].id, "c d");
        assert_eq!(d.path_of(cd.id).unwrap(), r#"$.a.b[2]["c d"]"#);
        assert_eq!(d.path_of(items[2].id).unwrap(), "$.a.b[2]");
        assert_eq!(d.path_of(root.id).unwrap(), "$");
        assert_eq!(d.node(cd.id).unwrap(), cd);
        assert_eq!(d.node(items[1].id).unwrap().index, Some(1));
        assert_eq!(d.value(items[2].id, 1000).unwrap(), r#"{"c d":true}"#);
        let _ = d.node(LEAF_BIT | 1); // a bogus leaf id never panics
        assert!(d.node(99_999).is_err());
        assert!(d.children(LEAF_BIT | 5, 0, 10).unwrap().is_empty());
    }

    #[test]
    fn pagination_uses_jump_points() {
        let n = 5000u64;
        let text = format!(
            "[{}]",
            (0..n)
                .map(|i| format!("{{\"i\":{i}}}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let (_g, d) = doc_from(&text);
        let root = d.root().unwrap();
        assert_eq!(root.child_count, Some(n));
        for (offset, limit) in [
            (0, 5),
            (1020, 10),
            (1024, 3),
            (2047, 2),
            (4990, 100),
            (4999, 1),
        ] {
            let page = d.children(root.id, offset, limit).unwrap();
            let want = limit.min(n - offset);
            assert_eq!(page.len() as u64, want, "offset {offset}");
            for (k, c) in page.iter().enumerate() {
                assert_eq!(c.index, Some(offset + k as u64));
                let inner = d.children(c.id, 0, 1).unwrap();
                assert_eq!(
                    inner[0].preview.as_deref(),
                    Some((offset + k as u64).to_string().as_str())
                );
            }
        }
        assert!(d.children(root.id, n, 10).unwrap().is_empty());
        // Leaf element past a jump point: index recovered by a bounded scan.
        let leaf_doc = format!(
            "[{}]",
            (0..3000)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let (_g2, d2) = doc_from(&leaf_doc);
        let page = d2.children(d2.root().unwrap().id, 2500, 2).unwrap();
        assert_eq!(page[0].preview.as_deref(), Some("2500"));
        assert_eq!(d2.path_of(page[1].id).unwrap(), "$[2501]");
        assert_eq!(d2.node(page[1].id).unwrap().index, Some(2501));
    }

    #[test]
    fn scalar_document() {
        let (_g, d) = doc_from("  \"hi\"  ");
        let root = d.root().unwrap();
        assert_eq!(
            (root.kind, root.preview.as_deref()),
            (JsonKind::String, Some("\"hi\""))
        );
        assert_eq!(d.path_of(root.id).unwrap(), "$");
        assert!(d.children(root.id, 0, 10).unwrap().is_empty());
    }

    #[test]
    fn duplicate_keys_are_kept_in_order() {
        let (_g, d) = doc_from(r#"{"k":1,"k":2,"k":{"k":3}}"#);
        let kids = d.children(d.root().unwrap().id, 0, 10).unwrap();
        assert_eq!(kids.len(), 3);
        assert!(kids.iter().all(|k| k.key.as_deref() == Some("k")));
        assert_eq!(kids[1].preview.as_deref(), Some("2"));
        assert_eq!(d.path_of(kids[1].id).unwrap(), "$.k");
    }

    #[test]
    fn invalid_file_reports_offset_and_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.json");
        std::fs::write(&p, b"{\"a\": [1, 2,, 3]}").unwrap();
        let e = JsonDoc::open(&p, &AtomicBool::new(false), &mut |_| {})
            .err()
            .unwrap();
        assert_eq!(e.code, "json_invalid_at_offset");
        assert_eq!(e.params.unwrap()["offset"], 12);
        std::fs::write(&p, b"").unwrap();
        assert_eq!(
            JsonDoc::open(&p, &AtomicBool::new(false), &mut |_| {})
                .err()
                .unwrap()
                .code,
            "json_invalid_at_offset"
        );
        let missing = dir.path().join("none.json");
        assert_eq!(
            JsonDoc::open(&missing, &AtomicBool::new(false), &mut |_| {})
                .err()
                .unwrap()
                .code,
            "not_found"
        );
    }

    #[test]
    fn detects_a_changed_file() {
        let (g, d) = doc_from(r#"{"a":1}"#);
        d.check_unchanged().unwrap();
        // Appending cannot fail on Windows for a mapped file; length changes.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(g.path().join("t.json"))
            .unwrap();
        f.write_all(b" ").unwrap();
        f.sync_all().unwrap();
        assert_eq!(d.check_unchanged().unwrap_err().code, "json_file_changed");
    }

    // ---- T053: search ------------------------------------------------------------------------

    fn run_search(
        d: &JsonDoc,
        opts: &search::SearchOptions,
    ) -> (Vec<search::SearchHit>, search::SearchSummary, usize) {
        let mut hits = Vec::new();
        let mut batches = 0;
        let s = search::search(d, opts, &AtomicBool::new(false), &mut |b| {
            batches += 1;
            hits.extend(b);
        })
        .unwrap();
        (hits, s, batches)
    }

    #[test]
    fn search_keys_values_case_and_paths() {
        let (_g, d) = doc_from(
            r#"{"Name":"Alice","list":[{"name":"bob"},"ALICE",{"x":"al\u0069ce"}],"n":12,"Ünï":"ÜNÏ"}"#,
        );
        let mut o = search::SearchOptions::new("alice");
        let (hits, s, _) = run_search(&d, &o);
        let paths: Vec<_> = hits.iter().map(|h| (h.path.as_str(), h.kind)).collect();
        assert_eq!(
            paths,
            [
                ("$.Name", search::HitKind::Value),
                ("$.list[1]", search::HitKind::Value),
                ("$.list[2].x", search::HitKind::Value)
            ]
        );
        assert_eq!(s.total, 3);
        assert!(!s.truncated);
        assert_eq!(hits[2].snippet, "alice");
        // The id of a hit resolves to the same node.
        assert_eq!(d.path_of(hits[1].node_id).unwrap(), "$.list[1]");

        o.case_sensitive = true;
        // Escapes are decoded before matching: `alice` is "alice".
        assert_eq!(run_search(&d, &o).0.len(), 1);
        o.query = "Alice".into();
        assert_eq!(run_search(&d, &o).0.len(), 1);

        let mut keys = search::SearchOptions::new("name");
        keys.in_values = false;
        let (hits, _, _) = run_search(&d, &keys);
        let ps: Vec<_> = hits.iter().map(|h| h.path.clone()).collect();
        assert_eq!(ps, ["$.Name", "$.list[0].name"]);
        assert!(hits.iter().all(|h| h.kind == search::HitKind::Key));

        let mut nums = search::SearchOptions::new("12");
        nums.in_keys = false;
        assert_eq!(run_search(&d, &nums).0[0].path, "$.n");

        let uni = search::SearchOptions::new("ünï");
        let (hits, _, _) = run_search(&d, &uni);
        assert_eq!(hits.len(), 2); // key and value
        assert_eq!(hits[0].path, r#"$["Ünï"]"#);
    }

    #[test]
    fn search_matches_keys_of_containers_and_resolves_their_ids() {
        let (_g, d) = doc_from(r#"{"target":{"inner":[1]},"other":2}"#);
        let (hits, _, _) = run_search(&d, &search::SearchOptions::new("target"));
        assert_eq!(hits.len(), 1);
        let node = d.node(hits[0].node_id).unwrap();
        assert_eq!(node.kind, JsonKind::Object);
        assert_eq!(d.path_of(node.id).unwrap(), "$.target");
    }

    #[test]
    fn search_caps_results_batches_and_cancels() {
        let text = format!(
            "[{}]",
            (0..2500).map(|_| "\"hit\"").collect::<Vec<_>>().join(",")
        );
        let (_g, d) = doc_from(&text);
        let mut o = search::SearchOptions::new("hit");
        o.batch_size = 1000;
        let (hits, s, batches) = run_search(&d, &o);
        assert_eq!((hits.len(), batches, s.truncated), (2500, 3, false));
        o.max_results = 1500;
        let (hits, s, batches) = run_search(&d, &o);
        assert_eq!(
            (hits.len(), s.total, s.truncated, batches),
            (1500, 1500, true, 2)
        );
        assert_eq!(hits.last().unwrap().path, "$[1499]");

        let cancel = AtomicBool::new(true);
        let big = format!(
            "[{}]",
            (0..200_000)
                .map(|_| "\"hit\"")
                .collect::<Vec<_>>()
                .join(",")
        );
        let (_g2, d2) = doc_from(&big);
        let mut all = search::SearchOptions::new("hit");
        all.max_results = usize::MAX;
        let e = search::search(&d2, &all, &cancel, &mut |_| {}).unwrap_err();
        assert_eq!(e.kind, crate::ErrorKind::Cancelled);
        assert_eq!(run_search(&d, &search::SearchOptions::new("")).0.len(), 0);
    }

    // ---- T054: format ------------------------------------------------------------------------

    #[test]
    fn format_node_pretty_prints_and_caps() {
        let (_g, d) = doc_from(r#"{"a":[1,{"b":"x"}],"c":2}"#);
        let a = find_child(&d, d.root().unwrap().id, "a");
        let s = format::format_node(&d, a.id, 1000).unwrap();
        assert_eq!(s, "[\n  1,\n  {\n    \"b\": \"x\"\n  }\n]");
        let e = format::format_node(&d, d.root().unwrap().id, 5).unwrap_err();
        assert_eq!(e.code, "json_value_too_large");
        let leaf = d.children(a.id, 0, 1).unwrap().remove(0);
        assert_eq!(format::format_node(&d, leaf.id, 100).unwrap(), "1");
    }

    #[test]
    fn save_formatted_streams_and_protects_the_source() {
        let (g, d) = doc_from(r#"{"a":[1,2],"b":{}}"#);
        let dest = g.path().join("out.json");
        let mut last = (0, 0);
        let n = format::save_formatted(&d, None, &dest, &AtomicBool::new(false), &mut |a, b| {
            last = (a, b)
        })
        .unwrap();
        let text = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(text, "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": {}\n}\n");
        assert_eq!(n as usize, text.len());
        assert_eq!(last.0, last.1);
        // Subtree only.
        let a = find_child(&d, d.root().unwrap().id, "a");
        format::save_formatted(
            &d,
            Some(a.id),
            &dest,
            &AtomicBool::new(false),
            &mut |_, _| {},
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "[\n  1,\n  2\n]\n");
        // Never overwrite the mapped source.
        let e = format::save_formatted(&d, None, d.path(), &AtomicBool::new(false), &mut |_, _| {})
            .unwrap_err();
        assert_eq!(e.code, "json_save_same_file");
        d.check_unchanged().unwrap();
        // Cancellation removes the partial file.
        let big = format!(
            "[{}]",
            (0..300_000)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let (g3, d3) = doc_from(&big);
        let dest3 = g3.path().join("o.json");
        let e = format::save_formatted(&d3, None, &dest3, &AtomicBool::new(true), &mut |_, _| {})
            .unwrap_err();
        assert_eq!(e.kind, crate::ErrorKind::Cancelled);
        assert!(!dest3.exists());

        // A pre-existing destination is not deleted when the save fails.
        std::fs::write(&dest3, "keep").unwrap();
        let e = format::save_formatted(&d3, None, &dest3, &AtomicBool::new(true), &mut |_, _| {})
            .unwrap_err();
        assert_eq!(e.kind, crate::ErrorKind::Cancelled);
        assert!(dest3.exists());
    }

    // ---- T055: fixtures + memory/time --------------------------------------------------------

    /// Writes a fixture of at least `target` bytes: a huge array of nested records with
    /// duplicate keys and Unicode, followed by one very long single line.
    fn write_fixture(path: &std::path::Path, target: u64, single_line: bool) -> u64 {
        let mut w = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
        let nl = if single_line { "" } else { "\n" };
        write!(
            w,
            "{{{nl}\"meta\":{{\"name\":\"fixture é 日本\",\"dup\":1,\"dup\":2}},{nl}\"items\":["
        )
        .unwrap();
        let mut written = 0u64;
        let mut i = 0u64;
        while written < target {
            let rec = format!(
                "{}{{\"id\":{i},\"name\":\"item-{i} \\u00e9\\n\",\"tags\":[\"a\",\"b\",{i}],\"nested\":{{\"x\":{x},\"y\":[true,false,null],\"deep\":{{\"k\":\"v{i}\"}}}}}}",
                if i == 0 { "" } else { "," },
                x = i as f64 / 3.0
            );
            w.write_all(rec.as_bytes()).unwrap();
            written += rec.len() as u64;
            i += 1;
        }
        write!(w, "]{nl}}}").unwrap();
        w.flush().unwrap();
        std::fs::metadata(path).unwrap().len()
    }

    /// Resident memory of this process in bytes (Windows/Linux); `None` elsewhere.
    fn resident_bytes() -> Option<u64> {
        use sysinfo::{get_current_pid, ProcessRefreshKind, ProcessesToUpdate, System};
        let pid = get_current_pid().ok()?;
        let mut sys = System::new().ok()?;
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        sys.process(pid).map(|p| p.memory())
    }

    fn scale_test(mb: u64, max_secs: u64) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.json");
        let size = write_fixture(&path, mb * 1024 * 1024, mb < 100);
        let before = resident_bytes();
        let t = std::time::Instant::now();
        let mut reports = 0;
        let d = JsonDoc::open(&path, &AtomicBool::new(false), &mut |_| reports += 1).unwrap();
        let index_time = t.elapsed();
        let containers = d.containers();
        let index_bytes = containers * std::mem::size_of::<indexer::Node>() as u64
            + d.index.jumps.len() as u64 * 8;

        // Navigation stays cheap whatever the size.
        let t = std::time::Instant::now();
        let root = d.root().unwrap();
        let items = find_child(&d, root.id, "items");
        let tail = d
            .children(items.id, items.child_count.unwrap() - 3, 3)
            .unwrap();
        let mid = d
            .children(items.id, items.child_count.unwrap() / 2, 50)
            .unwrap();
        let nav_time = t.elapsed();
        assert_eq!(tail.len(), 3);
        assert_eq!(mid.len(), 50);
        let t = std::time::Instant::now();
        let mut o = search::SearchOptions::new("item-7");
        o.in_keys = false;
        o.max_results = 100;
        let (hits, _, _) = run_search(&d, &o);
        let search_time = t.elapsed();
        assert_eq!(hits.len(), 100);
        let after = resident_bytes();
        let grew = before.zip(after).map(|(b, a)| a.saturating_sub(b));
        eprintln!(
            "[json fixture] file={:.1} MB containers={containers} index={:.1} MB ({:.1} B/container) \
             index_time={index_time:?} ({:.0} MB/s, {reports} progress events) nav={nav_time:?} search(100 hits)={search_time:?} \
             rss_growth={:?} MB (includes mmap pages touched)",
            size as f64 / 1e6,
            index_bytes as f64 / 1e6,
            index_bytes as f64 / containers as f64,
            size as f64 / 1e6 / index_time.as_secs_f64(),
            grew.map(|g| g / 1_000_000),
        );
        assert!(
            index_time.as_secs() < max_secs,
            "indexing took {index_time:?}"
        );
        assert!(nav_time.as_millis() < 2000, "navigation took {nav_time:?}");
        assert!(
            reports >= 50,
            "expected ~100 progress events, got {reports}"
        );
        // Containers only: bounded bytes per container (this fixture is container-dense).
        assert!(
            index_bytes / containers <= 48,
            "{index_bytes} B for {containers} containers"
        );
        if let Some(g) = grew {
            // Generous: mapped pages count as resident, so allow the file size plus the index.
            assert!(g < size + index_bytes + 200_000_000, "RSS grew {g} B");
        }
    }

    /// ~50 MB fixture. Set `DEVDECK_SKIP_SLOW=1` to skip it on very slow runners.
    #[test]
    fn fixture_50mb_indexes_within_budget() {
        if std::env::var_os("DEVDECK_SKIP_SLOW").is_some() {
            return;
        }
        scale_test(50, 60);
    }

    #[test]
    #[ignore = "500 MB fixture: run with `cargo test -p devdeck-core -- --ignored fixture_500mb`"]
    fn fixture_500mb_indexes_within_budget() {
        scale_test(500, 600);
    }

    #[test]
    fn fixture_shapes_single_line_and_dup_keys() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.json");
        write_fixture(&p, 20_000, true);
        let d = JsonDoc::open(&p, &AtomicBool::new(false), &mut |_| {}).unwrap();
        let meta = find_child(&d, d.root().unwrap().id, "meta");
        let dups = d.children(meta.id, 0, 10).unwrap();
        assert_eq!(
            dups.iter()
                .filter(|c| c.key.as_deref() == Some("dup"))
                .count(),
            2
        );
        assert!(std::fs::read_to_string(&p).unwrap().lines().count() == 1);
    }
}
