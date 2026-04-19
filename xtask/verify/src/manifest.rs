//! W3C rdf-tests manifest runner for `xtask verify` (ADR-0019 §2,
//! ADR-0020 §Acceptance).
//!
//! This module reads a W3C-shape manifest (`manifest.ttl`), extracts each
//! test entry, resolves the `mf:action` / `mf:result` IRIs back to
//! on-disk paths, and drives the main Phase-A parsers through the
//! `rdf_diff` harness.
//!
//! # Manifest shapes handled
//!
//! Per `http://www.w3.org/ns/rdftest#` (rdft) and
//! `http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#` (mf):
//!
//! | `rdf:type`                          | Behaviour                                                          |
//! |-------------------------------------|--------------------------------------------------------------------|
//! | `rdft:TestTurtlePositiveSyntax`     | Parse `mf:action` — parser MUST accept.                            |
//! | `rdft:TestTurtleNegativeSyntax`     | Parse `mf:action` — parser MUST reject.                            |
//! | `rdft:TestTurtleEval`               | Parse `mf:action` and `mf:result`, diff the canonical fact sets.   |
//! | `rdft:TestTurtleNegativeEval`       | Parse `mf:action` — parser MUST reject (negative-eval is rejection). |
//! | Same shapes with `NTriples`, `NQuads`, `TriG` prefixes             | Same semantics, dispatched to the matching main parser.            |
//!
//! Anything we don't recognise becomes a `skipped` entry; it never
//! registers a pass or a divergence.
//!
//! # Design notes
//!
//! * We parse the manifest with the main `rdf_turtle::TurtleParser`. The
//!   W3C manifests omit an `@base`; the parser needs one for relative
//!   IRI resolution, so we prepend a synthetic `@base <file://…/>`
//!   directive pointing at the manifest's directory. The synthetic base
//!   is only used to recover on-disk paths — the canonical fact set we
//!   inspect after parsing is pure data.
//! * Test-entry discovery is purely triple-driven: we index the
//!   canonical `Facts` by `(subject, predicate)` and scan for subjects
//!   with an `rdf:type` of any `rdft:Test*` class. This sidesteps the
//!   `rdf:first`/`rdf:rest` list traversal (which gets mangled by
//!   blank-node relabelling during canonicalisation) while still
//!   enumerating every declared entry.

#![allow(
    clippy::redundant_pub_crate,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::match_same_arms,
    clippy::manual_contains
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use rdf_diff::{Facts, ParseOutcome, Parser as DiffParser};
use rdf_ntriples::{NQuadsParser, NTriplesParser};
use rdf_turtle::{TriGParser, TurtleParser};

/// Kind of test declared by the W3C manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestKind {
    /// Parser must accept.
    PositiveSyntax,
    /// Parser must reject.
    NegativeSyntax,
    /// Parser must accept `mf:action` and the canonical facts must equal
    /// the canonical facts of `mf:result` (parsed as N-Triples/N-Quads).
    Eval,
    /// Parser must reject `mf:action` (negative-eval is a rejection test
    /// in the rdf-tests vocabulary).
    NegativeEval,
}

/// Outcome of running a single manifest entry.
#[derive(Debug, Clone)]
pub(crate) struct EntryOutcome {
    /// Short `mf:name`, or the subject IRI as a fallback.
    pub name: String,
    /// Test classification.
    pub kind: TestKind,
    /// `true` iff the entry passed under our main parser.
    pub pass: bool,
    /// Populated on failure — the variant name of the divergence (kept
    /// terse so the JSON report stays readable).
    pub divergence: Option<&'static str>,
    /// Human-readable explanation, used for triage in the report.
    pub message: String,
}

/// Aggregate outcome for one manifest file.
#[derive(Debug, Default, Clone)]
pub(crate) struct ManifestSummary {
    /// Entries we actually executed (positive/negative/eval).
    pub total: usize,
    /// Entries whose behaviour matched the manifest's assertion.
    pub pass: usize,
    /// Entries we recognised but the parser disagreed with.
    pub divergences: usize,
    /// Entries we saw but could not classify (unknown `rdf:type`).
    pub skipped: usize,
    /// Per-entry outcomes, in sorted-by-name order.
    pub entries: Vec<EntryOutcome>,
}

