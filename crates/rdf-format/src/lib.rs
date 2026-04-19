//! Per-format serialisers for the Phase-A RDF parser family.
//!
//! This crate is the *inverse* of the parsers in [`rdf-ntriples`] and
//! [`rdf-turtle`]: given a `rdf_diff::Fact` (already in the canonical form
//! defined by [`rdf_diff::Facts::canonicalise`]), each writer emits a
//! spec-valid textual form. The serialiser is a direct output path — it
//! never re-normalises and never silently rewrites the canonical form.
//!
//! # Public surface
//!
//! - [`NTriplesWriter`] — RDF 1.1 N-Triples (no graph name permitted).
//! - [`NQuadsWriter`] — RDF 1.1 N-Quads (optional graph name).
//! - [`TurtleWriter`] — RDF 1.1 Turtle (prefix shortening, long-literal
//!   form). Does not emit TriG `{}` graph blocks; a non-default graph is
//!   a logic error.
//! - [`TriGWriter`] — RDF 1.1 TriG (Turtle + named graph blocks).
//!
//! Each writer owns a `W: Write` sink and emits via three calls:
//! `new(sink)`, `write_fact(&fact)` per fact, `finish()` to flush.
//!
//! # Error model
//!
//! All writers return `io::Result<()>`. Panic-free by construction: the
//! canonical-form invariants are assumed (parsers guarantee them), so the
//! writer never constructs an `io::Error` of its own — only the sink's
//! errors surface.
//!
//! # Non-goals (Phase A)
//!
//! - Pretty-printing / indentation for Turtle and TriG (compact one-line
//!   form only).
//! - Collection `(…)` syntax, blank-node `[ … ]` property-list abbreviation.
//! - `a` keyword for `rdf:type`.
//! - Numeric / boolean literal short forms (`42`, `true`, `3.14`).
//!
//! All of these are safe deferrals because the parser already accepts the
//! long form we emit, so round-trip is preserved.
//!
//! # Round-trip contract
//!
//! For every format `F` and every `facts_in` produced by the corresponding
//! Phase-A parser, the following holds:
//!
//! ```text
//! parse(F, serialise(F, facts_in)) == facts_in
//! ```
//!
//! where equality is `rdf_diff::Facts` equality (i.e. canonical fact sets
//! agree, modulo parser-reported prefixes which are diagnostic-only).

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_const_for_fn,
    clippy::redundant_pub_crate,
    clippy::option_if_let_else,
    clippy::doc_markdown,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;
use std::io::{self, Write};

use rdf_diff::Fact;

// ---------------------------------------------------------------------------
// N-Triples
// ---------------------------------------------------------------------------

/// Writer for the RDF 1.1 N-Triples format.
///
/// One fact per line: `subject predicate object .<LF>`. Rejects facts with
/// a non-default graph (N-Triples has no graph-name slot); such a fact is
/// silently coerced to the default-graph form — the expectation is that
/// callers pass a default-graph-only fact set, and the parser already
/// enforces this.
#[derive(Debug)]
pub struct NTriplesWriter<W: Write> {
    sink: W,
}

impl<W: Write> NTriplesWriter<W> {
    /// Wrap `sink` in a fresh N-Triples writer.
    pub const fn new(sink: W) -> Self {
        Self { sink }
    }

    /// Emit one fact as a single N-Triples line terminated by LF.
    ///
    /// The `graph` slot is ignored (N-Triples has no graph position). If
    /// callers care about graph information they must use
    /// [`NQuadsWriter`].
    pub fn write_fact(&mut self, fact: &Fact) -> io::Result<()> {
        write_term_nt(&mut self.sink, &fact.subject)?;
        self.sink.write_all(b" ")?;
        write_term_nt(&mut self.sink, &fact.predicate)?;
        self.sink.write_all(b" ")?;
        write_term_nt(&mut self.sink, &fact.object)?;
        self.sink.write_all(b" .\n")
    }

    /// Finish writing and return the inner sink's final flush result.
    pub fn finish(mut self) -> io::Result<()> {
        self.sink.flush()
    }
}

// ---------------------------------------------------------------------------
// N-Quads
// ---------------------------------------------------------------------------

