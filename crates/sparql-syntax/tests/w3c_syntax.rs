//! W3C SPARQL 1.1 syntax test suite integration tests.
//!
//! Loads `external/tests/sparql/syntax-query/manifest.ttl`,
//! `syntax-update-1/manifest.ttl`, and `syntax-update-2/manifest.ttl`,
//! runs every syntax entry through [`SparqlParser`], and asserts:
//!
//! - `mf:PositiveSyntaxTest11` / `mf:PositiveUpdateSyntaxTest11` — parser
//!   MUST accept (no fatal diagnostics).
//! - `mf:NegativeSyntaxTest11` / `mf:NegativeUpdateSyntaxTest11` — parser
//!   MUST reject (fatal diagnostics present).
//!
//! The test fails (with a full per-entry summary) if any entry is unexpected.
//!
//! # Allow-listed entries
//!
//! Entries that are genuinely ambiguous or where a spec divergence has been
//! documented are collected in `ALLOW_LIST` below with `// ALLOW: <reason>`.
//! An allow-listed failure is reported but does not fail the test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use rdf_diff::Parser as _;
use rdf_turtle::TurtleParser;
use sparql_syntax::SparqlParser;

// ---------------------------------------------------------------------------
// Allow-list: entries that are expected to diverge from the W3C expectation.
// Each entry is a test name (mf:name literal) and a human-readable reason.
// ---------------------------------------------------------------------------

/// `(test_name, reason)` pairs for expected divergences.
/// An entry here makes a failure non-fatal (still reported).
///
/// Format per coding standard: `// ALLOW: <reason>`
const ALLOW_LIST: &[(&str, &str)] = &[
    // ALLOW: The W3C test suite includes entries that test features beyond
    // grammar-only concerns (e.g., VALUES cardinality checks). If any such
    // entries are discovered during the run they will be listed here with
    // the correct reason string added.
];

// ---------------------------------------------------------------------------
// Manifest-parsing helpers (mirrors the xtask/verify approach)
// ---------------------------------------------------------------------------

/// Index facts by (subject, predicate) -> [object].
fn build_sp_index(facts: &rdf_diff::Facts) -> BTreeMap<(String, String), Vec<String>> {
    let mut index: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for fact in facts.set.keys() {
        index
            .entry((fact.subject.clone(), fact.predicate.clone()))
            .or_default()
            .push(fact.object.clone());
    }
    index
}

/// Strip `"..."`, `"..."@tag`, `"..."^^<iri>` down to the bare lexical form.
fn strip_literal(term: &str) -> &str {
    if !term.starts_with('"') {
        return term;
    }
    let bytes = term.as_bytes();
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return term.get(1..i).unwrap_or(term),
            _ => i += 1,
        }
    }
    term
}

/// Render a directory as a `file://…/` IRI.
fn dir_to_file_iri(dir: &Path) -> String {
    let canonical = fs::canonicalize(dir).expect("canonicalise manifest dir");
    let mut s = String::from("file://");
    for ch in canonical.to_string_lossy().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '/' | '-' | '_' | '.' | '~') {
            s.push(ch);
        } else {
            let mut buf = [0u8; 4];
            for &b in ch.encode_utf8(&mut buf).as_bytes() {
                let _ = write!(s, "%{b:02X}");
            }
        }
    }
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

/// Minimal percent-decoder for ASCII paths.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = match bytes[i + 1] {
                b'0'..=b'9' => bytes[i + 1] - b'0',
                b'a'..=b'f' => bytes[i + 1] - b'a' + 10,
                b'A'..=b'F' => bytes[i + 1] - b'A' + 10,
                _ => {
                    out.push(bytes[i]);
                    i += 1;
                    continue;
                }
            };
            let lo = match bytes[i + 2] {
                b'0'..=b'9' => bytes[i + 2] - b'0',
                b'a'..=b'f' => bytes[i + 2] - b'a' + 10,
                b'A'..=b'F' => bytes[i + 2] - b'A' + 10,
                _ => {
                    out.push(bytes[i]);
                    i += 1;
                    continue;
                }
            };
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).expect("percent-decoded bytes are valid UTF-8")
}

/// Reverse an IRI (with or without `<…>` angle brackets) to a local path.
fn iri_to_path(iri: &str, base_iri: &str, manifest_dir: &Path) -> Option<PathBuf> {
    let stripped = iri
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(iri);
    if let Some(tail) = stripped.strip_prefix(base_iri) {
        return Some(manifest_dir.join(percent_decode(tail)));
    }
    if let Some(rest) = stripped.strip_prefix("file://") {
        return Some(PathBuf::from(percent_decode(rest)));
    }
    None
}

// ---------------------------------------------------------------------------
// Entry classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Positive,
    Negative,
}

const MF_NS: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";

