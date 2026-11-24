//! Pretty-printing without parsing into a `Value` (T054): subtree formatting with a size cap and
//! streaming "save formatted". Works on data already validated by the indexer.

use super::reader::JsonDoc;
use crate::error::{Error, ErrorKind, Result};
use memchr::memchr2;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Largest subtree (raw bytes) that `format_node` pretty-prints in memory.
pub const MAX_FORMAT_BYTES: u64 = 5 * 1024 * 1024;
const INDENT: usize = 2;
const TICK_BYTES: usize = 256 * 1024;
const SPACES: [u8; 64] = [b' '; 64];

fn string_end(data: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while let Some(off) = memchr2(b'"', b'\\', &data[i..]) {
        i += off;
        if data[i] == b'"' {
            return i + 1;
        }
        i += 2; // skip the escaped byte
    }
    data.len()
}

fn newline<W: Write>(out: &mut W, depth: usize) -> std::io::Result<()> {
    out.write_all(b"\n")?;
    let mut n = depth * INDENT;
    while n > 0 {
        let k = n.min(SPACES.len());
        out.write_all(&SPACES[..k])?;
        n -= k;
    }
    Ok(())
}

/// Pretty-prints `data[span]` (indent 2, `"key": value`, empty containers as `{}` / `[]`).
/// `tick(pos)` is called about every 256 KiB and may abort the operation.
pub fn pretty_print<W: Write>(
    data: &[u8],
    span: (usize, usize),
    out: &mut W,
    tick: &mut dyn FnMut(usize) -> Result<()>,
) -> Result<()> {
    let (mut i, end) = span;
    let end = end.min(data.len());
    let mut depth = 0usize;
    let mut next_tick = i + TICK_BYTES;
    while i < end {
        if i >= next_tick {
            tick(i)?;
            next_tick = i + TICK_BYTES;
        }
        let b = data[i];
        match b {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'"' => {
                let e = string_end(data, i).min(end);
                out.write_all(&data[i..e])?;
                i = e;
            }
            b'{' | b'[' => {
                let close = if b == b'{' { b'}' } else { b']' };
                let mut j = i + 1;
                while j < end && matches!(data[j], b' ' | b'\t' | b'\n' | b'\r') {
                    j += 1;
                }
                if j < end && data[j] == close {
                    out.write_all(&[b, close])?;
                    i = j + 1;
                } else {
                    out.write_all(&[b])?;
                    depth += 1;
                    newline(out, depth)?;
                    i += 1;
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                newline(out, depth)?;
                out.write_all(&[b])?;
                i += 1;
            }
            b',' => {
                out.write_all(b",")?;
                newline(out, depth)?;
                i += 1;
            }
            b':' => {
                out.write_all(b": ")?;
                i += 1;
            }
            _ => {
                let mut j = i;
                while j < end
                    && !matches!(
                        data[j],
                        b' ' | b'\t' | b'\n' | b'\r' | b',' | b']' | b'}' | b':'
                    )
                {
                    j += 1;
                }
                out.write_all(&data[i..j])?;
                i = j;
            }
        }
    }
    Ok(())
}

/// Pretty-prints one node into memory. Fails with `json_value_too_large` when its raw size exceeds
/// `max_bytes` (capped to [`MAX_FORMAT_BYTES`]).
pub fn format_node(doc: &JsonDoc, id: u64, max_bytes: u64) -> Result<String> {
    let (s, e) = doc.value_span(id)?;
    let cap = max_bytes.min(MAX_FORMAT_BYTES);
    if (e - s) as u64 > cap {
        return Err(Error::new(ErrorKind::Unsupported, "json_value_too_large")
            .with_param("maxBytes", cap)
            .with_param("size", (e - s) as u64));
    }
    let mut out = Vec::with_capacity((e - s) * 2);
    pretty_print(doc.data(), (s, e), &mut out, &mut |_| Ok(()))?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// Streams the formatted node (or the whole document) to `dest`. Returns the bytes written.
/// Refuses to overwrite the source file (it is memory-mapped). A partial file is removed on
/// error or cancellation. `progress(done, total)` is in source bytes.
pub fn save_formatted(
    doc: &JsonDoc,
    id: Option<u64>,
    dest: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<u64> {
    if dest.exists() {
        let same = std::fs::canonicalize(dest)? == std::fs::canonicalize(doc.path())?;
        if same {
            return Err(Error::not_allowed("json_save_same_file"));
        }
    }
    let (s, e) = match id {
        Some(id) => doc.value_span(id)?,
        None => (0, doc.data().len()),
    };
    let total = (e - s) as u64;
    let result = (|| -> Result<u64> {
        let file = File::create(dest)?;
        let mut w = BufWriter::with_capacity(1 << 20, file);
        let mut tick = |pos: usize| {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::cancelled());
            }
            progress((pos - s) as u64, total);
            Ok(())
        };
        pretty_print(doc.data(), (s, e), &mut w, &mut tick)?;
        w.write_all(b"\n")?;
        w.flush()?;
        let written = w.get_ref().metadata()?.len();
        progress(total, total);
        Ok(written)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pp(src: &str) -> String {
        let mut out = Vec::new();
        pretty_print(src.as_bytes(), (0, src.len()), &mut out, &mut |_| Ok(())).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn formats_nested_values() {
        let got = pp(r#"{"a":[1,2,{"b":null}],"c":{},"d":[],"e":"x, y: {z}"}"#);
        let want = "{\n  \"a\": [\n    1,\n    2,\n    {\n      \"b\": null\n    }\n  ],\n  \"c\": {},\n  \"d\": [],\n  \"e\": \"x, y: {z}\"\n}";
        assert_eq!(got, want);
    }

    #[test]
    fn keeps_escapes_and_unicode_verbatim() {
        let src = r#"["a\"b\\","日本語 😀","é"]"#;
        let got = pp(src);
        assert!(got.contains(r#""a\"b\\""#));
        assert!(got.contains("日本語 😀"));
        let reparsed: serde_json::Value = serde_json::from_str(&got).unwrap();
        let orig: serde_json::Value = serde_json::from_str(src).unwrap();
        assert_eq!(reparsed, orig);
    }

    #[test]
    fn scalars_and_whitespace_input() {
        assert_eq!(pp("  42 "), "42");
        assert_eq!(
            pp("{ \"a\" :\n 1 ,\t\"b\" : [ ] }"),
            "{\n  \"a\": 1,\n  \"b\": []\n}"
        );
    }

    #[test]
    fn tick_can_abort() {
        let src = format!("[{}]", "1,".repeat(300_000) + "1");
        let mut out = Vec::new();
        let e = pretty_print(src.as_bytes(), (0, src.len()), &mut out, &mut |_| {
            Err(Error::cancelled())
        })
        .unwrap_err();
        assert_eq!(e.kind, ErrorKind::Cancelled);
    }
}
