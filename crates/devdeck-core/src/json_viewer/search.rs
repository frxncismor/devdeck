//! Batched background search over the memory map (T053). One structural pass with a path stack;
//! hits are produced in document order and delivered in batches.

use super::indexer::{walk, ContainerKind, JsonKind, Member, Visitor, LEAF_BIT};
use super::reader::JsonDoc;
use super::{format_path, unescape_inner, unescape_quoted, PathSeg};
use crate::error::{Error, ErrorKind, Result};
use memchr::{memchr, memchr2, memmem};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

pub const DEFAULT_MAX_RESULTS: usize = 10_000;
pub const DEFAULT_BATCH: usize = 500;
const SNIPPET_BYTES: usize = 160;
const LIMIT_CODE: &str = "json_search_limit";

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub in_keys: bool,
    pub in_values: bool,
    pub case_sensitive: bool,
    pub max_results: usize,
    pub batch_size: usize,
}

impl SearchOptions {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_owned(),
            in_keys: true,
            in_values: true,
            case_sensitive: false,
            max_results: DEFAULT_MAX_RESULTS,
            batch_size: DEFAULT_BATCH,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HitKind {
    Key,
    Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub node_id: u64,
    pub path: String,
    pub kind: HitKind,
    pub snippet: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSummary {
    pub total: usize,
    /// The result cap was reached before the end of the file.
    pub truncated: bool,
}

enum Matcher {
    Exact(Box<memmem::Finder<'static>>),
    AsciiNoCase(Vec<u8>),
    Unicode(String),
}

impl Matcher {
    fn new(query: &str, case_sensitive: bool) -> Self {
        if case_sensitive {
            Matcher::Exact(Box::new(memmem::Finder::new(query.as_bytes()).into_owned()))
        } else if query.is_ascii() {
            Matcher::AsciiNoCase(query.to_ascii_lowercase().into_bytes())
        } else {
            Matcher::Unicode(query.to_lowercase())
        }
    }

    /// `raw` is the JSON text of a string's content (escapes undecoded) or of a scalar.
    fn is_match(&self, raw: &[u8]) -> bool {
        let decoded;
        let hay: &[u8] = if memchr(b'\\', raw).is_some() {
            decoded = unescape_inner(raw);
            decoded.as_bytes()
        } else {
            raw
        };
        match self {
            Matcher::Exact(f) => f.find(hay).is_some(),
            Matcher::AsciiNoCase(q) => {
                let (lo, up) = (q[0], q[0].to_ascii_uppercase());
                let mut from = 0;
                while let Some(p) = memchr2(lo, up, &hay[from..]) {
                    let at = from + p;
                    if hay.len() - at >= q.len() && hay[at..at + q.len()].eq_ignore_ascii_case(q) {
                        return true;
                    }
                    from = at + 1;
                }
                false
            }
            Matcher::Unicode(q) => String::from_utf8_lossy(hay)
                .to_lowercase()
                .contains(q.as_str()),
        }
    }
}

#[derive(Clone, Copy)]
enum RawSeg {
    Key(usize, usize),
    Index(u32),
}

struct Searcher<'a, 'b> {
    doc: &'a JsonDoc,
    data: &'a [u8],
    opts: &'a SearchOptions,
    matcher: Matcher,
    /// Open containers with the segment that leads to each from its parent (`None` for the root).
    stack: Vec<Option<RawSeg>>,
    batch: Vec<SearchHit>,
    total: usize,
    cancel: &'a AtomicBool,
    on_batch: &'b mut dyn FnMut(Vec<SearchHit>),
}

impl Searcher<'_, '_> {
    fn seg_of(member: &Member) -> RawSeg {
        match member.key {
            Some((s, e)) => RawSeg::Key(s, e),
            None => RawSeg::Index(member.index),
        }
    }

    fn path(&self, member: Option<&Member>) -> String {
        let mut segs = Vec::new();
        let raw = self
            .stack
            .iter()
            .flatten()
            .copied()
            .chain(member.map(Self::seg_of));
        for s in raw {
            segs.push(match s {
                RawSeg::Key(a, b) => PathSeg::Key(unescape_quoted(&self.data[a..b])),
                RawSeg::Index(i) => PathSeg::Index(u64::from(i)),
            });
        }
        format_path(&segs)
    }

    fn push(&mut self, node_id: u64, path: String, kind: HitKind, snippet: String) -> Result<()> {
        self.batch.push(SearchHit {
            node_id,
            path,
            kind,
            snippet,
        });
        self.total += 1;
        if self.batch.len() >= self.opts.batch_size {
            (self.on_batch)(std::mem::take(&mut self.batch));
        }
        if self.total >= self.opts.max_results {
            return Err(Error::new(ErrorKind::Cancelled, LIMIT_CODE));
        }
        Ok(())
    }

    fn key_hit(&mut self, member: &Member, node_id: u64) -> Result<()> {
        if !self.opts.in_keys {
            return Ok(());
        }
        let Some((s, e)) = member.key else {
            return Ok(());
        };
        if self.matcher.is_match(&self.data[s + 1..e - 1]) {
            let snippet = snippet(&unescape_quoted(&self.data[s..e]));
            let path = self.path(Some(member));
            self.push(node_id, path, HitKind::Key, snippet)?;
        }
        Ok(())
    }
}

fn snippet(s: &str) -> String {
    if s.len() <= SNIPPET_BYTES {
        return s.to_owned();
    }
    let mut end = SNIPPET_BYTES;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

impl Visitor for Searcher<'_, '_> {
    fn enter(&mut self, _kind: ContainerKind, member: &Member, start: usize) -> Result<()> {
        if !self.stack.is_empty() {
            let id = self.doc.index.node_at(start as u64).map_or(0, u64::from);
            self.key_hit(member, id)?;
        }
        let seg = (!self.stack.is_empty()).then(|| Self::seg_of(member));
        self.stack.push(seg);
        Ok(())
    }

    fn exit(&mut self, _end: usize, _child_count: u32) -> Result<()> {
        self.stack.pop();
        Ok(())
    }

    fn leaf(&mut self, member: &Member, kind: JsonKind, start: usize, end: usize) -> Result<()> {
        let id = LEAF_BIT | member.start as u64;
        self.key_hit(member, id)?;
        if self.opts.in_values {
            let raw = if kind == JsonKind::String {
                &self.data[start + 1..end - 1]
            } else {
                &self.data[start..end]
            };
            if self.matcher.is_match(raw) {
                let text = if kind == JsonKind::String {
                    unescape_inner(raw)
                } else {
                    String::from_utf8_lossy(raw).into_owned()
                };
                let path = self.path((!self.stack.is_empty()).then_some(member));
                self.push(id, path, HitKind::Value, snippet(&text))?;
            }
        }
        Ok(())
    }

    fn jump(&mut self, _member_start: usize) {}

    fn tick(&mut self, _pos: usize) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(Error::cancelled());
        }
        Ok(())
    }
}

/// Searches keys and/or values. Hits are delivered through `on_batch` (at most `batch_size` each);
/// the search stops after `max_results` hits (`truncated`). Cancellation returns `Error::cancelled()`.
pub fn search(
    doc: &JsonDoc,
    opts: &SearchOptions,
    cancel: &AtomicBool,
    on_batch: &mut dyn FnMut(Vec<SearchHit>),
) -> Result<SearchSummary> {
    if opts.query.is_empty() || (!opts.in_keys && !opts.in_values) || opts.max_results == 0 {
        return Ok(SearchSummary {
            total: 0,
            truncated: false,
        });
    }
    let mut s = Searcher {
        doc,
        data: doc.data(),
        opts,
        matcher: Matcher::new(&opts.query, opts.case_sensitive),
        stack: Vec::new(),
        batch: Vec::new(),
        total: 0,
        cancel,
        on_batch,
    };
    let truncated = match walk(doc.data(), &mut s) {
        Ok(()) => false,
        Err(e) if e.code == LIMIT_CODE => true,
        Err(e) => return Err(e),
    };
    if !s.batch.is_empty() {
        let rest = std::mem::take(&mut s.batch);
        (s.on_batch)(rest);
    }
    Ok(SearchSummary {
        total: s.total,
        truncated,
    })
}
