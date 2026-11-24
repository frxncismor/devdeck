//! Single-pass JSON tokenizer (T049), grammar walker and container index (T050).
//!
//! The tokenizer works over `&[u8]` and never allocates values: tokens are byte spans. The walker
//! enforces the JSON grammar iteratively (no recursion, so depth is only limited by memory) and
//! reports events to a [`Visitor`]. The index builder is one visitor, the search is another.

use crate::error::{Error, ErrorKind, Result};
use std::sync::atomic::{AtomicBool, Ordering};

/// Every `JUMP_EVERY`-th child of a container is stored as a jump point (sparse index).
pub const JUMP_EVERY: u32 = 1024;
/// Marks a node id as a leaf id: `LEAF_BIT | member_start_offset`. Kept below 2^53 so it is a safe JS number.
pub const LEAF_BIT: u64 = 1 << 52;
const NO_PARENT: u32 = u32::MAX;
const NO_JUMPS: u32 = u32::MAX;
/// How often (bytes) the walker checks cancellation and reports progress.
const TICK_BYTES: usize = 256 * 1024;

pub fn invalid(offset: usize, detail: &str) -> Error {
    Error::new(ErrorKind::Parse, "json_invalid_at_offset")
        .with_param("offset", offset as u64)
        .with_detail(detail)
}

fn too_large() -> Error {
    Error::new(ErrorKind::Unsupported, "json_too_large")
}

// ---------------------------------------------------------------------------------------------
// Tokenizer (T049)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    BeginObject,
    EndObject,
    BeginArray,
    EndArray,
    Colon,
    Comma,
    String,
    Number,
    True,
    False,
    Null,
}

/// A token and its byte span `start..end` (strings include their quotes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

pub struct Tokenizer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    /// Starts at the beginning of `data`, skipping an optional UTF-8 BOM.
    pub fn new(data: &'a [u8]) -> Self {
        let pos = if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            3
        } else {
            0
        };
        Self { data, pos }
    }

    /// Starts at an arbitrary offset (used by the offset reader on already validated data).
    pub fn at(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Skips whitespace and returns the new position.
    pub fn skip_ws(&mut self) -> usize {
        while let Some(&b) = self.data.get(self.pos) {
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.pos
    }

    /// Next token, or `None` at the end of the input.
    pub fn next_token(&mut self) -> Result<Option<Token>> {
        let start = self.skip_ws();
        let Some(&b) = self.data.get(start) else {
            return Ok(None);
        };
        let single = |kind| Token {
            kind,
            start,
            end: start + 1,
        };
        let tok = match b {
            b'{' => single(TokenKind::BeginObject),
            b'}' => single(TokenKind::EndObject),
            b'[' => single(TokenKind::BeginArray),
            b']' => single(TokenKind::EndArray),
            b':' => single(TokenKind::Colon),
            b',' => single(TokenKind::Comma),
            b'"' => Token {
                kind: TokenKind::String,
                start,
                end: self.scan_string(start)?,
            },
            b'-' | b'0'..=b'9' => Token {
                kind: TokenKind::Number,
                start,
                end: self.scan_number(start)?,
            },
            b't' => self.literal(start, b"true", TokenKind::True)?,
            b'f' => self.literal(start, b"false", TokenKind::False)?,
            b'n' => self.literal(start, b"null", TokenKind::Null)?,
            _ => return Err(invalid(start, "unexpected character")),
        };
        self.pos = tok.end;
        Ok(Some(tok))
    }

    fn literal(&self, start: usize, word: &[u8], kind: TokenKind) -> Result<Token> {
        if self.data[start..].starts_with(word) {
            Ok(Token {
                kind,
                start,
                end: start + word.len(),
            })
        } else {
            Err(invalid(start, "invalid literal"))
        }
    }

    /// Returns the end offset (after the closing quote). Validates escapes, control characters and UTF-8.
    fn scan_string(&self, start: usize) -> Result<usize> {
        let data = self.data;
        let mut i = start + 1;
        let mut non_ascii = false;
        loop {
            // Fast path over plain ASCII bytes.
            while let Some(&b) = data.get(i) {
                if (0x20..0x80).contains(&b) && b != b'"' && b != b'\\' {
                    i += 1;
                } else {
                    break;
                }
            }
            let Some(&b) = data.get(i) else {
                return Err(invalid(start, "unterminated string"));
            };
            match b {
                b'"' => break,
                b'\\' => match data.get(i + 1) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => i += 2,
                    Some(b'u') => {
                        let hex = data.get(i + 2..i + 6);
                        if !hex.is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)) {
                            return Err(invalid(i, "invalid unicode escape"));
                        }
                        i += 6;
                    }
                    _ => return Err(invalid(i, "invalid escape")),
                },
                0..=0x1f => return Err(invalid(i, "control character in string")),
                _ => {
                    non_ascii = true;
                    i += 1;
                }
            }
        }
        if non_ascii {
            if let Err(e) = std::str::from_utf8(&data[start + 1..i]) {
                return Err(invalid(start + 1 + e.valid_up_to(), "invalid UTF-8"));
            }
        }
        Ok(i + 1)
    }

    /// `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`
    fn scan_number(&self, start: usize) -> Result<usize> {
        let data = self.data;
        let mut i = start;
        let digits = |mut i: usize| {
            while data.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            i
        };
        if data.get(i) == Some(&b'-') {
            i += 1;
        }
        match data.get(i) {
            Some(b'0') => i += 1,
            Some(b'1'..=b'9') => i = digits(i),
            _ => return Err(invalid(i, "invalid number")),
        }
        if data.get(i) == Some(&b'.') {
            let after = digits(i + 1);
            if after == i + 1 {
                return Err(invalid(i + 1, "digits expected after '.'"));
            }
            i = after;
        }
        if matches!(data.get(i), Some(b'e' | b'E')) {
            let mut j = i + 1;
            if matches!(data.get(j), Some(b'+' | b'-')) {
                j += 1;
            }
            let after = digits(j);
            if after == j {
                return Err(invalid(j, "digits expected in exponent"));
            }
            i = after;
        }
        Ok(i)
    }
}