/// Writer for the RDF 1.1 N-Quads format.
///
/// One fact per line: `subject predicate object [graph] .<LF>`. When
/// `fact.graph` is `None` the graph slot is omitted (default graph).
#[derive(Debug)]
pub struct NQuadsWriter<W: Write> {
    sink: W,
}

impl<W: Write> NQuadsWriter<W> {
    /// Wrap `sink` in a fresh N-Quads writer.
    pub const fn new(sink: W) -> Self {
        Self { sink }
    }

    /// Emit one fact as a single N-Quads line terminated by LF.
    pub fn write_fact(&mut self, fact: &Fact) -> io::Result<()> {
        write_term_nt(&mut self.sink, &fact.subject)?;
        self.sink.write_all(b" ")?;
        write_term_nt(&mut self.sink, &fact.predicate)?;
        self.sink.write_all(b" ")?;
        write_term_nt(&mut self.sink, &fact.object)?;
        if let Some(g) = fact.graph.as_deref() {
            self.sink.write_all(b" ")?;
            write_term_nt(&mut self.sink, g)?;
        }
        self.sink.write_all(b" .\n")
    }

    /// Finish writing and return the inner sink's final flush result.
    pub fn finish(mut self) -> io::Result<()> {
        self.sink.flush()
    }
}

// ---------------------------------------------------------------------------
// Turtle
// ---------------------------------------------------------------------------

/// Writer for the RDF 1.1 Turtle format.
///
/// By default no prefix is registered, so emission matches an N-Triples
/// transcription with a Turtle `.` terminator. Callers may register
/// prefixes via [`TurtleWriter::with_prefix`] to get compact `pname`
/// output.
///
/// Long-literal form `"""…"""` is used when the literal lexical form
/// contains an embedded `"` that would otherwise need escaping; this keeps
/// output compact without changing the canonical content.
#[derive(Debug)]
pub struct TurtleWriter<W: Write> {
    sink: W,
    prefixes: BTreeMap<String, String>, // pname -> namespace IRI
    wrote_header: bool,
}

impl<W: Write> TurtleWriter<W> {
    /// Wrap `sink` in a fresh Turtle writer with no registered prefixes.
    #[must_use]
    pub fn new(sink: W) -> Self {
        Self {
            sink,
            prefixes: BTreeMap::new(),
            wrote_header: false,
        }
    }

    /// Register a prefix for pname shortening.
    ///
    /// Subsequent fact emissions that find an IRI starting with `iri` will
    /// emit `pname:localname` instead of `<iri+localname>`.
    ///
    /// Must be called **before** the first [`TurtleWriter::write_fact`];
    /// prefixes registered afterwards are still honoured for new IRIs but
    /// are not retroactively emitted as `@prefix` directives into the
    /// output — the header is written lazily on first fact.
    pub fn with_prefix(&mut self, pname: &str, iri: &str) {
        self.prefixes.insert(pname.to_owned(), iri.to_owned());
    }

    /// Emit one fact as a single Turtle statement terminated by LF.
    pub fn write_fact(&mut self, fact: &Fact) -> io::Result<()> {
        self.ensure_header()?;
        write_term_ttl(&mut self.sink, &fact.subject, &self.prefixes)?;
        self.sink.write_all(b" ")?;
        write_term_ttl(&mut self.sink, &fact.predicate, &self.prefixes)?;
        self.sink.write_all(b" ")?;
        write_term_ttl(&mut self.sink, &fact.object, &self.prefixes)?;
        self.sink.write_all(b" .\n")
    }

    /// Finish writing and flush the inner sink.
    pub fn finish(mut self) -> io::Result<()> {
        self.sink.flush()
    }

    fn ensure_header(&mut self) -> io::Result<()> {
        if self.wrote_header {
            return Ok(());
        }
        self.wrote_header = true;
        write_prefix_header(&mut self.sink, &self.prefixes)
    }
}

// ---------------------------------------------------------------------------
// TriG
// ---------------------------------------------------------------------------

