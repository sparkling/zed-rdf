//! Human-readable rendering for [`Diagnostic`].
//!
//! The LSP layer does its own rendering directly from the
//! structured value; this module exists for CLI output, test
//! snapshots, and `Debug`-style inspection.

use std::fmt::Write as _;

use crate::{Diagnostic, Span};

/// Render a diagnostic as a multi-line human-readable string, given
/// the original `source` the span refers to.
///
/// Output shape (trailing newline included):
///
/// ```text
/// error[NT-LITESC-001]: invalid UCHAR escape at byte 17: surrogate
///   --> line 2, column 5
///    |
///  2 |     "\uD800"
///    |     ^^^^^^^^
///    = hint: use a non-surrogate Unicode scalar value
///    = note: matching literal opens here (line 2, column 5)
/// ```
///
/// The caret line underlines the span using `^` characters; zero-width
/// spans render a single `^` at the insertion point.  Line + column
/// numbering is **1-based**, counting columns in bytes (the LSP layer
/// re-computes UTF-16 columns when it hands the diagnostic to the
/// client; CLI output keeps the byte column so the caret aligns with
/// a fixed-width font over the raw bytes).
///
/// If the span falls outside `source` (e.g. the parser was handed
/// the wrong slice), the caret frame is omitted and only the header
/// + hint + notes are emitted — rendering never panics.
#[must_use]
pub fn render(diagnostic: &Diagnostic, source: &str) -> String {
    let mut out = String::new();

    // Header line: `<severity>[<code>]: <message>`.
    let _ = writeln!(
        out,
        "{sev}[{code}]: {msg}",
        sev = diagnostic.severity.label(),
        code = diagnostic.code,
        msg = diagnostic.message
    );

    // Location + caret frame, iff the span is in-bounds.
    if let Some(frame) = frame(source, diagnostic.span) {
        let _ = writeln!(out, "  --> line {}, column {}", frame.line, frame.column);
        let gutter = frame.line.to_string();
        let pad = " ".repeat(gutter.len());
        let _ = writeln!(out, " {pad} |");
        let _ = writeln!(out, " {gutter} | {line}", line = frame.source_line);
        let caret_width = frame.caret_width.max(1);
        let carets = "^".repeat(caret_width);
        let lead = " ".repeat(frame.column.saturating_sub(1));
        let _ = writeln!(out, " {pad} | {lead}{carets}");
    }

    if let Some(hint) = &diagnostic.hint {
        let _ = writeln!(out, "   = hint: {hint}");
    }

    for related in &diagnostic.related {
        if let Some(loc) = line_col(source, related.span.start) {
            let _ = writeln!(
                out,
                "   = note: {msg} (line {line}, column {col})",
                msg = related.message,
                line = loc.0,
                col = loc.1
            );
        } else {
            let _ = writeln!(out, "   = note: {msg}", msg = related.message);
        }
    }

    out
}

struct Frame<'a> {
    line: usize,
    column: usize,
    source_line: &'a str,
    caret_width: usize,
}

fn frame(source: &str, span: Span) -> Option<Frame<'_>> {
    if span.start > source.len() || span.end > source.len() || span.start > span.end {
        return None;
    }
    let (line, column) = line_col(source, span.start)?;
    let line_start = nth_line_start(source, line)?;
    let line_end = source[line_start..]
        .find('\n')
        .map_or(source.len(), |n| line_start + n);
    let source_line = &source[line_start..line_end];
    // Clamp caret width to the remaining bytes on the same line so we
    // don't underline a newline or run past EOF.
    let max_end = line_end.min(span.end);
    let caret_width = max_end.saturating_sub(span.start);
    Some(Frame {
        line,
        column,
        source_line,
        caret_width,
    })
}

/// 1-based (line, column-in-bytes) for `offset`, or `None` if out of
/// bounds.  `offset == source.len()` maps to one-past-EOF on the last
/// line.
fn line_col(source: &str, offset: usize) -> Option<(usize, usize)> {
    if offset > source.len() {
        return None;
    }
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
    let last_nl = prefix.rfind('\n').map_or(0, |i| i + 1);
    let column = offset - last_nl + 1;
    Some((line, column))
}

/// Byte offset at which the 1-based `line` starts, or `None` if there
/// aren't that many lines.
fn nth_line_start(source: &str, line: usize) -> Option<usize> {
    if line == 0 {
        return None;
    }
    if line == 1 {
        return Some(0);
    }
    // Walk '\n' byte positions.
    let mut count = 1;
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            count += 1;
            if count == line {
                return Some(i + 1);
            }
        }
    }
    None
}
