//! Offset reader over a memory-mapped file (T051): paginated children, values on demand, node
//! paths and changed-file detection. Leaves are never indexed; they are located by the offset of
//! their member and encoded in the id as `LEAF_BIT | member_start`.

use super::indexer::{
    build_index, invalid, ContainerKind, Index, JsonKind, Node, Token, TokenKind, Tokenizer,
    JUMP_EVERY, LEAF_BIT,
};
use super::{format_path, unescape_quoted, JsonNode, PathSeg};
use crate::error::{Error, ErrorKind, Result};
use memmap2::Mmap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

const PREVIEW_BYTES: usize = 200;
const KEY_DISPLAY_BYTES: usize = 4096;
/// Hard cap for one `children` page.
pub const MAX_PAGE: u64 = 10_000;

pub struct JsonDoc {
    mmap: Mmap,
    pub index: Index,
    path: PathBuf,
    size: u64,
    mtime: Option<SystemTime>,
}

/// One member read from the file.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Child {
    pub start: usize,
    pub key: Option<(usize, usize)>,
    pub value: (usize, usize),
    pub kind: JsonKind,
    pub node: Option<u32>,
}

fn changed() -> Error {
    Error::new(ErrorKind::Io, "json_file_changed")
}

fn node_not_found() -> Error {
    Error::not_found("json_node_not_found")
}

