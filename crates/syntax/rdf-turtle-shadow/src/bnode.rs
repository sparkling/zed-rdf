//! Blank-node label allocator and canonicaliser.
//!
//! Turtle/`TriG` blank-node labels are document-scoped — they do not reset
//! across `@prefix` redefinitions. This allocator maps the source document's
//! labels to a deterministic `_:b0`, `_:b1`, … sequence per document,
//! suitable for cross-parser diffing.
//!
//! Blank-node label space is shared within a single document. `TriG` named
//! graphs share the same blank-node label space as the default graph within a
//! document, per W3C `TriG` §3.3.

use std::collections::HashMap;

/// Manages blank-node label allocation for one parse run.
#[derive(Debug, Default)]
pub struct BNodeAllocator {
    /// Maps source labels to canonical labels.
    mapping: HashMap<String, String>,
    /// Counter for fresh allocations.
    counter: usize,
}

impl BNodeAllocator {
    /// Create a fresh allocator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Canonicalise a named blank-node label from the document.
    ///
    /// The same source label always returns the same canonical label within
    /// one document.
    pub fn named(&mut self, source_label: &str) -> String {
        if let Some(canonical) = self.mapping.get(source_label) {
            return canonical.clone();
        }
        let canonical = self.fresh_label();
        self.mapping.insert(source_label.to_owned(), canonical.clone());
        canonical
    }

    /// Allocate a fresh anonymous blank node (from `[]` or collections).
    pub fn fresh(&mut self) -> String {
        self.fresh_label()
    }

    fn fresh_label(&mut self) -> String {
        let label = format!("_:b{}", self.counter);
        self.counter += 1;
        label
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_relabelling() {
        let mut alloc = BNodeAllocator::new();
        let a = alloc.named("alice");
        let b = alloc.named("bob");
        let a2 = alloc.named("alice");
        assert_eq!(a, "_:b0");
        assert_eq!(b, "_:b1");
        assert_eq!(a, a2);
    }

    #[test]
    fn fresh_distinct_from_named() {
        let mut alloc = BNodeAllocator::new();
        let f = alloc.fresh();
        let n = alloc.named("x");
        assert_ne!(f, n);
    }
}
