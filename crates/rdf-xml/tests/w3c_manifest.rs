//! W3C RDF/XML test-suite integration harness.
//!
//! Iterates over `external/tests/rdfxml/manifest.ttl`, finds every
//! `rdft:TestXMLEval` (positive — parser must accept) and
//! `rdft:TestXMLNegativeSyntax` (negative — parser must reject) entry, runs
//! each through [`rdf_xml::RdfXmlParser`], and records the outcome.

#![allow(
    clippy::items_after_test_module,
    clippy::similar_names,
    clippy::single_match_else
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rdf_xml::RdfXmlParser;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn rdfxml_dir() -> PathBuf {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_root
        .ancestors()
        .nth(2)
        .expect("workspace root not found")
        .join("external/tests/rdfxml")
}

fn parse_manifest(dir: &Path) -> Vec<ManifestEntry> {
    let manifest_path = dir.join("manifest.ttl");
    let src = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", manifest_path.display()));

    let mut entries: BTreeMap<String, ManifestEntry> = BTreeMap::new();
    let mut current_subject: Option<String> = None;

    for raw_line in src.lines() {
        let line = raw_line.trim();

        if let Some(rest) = line.strip_prefix("<#") {
            if let Some(frag) = rest.split('>').next() {
                let subject = frag.to_owned();
                let kind = if line.contains("rdft:TestXMLEval") {
                    Some(EntryKind::Positive)
                } else if line.contains("rdft:TestXMLNegativeSyntax") {
                    Some(EntryKind::Negative)
                } else {
                    None
                };
                current_subject = Some(subject.clone());
                if let Some(k) = kind {
                    entries.insert(
                        subject.clone(),
                        ManifestEntry {
                            name: subject,
                            action: PathBuf::new(),
                            result: None,
                            kind: k,
                        },
                    );
                }
            }
            continue;
        }

        if line.starts_with("mf:name") {
            if let Some(subj) = &current_subject {
                if let Some(entry) = entries.get_mut(subj) {
                    let name = line
                        .trim_start_matches("mf:name")
                        .trim()
                        .trim_matches('"')
                        .trim_end_matches(';')
                        .trim_matches('"')
                        .to_owned();
                    if !name.is_empty() {
                        entry.name = name;
                    }
                }
            }
            continue;
        }

        if line.starts_with("mf:action") {
            if let Some(subj) = &current_subject {
                if let Some(entry) = entries.get_mut(subj) {
                    if let (Some(open), Some(close)) = (line.find('<'), line.rfind('>')) {
                        let rel = &line[open + 1..close];
                        entry.action = dir.join(rel);
                    }
                }
            }
            continue;
        }

        if line.starts_with("mf:result") {
            if let Some(subj) = &current_subject {
                if let Some(entry) = entries.get_mut(subj) {
                    if let (Some(open), Some(close)) = (line.find('<'), line.rfind('>')) {
                        let rel = &line[open + 1..close];
                        entry.result = Some(dir.join(rel));
                    }
                }
            }
            continue;
        }

        if line.is_empty() || line == "." {
            current_subject = None;
        }
    }

    let mut result: Vec<ManifestEntry> = entries.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EntryKind {
    Positive,
    Negative,
}

#[derive(Debug, Clone)]
struct ManifestEntry {
    name: String,
    action: PathBuf,
    result: Option<PathBuf>,
    kind: EntryKind,
}

// ---------------------------------------------------------------------------
// The assumed base URI for the W3C test suite.
// ---------------------------------------------------------------------------

const SUITE_BASE: &str = "https://w3c.github.io/rdf-tests/rdf/rdf11/rdf-xml/";

// ---------------------------------------------------------------------------
// Test cases
// ---------------------------------------------------------------------------

#[test]
fn negative_syntax_tests_are_rejected() {
    let dir = rdfxml_dir();
    if !dir.is_dir() {
        eprintln!("SKIP: rdfxml fixture directory not found at {}", dir.display());
        return;
    }

    let entries = parse_manifest(&dir);
    let parser = RdfXmlParser::new();

    let negatives: Vec<_> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Negative)
        .collect();

    assert!(
        !negatives.is_empty(),
        "Expected at least one negative-syntax test in the manifest"
    );

    let mut failures: Vec<String> = Vec::new();

    for entry in &negatives {
        let bytes = match fs::read(&entry.action) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("SKIP {}: cannot read {}: {e}", entry.name, entry.action.display());
                continue;
            }
        };

        // Build per-test base URI from the file path relative to suite base.
        let rel = entry.action.strip_prefix(&dir).unwrap_or(&entry.action);
        let test_base = format!("{}{}", SUITE_BASE, rel.to_string_lossy().replace('\\', "/"));

        match parser.parse_with_base(&bytes, &test_base) {
            Err(_) => {} // correct: parser rejected
            Ok(_) => {
                failures.push(format!(
                    "{} — accepted but should have been rejected ({})",
                    entry.name,
                    entry.action.display()
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} negative-syntax test(s) were incorrectly accepted:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}

#[test]
fn positive_eval_tests_stubbed_parser_output() {
    let dir = rdfxml_dir();
    if !dir.is_dir() {
        eprintln!("SKIP: rdfxml fixture directory not found at {}", dir.display());
        return;
    }

    let entries = parse_manifest(&dir);
    let parser = RdfXmlParser::new();

    let positives: Vec<_> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Positive)
        .collect();

    assert!(!positives.is_empty(), "Expected at least one positive/eval test");

    let mut failures: Vec<String> = Vec::new();

    for entry in &positives {
        let bytes = match fs::read(&entry.action) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("SKIP {}: cannot read {}: {e}", entry.name, entry.action.display());
                continue;
            }
        };

        let rel = entry.action.strip_prefix(&dir).unwrap_or(&entry.action);
        let test_base = format!("{}{}", SUITE_BASE, rel.to_string_lossy().replace('\\', "/"));

        match parser.parse_with_base(&bytes, &test_base) {
            Err(d) => {
                failures.push(format!(
                    "{} — rejected: {}",
                    entry.name,
                    d.messages.join("; ")
                ));
            }
            Ok(_) => {} // accepted — pass for now (content diff done by xtask)
        }
    }

    if !failures.is_empty() {
        panic!(
            "{}/{} positive/eval test(s) were incorrectly rejected:\n  {}",
            failures.len(),
            positives.len(),
            failures.join("\n  ")
        );
    }
}

#[test]
fn manifest_is_discoverable() {
    let dir = rdfxml_dir();
    if !dir.is_dir() {
        eprintln!("SKIP: rdfxml fixture directory not found at {}", dir.display());
        return;
    }

    let entries = parse_manifest(&dir);
    let positives = entries.iter().filter(|e| e.kind == EntryKind::Positive).count();
    let negatives = entries.iter().filter(|e| e.kind == EntryKind::Negative).count();

    assert!(positives > 0, "Expected positive/eval tests in manifest, found 0");
    assert!(negatives > 0, "Expected negative-syntax tests in manifest, found 0");

    eprintln!(
        "Manifest discovered: {positives} positive/eval, {negatives} negative-syntax tests"
    );
}