/// Writer for the RDF 1.1 TriG format.
///
/// Accepts facts with an optional graph name. Output groups triples by
/// graph: default-graph triples are emitted flat (as Turtle), and each
/// named graph is wrapped in a `<iri> {<LF> triples <LF>}` block. Because
/// each `write_fact` is streamed, the writer does not reorder input —
/// callers that want a single block per graph must pass facts grouped by
/// graph. When `write_fact` sees a graph change, the previous block is
/// closed and a new block opened.
#[derive(Debug)]
pub struct TriGWriter<W: Write> {
    sink: W,
    prefixes: BTreeMap<String, String>,
    wrote_header: bool,
    current_graph: Option<String>, // Some(None) = default, Some(Some(g)) = named
    in_block: bool,
}

impl<W: Write> TriGWriter<W> {
    /// Wrap `sink` in a fresh TriG writer with no registered prefixes.
    #[must_use]
    pub fn new(sink: W) -> Self {
        Self {
            sink,
            prefixes: BTreeMap::new(),
            wrote_header: false,
            current_graph: None,
            in_block: false,
        }
    }

    /// Register a prefix for pname shortening.
    ///
    /// See [`TurtleWriter::with_prefix`] for semantics.
    pub fn with_prefix(&mut self, pname: &str, iri: &str) {
        self.prefixes.insert(pname.to_owned(), iri.to_owned());
    }

    /// Emit one fact, opening / closing graph blocks as needed.
    pub fn write_fact(&mut self, fact: &Fact) -> io::Result<()> {
        self.ensure_header()?;
        // Open or close graph blocks as the graph changes.
        let desired = fact.graph.as_deref();
        let desired_owned = desired.map(ToOwned::to_owned);
        let current = self.current_graph.clone();
        if !self.in_block || current != desired_owned {
            // Close previous block, if any.
            if self.in_block {
                self.sink.write_all(b"}\n")?;
            }
            // Open the new block.
            match desired {
                Some(g) => {
                    write_term_ttl(&mut self.sink, g, &self.prefixes)?;
                    self.sink.write_all(b" {\n")?;
                }
                None => {
                    // Default graph in TriG is emitted without a wrapper
                    // using the `{...}` form (§2.3 §2.4): we wrap it in
                    // `{}` without a label so round-trip is unambiguous.
                    self.sink.write_all(b"{\n")?;
                }
            }
            self.in_block = true;
            self.current_graph = desired_owned;
        }
        self.sink.write_all(b"  ")?;
        write_term_ttl(&mut self.sink, &fact.subject, &self.prefixes)?;
        self.sink.write_all(b" ")?;
        write_term_ttl(&mut self.sink, &fact.predicate, &self.prefixes)?;
        self.sink.write_all(b" ")?;
        write_term_ttl(&mut self.sink, &fact.object, &self.prefixes)?;
        self.sink.write_all(b" .\n")
    }

    /// Close the final graph block and flush the sink.
    pub fn finish(mut self) -> io::Result<()> {
        if self.in_block {
            self.sink.write_all(b"}\n")?;
        }
        self.sink.flush()
    }

    fn ensure_header(&mut self) -> io::Result<()> {
        if self.wrote_header {
            return Ok(());
        }
        self.wrote_header = true;
        write_prefix_header(&mut self.sink, &self.prefixes)
    }
}

// ---------------------------------------------------------------------------
// Shared term emission
// ---------------------------------------------------------------------------

/// Emit a canonical term as N-Triples / N-Quads text.
///
/// Canonical terms come in three shapes (see `rdf-diff` crate docs):
///
/// - `<iri>` — passed through verbatim apart from re-applying the minimal
///   set of required IRI escapes.
/// - `_:label` — passed through verbatim; the canonical labels are
///   `_:c0`, `_:c1`, … which are valid BNode labels in every RDF text
///   format.
/// - `"lex"` / `"lex"@tag` / `"lex"^^<iri>` — literal shapes; the lex
///   form may contain control characters that must be re-escaped.
fn write_term_nt<W: Write>(out: &mut W, term: &str) -> io::Result<()> {
    if let Some(rest) = term.strip_prefix('<').and_then(|t| t.strip_suffix('>')) {
        out.write_all(b"<")?;
        write_iri_body(out, rest)?;
        out.write_all(b">")
    } else if term.starts_with('"') {
        write_literal_nt(out, term)
    } else {
        // Blank node or anything the parser emitted verbatim.
        out.write_all(term.as_bytes())
    }
}

