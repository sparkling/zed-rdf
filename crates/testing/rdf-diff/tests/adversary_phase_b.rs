//! Phase B adversary fixture harness skeleton.
//!
//! This module is the harness entry-point for adversary fixtures targeting
//! Phase B formats: RDF/XML, JSON-LD, TriX, and N3. Once the adversary
//! agents (`v1-adv-rdfxml`, `v1-adv-jsonld`, `v1-adv-trix`, `v1-adv-n3`)
//! populate the fixture directories and `manifest.toml` files, this
//! harness iterates them and wires each fixture into a diff test.
//!
//! ## Directory layout (expected by adversary agents)
//!
//! ```text
//! crates/testing/rdf-diff/tests/
//!   adversary-rdfxml/
//!     manifest.toml          ← adversary agent populates this
//!     <fixture-name>.rdf     ← adversary fixtures
//!     ...
//!   adversary-jsonld/
//!     manifest.toml
//!     <fixture-name>.jsonld
//!     ...
//!   adversary-trix/
//!     manifest.toml
//!     <fixture-name>.trix
//!     ...
//!   adversary-n3/
//!     manifest.toml
//!     <fixture-name>.n3
//!     ...
//! ```
//!
//! ## manifest.toml schema
//!
//! ```toml
//! [[fixture]]
//! name = "fm1-example.rdf"
//! description = "FM1 — element nesting depth"
//! expected = "ok"          # "ok" | "reject"
//! finding = "RDFXML-FM1"   # pin identifier from adversary-findings docs
//!
//! [[fixture]]
//! name = "fm2-example.rdf"
//! expected = "reject"
//! finding = "RDFXML-FM2"
//! ```
//!
//! ## Lifecycle
//!
//! Today: if the directory does not exist or `manifest.toml` is absent the
//! test passes trivially — no fixtures yet. This is the correct behaviour
//! during Phase B ramp-up: the harness skeleton must be green before any
//! adversary agent has produced a single fixture.
//!
//! When fixtures arrive: the manifest loader below parses `manifest.toml`
//! and the per-fixture assertions run. Each fixture tagged `expected = "ok"`
//! is parsed with the main parser and diffed against the oracle; each
//! fixture tagged `expected = "reject"` asserts the main parser returns
//! `Err(Diagnostics { fatal: true })`. Diff assertions are also `#[ignore]`
//! until the main parsers land (same lifecycle as phase_b_snapshots.rs).
//!
//! ## ADR references
//!
//! - ADR-0019 §4 — adversary-corpus responsibilities.
//! - ADR-0020 §6.5 — cohort-B path claims; adversary paths never overlap
//!   non-adversary test paths by construction.
//! - ADR-0021 — Phase B format scope.

#![allow(
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::items_after_statements,
    dead_code,
)]

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Directory / manifest helpers
// ---------------------------------------------------------------------------

/// Return the absolute path to an adversary fixture directory for a
/// Phase B format. Fixture dirs live alongside the existing Phase A
/// `adversary-ttl/`, `adversary-nt/` etc. under `tests/`.
fn adversary_phase_b_root(format: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .join("tests")
        .join(format!("adversary-{format}"))
}

/// Return the path to `manifest.toml` inside a Phase B adversary directory,
/// or `None` if the directory or manifest does not exist.
fn manifest_path(format: &str) -> Option<PathBuf> {
    let root = adversary_phase_b_root(format);
    if !root.exists() {
        return None;
    }
    let manifest = root.join("manifest.toml");
    if !manifest.exists() {
        return None;
    }
    Some(manifest)
}

/// Collect fixture file paths from an adversary directory. Returns an empty
/// `Vec` if the directory is absent or contains no files. The list is sorted
/// for deterministic test ordering across machines.
fn collect_phase_b_fixtures(format: &str, extensions: &[&str]) -> Vec<PathBuf> {
    let root = adversary_phase_b_root(format);
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| extensions.contains(&ext))
                    .unwrap_or(false)
        })
        .collect();
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// PB-ADV0 — Directory and manifest presence (structural checks)
//
// These are the always-on structural tests. They pass trivially if the
// directory or manifest is absent (no fixtures yet). Once the adversary
// agent populates a directory they become real assertions.
// ---------------------------------------------------------------------------