// ---------------------------------------------------------------------------------------------
// Walker
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Object,
    Array,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JsonKind {
    Object,
    Array,
    String,
    Number,
    Bool,
    Null,
}

impl From<ContainerKind> for JsonKind {
    fn from(k: ContainerKind) -> Self {
        match k {
            ContainerKind::Object => JsonKind::Object,
            ContainerKind::Array => JsonKind::Array,
        }
    }
}

/// Where a value sits inside its parent. `start` is the member start: the key's opening quote for
/// object members, the value start for array elements and the top-level value.
#[derive(Debug, Clone, Copy, Default)]
pub struct Member {
    pub start: usize,
    /// Quoted key span `start..end` (object members only).
    pub key: Option<(usize, usize)>,
    /// Child index inside the parent.
    pub index: u32,
}

pub trait Visitor {
    fn enter(&mut self, kind: ContainerKind, member: &Member, start: usize) -> Result<()>;
    /// `end` is the offset after the closing bracket.
    fn exit(&mut self, end: usize, child_count: u32) -> Result<()>;
    fn leaf(&mut self, member: &Member, kind: JsonKind, start: usize, end: usize) -> Result<()>;
    /// Start of child number `k * JUMP_EVERY` of the innermost open container.
    fn jump(&mut self, member_start: usize);
    /// Called about every 256 KiB: cancellation / progress.
    fn tick(&mut self, _pos: usize) -> Result<()> {
        Ok(())
    }
}

struct Frame {
    kind: ContainerKind,
    count: u32,
}

enum State {
    Value,
    FirstChildOrClose,
    After,
}

fn close<V: Visitor>(stack: &mut Vec<Frame>, v: &mut V, end: usize) -> Result<()> {
    let f = stack.pop().expect("close without an open container");
    v.exit(end, f.count)?;
    if let Some(parent) = stack.last_mut() {
        parent.count = parent.count.checked_add(1).ok_or_else(too_large)?;
    }
    Ok(())
}

/// Reads `"key" :` and returns the quoted key span.
fn read_key(tk: &mut Tokenizer) -> Result<(usize, usize)> {
    let key = tk
        .next_token()?
        .ok_or_else(|| invalid(tk.data.len(), "unexpected end of input"))?;
    if key.kind != TokenKind::String {
        return Err(invalid(key.start, "object key must be a string"));
    }
    let colon = tk
        .next_token()?
        .ok_or_else(|| invalid(tk.data.len(), "unexpected end of input"))?;
    if colon.kind != TokenKind::Colon {
        return Err(invalid(colon.start, "':' expected after the key"));
    }
    Ok((key.start, key.end))
}