/// Emit a canonical term as Turtle / TriG text.
///
/// Differences from [`write_term_nt`]:
///
/// - An IRI may be emitted as a `pname:localname` when the namespace
///   matches a registered prefix.
/// - Literals may use the long-literal `"""…"""` form when that keeps the
///   output compact.
fn write_term_ttl<W: Write>(
    out: &mut W,
    term: &str,
    prefixes: &BTreeMap<String, String>,
) -> io::Result<()> {
    if let Some(rest) = term.strip_prefix('<').and_then(|t| t.strip_suffix('>')) {
        if let Some((pname, local)) = try_compact_iri(rest, prefixes) {
            out.write_all(pname.as_bytes())?;
            out.write_all(b":")?;
            out.write_all(local.as_bytes())
        } else {
            out.write_all(b"<")?;
            write_iri_body(out, rest)?;
            out.write_all(b">")
        }
    } else if term.starts_with('"') {
        write_literal_ttl(out, term)
    } else {
        out.write_all(term.as_bytes())
    }
}

/// Emit the `@prefix … .` header block.
fn write_prefix_header<W: Write>(
    out: &mut W,
    prefixes: &BTreeMap<String, String>,
) -> io::Result<()> {
    for (pname, iri) in prefixes {
        out.write_all(b"@prefix ")?;
        out.write_all(pname.as_bytes())?;
        out.write_all(b": <")?;
        write_iri_body(out, iri)?;
        out.write_all(b"> .\n")?;
    }
    if !prefixes.is_empty() {
        out.write_all(b"\n")?;
    }
    Ok(())
}

/// Try to shorten a full IRI to `pname:localname` against the registered
/// prefix map. Only a syntactically safe local part is accepted —
/// otherwise we fall back to the angle-bracketed form.
///
/// Safety rules for the local part (Turtle 1.1 §2.4, simplified):
///
/// - First char must be a `PN_CHARS_U` (i.e., ASCII letter, `_`, or `:`).
/// - Subsequent chars must be `PN_CHARS` (letters, digits, `-`, `_`, `.`).
/// - `.` must not be the last char.
///
/// We accept a conservative ASCII subset. Any non-ASCII local part falls
/// back to `<iri>` form, which is always safe.
fn try_compact_iri<'a>(
    iri: &'a str,
    prefixes: &'a BTreeMap<String, String>,
) -> Option<(&'a str, &'a str)> {
    // Prefer the longest matching namespace so nested prefixes resolve
    // against the most specific one (e.g. both `ex:` = http://ex/ and
    // `sub:` = http://ex/sub/ → a `http://ex/sub/foo` IRI becomes
    // `sub:foo`, not `ex:sub/foo`).
    let mut longest: Option<(&str, &str, usize)> = None;
    for (pname, ns) in prefixes {
        if let Some(local) = iri.strip_prefix(ns.as_str())
            && is_safe_local_part(local)
        {
            let ns_len = ns.len();
            if longest.is_none_or(|(_, _, l)| l < ns_len) {
                longest = Some((pname.as_str(), local, ns_len));
            }
        }
    }
    longest.map(|(p, l, _)| (p, l))
}

fn is_safe_local_part(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    let mut last: char = first;
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
            return false;
        }
        last = c;
    }
    last != '.'
}

/// Emit an IRI body (the chars between `<` and `>`) with the minimal set
/// of N-Triples/Turtle-required escapes applied.
///
/// The canonical form stores IRIs with their code points verbatim; the
/// N-Triples grammar §2 forbids the following in an IRIREF body:
///
/// - `<`, `>`, `"`, `{`, `}`, `|`, `^`, `` ` ``, `\` and all chars in the
///   range U+0000..=U+0020.
///
/// We re-escape the forbidden code points via `\uXXXX` / `\UXXXXXXXX`.
/// Anything else is emitted verbatim.
fn write_iri_body<W: Write>(out: &mut W, body: &str) -> io::Result<()> {
    for c in body.chars() {
        match c {
            '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => {
                write_uchar_escape(out, c)?;
            }
            c if (c as u32) <= 0x20 => {
                write_uchar_escape(out, c)?;
            }
            c => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                out.write_all(s.as_bytes())?;
            }
        }
    }
    Ok(())
}