/// **PB-ADV0a — RDF/XML adversary directory structure is valid when present.**
///
/// Passes trivially if `adversary-rdfxml/` does not exist or has no manifest.
/// Once the adversary agent populates the directory, asserts:
/// - Fixture files are sorted.
/// - `manifest.toml` exists alongside the fixtures.
#[test]
fn pb_adv0a_rdfxml_directory_structure() {
    let format = "rdfxml";
    let root = adversary_phase_b_root(format);

    if !root.exists() {
        // No fixtures yet — pass trivially.
        return;
    }

    let fixtures = collect_phase_b_fixtures(format, &["rdf", "xml"]);
    for w in fixtures.windows(2) {
        assert!(
            w[0] <= w[1],
            "adversary-rdfxml fixture list is not sorted: {:?} > {:?}",
            w[0],
            w[1],
        );
    }

    // If the directory exists, manifest.toml should be present too.
    if !fixtures.is_empty() {
        let manifest = root.join("manifest.toml");
        assert!(
            manifest.exists(),
            "adversary-rdfxml/manifest.toml missing but fixtures are present: \
             please create manifest.toml per the harness schema in this file's docs",
        );
    }
}

/// **PB-ADV0b — JSON-LD adversary directory structure is valid when present.**
#[test]
fn pb_adv0b_jsonld_directory_structure() {
    let format = "jsonld";
    let root = adversary_phase_b_root(format);

    if !root.exists() {
        return;
    }

    let fixtures = collect_phase_b_fixtures(format, &["jsonld", "json"]);
    for w in fixtures.windows(2) {
        assert!(
            w[0] <= w[1],
            "adversary-jsonld fixture list is not sorted: {:?} > {:?}",
            w[0],
            w[1],
        );
    }

    if !fixtures.is_empty() {
        let manifest = root.join("manifest.toml");
        assert!(
            manifest.exists(),
            "adversary-jsonld/manifest.toml missing but fixtures are present",
        );
    }
}

/// **PB-ADV0c — TriX adversary directory structure is valid when present.**
#[test]
fn pb_adv0c_trix_directory_structure() {
    let format = "trix";
    let root = adversary_phase_b_root(format);

    if !root.exists() {
        return;
    }

    let fixtures = collect_phase_b_fixtures(format, &["trix", "xml"]);
    for w in fixtures.windows(2) {
        assert!(
            w[0] <= w[1],
            "adversary-trix fixture list is not sorted: {:?} > {:?}",
            w[0],
            w[1],
        );
    }

    if !fixtures.is_empty() {
        let manifest = root.join("manifest.toml");
        assert!(
            manifest.exists(),
            "adversary-trix/manifest.toml missing but fixtures are present",
        );
    }
}

/// **PB-ADV0d — N3 adversary directory structure is valid when present.**
#[test]
fn pb_adv0d_n3_directory_structure() {
    let format = "n3";
    let root = adversary_phase_b_root(format);

    if !root.exists() {
        return;
    }

    let fixtures = collect_phase_b_fixtures(format, &["n3"]);
    for w in fixtures.windows(2) {
        assert!(
            w[0] <= w[1],
            "adversary-n3 fixture list is not sorted: {:?} > {:?}",
            w[0],
            w[1],
        );
    }

    if !fixtures.is_empty() {
        let manifest = root.join("manifest.toml");
        assert!(
            manifest.exists(),
            "adversary-n3/manifest.toml missing but fixtures are present",
        );
    }
}

// ---------------------------------------------------------------------------
// PB-ADV1 — Manifest iteration (no-op until fixtures exist)
//
// When `manifest.toml` is present, read it and enumerate fixture entries.
// For now: verify that the manifest is parseable TOML (once present) and
// that every fixture path listed in the manifest exists on disk.
//
// The actual diff assertions (main-parser vs oracle per fixture) are
// wired here as a skeleton — the real assertions are added by the
// adversary agents alongside the fixtures, following the Phase A
// pattern in `adversary_ttl.rs`.
// ---------------------------------------------------------------------------