fn classify(type_iri: &str) -> Option<Kind> {
    let inner = type_iri.strip_prefix('<')?.strip_suffix('>')?;
    let local = inner.strip_prefix(MF_NS)?;
    match local {
        "PositiveSyntaxTest11" | "PositiveUpdateSyntaxTest11" => Some(Kind::Positive),
        "NegativeSyntaxTest11" | "NegativeUpdateSyntaxTest11" => Some(Kind::Negative),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Per-manifest runner
// ---------------------------------------------------------------------------

struct Entry {
    name: String,
    kind: Kind,
    path: PathBuf,
}

fn load_manifest(manifest_path: &Path) -> Vec<Entry> {
    let raw = fs::read(manifest_path)
        .unwrap_or_else(|e| panic!("cannot read manifest {}: {e}", manifest_path.display()));

    let manifest_dir = manifest_path
        .parent()
        .expect("manifest has parent directory");
    let base_iri = dir_to_file_iri(manifest_dir);

    // Prepend a synthetic @base so relative mf:action IRIs resolve.
    let mut prepended = Vec::with_capacity(raw.len() + 64);
    prepended.extend_from_slice(b"@base <");
    prepended.extend_from_slice(base_iri.as_bytes());
    prepended.extend_from_slice(b"> .\n");
    prepended.extend_from_slice(&raw);

    let turtle = TurtleParser::new();
    let outcome = turtle
        .parse(&prepended)
        .unwrap_or_else(|d| panic!("manifest {} failed to parse: {:?}", manifest_path.display(), d.messages));
    let facts = outcome.facts;
    let index = build_sp_index(&facts);

    let rdf_type = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
    let mf_action = format!("<{MF_NS}action>");
    let mf_name = format!("<{MF_NS}name>");

    let mut entries: Vec<Entry> = Vec::new();
    for ((subject, predicate), objects) in &index {
        if predicate != rdf_type {
            continue;
        }
        for obj in objects {
            let Some(kind) = classify(obj) else { continue };

            let name = index
                .get(&(subject.clone(), mf_name.clone()))
                .and_then(|objs| objs.first())
                .map(|n| strip_literal(n).to_owned())
                .unwrap_or_else(|| subject.clone());

            let action_iri = index
                .get(&(subject.clone(), mf_action.clone()))
                .and_then(|objs| objs.first())
                .cloned()
                .unwrap_or_default();

            let path = iri_to_path(&action_iri, &base_iri, manifest_dir)
                .unwrap_or_else(|| panic!("cannot resolve action IRI {action_iri} for test {name}"));

            entries.push(Entry { name, kind, path });
        }
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

// ---------------------------------------------------------------------------
// Main test
// ---------------------------------------------------------------------------

fn is_allowed(name: &str) -> Option<&'static str> {
    ALLOW_LIST
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, reason)| *reason)
}

fn run_manifest_tests(manifest_path: &Path, label: &str) -> (usize, usize, usize) {
    let entries = load_manifest(manifest_path);
    let parser = SparqlParser::new();

    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut allowed = 0usize;

    for entry in &entries {
        let src = match fs::read(&entry.path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[{label}] INFRA-ERROR reading {}: {e}", entry.path.display());
                fail += 1;
                continue;
            }
        };

        let result = parser.parse(&src);
        let got_err = result.is_err();

        let expected_reject = entry.kind == Kind::Negative;
        let ok = got_err == expected_reject;

        if ok {
            pass += 1;
        } else {
            let reason = is_allowed(&entry.name);
            if let Some(r) = reason {
                eprintln!(
                    "[{label}] ALLOW ({r}) {kind:?} {name}: {outcome}",
                    kind = entry.kind,
                    name = entry.name,
                    outcome = if got_err { "rejected" } else { "accepted" },
                );
                allowed += 1;
                pass += 1;
            } else {
                let diag = match result {
                    Ok(_) => String::from("(accepted)"),
                    Err(d) => d.messages.join("; "),
                };
                eprintln!(
                    "[{label}] FAIL {kind:?} {name}: expected {exp}, got {got} — {diag}",
                    kind = entry.kind,
                    name = entry.name,
                    exp = if expected_reject { "reject" } else { "accept" },
                    got = if got_err { "reject" } else { "accept" },
                    diag = diag,
                );
                fail += 1;
            }
        }
    }

    (pass, fail, allowed)
}

#[test]
fn w3c_sparql_syntax() {
    // Locate the workspace root (walk up from CARGO_MANIFEST_DIR until we
    // find a Cargo.toml with `[workspace]`).
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = {
        let mut cur = crate_dir.as_path();
        loop {
            let candidate = cur.join("Cargo.toml");
            if candidate.is_file() {
                if let Ok(contents) = fs::read_to_string(&candidate) {
                    if contents.contains("[workspace]") {
                        break cur.to_path_buf();
                    }
                }
            }
            cur = cur.parent().expect("no workspace Cargo.toml found");
        }
    };

    let sparql_root = repo_root.join("external/tests/sparql");

    let manifests: &[(&str, &str)] = &[
        ("syntax-query/manifest.ttl", "query"),
        ("syntax-update-1/manifest.ttl", "update-1"),
        ("syntax-update-2/manifest.ttl", "update-2"),
    ];

    let mut total_pass = 0usize;
    let mut total_fail = 0usize;
    let mut total_allowed = 0usize;

    for (rel, label) in manifests {
        let path = sparql_root.join(rel);
        if !path.is_file() {
            eprintln!("W3C SPARQL manifest not found: {} — skipping", path.display());
            continue;
        }
        let (p, f, a) = run_manifest_tests(&path, label);
        eprintln!(
            "[{label}] pass={p} fail={f} allow-listed={a} (total={})",
            p + f - a
        );
        total_pass += p;
        total_fail += f;
        total_allowed += a;
    }

    eprintln!(
        "W3C SPARQL syntax: total_pass={total_pass} total_fail={total_fail} \
         allow_listed={total_allowed}"
    );

    assert_eq!(
        total_fail,
        0,
        "{total_fail} W3C SPARQL syntax test(s) failed \
         (pass={total_pass}, allow-listed={total_allowed}). \
         Run with `cargo test -p sparql-syntax -- --nocapture` for details."
    );
}