impl ManifestSummary {
    /// Accumulate another manifest's results into this one.
    pub fn extend(&mut self, other: Self) {
        self.total += other.total;
        self.pass += other.pass;
        self.divergences += other.divergences;
        self.skipped += other.skipped;
        self.entries.extend(other.entries);
    }
}

/// Drive every test entry in `manifest_path` through the main parser
/// selected for `language`. Returns a summary on success; returns `Err`
/// only for *infrastructural* failures (cannot read manifest, manifest
/// unparseable). A parser disagreeing with a test is surfaced as a
/// divergence inside the summary — never as an `Err`.
///
/// # Errors
///
/// Propagates IO / parse errors when the manifest itself cannot be read
/// or parsed. Per-entry failures (parser rejecting a positive-syntax
/// test, etc.) are captured in the returned [`ManifestSummary`], not as
/// errors.
pub(crate) fn run_manifest(
    manifest_path: &Path,
    language: &str,
) -> Result<ManifestSummary, String> {
    let raw = fs::read(manifest_path)
        .map_err(|e| format!("read manifest {}: {e}", manifest_path.display()))?;

    // Inject a synthetic @base pointing at the manifest's parent dir so
    // relative mf:action / mf:result IRIs resolve to file:// URLs we can
    // reverse back into on-disk paths. This is the *only* reason we
    // modify the input bytes.
    let manifest_dir = manifest_path
        .parent()
        .ok_or_else(|| format!("manifest {} has no parent", manifest_path.display()))?;
    let base_iri = dir_to_file_iri(manifest_dir)?;
    let mut prepended = Vec::with_capacity(raw.len() + 64);
    prepended.extend_from_slice(b"@base <");
    prepended.extend_from_slice(base_iri.as_bytes());
    prepended.extend_from_slice(b"> .\n");
    prepended.extend_from_slice(&raw);

    let turtle = TurtleParser::new();
    let outcome = turtle
        .parse(&prepended)
        .map_err(|d| format!("manifest parse failed: {:?}", d.messages))?;
    let facts = outcome.facts;

    // Index by (subject, predicate) for O(1) property lookups.
    let index = build_sp_index(&facts);

    let rdf_type = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
    let mf_action = "<http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#action>";
    let mf_result = "<http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#result>";
    let mf_name = "<http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#name>";

    // Walk every subject and check whether its rdf:type is a test class.
    let mut entries: Vec<(String, TestKind)> = Vec::new();
    for ((subject, predicate), objects) in &index {
        if predicate == rdf_type {
            for obj in objects {
                if let Some(kind) = classify_test_type(obj) {
                    entries.push((subject.clone(), kind));
                }
            }
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut summary = ManifestSummary::default();
    for (subject, kind) in entries {
        let name = index
            .get(&(subject.clone(), mf_name.to_owned()))
            .and_then(|objs| objs.first())
            .map_or_else(|| subject.clone(), |n| strip_literal(n).to_owned());

        let action = match index
            .get(&(subject.clone(), mf_action.to_owned()))
            .and_then(|objs| objs.first())
        {
            Some(iri) => iri.clone(),
            None => {
                summary.skipped += 1;
                summary.entries.push(EntryOutcome {
                    name,
                    kind,
                    pass: false,
                    divergence: Some("MissingAction"),
                    message: "entry has no mf:action".into(),
                });
                continue;
            }
        };
        let result = index
            .get(&(subject.clone(), mf_result.to_owned()))
            .and_then(|objs| objs.first())
            .cloned();

        let action_path = match iri_to_local_path(&action, &base_iri, manifest_dir) {
            Ok(p) => p,
            Err(e) => {
                summary.skipped += 1;
                summary.entries.push(EntryOutcome {
                    name,
                    kind,
                    pass: false,
                    divergence: Some("UnresolvableAction"),
                    message: e,
                });
                continue;
            }
        };

        let entry_outcome = run_entry(kind, language, &name, &action_path, result.as_deref(), &base_iri, manifest_dir);
        if entry_outcome.pass {
            summary.pass += 1;
        } else {
            summary.divergences += 1;
        }
        summary.total += 1;
        summary.entries.push(entry_outcome);
    }

    Ok(summary)
}

/// Execute a single classified entry and return its outcome.
fn run_entry(
    kind: TestKind,
    language: &str,
    name: &str,
    action_path: &Path,
    result_iri: Option<&str>,
    base_iri: &str,
    manifest_dir: &Path,
) -> EntryOutcome {
    let action_bytes = match fs::read(action_path) {
        Ok(b) => b,
        Err(e) => {
            return EntryOutcome {
                name: name.to_owned(),
                kind,
                pass: false,
                divergence: Some("ActionIoError"),
                message: format!("cannot read {}: {e}", action_path.display()),
            };
        }
    };

    let parse_main = parse_for_language(language, &action_bytes);

    match kind {
        TestKind::PositiveSyntax => match parse_main {
            Ok(_) => EntryOutcome {
                name: name.to_owned(),
                kind,
                pass: true,
                divergence: None,
                message: "accepted".into(),
            },
            Err(msg) => EntryOutcome {
                name: name.to_owned(),
                kind,
                pass: false,
                divergence: Some("AcceptRejectSplit"),
                message: format!("expected-accept but rejected: {msg}"),
            },
        },
        TestKind::NegativeSyntax | TestKind::NegativeEval => match parse_main {
            Ok(_) => EntryOutcome {
                name: name.to_owned(),
                kind,
                pass: false,
                divergence: Some("AcceptRejectSplit"),
                message: "expected-reject but accepted".into(),
            },
            Err(_) => EntryOutcome {
                name: name.to_owned(),
                kind,
                pass: true,
                divergence: None,
                message: "rejected".into(),
            },
        },
        TestKind::Eval => run_eval(
            name,
            language,
            parse_main,
            result_iri,
            base_iri,
            manifest_dir,
        ),
    }
}

/// Expected-vs-actual comparison for eval-shape tests.
fn run_eval(
    name: &str,
    language: &str,
    parse_main: Result<ParseOutcome, String>,
    result_iri: Option<&str>,
    base_iri: &str,
    manifest_dir: &Path,
) -> EntryOutcome {
    let actual = match parse_main {
        Ok(o) => o.facts,
        Err(msg) => {
            return EntryOutcome {
                name: name.to_owned(),
                kind: TestKind::Eval,
                pass: false,
                divergence: Some("AcceptRejectSplit"),
                message: format!("expected-eval but main parser rejected action: {msg}"),
            };
        }
    };
    let Some(result_iri) = result_iri else {
        return EntryOutcome {
            name: name.to_owned(),
            kind: TestKind::Eval,
            pass: false,
            divergence: Some("MissingResult"),
            message: "eval test missing mf:result".into(),
        };
    };
    let result_path = match iri_to_local_path(result_iri, base_iri, manifest_dir) {
        Ok(p) => p,
        Err(e) => {
            return EntryOutcome {
                name: name.to_owned(),
                kind: TestKind::Eval,
                pass: false,
                divergence: Some("UnresolvableResult"),
                message: e,
            };
        }
    };
    let result_bytes = match fs::read(&result_path) {
        Ok(b) => b,
        Err(e) => {
            return EntryOutcome {
                name: name.to_owned(),
                kind: TestKind::Eval,
                pass: false,
                divergence: Some("ResultIoError"),
                message: format!("cannot read {}: {e}", result_path.display()),
            };
        }
    };
    // Eval tests always express the expected output in N-Triples
    // (triples) or N-Quads (quads). Pick the parser by the result
    // file's extension with a language-based fallback.
    let expected_parser_lang = match result_path.extension().and_then(|s| s.to_str()) {
        Some("nq") => "nq",
        Some("nt") => "nt",
        _ => match language {
            "nq" | "trig" => "nq",
            _ => "nt",
        },
    };
    let expected = match parse_for_language(expected_parser_lang, &result_bytes) {
        Ok(o) => o.facts,
        Err(msg) => {
            return EntryOutcome {
                name: name.to_owned(),
                kind: TestKind::Eval,
                pass: false,
                divergence: Some("ExpectedParseError"),
                message: format!("expected-output parse failed: {msg}"),
            };
        }
    };
    match rdf_diff::diff(&actual, &expected) {
        Ok(report) if report.is_clean() => EntryOutcome {
            name: name.to_owned(),
            kind: TestKind::Eval,
            pass: true,
            divergence: None,
            message: "eval match".into(),
        },
        Ok(report) => EntryOutcome {
            name: name.to_owned(),
            kind: TestKind::Eval,
            pass: false,
            divergence: Some(first_divergence_variant(&report)),
            message: format!(
                "eval mismatch: {} divergence(s) — {}",
                report.divergences.len(),
                report.triage_hint
            ),
        },
        Err(e) => EntryOutcome {
            name: name.to_owned(),
            kind: TestKind::Eval,
            pass: false,
            divergence: Some("NonCanonical"),
            message: format!("diff error: {e}"),
        },
    }
}

/// Parse `input` through the main parser selected by `language`. Returns
/// a human-readable error message on rejection (the caller only cares
/// about the accept/reject split plus the fact set on accept).
fn parse_for_language(language: &str, input: &[u8]) -> Result<ParseOutcome, String> {
    match language {
        "ttl" => TurtleParser::new()
            .parse(input)
            .map_err(|d| format_diag(&d.messages)),
        "trig" => TriGParser::new()
            .parse(input)
            .map_err(|d| format_diag(&d.messages)),
        "nt" => NTriplesParser
            .parse(input)
            .map_err(|d| format_diag(&d.messages)),
        "nq" => NQuadsParser
            .parse(input)
            .map_err(|d| format_diag(&d.messages)),
        other => Err(format!("no main parser registered for language `{other}`")),
    }
}

fn format_diag(messages: &[String]) -> String {
    if messages.is_empty() {
        "parser rejected input".to_owned()
    } else {
        messages.join("; ")
    }
}

/// Index `Facts` by `(subject, predicate) -> [object]` preserving
/// deterministic object order.
fn build_sp_index(facts: &Facts) -> BTreeMap<(String, String), Vec<String>> {
    let mut index: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for fact in facts.set.keys() {
        index
            .entry((fact.subject.clone(), fact.predicate.clone()))
            .or_default()
            .push(fact.object.clone());
    }
    index
}

/// Recognise an rdft test-type IRI. Returns `None` for list entries,
/// `mf:Manifest`, or anything outside the rdft vocabulary.
fn classify_test_type(type_iri: &str) -> Option<TestKind> {
    // Strip the angle brackets we added during canonicalisation.
    let inner = type_iri.strip_prefix('<')?.strip_suffix('>')?;
    // Trim the rdft namespace. We accept both the standard `ns/rdftest#`
    // root and the legacy `test-manifest#` prefix a few older manifests
    // still use.
    let local = inner
        .strip_prefix("http://www.w3.org/ns/rdftest#")
        .or_else(|| inner.strip_prefix("http://www.w3.org/2013/TurtleTests/"))?;
    if local.ends_with("PositiveSyntax") {
        Some(TestKind::PositiveSyntax)
    } else if local.ends_with("NegativeSyntax") {
        Some(TestKind::NegativeSyntax)
    } else if local.ends_with("NegativeEval") {
        Some(TestKind::NegativeEval)
    } else if local.ends_with("Eval") {
        Some(TestKind::Eval)
    } else {
        None
    }
}

/// Extract the lexical form of a canonical literal (`"text"` or
/// `"text"@tag` or `"text"^^<iri>`). Returns the input unchanged if it
/// isn't a literal.
fn strip_literal(term: &str) -> &str {
    if !term.starts_with('"') {
        return term;
    }
    // Walk to the matching unescaped quote.
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

/// First-in-sorted-order divergence variant, for terse reporting.
fn first_divergence_variant(report: &rdf_diff::DiffReport) -> &'static str {
    match report.divergences.first() {
        Some(rdf_diff::Divergence::FactOnlyIn { .. }) => "FactOnlyIn",
        Some(rdf_diff::Divergence::ObjectMismatch { .. }) => "ObjectMismatch",
        Some(rdf_diff::Divergence::AcceptRejectSplit { .. }) => "AcceptRejectSplit",
        None => "None",
    }
}

/// Render a directory as a `file://…/` IRI suitable for `@base`.
fn dir_to_file_iri(dir: &Path) -> Result<String, String> {
    let canonical = fs::canonicalize(dir)
        .map_err(|e| format!("canonicalise {}: {e}", dir.display()))?;
    let mut s = String::from("file://");
    // Percent-encode path components that aren't safe in an IRI.
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
    Ok(s)
}

/// Reverse an IRI we emitted (via the synthetic `@base`) into an on-disk
/// path. Both `<...>` wrapping and raw IRIs are accepted because the
/// canonical form ships with the brackets.
fn iri_to_local_path(iri: &str, base_iri: &str, manifest_dir: &Path) -> Result<PathBuf, String> {
    let stripped = iri
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(iri);
    if let Some(tail) = stripped.strip_prefix(base_iri) {
        // Tail is the relative segment, percent-encoded. We decode
        // percent escapes and join to the manifest directory.
        let decoded = percent_decode(tail)?;
        return Ok(manifest_dir.join(decoded));
    }
    // Fallback: if the IRI is a bare `file://` URL outside our base, try
    // to materialise it directly. Anything else is out of scope.
    if let Some(rest) = stripped.strip_prefix("file://") {
        let decoded = percent_decode(rest)?;
        return Ok(PathBuf::from(decoded));
    }
    Err(format!("cannot resolve IRI {iri} to an on-disk path"))
}

/// Minimal percent-decoder for ASCII paths. We only handle UTF-8 bytes
/// and `%XX` escapes; anything else is a hard error so we never silently
/// produce a wrong path.
fn percent_decode(input: &str) -> Result<String, String> {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(format!("truncated percent escape in {input}"));
            }
            let hi = hex_digit(bytes[i + 1])?;
            let lo = hex_digit(bytes[i + 2])?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|e| format!("percent-decoded bytes not UTF-8: {e}"))
}