/// Walks a whole document, validating the grammar, and reports events to `v`.
pub fn walk<V: Visitor>(data: &[u8], v: &mut V) -> Result<()> {
    let mut tk = Tokenizer::new(data);
    let eof = || invalid(data.len(), "unexpected end of input");
    let mut stack: Vec<Frame> = Vec::new();
    let mut member = Member::default();
    let mut state = State::Value;
    let mut next_tick = TICK_BYTES;
    loop {
        if tk.pos >= next_tick {
            v.tick(tk.pos)?;
            next_tick = tk.pos + TICK_BYTES;
        }
        match state {
            State::Value => {
                let tok = tk.next_token()?.ok_or_else(eof)?;
                match stack.last() {
                    Some(f) if f.kind == ContainerKind::Object => {} // member prepared with its key
                    Some(f) => {
                        member = Member {
                            start: tok.start,
                            key: None,
                            index: f.count,
                        }
                    }
                    None => member = Member::default().with_start(tok.start),
                }
                match tok.kind {
                    TokenKind::BeginObject | TokenKind::BeginArray => {
                        let kind = if tok.kind == TokenKind::BeginObject {
                            ContainerKind::Object
                        } else {
                            ContainerKind::Array
                        };
                        v.enter(kind, &member, tok.start)?;
                        stack.push(Frame { kind, count: 0 });
                        state = State::FirstChildOrClose;
                    }
                    TokenKind::String
                    | TokenKind::Number
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::Null => {
                        let kind = match tok.kind {
                            TokenKind::String => JsonKind::String,
                            TokenKind::Number => JsonKind::Number,
                            TokenKind::Null => JsonKind::Null,
                            _ => JsonKind::Bool,
                        };
                        v.leaf(&member, kind, tok.start, tok.end)?;
                        if let Some(f) = stack.last_mut() {
                            f.count = f.count.checked_add(1).ok_or_else(too_large)?;
                        }
                        state = State::After;
                    }
                    _ => return Err(invalid(tok.start, "value expected")),
                }
            }
            State::FirstChildOrClose => {
                let kind = stack.last().expect("open container").kind;
                let tok = tk.next_token()?.ok_or_else(eof)?;
                match (tok.kind, kind) {
                    (TokenKind::EndObject, ContainerKind::Object)
                    | (TokenKind::EndArray, ContainerKind::Array) => {
                        close(&mut stack, v, tok.end)?;
                        state = State::After;
                    }
                    (_, ContainerKind::Object) => {
                        tk.set_pos(tok.start);
                        let key = read_key(&mut tk)?;
                        member = Member {
                            start: key.0,
                            key: Some(key),
                            index: 0,
                        };
                        state = State::Value;
                    }
                    (_, ContainerKind::Array) => {
                        tk.set_pos(tok.start);
                        state = State::Value;
                    }
                }
            }
            State::After => {
                let Some(frame) = stack.last() else {
                    return match tk.next_token()? {
                        None => Ok(()),
                        Some(t) => Err(invalid(t.start, "unexpected data after the value")),
                    };
                };
                let (kind, count) = (frame.kind, frame.count);
                let tok = tk.next_token()?.ok_or_else(eof)?;
                match (tok.kind, kind) {
                    (TokenKind::Comma, _) => {
                        if count % JUMP_EVERY == 0 {
                            v.jump(tk.skip_ws());
                        }
                        if kind == ContainerKind::Object {
                            let key = read_key(&mut tk)?;
                            member = Member {
                                start: key.0,
                                key: Some(key),
                                index: count,
                            };
                        }
                        state = State::Value;
                    }
                    (TokenKind::EndObject, ContainerKind::Object)
                    | (TokenKind::EndArray, ContainerKind::Array) => {
                        close(&mut stack, v, tok.end)?;
                    }
                    _ => return Err(invalid(tok.start, "',' or closing bracket expected")),
                }
            }
        }
    }
}

impl Member {
    fn with_start(mut self, start: usize) -> Self {
        self.start = start;
        self
    }
}