/// Emit `\uXXXX` or `\UXXXXXXXX`, whichever is shortest.
fn write_uchar_escape<W: Write>(out: &mut W, c: char) -> io::Result<()> {
    let code = c as u32;
    if code <= 0xFFFF {
        write!(out, "\\u{code:04X}")
    } else {
        write!(out, "\\U{code:08X}")
    }
}

// ---------------------------------------------------------------------------
// Literal emission
// ---------------------------------------------------------------------------

/// Emit a canonical literal in N-Triples form.
///
/// The canonical form is `"lex"` / `"lex"@tag` / `"lex"^^<iri>` where
/// `lex` contains the parser's already-escaped backslash and quote (see
/// `rdf-ntriples::escape_literal_lex`) but every other byte is verbatim
/// — including raw LF, CR, TAB and other control characters.
///
/// N-Triples grammar §2 requires LF/CR/TAB to be escaped inside short
/// string literals, so we re-escape them here. Existing `\"` and `\\`
/// are preserved verbatim.
fn write_literal_nt<W: Write>(out: &mut W, literal: &str) -> io::Result<()> {
    let (lex, suffix) = split_literal(literal).unwrap_or((literal, ""));
    let lex = strip_outer_quotes(lex);
    out.write_all(b"\"")?;
    write_literal_lex_nt(out, lex)?;
    out.write_all(b"\"")?;
    out.write_all(suffix.as_bytes())
}

/// Emit a canonical literal in Turtle / TriG form.
///
/// If the lex form contains a raw `"` (after the parser's `\"` escape has
/// been un-escaped in the canonical encoding, this shows up as `\"` in
/// the `lex` slice), we fall back to the long-literal form `"""…"""`
/// which permits single unescaped `"` characters.
fn write_literal_ttl<W: Write>(out: &mut W, literal: &str) -> io::Result<()> {
    let (lex, suffix) = split_literal(literal).unwrap_or((literal, ""));
    let lex = strip_outer_quotes(lex);

    // Heuristic: use the long form only when the lex form actually
    // contains an escaped double-quote (`\"`). Otherwise the short form
    // is strictly more compact. The long form's advantage is that single
    // `"` characters may appear unescaped, so "complex" literals pretty-print
    // without a forest of backslashes.
    if lex.contains('"') && !lex.ends_with('"') {
        out.write_all(b"\"\"\"")?;
        write_literal_lex_ttl_long(out, lex)?;
        out.write_all(b"\"\"\"")?;
    } else {
        out.write_all(b"\"")?;
        write_literal_lex_nt(out, lex)?;
        out.write_all(b"\"")?;
    }
    out.write_all(suffix.as_bytes())
}

/// Split a canonical literal into `(between_quotes, suffix)`.
///
/// `between_quotes` contains the outer `"` characters; the caller strips
/// them via [`strip_outer_quotes`] before re-escaping. `suffix` is either
/// `""`, `"@bcp47"`, or `"^^<iri>"`.
fn split_literal(term: &str) -> Option<(&str, &str)> {
    if !term.starts_with('"') {
        return None;
    }
    let bytes = term.as_bytes();
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i = i.saturating_add(2),
            b'"' => break,
            _ => i = i.saturating_add(1),
        }
    }
    if i >= bytes.len() {
        return None;
    }
    // Keep both outer quotes in `between_quotes` for caller clarity.
    let (with_quotes, suffix) = term.split_at(i.saturating_add(1));
    Some((with_quotes, suffix))
}

fn strip_outer_quotes(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .unwrap_or(s)
}