fn hex_digit(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        other => Err(format!("bad hex digit 0x{other:02x}")),
    }
}

/// Look for `external/tests/<lang>/manifest.ttl` first (the layout the
/// pa-w3c-vendor sibling produces). Fall back to scanning the
/// upstream-vendored `external/tests/**` tree and classifying manifests
/// by the directory name convention used by the W3C rdf-tests repo
/// (`rdf-turtle`, `rdf-n-triples`, …).
#[must_use]
pub(crate) fn discover_manifests_for_language(
    vendored_root: &Path,
    language: &str,
) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();

    // Contract-shape first.
    let direct = vendored_root.join(language).join("manifest.ttl");
    if direct.is_file() {
        found.push(direct);
    }

    // Legacy shape: `external/tests/<vendor>/rdf/**/rdf-<alias>/manifest.ttl`.
    let aliases: &[&str] = match language {
        "nt" => &["rdf-n-triples"],
        "nq" => &["rdf-n-quads"],
        "ttl" => &["rdf-turtle"],
        "trig" => &["rdf-trig"],
        "rdfxml" => &["rdf-xml"],
        "sparql" => &[], // SPARQL manifests don't use rdft syntax tests.
        _ => &[],
    };
    if !aliases.is_empty() {
        let mut stack = vec![vendored_root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = fs::read_dir(&dir) else { continue };
            for entry in rd.flatten() {
                let path = entry.path();
                let Ok(ft) = entry.file_type() else { continue };
                if ft.is_symlink() {
                    continue;
                }
                if ft.is_dir() {
                    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if aliases.iter().any(|a| *a == name) {
                        let m = path.join("manifest.ttl");
                        if m.is_file() && !found.contains(&m) {
                            found.push(m);
                        }
                    } else {
                        stack.push(path);
                    }
                }
            }
        }
    }

    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_recognises_rdft_types() {
        assert_eq!(
            classify_test_type("<http://www.w3.org/ns/rdftest#TestTurtlePositiveSyntax>"),
            Some(TestKind::PositiveSyntax)
        );
        assert_eq!(
            classify_test_type("<http://www.w3.org/ns/rdftest#TestTurtleNegativeSyntax>"),
            Some(TestKind::NegativeSyntax)
        );
        assert_eq!(
            classify_test_type("<http://www.w3.org/ns/rdftest#TestTurtleEval>"),
            Some(TestKind::Eval)
        );
        assert_eq!(
            classify_test_type("<http://www.w3.org/ns/rdftest#TestTurtleNegativeEval>"),
            Some(TestKind::NegativeEval)
        );
        assert_eq!(
            classify_test_type("<http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#Manifest>"),
            None
        );
    }

    #[test]
    fn strip_literal_peels_quotes() {
        assert_eq!(strip_literal("\"hello\""), "hello");
        assert_eq!(strip_literal("\"hi\"@en"), "hi");
        assert_eq!(strip_literal("<http://ex/>"), "<http://ex/>");
    }

    #[test]
    fn hex_digit_rejects_non_hex() {
        assert!(hex_digit(b'g').is_err());
        assert_eq!(hex_digit(b'F').unwrap(), 15);
    }

    #[test]
    fn percent_decode_roundtrip() {
        assert_eq!(percent_decode("a%20b").unwrap(), "a b");
        assert_eq!(percent_decode("plain").unwrap(), "plain");
        assert!(percent_decode("%2").is_err());
    }
}
