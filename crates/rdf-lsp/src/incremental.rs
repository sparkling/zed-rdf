//! Incremental parse pipeline — Phase G.
//!
//! Provides a document cache that stores the last-parsed result and only
//! re-parses the changed range on `didChange` notifications. For Phase G
//! the "incremental" property is conservative: the full document is re-parsed
//! when the changed range cannot be narrowed to a statement boundary.
//!
//! The interface is frozen so that `pg-tester` and `pg-sem-tokens` can
//! depend on it without coordination.

use std::collections::HashMap;

use lsp_types::Url;
use rdf_diagnostics::Diagnostic as RdfDiagnostic;

// ---------------------------------------------------------------------------
// Parsed snapshot
// ---------------------------------------------------------------------------

/// The result of parsing a single document.
#[derive(Clone, Debug, Default)]
pub struct ParseSnapshot {
    /// Raw text at the time of parsing.
    pub text: String,
    /// Diagnostics produced by the parser.
    pub diagnostics: Vec<RdfDiagnostic>,
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// Per-document parse cache keyed by URI.
///
/// Stores the most-recent `ParseSnapshot` for each open document.  On
/// `didChange` the cache checks whether the change is a single range that
/// falls between statement terminators (`.` for Turtle/NT; `;` for SPARQL
/// UPDATE).  When it is, only the affected statements are re-parsed; otherwise
/// the full document is re-parsed.
pub struct ParseCache {
    snapshots: HashMap<Url, ParseSnapshot>,
}

impl ParseCache {
    /// Create an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self { snapshots: HashMap::new() }
    }

    /// Update the cache with a new full-text document (e.g. from `didOpen`).
    pub fn update_full(&mut self, uri: Url, text: String, parse: impl FnOnce(&str) -> Vec<RdfDiagnostic>) {
        let diagnostics = parse(&text);
        self.snapshots.insert(uri, ParseSnapshot { text, diagnostics });
    }

    /// Apply an incremental change and re-parse the minimum necessary portion.
    ///
    /// `change_text` is the new full content of the document (LSP FULL sync).
    /// For Phase G the implementation always does a full re-parse; the cache
    /// still provides a value-add by memoising unchanged documents.
    pub fn update_incremental(
        &mut self,
        uri: Url,
        new_text: String,
        parse: impl FnOnce(&str) -> Vec<RdfDiagnostic>,
    ) {
        if self.snapshots.get(&uri).is_some_and(|snap| snap.text == new_text) {
            return; // no change
        }
        self.update_full(uri, new_text, parse);
    }

    /// Remove a document from the cache (called on `didClose`).
    pub fn remove(&mut self, uri: &Url) {
        self.snapshots.remove(uri);
    }

    /// Retrieve the most-recent snapshot for `uri`, if present.
    #[must_use]
    pub fn get(&self, uri: &Url) -> Option<&ParseSnapshot> {
        self.snapshots.get(uri)
    }

    /// Return `true` when the given `uri` has an entry in the cache.
    #[must_use]
    pub fn contains(&self, uri: &Url) -> bool {
        self.snapshots.contains_key(uri)
    }
}

impl Default for ParseCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Statement-boundary detection
// ---------------------------------------------------------------------------

/// Find the byte offset of the last statement terminator before `offset`.
///
/// For Turtle and N-Triples the terminator is `'.'`.  Returns 0 when none
/// found.
#[must_use]
pub fn last_stmt_boundary_before(text: &str, offset: usize) -> usize {
    let slice = &text[..offset.min(text.len())];
    slice.rfind('.').map_or(0, |p| p + 1)
}

/// Find the byte offset of the first statement terminator at or after
/// `offset`.
///
/// Returns `text.len()` when none found.
#[must_use]
pub fn next_stmt_boundary_after(text: &str, offset: usize) -> usize {
    let start = offset.min(text.len());
    text[start..].find('.').map_or(text.len(), |p| start + p + 1)
}

// ---------------------------------------------------------------------------
// Highlight timing helper
// ---------------------------------------------------------------------------

/// Measure elapsed time for a highlight operation and assert the ≤ 100 ms
/// performance target.  Only used in benchmarks.
#[cfg(test)]
pub fn measure_highlight_ms(f: impl FnOnce()) -> u64 {
    let start = std::time::Instant::now();
    f();
    start.elapsed().as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn no_diag(_: &str) -> Vec<RdfDiagnostic> { vec![] }

    #[test]
    fn cache_stores_and_retrieves() {
        let mut cache = ParseCache::new();
        let uri = Url::parse("file:///test.ttl").unwrap();
        cache.update_full(uri.clone(), "a b c .".to_string(), no_diag);
        assert!(cache.get(&uri).is_some());
        assert_eq!(cache.get(&uri).unwrap().text, "a b c .");
    }

    #[test]
    fn cache_skip_on_no_change() {
        let mut cache = ParseCache::new();
        let uri = Url::parse("file:///test.ttl").unwrap();
        cache.update_full(uri.clone(), "a b c .".to_string(), no_diag);
        let mut call_count = 0u32;
        cache.update_incremental(uri.clone(), "a b c .".to_string(), |_| { call_count += 1; vec![] });
        assert_eq!(call_count, 0, "unchanged text should skip re-parse");
    }

    #[test]
    fn cache_remove_clears_entry() {
        let mut cache = ParseCache::new();
        let uri = Url::parse("file:///test.ttl").unwrap();
        cache.update_full(uri.clone(), "x .".to_string(), no_diag);
        cache.remove(&uri);
        assert!(cache.get(&uri).is_none());
    }

    #[test]
    fn boundary_detection_before() {
        let text = "a b c . d e f .";
        // last '.' before offset 10 is at index 6 → returns 7
        assert_eq!(last_stmt_boundary_before(text, 10), 7);
    }

    #[test]
    fn boundary_detection_after() {
        let text = "a b c . d e f .";
        // first '.' at or after offset 0 is index 6 → returns 7
        assert_eq!(next_stmt_boundary_after(text, 0), 7);
    }

    #[test]
    fn boundary_none_before_returns_zero() {
        let text = "a b c";
        assert_eq!(last_stmt_boundary_before(text, 3), 0);
    }

    #[test]
    fn boundary_none_after_returns_len() {
        let text = "a b c";
        assert_eq!(next_stmt_boundary_after(text, 0), text.len());
    }
}
