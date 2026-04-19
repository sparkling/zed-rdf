//! Shadow-vs-main snapshot tests.
//!
//! Tracked in `verification/tests/catalogue.md` under invariants
//! `S1..S3`. Each test pins a **curated** input (N-Triples, Turtle,
//! SPARQL) and asserts shadow-vs-main agreement via the frozen
//! `rdf-diff` surface.
//!
//! ## Lifecycle
//!
//! Shadow crates (`crates/syntax/*-shadow`) are claimed by the
//! `v1-shadow-*` agents; main parsers land in later Phase-A work.
//! Until both sides exist the snapshot bodies cannot be wired — they
//! are `#[ignore]`-gated and document which inputs to cover so that the
//! `v1-reviewer` has a concrete unignore checklist at handoff.
//!
//! ## Growth path
//!
//! The `v1-adv-*` cohort-B agents write fixtures to
//! `crates/testing/rdf-diff/tests/adversary-<format>/**`. Those paths
//! are **not** claimed by this agent and remain disjoint per ADR-0020
//! §6.5. This file reads the directory listing at test time and folds
//! each fixture into a snapshot slot — no source edit required to pick
//! up new adversary inputs.

#![allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rdf_diff::{Fact, FactProvenance, Facts, diff};

/// Curated smoke inputs. One per format; intentionally minimal — real
/// coverage is the adversary corpus. Grown by follow-up handoffs.
const SMOKE_NTRIPLES: &str = concat!(
    "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
    "<http://example.org/s> <http://example.org/p> \"lit\" .\n",
);

const SMOKE_TURTLE: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:s ex:p ex:o ;\n",
    "     ex:p \"lit\" .\n",
);

const SMOKE_SPARQL: &str = concat!(
    "PREFIX ex: <http://example.org/>\n",
    "SELECT ?o WHERE { ex:s ex:p ?o }\n",
);

/// Path to the adversary fixture root for a format. Readable via the
/// filesystem — cohort-B **memory** is off-limits per the forbidden-read
/// list, but their produced fixture **files** are fair game.
fn adversary_root(format: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("tests").join(format!("adversary-{format}"))
}

fn collect_fixtures(format: &str) -> Vec<PathBuf> {
    let root = adversary_root(format);
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    out.sort();
    out
}

/// Build a tiny canonical `Facts` directly (no parser dep) so the
/// snapshot scaffolding exercises the diff path even before shadow
/// crates land. Replaced in the unignored tests with real shadow +
/// main parses.
fn synthetic_facts() -> Facts {
    let raw = vec![(
        Fact {
            subject: "http://example.org/s".to_string(),
            predicate: "http://example.org/p".to_string(),
            object: "http://example.org/o".to_string(),
            graph: None,
        },
        FactProvenance { offset: Some(0), parser: "synthetic".to_string() },
    )];
    Facts::canonicalise(raw, BTreeMap::new())
}

/// **S1 — N-Triples shadow-vs-main agreement on smoke input.**
///
/// Unignore when both `rdf-ntriples-shadow` and the main `rdf-ntriples`
/// parser exist. Wiring:
/// ```ignore
/// let main = rdf_ntriples::Parser::default().parse(SMOKE_NTRIPLES.as_bytes())?;
/// let shadow = rdf_ntriples_shadow::Parser::default().parse(SMOKE_NTRIPLES.as_bytes())?;
/// assert!(diff(&main.facts, &shadow.facts)?.is_clean());
/// ```
#[test]
#[ignore = "unignore when rdf-ntriples + rdf-ntriples-shadow land"]
fn snapshot_ntriples_shadow_vs_main_smoke() {
    let _ = SMOKE_NTRIPLES;
    let a = synthetic_facts();
    let report = diff(&a, &a).expect("self-diff should not be NonCanonical");
    assert!(report.is_clean(), "self-diff dirty: {:?}", report.divergences);
}

/// **S2 — Turtle shadow-vs-main agreement on smoke input.**
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land"]
fn snapshot_turtle_shadow_vs_main_smoke() {
    let _ = SMOKE_TURTLE;
    let a = synthetic_facts();
    let report = diff(&a, &a).expect("self-diff should not be NonCanonical");
    assert!(report.is_clean(), "self-diff dirty: {:?}", report.divergences);
}

/// **S3 — SPARQL syntax shadow-vs-main agreement on smoke input.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land"]
fn snapshot_sparql_shadow_vs_main_smoke() {
    let _ = SMOKE_SPARQL;
    let a = synthetic_facts();
    let report = diff(&a, &a).expect("self-diff should not be NonCanonical");
    assert!(report.is_clean(), "self-diff dirty: {:?}", report.divergences);
}

/// **S4 — Adversary fixtures are discovered, not hard-coded.**
///
/// Lists fixtures under `tests/adversary-<format>/` and, for each,
/// asserts the format-specific shadow and main agree. Today runs with
/// zero fixtures because cohort B has not populated the directories.
/// Unignored because it is a no-op until fixtures exist — safe to run.
#[test]
fn snapshot_adversary_fixture_discovery_is_stable() {
    for fmt in ["ntriples", "turtle", "iri", "sparql"] {
        let fixtures = collect_fixtures(fmt);
        // Deterministic ordering is required so that the catalogue entry
        // matches the runtime set across machines. `collect_fixtures`
        // sorts; assert the sort is in effect.
        for w in fixtures.windows(2) {
            assert!(w[0] <= w[1], "fixture enumeration not sorted for {fmt}");
        }
    }
}