// ---------------------------------------------------------------------------------------------
// Container index (T050)
// ---------------------------------------------------------------------------------------------

/// A container (object/array). 40 bytes; leaves are never indexed.
#[derive(Debug, Clone, Copy)]
pub struct Node {
    /// Offset of `{` / `[`.
    pub start: u64,
    /// Object member: offset of the key's opening quote. Array element: the index.
    pub key_off: u64,
    /// Byte length of the container (`start + len` = offset after the closing bracket).
    pub len: u32,
    pub child_count: u32,
    pub parent: u32,
    /// Object member: byte length of the quoted key. Array element / root: 0.
    pub key_len: u32,
    /// Index into `Index::jumps` of this container's first jump point, or `u32::MAX`.
    pub jump_start: u32,
    pub kind: ContainerKind,
}

impl Node {
    pub fn end(&self) -> u64 {
        self.start + u64::from(self.len)
    }
}

/// Containers in pre-order (so `start` is increasing and a node can be found by binary search)
/// plus the jump points of the containers with more than `JUMP_EVERY` children.
#[derive(Debug, Default)]
pub struct Index {
    pub nodes: Vec<Node>,
    pub jumps: Vec<u64>,
    /// Id of the root: a node index, or a leaf id when the document is a scalar.
    pub root_id: u64,
}

impl Index {
    /// Jump points of `node` (child `k * JUMP_EVERY` starts at `jumps(node)[k - 1]`).
    pub fn jumps_of(&self, node: &Node) -> &[u64] {
        if node.jump_start == NO_JUMPS {
            return &[];
        }
        let n = (node.child_count / JUMP_EVERY) as usize;
        &self.jumps[node.jump_start as usize..node.jump_start as usize + n]
    }

    /// Node whose container starts exactly at `offset`.
    pub fn node_at(&self, offset: u64) -> Option<u32> {
        let i = self.nodes.partition_point(|n| n.start < offset);
        (self.nodes.get(i)?.start == offset).then_some(i as u32)
    }

    /// Innermost container that contains `offset`.
    pub fn container_of(&self, offset: u64) -> Option<u32> {
        let mut i = self
            .nodes
            .partition_point(|n| n.start <= offset)
            .checked_sub(1)? as u32;
        loop {
            let n = &self.nodes[i as usize];
            if offset < n.end() {
                return Some(i);
            }
            if n.parent == NO_PARENT {
                return None;
            }
            i = n.parent;
        }
    }
}

struct Open {
    id: u32,
    jumps: Vec<u64>,
}

struct Builder<'a> {
    index: Index,
    open: Vec<Open>,
    total: u64,
    step: u64,
    last_reported: u64,
    cancel: &'a AtomicBool,
    progress: &'a mut dyn FnMut(u64),
    scalar_root: Option<usize>,
}

impl Visitor for Builder<'_> {
    fn enter(&mut self, kind: ContainerKind, member: &Member, start: usize) -> Result<()> {
        let id = u32::try_from(self.index.nodes.len()).map_err(|_| too_large())?;
        let (key_off, key_len) = match member.key {
            Some((s, e)) => (s as u64, u32::try_from(e - s).map_err(|_| too_large())?),
            None => (u64::from(member.index), 0),
        };
        let parent = self.open.last().map_or(NO_PARENT, |o| o.id);
        self.index.nodes.push(Node {
            start: start as u64,
            key_off,
            len: 0,
            child_count: 0,
            parent,
            key_len,
            jump_start: NO_JUMPS,
            kind,
        });
        self.open.push(Open {
            id,
            jumps: Vec::new(),
        });
        Ok(())
    }

    fn exit(&mut self, end: usize, child_count: u32) -> Result<()> {
        let open = self.open.pop().expect("exit without enter");
        let node = &mut self.index.nodes[open.id as usize];
        node.len = u32::try_from(end as u64 - node.start).map_err(|_| too_large())?;
        node.child_count = child_count;
        if !open.jumps.is_empty() {
            node.jump_start = u32::try_from(self.index.jumps.len()).map_err(|_| too_large())?;
            self.index.jumps.extend_from_slice(&open.jumps);
        }
        Ok(())
    }

    fn leaf(&mut self, member: &Member, _kind: JsonKind, _start: usize, _end: usize) -> Result<()> {
        if self.open.is_empty() {
            self.scalar_root = Some(member.start);
        }
        Ok(())
    }

    fn jump(&mut self, member_start: usize) {
        if let Some(o) = self.open.last_mut() {
            o.jumps.push(member_start as u64);
        }
    }

    fn tick(&mut self, pos: usize) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(Error::cancelled());
        }
        let pos = pos as u64;
        if pos - self.last_reported >= self.step {
            self.last_reported = pos;
            (self.progress)(pos.min(self.total));
        }
        Ok(())
    }
}