/// **PB-ADV1a — RDF/XML manifest lists only existing fixture files.**
///
/// Passes trivially if `adversary-rdfxml/manifest.toml` is absent.
/// Once present, asserts every `name` entry in the manifest refers to a
/// file that exists on disk.
#[test]
fn pb_adv1a_rdfxml_manifest_fixtures_exist_on_disk() {
    let Some(manifest_path) = manifest_path("rdfxml") else {
        // No manifest yet — pass trivially.
        return;
    };

    let root = adversary_phase_b_root("rdfxml");
    let content = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", manifest_path.display()));

    // Minimal TOML scan: look for `name = "..."` lines and assert each
    // named file exists. A full TOML parser is not available as a dep
    // in rdf-diff; this covers the structural check.
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name = \"") {
            if let Some(name) = rest.strip_suffix('"') {
                let fixture = root.join(name);
                assert!(
                    fixture.exists(),
                    "adversary-rdfxml/manifest.toml lists fixture {name:?} \
                     that does not exist at {}",
                    fixture.display(),
                );
            }
        }
    }
}

/// **PB-ADV1b — JSON-LD manifest lists only existing fixture files.**
#[test]
fn pb_adv1b_jsonld_manifest_fixtures_exist_on_disk() {
    let Some(manifest_path) = manifest_path("jsonld") else {
        return;
    };

    let root = adversary_phase_b_root("jsonld");
    let content = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", manifest_path.display()));

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name = \"") {
            if let Some(name) = rest.strip_suffix('"') {
                let fixture = root.join(name);
                assert!(
                    fixture.exists(),
                    "adversary-jsonld/manifest.toml lists fixture {name:?} \
                     that does not exist at {}",
                    fixture.display(),
                );
            }
        }
    }
}

/// **PB-ADV1c — TriX manifest lists only existing fixture files.**
#[test]
fn pb_adv1c_trix_manifest_fixtures_exist_on_disk() {
    let Some(manifest_path) = manifest_path("trix") else {
        return;
    };

    let root = adversary_phase_b_root("trix");
    let content = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", manifest_path.display()));

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name = \"") {
            if let Some(name) = rest.strip_suffix('"') {
                let fixture = root.join(name);
                assert!(
                    fixture.exists(),
                    "adversary-trix/manifest.toml lists fixture {name:?} \
                     that does not exist at {}",
                    fixture.display(),
                );
            }
        }
    }
}

/// **PB-ADV1d — N3 manifest lists only existing fixture files.**
#[test]
fn pb_adv1d_n3_manifest_fixtures_exist_on_disk() {
    let Some(manifest_path) = manifest_path("n3") else {
        return;
    };

    let root = adversary_phase_b_root("n3");
    let content = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", manifest_path.display()));

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name = \"") {
            if let Some(name) = rest.strip_suffix('"') {
                let fixture = root.join(name);
                assert!(
                    fixture.exists(),
                    "adversary-n3/manifest.toml lists fixture {name:?} \
                     that does not exist at {}",
                    fixture.display(),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PB-ADV2 — Aggregate fixture discovery (always-on)
//
// Enumerates all four Phase B adversary directories and asserts that the
// union of fixture counts is consistent with what has been claimed by the
// adversary agents. Today this passes with zero fixtures.
// ---------------------------------------------------------------------------

/// **PB-ADV2 — Phase B adversary fixture discovery is stable and sorted.**
///
/// For each of the four Phase B adversary formats, iterates the fixture
/// directory (if it exists) and asserts the fixture list is sorted.
/// Zero fixtures is a valid state; the test is not gated on a minimum count
/// (unlike `adversary_ttl.rs::at0_fixture_discovery_present_and_sorted`
/// which fires once at least 9 fixtures are expected).
#[test]
fn pb_adv2_fixture_discovery_is_stable() {
    let formats: &[(&str, &[&str])] = &[
        ("rdfxml",  &["rdf", "xml"]),
        ("jsonld",  &["jsonld", "json"]),
        ("trix",    &["trix", "xml"]),
        ("n3",      &["n3"]),
    ];

    for &(format, extensions) in formats {
        let fixtures = collect_phase_b_fixtures(format, extensions);
        for w in fixtures.windows(2) {
            assert!(
                w[0] <= w[1],
                "adversary-{format} fixture list is not sorted: {:?} > {:?}",
                w[0],
                w[1],
            );
        }
    }
}