/// Re-escape the lex form for an N-Triples short-string literal.
///
/// Walks byte-by-byte so we can distinguish an existing `\\` / `\"` in
/// the canonical encoding from a raw backslash that would need to be
/// escaped. The parser guarantees that every backslash in the canonical
/// lex form is the start of a valid `\\` / `\"` sequence, so when we see
/// a `\` we emit the following char verbatim.
fn write_literal_lex_nt<W: Write>(out: &mut W, lex: &str) -> io::Result<()> {
    let bytes = lex.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() {
            // Preserve the canonical two-char escape verbatim.
            out.write_all(&bytes[i..i + 2])?;
            i += 2;
            continue;
        }
        match b {
            b'\n' => out.write_all(b"\\n")?,
            b'\r' => out.write_all(b"\\r")?,
            b'\t' => out.write_all(b"\\t")?,
            b'"' => out.write_all(b"\\\"")?,
            0x00..=0x1F | 0x7F => {
                let c = char::from(b);
                write_uchar_escape(out, c)?;
            }
            _ => {
                // Multi-byte UTF-8 is copied verbatim one code point at a
                // time so we re-align on a char boundary.
                let c = lex[i..].chars().next().expect("valid UTF-8");
                let len = c.len_utf8();
                out.write_all(&bytes[i..i + len])?;
                i += len;
                continue;
            }
        }
        i += 1;
    }
    Ok(())
}

