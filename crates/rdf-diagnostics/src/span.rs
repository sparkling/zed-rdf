//! Byte-offset half-open source spans.

/// A half-open byte range `[start, end)` into the original source
/// string of a parse.
///
/// Offsets are **byte** offsets, not UTF-8 char offsets.  This matches
/// `str`-slicing semantics (`&source[span.start..span.end]`) and lets
/// the LSP layer convert to UTF-16 positions losslessly.
///
/// Invariant: `start <= end`.  Constructors enforce this; manually
/// building a malformed `Span` with public fields only breaks the
/// render caret, not safety (the crate is `#![forbid(unsafe_code)]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Span {
    /// Inclusive byte offset of the first byte of the span.
    pub start: usize,
    /// Exclusive byte offset — one past the last byte of the span.
    pub end: usize,
}

impl Span {
    /// Create a new span.  Panics (in debug builds) if `start > end`.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "Span::new: start must be <= end");
        Self { start, end }
    }

    /// Zero-width span at `offset` (`start == end == offset`).  Useful
    /// for "expected token here" diagnostics where the insertion point
    /// has no width.
    #[must_use]
    pub const fn point(offset: usize) -> Self {
        Self { start: offset, end: offset }
    }

    /// Width of the span in bytes (`end - start`).
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// `true` iff [`Span::len`] is zero.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Smallest span containing both `self` and `other`.  If either
    /// span is empty the other is returned verbatim; otherwise the
    /// covering span is `[min(start), max(end))`.
    #[must_use]
    pub fn cover(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}