/// Builds the container index in one pass. `progress(bytes_done)` is called about every 1 % and
/// `cancel` is polled about every 256 KiB (`Error::cancelled()` when set).
pub fn build_index(
    data: &[u8],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<Index> {
    let total = data.len() as u64;
    let mut b = Builder {
        index: Index::default(),
        open: Vec::new(),
        total,
        step: (total / 100).max(1),
        last_reported: 0,
        cancel,
        progress,
        scalar_root: None,
    };
    walk(data, &mut b)?;
    (b.progress)(total);
    let mut index = b.index;
    index.root_id = match b.scalar_root {
        Some(start) => LEAF_BIT | start as u64,
        None => 0,
    };
    index.nodes.shrink_to_fit();
    index.jumps.shrink_to_fit();
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(s: &str) -> Result<Vec<(TokenKind, &str)>> {
        let mut tk = Tokenizer::new(s.as_bytes());
        let mut out = Vec::new();
        while let Some(t) = tk.next_token()? {
            out.push((t.kind, &s[t.start..t.end]));
        }
        Ok(out)
    }

    fn err_offset(s: &str) -> u64 {
        let e = build_index(s.as_bytes(), &AtomicBool::new(false), &mut |_| {}).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Parse);
        assert_eq!(e.code, "json_invalid_at_offset");
        e.params.unwrap()["offset"].as_u64().unwrap()
    }

    fn idx(s: &str) -> Index {
        build_index(s.as_bytes(), &AtomicBool::new(false), &mut |_| {}).unwrap()
    }

    #[test]
    fn tokenizes_structure_and_literals() {
        let t = tokens(r#" {"a": [1, true, false, null]} "#).unwrap();
        let kinds: Vec<_> = t.iter().map(|(k, _)| *k).collect();
        use TokenKind::*;
        assert_eq!(
            kinds,
            [
                BeginObject,
                String,
                Colon,
                BeginArray,
                Number,
                Comma,
                True,
                Comma,
                False,
                Comma,
                Null,
                EndArray,
                EndObject
            ]
        );
    }

    #[test]
    fn strings_with_unicode_and_escapes() {
        let src = r#""héllo é 日本語 😀 \" \\ \/ \b\f\n\r\t""#;
        let t = tokens(src).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].1, src);
    }

    #[test]
    fn rejects_bad_strings() {
        assert!(tokens(r#""abc"#).is_err());
        assert!(tokens(r#""\x""#).is_err());
        assert!(tokens(r#""\u12G4""#).is_err());
        assert!(tokens("\"a\nb\"").is_err());
        let e = tokens("\"a\tb\"").unwrap_err();
        assert_eq!(e.params.unwrap()["offset"], 2);
        let bad_utf8 = [b'"', b'a', 0xFF, b'"'];
        assert!(Tokenizer::new(&bad_utf8).next_token().is_err());
    }

    #[test]
    fn numbers() {
        for ok in ["0", "-0", "12", "-12.5", "1e5", "1E+5", "1.5e-10", "0.0"] {
            let t = tokens(ok).unwrap();
            assert_eq!(t, vec![(TokenKind::Number, ok)], "{ok}");
        }
        for bad in ["-", "1.", ".5", "1e", "1e+", "-a"] {
            assert!(tokens(bad).is_err(), "{bad}");
        }
        // Leading zeros: the number is `0`, the walker rejects the rest.
        assert_eq!(err_offset("01"), 1);
    }

    #[test]
    fn literals_and_garbage() {
        assert!(tokens("tru").is_err());
        assert!(tokens("nul").is_err());
        assert!(tokens("@").is_err());
        assert_eq!(err_offset("[1, x]"), 4);
    }

    #[test]
    fn skips_utf8_bom() {
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(b"[1]");
        let i = build_index(&data, &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(i.nodes[0].start, 3);
    }

    #[test]
    fn grammar_errors_report_the_offset() {
        assert_eq!(err_offset("[1,]"), 3);
        assert_eq!(err_offset("{\"a\" 1}"), 5);
        assert_eq!(err_offset("{\"a\":1,}"), 7);
        assert_eq!(err_offset("{1:2}"), 1);
        assert_eq!(err_offset("[1 2]"), 3);
        assert_eq!(err_offset("[1]]"), 3);
        assert_eq!(err_offset("[1"), 2);
        assert_eq!(err_offset(""), 0);
        assert_eq!(err_offset("   "), 3);
        assert_eq!(err_offset("{\"a\":}"), 5);
    }

    #[test]
    fn indexes_only_containers() {
        let i = idx(r#"{"a":[1,2,{"b":null}],"c":{},"d":"x"}"#);
        assert_eq!(i.nodes.len(), 4); // root, a, a[2], c
        let root = &i.nodes[0];
        assert_eq!(
            (root.kind, root.child_count, root.parent),
            (ContainerKind::Object, 3, NO_PARENT)
        );
        assert_eq!(root.start, 0);
        assert_eq!(
            root.len as usize,
            r#"{"a":[1,2,{"b":null}],"c":{},"d":"x"}"#.len()
        );
        let a = &i.nodes[1];
        assert_eq!(
            (a.kind, a.child_count, a.parent),
            (ContainerKind::Array, 3, 0)
        );
        let inner = &i.nodes[2];
        assert_eq!((inner.child_count, inner.parent, inner.key_off), (1, 1, 2)); // index 2 in `a`
        let c = &i.nodes[3];
        assert_eq!((c.child_count, c.parent), (0, 0));
        assert_eq!(i.root_id, 0);
        assert_eq!(i.node_at(inner.start), Some(2));
        assert_eq!(i.node_at(inner.start + 1), None);
        assert_eq!(i.container_of(inner.start + 2), Some(2));
    }

    #[test]
    fn scalar_root() {
        let i = idx("  42 ");
        assert!(i.nodes.is_empty());
        assert_eq!(i.root_id, LEAF_BIT | 2);
    }

    #[test]
    fn records_jump_points_every_1024_children() {
        let n = 3000;
        let doc = format!(
            "[{}]",
            (0..n).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
        );
        let i = idx(&doc);
        let node = &i.nodes[0];
        assert_eq!(node.child_count, n);
        let jumps = i.jumps_of(node);
        assert_eq!(jumps.len(), 2);
        assert_eq!(
            &doc[jumps[0] as usize..].split(',').next().unwrap(),
            &"1024"
        );
        assert_eq!(
            &doc[jumps[1] as usize..].split(',').next().unwrap(),
            &"2048"
        );
    }

    #[test]
    fn jump_points_for_object_members_point_at_the_key() {
        let doc = format!(
            "{{{}}}",
            (0..2050)
                .map(|i| format!("\"k{i}\":{i}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let i = idx(&doc);
        let jumps = i.jumps_of(&i.nodes[0]);
        assert_eq!(jumps.len(), 2);
        assert!(doc[jumps[0] as usize..].starts_with("\"k1024\":1024"));
        assert!(doc[jumps[1] as usize..].starts_with("\"k2048\":2048"));
    }

    #[test]
    fn deep_nesting_does_not_overflow_the_stack() {
        let depth = 200_000;
        let doc = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let i = idx(&doc);
        assert_eq!(i.nodes.len(), depth);
    }

    #[test]
    fn cancellation_and_progress() {
        let doc = format!(
            "[{}]",
            (0..400_000)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let cancel = AtomicBool::new(true);
        let e = build_index(doc.as_bytes(), &cancel, &mut |_| {}).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Cancelled);

        let mut seen = Vec::new();
        build_index(doc.as_bytes(), &AtomicBool::new(false), &mut |p| {
            seen.push(p)
        })
        .unwrap();
        assert!(seen.len() > 2, "progress reported {} times", seen.len());
        assert!(seen.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(*seen.last().unwrap(), doc.len() as u64);
    }

    #[test]
    fn node_is_compact() {
        assert!(std::mem::size_of::<Node>() <= 40);
    }
}