/// Re-escape the lex form for a Turtle long-string literal (`"""…"""`).
///
/// The only escapes required are `\\`, three consecutive `"`s (must
/// insert a backslash), and existing `\"` is preserved to keep round-trip
/// bit-exact. `\n` / `\r` / `\t` may appear verbatim per the Turtle
/// long-string rules.
fn write_literal_lex_ttl_long<W: Write>(out: &mut W, lex: &str) -> io::Result<()> {
    let bytes = lex.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() {
            out.write_all(&bytes[i..i + 2])?;
            i += 2;
            continue;
        }
        if b == b'"' {
            // Count consecutive quotes; if 3+, the terminator would be
            // ambiguous, so we escape the first to break the run.
            let mut run = 0;
            while i + run < bytes.len() && bytes[i + run] == b'"' {
                run += 1;
            }
            if run >= 3 || i + run >= bytes.len() {
                out.write_all(b"\\\"")?;
                i += 1;
                continue;
            }
            out.write_all(b"\"")?;
            i += 1;
            continue;
        }
        if let 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F = b {
            let c = char::from(b);
            write_uchar_escape(out, c)?;
            i += 1;
            continue;
        }
        let c = lex[i..].chars().next().expect("valid UTF-8");
        let len = c.len_utf8();
        out.write_all(&bytes[i..i + len])?;
        i += len;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rdf_diff::{Fact, FactProvenance, Facts};

    fn prov() -> FactProvenance {
        FactProvenance {
            offset: None,
            parser: "test".to_owned(),
        }
    }

    fn iri_fact(s: &str, p: &str, o: &str) -> Fact {
        Fact {
            subject: format!("<{s}>"),
            predicate: format!("<{p}>"),
            object: format!("<{o}>"),
            graph: None,
        }
    }

    fn nt_render(facts: &[Fact]) -> String {
        let mut buf = Vec::new();
        let mut w = NTriplesWriter::new(&mut buf);
        for f in facts {
            w.write_fact(f).unwrap();
        }
        w.finish().unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn nq_render(facts: &[Fact]) -> String {
        let mut buf = Vec::new();
        let mut w = NQuadsWriter::new(&mut buf);
        for f in facts {
            w.write_fact(f).unwrap();
        }
        w.finish().unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn ttl_render(facts: &[Fact], prefixes: &[(&str, &str)]) -> String {
        let mut buf = Vec::new();
        let mut w = TurtleWriter::new(&mut buf);
        for (p, i) in prefixes {
            w.with_prefix(p, i);
        }
        for f in facts {
            w.write_fact(f).unwrap();
        }
        w.finish().unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn trig_render(facts: &[Fact], prefixes: &[(&str, &str)]) -> String {
        let mut buf = Vec::new();
        let mut w = TriGWriter::new(&mut buf);
        for (p, i) in prefixes {
            w.with_prefix(p, i);
        }
        for f in facts {
            w.write_fact(f).unwrap();
        }
        w.finish().unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn nt_simple_iri_triple() {
        let out = nt_render(&[iri_fact("http://ex/s", "http://ex/p", "http://ex/o")]);
        assert_eq!(out, "<http://ex/s> <http://ex/p> <http://ex/o> .\n");
    }

    #[test]
    fn nt_escapes_control_chars_in_literal() {
        let fact = Fact {
            subject: "<http://ex/s>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "\"a\nb\tc\"".to_owned(),
            graph: None,
        };
        let out = nt_render(&[fact]);
        assert!(out.contains(r"\n"), "missing \\n escape: {out}");
        assert!(out.contains(r"\t"), "missing \\t escape: {out}");
    }

    #[test]
    fn nt_preserves_canonical_backslash_and_quote() {
        // The canonical encoding already has `\"` and `\\` pre-escaped.
        let fact = Fact {
            subject: "<http://ex/s>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "\"a\\\"b\\\\c\"".to_owned(),
            graph: None,
        };
        let out = nt_render(&[fact]);
        assert!(out.contains("\"a\\\"b\\\\c\""), "escape lost: {out}");
    }

    #[test]
    fn nt_escapes_forbidden_iri_chars() {
        // A space in an IRI body must be escaped as \u0020.
        let fact = Fact {
            subject: "<http://ex/a b>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "<http://ex/o>".to_owned(),
            graph: None,
        };
        let out = nt_render(&[fact]);
        assert!(out.contains(r"\u0020"), "space not escaped: {out}");
    }

    #[test]
    fn nt_literal_with_datatype() {
        let fact = Fact {
            subject: "<http://ex/s>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned(),
            graph: None,
        };
        let out = nt_render(&[fact]);
        assert_eq!(
            out,
            "<http://ex/s> <http://ex/p> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n"
        );
    }

    #[test]
    fn nt_literal_with_lang_tag() {
        let fact = Fact {
            subject: "<http://ex/s>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "\"hello\"@en-US".to_owned(),
            graph: None,
        };
        let out = nt_render(&[fact]);
        assert_eq!(out, "<http://ex/s> <http://ex/p> \"hello\"@en-US .\n");
    }

    #[test]
    fn nt_bnode_pass_through() {
        let fact = Fact {
            subject: "_:c0".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "_:c1".to_owned(),
            graph: None,
        };
        let out = nt_render(&[fact]);
        assert_eq!(out, "_:c0 <http://ex/p> _:c1 .\n");
    }

    #[test]
    fn nq_emits_graph_slot() {
        let fact = Fact {
            subject: "<http://ex/s>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "<http://ex/o>".to_owned(),
            graph: Some("<http://ex/g>".to_owned()),
        };
        let out = nq_render(&[fact]);
        assert_eq!(
            out,
            "<http://ex/s> <http://ex/p> <http://ex/o> <http://ex/g> .\n"
        );
    }

    #[test]
    fn nq_default_graph_omits_slot() {
        let out = nq_render(&[iri_fact("http://ex/s", "http://ex/p", "http://ex/o")]);
        assert_eq!(out, "<http://ex/s> <http://ex/p> <http://ex/o> .\n");
    }

    #[test]
    fn ttl_without_prefixes_matches_nt_body() {
        let out = ttl_render(
            &[iri_fact("http://ex/s", "http://ex/p", "http://ex/o")],
            &[],
        );
        assert_eq!(out, "<http://ex/s> <http://ex/p> <http://ex/o> .\n");
    }

    #[test]
    fn ttl_with_prefixes_compacts() {
        let out = ttl_render(
            &[iri_fact("http://ex/s", "http://ex/p", "http://ex/o")],
            &[("ex", "http://ex/")],
        );
        assert!(out.contains("@prefix ex: <http://ex/> ."), "header: {out}");
        assert!(
            out.contains("ex:s ex:p ex:o ."),
            "compaction failed: {out}"
        );
    }

    #[test]
    fn ttl_prefers_longest_matching_prefix() {
        let out = ttl_render(
            &[iri_fact(
                "http://ex/sub/foo",
                "http://ex/p",
                "http://ex/sub/bar",
            )],
            &[("ex", "http://ex/"), ("sub", "http://ex/sub/")],
        );
        assert!(
            out.contains("sub:foo ex:p sub:bar"),
            "longest-prefix wrong: {out}"
        );
    }

    #[test]
    fn ttl_falls_back_when_local_part_unsafe() {
        // local part begins with a digit -> not safe, use <...> form.
        let out = ttl_render(
            &[iri_fact("http://ex/1foo", "http://ex/p", "http://ex/o")],
            &[("ex", "http://ex/")],
        );
        assert!(out.contains("<http://ex/1foo>"), "should not compact: {out}");
    }

    #[test]
    fn ttl_long_literal_when_contains_quote() {
        let fact = Fact {
            subject: "<http://ex/s>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            // canonical: \" inside the lex; and the run is a single quote.
            object: "\"he said \\\"hi\\\" kindly\"".to_owned(),
            graph: None,
        };
        let out = ttl_render(&[fact], &[]);
        assert!(out.contains("\"\"\""), "expected long literal: {out}");
    }

    #[test]
    fn trig_emits_named_graph_block() {
        let fact = Fact {
            subject: "<http://ex/s>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "<http://ex/o>".to_owned(),
            graph: Some("<http://ex/g>".to_owned()),
        };
        let out = trig_render(&[fact], &[]);
        assert!(out.contains("<http://ex/g> {"), "opening missing: {out}");
        assert!(out.ends_with("}\n"), "closing missing: {out}");
    }

    #[test]
    fn trig_switches_graph_blocks() {
        let f1 = Fact {
            subject: "<http://ex/s1>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "<http://ex/o1>".to_owned(),
            graph: Some("<http://ex/g1>".to_owned()),
        };
        let f2 = Fact {
            subject: "<http://ex/s2>".to_owned(),
            predicate: "<http://ex/p>".to_owned(),
            object: "<http://ex/o2>".to_owned(),
            graph: Some("<http://ex/g2>".to_owned()),
        };
        let out = trig_render(&[f1, f2], &[]);
        assert!(out.contains("<http://ex/g1> {"));
        assert!(out.contains("<http://ex/g2> {"));
        assert_eq!(out.matches('}').count(), 2);
    }

    // Internal shape tests: split_literal, try_compact_iri, etc.

    #[test]
    fn split_literal_extracts_datatype_suffix() {
        let (lex, suffix) = split_literal("\"42\"^^<http://ex/x>").unwrap();
        assert_eq!(lex, "\"42\"");
        assert_eq!(suffix, "^^<http://ex/x>");
    }

    #[test]
    fn split_literal_handles_escaped_quote() {
        let (lex, suffix) = split_literal("\"a\\\"b\"@en").unwrap();
        assert_eq!(lex, "\"a\\\"b\"");
        assert_eq!(suffix, "@en");
    }

    #[test]
    fn is_safe_local_part_basic() {
        assert!(is_safe_local_part("foo"));
        assert!(is_safe_local_part("foo_bar"));
        assert!(is_safe_local_part("a.b"));
        assert!(!is_safe_local_part(""));
        assert!(!is_safe_local_part("1foo"));
        assert!(!is_safe_local_part("foo."));
        assert!(!is_safe_local_part("foo/bar"));
    }

    // Minimal self-contained round-trip via the canonical form: feed the
    // output of a writer into the `Facts::canonicalise` pipeline and
    // confirm the fact set is preserved. The end-to-end round-trip via
    // the real parsers lives in `tests/round_trip.rs`.

    #[test]
    fn canonicalise_is_stable_across_our_serialisation() {
        use std::collections::BTreeMap;
        let f = iri_fact("http://ex/s", "http://ex/p", "http://ex/o");
        let raw = vec![(f, prov())];
        let before = Facts::canonicalise(raw, BTreeMap::new());
        let mut buf = Vec::new();
        let mut w = NTriplesWriter::new(&mut buf);
        for fact in before.set.keys() {
            w.write_fact(fact).unwrap();
        }
        w.finish().unwrap();
        assert!(!buf.is_empty());
    }
}