impl JsonDoc {
    /// Maps and indexes the file. `progress(bytes_done)` ~every 1 %; `cancel` aborts with `Error::cancelled()`.
    pub fn open(path: &Path, cancel: &AtomicBool, progress: &mut dyn FnMut(u64)) -> Result<Self> {
        let file = File::open(path)?;
        let meta = file.metadata()?;
        if !meta.is_file() {
            return Err(Error::not_found("not_found"));
        }
        let size = meta.len();
        if size == 0 {
            return Err(invalid(0, "empty file"));
        }
        // SAFETY: read-only mapping. If another process truncates the file while it is mapped the
        // OS may fault; every read is preceded by `check_unchanged` in the commands to detect edits.
        let mmap = unsafe { Mmap::map(&file)? };
        let index = build_index(&mmap, cancel, progress)?;
        Ok(Self {
            mmap,
            index,
            path: path.to_owned(),
            size,
            mtime: meta.modified().ok(),
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.mmap
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn containers(&self) -> u64 {
        self.index.nodes.len() as u64
    }

    pub fn root_id(&self) -> u64 {
        self.index.root_id
    }

    /// Detects that the file changed on disk after indexing (`json_file_changed`).
    pub fn check_unchanged(&self) -> Result<()> {
        let meta = std::fs::metadata(&self.path).map_err(|_| changed())?;
        if meta.len() != self.size || meta.modified().ok() != self.mtime {
            return Err(changed());
        }
        Ok(())
    }

    fn node_ref(&self, id: u64) -> Result<&Node> {
        self.index.nodes.get(id as usize).ok_or_else(node_not_found)
    }

    fn jk(&self, tok: &Token) -> Result<JsonKind> {
        Ok(match tok.kind {
            TokenKind::String => JsonKind::String,
            TokenKind::Number => JsonKind::Number,
            TokenKind::True | TokenKind::False => JsonKind::Bool,
            TokenKind::Null => JsonKind::Null,
            _ => return Err(invalid(tok.start, "value expected")),
        })
    }

    /// Reads the member starting at the tokenizer position (after whitespace) and leaves the
    /// tokenizer after the member and its trailing comma, if any.
    pub(crate) fn read_member(&self, tk: &mut Tokenizer, parent: ContainerKind) -> Result<Child> {
        let data = self.data();
        let start = tk.skip_ws();
        let key = if parent == ContainerKind::Object {
            let k = tk.next_token()?.filter(|t| t.kind == TokenKind::String);
            let k = k.ok_or_else(|| invalid(start, "object key expected"))?;
            let colon = tk.next_token()?;
            if colon.map(|c| c.kind) != Some(TokenKind::Colon) {
                return Err(invalid(k.end, "':' expected"));
            }
            Some((k.start, k.end))
        } else {
            None
        };
        let vstart = tk.skip_ws();
        let (value, kind, node) = if matches!(data.get(vstart), Some(b'{' | b'[')) {
            let id = self
                .index
                .node_at(vstart as u64)
                .ok_or_else(|| invalid(vstart, "container not indexed"))?;
            let n = &self.index.nodes[id as usize];
            tk.set_pos(n.end() as usize);
            ((vstart, n.end() as usize), n.kind.into(), Some(id))
        } else {
            let tok = tk
                .next_token()?
                .ok_or_else(|| invalid(vstart, "value expected"))?;
            ((tok.start, tok.end), self.jk(&tok)?, None)
        };
        let p = tk.skip_ws();
        if data.get(p) == Some(&b',') {
            tk.set_pos(p + 1);
        }
        Ok(Child {
            start,
            key,
            value,
            kind,
            node,
        })
    }

    /// Position of child `offset` of `node` as `(child_index, member_position)`: the nearest
    /// jump point at or before it.
    fn seek(&self, node: &Node, offset: u64) -> (u64, usize) {
        let k = (offset / u64::from(JUMP_EVERY)) as usize;
        if k == 0 {
            (0, node.start as usize + 1)
        } else {
            let jumps = self.index.jumps_of(node);
            (k as u64 * u64::from(JUMP_EVERY), jumps[k - 1] as usize)
        }
    }

    fn make_node(&self, c: &Child, parent: ContainerKind, index: u64) -> Result<JsonNode> {
        let data = self.data();
        let (id, child_count, has_children, preview) = match c.node {
            Some(n) => {
                let cc = u64::from(self.index.nodes[n as usize].child_count);
                (u64::from(n), Some(cc), cc > 0, None)
            }
            None => (
                LEAF_BIT | c.start as u64,
                None,
                false,
                Some(preview(&data[c.value.0..c.value.1])),
            ),
        };
        let (key, index) = match (parent, c.key) {
            (ContainerKind::Object, Some(k)) => (Some(decode_key(&data[k.0..k.1])), None),
            _ => (None, Some(index)),
        };
        Ok(JsonNode {
            id,
            key,
            index,
            kind: c.kind,
            child_count,
            preview,
            has_children,
        })
    }

    /// The root node of the document.
    pub fn root(&self) -> Result<JsonNode> {
        self.node(self.index.root_id)
    }

    /// Info about any node id (container or leaf).
    pub fn node(&self, id: u64) -> Result<JsonNode> {
        let data = self.data();
        if id & LEAF_BIT == 0 {
            let n = self.node_ref(id)?;
            let (key, index) = self.member_label(n);
            return Ok(JsonNode {
                id,
                key,
                index,
                kind: n.kind.into(),
                child_count: Some(u64::from(n.child_count)),
                preview: None,
                has_children: n.child_count > 0,
            });
        }
        let (child, parent, index) = self.locate_leaf(id)?;
        let (key, idx) = match (parent, child.key) {
            (Some(ContainerKind::Object), Some(k)) => (Some(decode_key(&data[k.0..k.1])), None),
            (Some(_), _) => (None, Some(index)),
            _ => (None, None),
        };
        Ok(JsonNode {
            id,
            key,
            index: idx,
            kind: child.kind,
            child_count: None,
            preview: Some(preview(&data[child.value.0..child.value.1])),
            has_children: false,
        })
    }

    fn member_label(&self, n: &Node) -> (Option<String>, Option<u64>) {
        if n.parent == u32::MAX {
            return (None, None);
        }
        match self.index.nodes[n.parent as usize].kind {
            ContainerKind::Object => {
                let (s, e) = (n.key_off as usize, n.key_off as usize + n.key_len as usize);
                (Some(decode_key(&self.data()[s..e])), None)
            }
            ContainerKind::Array => (None, Some(n.key_off)),
        }
    }

    /// Reads a leaf by id: returns the member, its parent kind and its index in the parent.
    fn locate_leaf(&self, id: u64) -> Result<(Child, Option<ContainerKind>, u64)> {
        let member_start = (id & !LEAF_BIT) as usize;
        if member_start >= self.data().len() {
            return Err(node_not_found());
        }
        let parent = self.index.container_of(member_start as u64);
        let kind = parent.map(|p| self.index.nodes[p as usize].kind);
        let mut tk = Tokenizer::at(self.data(), member_start);
        let child = self.read_member(&mut tk, kind.unwrap_or(ContainerKind::Array))?;
        if child.start != member_start || child.node.is_some() {
            return Err(node_not_found());
        }
        let index = match parent {
            Some(p) if kind == Some(ContainerKind::Array) => {
                self.array_index_of(p, member_start)?
            }
            _ => 0,
        };
        Ok((child, kind, index))
    }

    /// Index of the element starting at `member_start` inside array `parent` (O(1024) scan).
    fn array_index_of(&self, parent: u32, member_start: usize) -> Result<u64> {
        let node = &self.index.nodes[parent as usize];
        let jumps = self.index.jumps_of(node);
        let k = jumps.partition_point(|&j| j as usize <= member_start);
        let (mut idx, pos) = if k == 0 {
            (0u64, node.start as usize + 1)
        } else {
            (k as u64 * u64::from(JUMP_EVERY), jumps[k - 1] as usize)
        };
        let mut tk = Tokenizer::at(self.data(), pos);
        while tk.skip_ws() < member_start {
            self.read_member(&mut tk, ContainerKind::Array)?;
            idx += 1;
        }
        Ok(idx)
    }

    /// Children `[offset, offset + limit)` of a container, re-scanned from the nearest jump point.
    pub fn children(&self, id: u64, offset: u64, limit: u64) -> Result<Vec<JsonNode>> {
        if id & LEAF_BIT != 0 {
            return Ok(Vec::new());
        }
        let node = self.node_ref(id)?;
        let total = u64::from(node.child_count);
        if offset >= total {
            return Ok(Vec::new());
        }
        let take = limit.min(MAX_PAGE).min(total - offset);
        let (mut idx, pos) = self.seek(node, offset);
        let mut tk = Tokenizer::at(self.data(), pos);
        let mut out = Vec::with_capacity(take as usize);
        while idx < offset + take {
            let c = self.read_member(&mut tk, node.kind)?;
            if idx >= offset {
                out.push(self.make_node(&c, node.kind, idx)?);
            }
            idx += 1;
        }
        Ok(out)
    }

    /// Raw byte span of a node's value.
    pub fn value_span(&self, id: u64) -> Result<(usize, usize)> {
        if id & LEAF_BIT == 0 {
            let n = self.node_ref(id)?;
            return Ok((n.start as usize, n.end() as usize));
        }
        Ok(self.locate_leaf(id)?.0.value)
    }

    /// Raw JSON text of a node, cut at `max_bytes` (on a char boundary, with `…`).
    pub fn value(&self, id: u64, max_bytes: u64) -> Result<String> {
        let (s, e) = self.value_span(id)?;
        let raw = &self.data()[s..e];
        let max = usize::try_from(max_bytes).unwrap_or(usize::MAX);
        if raw.len() <= max {
            return Ok(String::from_utf8_lossy(raw).into_owned());
        }
        Ok(cut(raw, max))
    }

    /// `$.a.b[3]` path of a node.
    pub fn path_of(&self, id: u64) -> Result<String> {
        let data = self.data();
        let mut segs: Vec<PathSeg> = Vec::new();
        let mut cur: Option<u32>;
        if id & LEAF_BIT == 0 {
            self.node_ref(id)?;
            cur = Some(id as u32);
        } else {
            let (child, kind, index) = self.locate_leaf(id)?;
            match (kind, child.key) {
                (Some(ContainerKind::Object), Some(k)) => {
                    segs.push(PathSeg::Key(unescape_quoted(&data[k.0..k.1])))
                }
                (Some(_), _) => segs.push(PathSeg::Index(index)),
                _ => {}
            }
            cur = self.index.container_of(child.start as u64);
        }
        while let Some(n) = cur {
            let node = &self.index.nodes[n as usize];
            if node.parent != u32::MAX {
                match self.index.nodes[node.parent as usize].kind {
                    ContainerKind::Object => {
                        let (s, e) = (
                            node.key_off as usize,
                            (node.key_off + u64::from(node.key_len)) as usize,
                        );
                        segs.push(PathSeg::Key(unescape_quoted(&data[s..e])));
                    }
                    ContainerKind::Array => segs.push(PathSeg::Index(node.key_off)),
                }
                cur = Some(node.parent);
            } else {
                cur = None;
            }
        }
        segs.reverse();
        Ok(format_path(&segs))
    }
}

fn decode_key(quoted: &[u8]) -> String {
    let mut s = unescape_quoted(quoted);
    if s.len() > KEY_DISPLAY_BYTES {
        let mut end = KEY_DISPLAY_BYTES;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
        s.push('…');
    }
    s
}

fn cut(raw: &[u8], max: usize) -> String {
    let mut end = max.min(raw.len());
    // Do not split a UTF-8 sequence: back up over continuation bytes.
    while end > 0 && end < raw.len() && (raw[end] & 0xC0) == 0x80 {
        end -= 1;
    }
    let mut s = String::from_utf8_lossy(&raw[..end]).into_owned();
    s.push('…');
    s
}

fn preview(raw: &[u8]) -> String {
    if raw.len() <= PREVIEW_BYTES {
        String::from_utf8_lossy(raw).into_owned()
    } else {
        cut(raw, PREVIEW_BYTES)
    }
}
