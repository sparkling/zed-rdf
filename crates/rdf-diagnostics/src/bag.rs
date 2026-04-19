//! [`DiagnosticBag`] — ordered accumulator of [`Diagnostic`]s.

use crate::Diagnostic;

/// Ordered accumulator of parser diagnostics.
///
/// Parsers typically hold a `DiagnosticBag` for the duration of a
/// parse, push into it as problems surface, and hand
/// [`DiagnosticBag::into_vec`] back to the caller on exit.  Ordering
/// is insertion order — callers that want severity-sorted output
/// should sort the returned `Vec` themselves.
///
/// The bag is cheap to construct ([`DiagnosticBag::default`] or
/// [`DiagnosticBag::new`]) and carries no implicit source-text
/// reference — rendering is a separate step, performed by
/// [`render`](crate::render).
#[derive(Debug, Clone, Default)]
pub struct DiagnosticBag {
    inner: Vec<Diagnostic>,
}

impl DiagnosticBag {
    /// Create an empty bag.
    #[must_use]
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Create an empty bag with capacity for at least `n` diagnostics.
    /// Zero-cost if the parser has a prior estimate.
    #[must_use]
    pub fn with_capacity(n: usize) -> Self {
        Self {
            inner: Vec::with_capacity(n),
        }
    }

    /// Push a single diagnostic onto the end of the bag.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.inner.push(diagnostic);
    }

    /// Extend the bag with all diagnostics from `iter`, preserving
    /// order.
    pub fn extend<I: IntoIterator<Item = Diagnostic>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }

    /// Number of diagnostics currently in the bag.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// `true` iff no diagnostics have been pushed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// `true` iff the bag contains at least one fatal diagnostic
    /// (i.e. at least one [`Severity::Error`](crate::Severity::Error)).
    #[must_use]
    pub fn is_fatal(&self) -> bool {
        self.inner.iter().any(Diagnostic::is_fatal)
    }

    /// Borrow the underlying slice for inspection without consuming
    /// the bag.
    #[must_use]
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.inner
    }

    /// Consume the bag and return its diagnostics in insertion order.
    #[must_use]
    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.inner
    }

    /// Borrowing iterator over diagnostics in insertion order.
    /// Equivalent to `(&bag).into_iter()` but usable in method-call
    /// position without a turbofish.
    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic> {
        self.inner.iter()
    }
}

impl IntoIterator for DiagnosticBag {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a DiagnosticBag {
    type Item = &'a Diagnostic;
    type IntoIter = std::slice::Iter<'a, Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl Extend<Diagnostic> for DiagnosticBag {
    fn extend<I: IntoIterator<Item = Diagnostic>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

impl FromIterator<Diagnostic> for DiagnosticBag {
    fn from_iter<I: IntoIterator<Item = Diagnostic>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}
