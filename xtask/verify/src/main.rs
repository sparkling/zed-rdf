//! `cargo run -p xtask -- verify` — the PR-gate entry point for the
//! verification-v1 sweep.
//!
//! Behaviour (ADR-0020 §3, `v1-ci-wiring` + Phase-A `pa-manifest-runner`):
//!
//! 1. Discover format corpora — W3C manifests under
//!    `external/tests/<lang>/manifest.ttl` (the pa-w3c-vendor contract)
//!    or, as a fallback, any `manifest.ttl` anywhere under
//!    `external/tests/**` that matches the W3C rdf-tests directory-name
//!    conventions (`rdf-turtle`, `rdf-n-triples`, …). Edge-case inputs
//!    plus the smoke-fixture fallback under
//!    `external/fact-oracles/fixtures/smoke/<lang>/**` are still
//!    enumerated for report provenance.
//! 2. Discover oracle JSON fact corpora under
//!    `external/fact-oracles/<lang>/*.json` (materialised out-of-process
//!    by the `fact-oracles.yml` workflow — **no JVM runs here**; JSON is
//!    consumed as data).
//! 3. Drive every classified manifest entry through the matching main
//!    parser (`rdf_turtle::TurtleParser` / `TriGParser`,
//!    `rdf_ntriples::NTriplesParser` / `NQuadsParser`) and the
//!    `rdf_diff` harness. Positive-syntax entries assert accept,
//!    negative-syntax assert reject, eval entries diff the action's
//!    canonical facts against the parsed `mf:result` expected output.
//! 4. Emit one `DiffReport` JSON per format, a `summary.json`, and a
//!    Phase-A exit-gate report at
//!    `target/verification-reports/phase-a-exit-gate.json`. The workflow
//!    uploads this tree as a build artifact on failure
//!    (`.github/workflows/verification.yml`).
//! 5. Exit non-zero on any non-allow-listed divergence. Allow-list file
//!    path: `crates/testing/rdf-diff/ALLOWLIST.md` (ADR-0019 §2).
//!
//! Exit-gate matrix (ADR-0020 §Acceptance, `pa-manifest-runner`):
//!
//! | Condition                                                    | Exit |
//! |--------------------------------------------------------------|------|
//! | `smoke=true`                                                 | 0    |
//! | `smoke=false`, no tests ran (manifests absent)               | 1    |
//! | `smoke=false`, manifests ran, zero divergences               | 0    |
//! | `smoke=false`, divergences but all allow-listed              | 0 + warn |
//! | `smoke=false`, positive-syntax rejected or negative accepted | 1    |
//! | `smoke=false`, other non-allow-listed divergence             | 1    |
//!
//! Deliberate non-features:
//!
//! - No JVM invocation, ever. This binary only reads JSON that the
//!   scheduled `fact-oracles.yml` materialised.
//! - No `serde`/`serde_json` dependency — we serialise summary/report
//!   JSON by hand so the xtask graph stays minimal and does not risk
//!   pulling any banned crate transitively (ADR-0019 §1, `deny.toml`).
//! - No dependency on shadow crates at compile time: shadow parsers
//!   remain behind the optional `shadow-*` features.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// pedantic lint carve-outs kept narrow to the bits this single-file
// driver genuinely trips on.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions
)]

mod manifest;

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::manifest::{EntryOutcome, ManifestSummary, TestKind};

/// Repository-relative paths consulted by `verify`. Centralised so the
/// CI workflow and the binary agree on layout without a hidden coupling.
mod layout {
    /// Parent directory of the vendored W3C suites (per language
    /// subdirectory; see ADR-0018 phase-A prerequisite).
    pub const VENDORED_SUITE_ROOT: &str = "external/tests";
    /// Parent directory of the JVM-materialised oracle JSON corpora.
    pub const ORACLE_ROOT: &str = "external/fact-oracles";
    /// Smoke-fixture fallback when `VENDORED_SUITE_ROOT` is absent. Used
    /// by `cargo run -p xtask -- verify` on a freshly-cloned checkout.
    pub const SMOKE_ROOT: &str = "external/fact-oracles/fixtures/smoke";
    /// Allow-list for intentional divergences (ADR-0019 §2).
    pub const ALLOWLIST: &str = "crates/testing/rdf-diff/ALLOWLIST.md";
    /// Output tree for per-format reports + summary.
    pub const REPORT_DIR: &str = "target/verification-reports";
}

/// Languages the harness enumerates. Kept in lockstep with
/// `fact-oracles.yml`'s matrix so a mismatch is immediately visible.
const LANGUAGES: &[&str] = &["nt", "nq", "ttl", "trig", "rdfxml", "sparql"];

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("verify") => match run_verify(&args[1..]) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("xtask verify: {e}");
                ExitCode::from(2)
            }
        },
        Some(other) => {
            eprintln!("xtask: unknown subcommand `{other}` (expected `verify`)");
            ExitCode::from(2)
        }
        None => {
            eprintln!("xtask: missing subcommand (expected `verify`)");
            ExitCode::from(2)
        }
    }
}

/// Parse flags and dispatch. Returns the process exit code.
fn run_verify(args: &[String]) -> Result<ExitCode, String> {
    let mut smoke = false;
    let mut coverage = false;
    for a in args {
        match a.as_str() {
            "--smoke" => smoke = true,
            "--coverage" => coverage = true,
            "--help" | "-h" => {
                print_help();
                return Ok(ExitCode::SUCCESS);
            }
            other => return Err(format!("unknown flag `{other}`")),
        }
    }

    // Coverage is a pure convenience wrapper around `cargo llvm-cov`
    // (ADR-0020 §1.2). Runs independently of the diff harness so a
    // local developer can request it without having to stand up the
    // fact-oracle JSONs first. Mirrors the CI gate semantics in
    // `.github/workflows/coverage.yml`.
    if coverage {
        return run_coverage();
    }

    let repo_root = repo_root()?;
    let report_dir = repo_root.join(layout::REPORT_DIR);
    fs::create_dir_all(&report_dir)
        .map_err(|e| format!("cannot create {}: {e}", report_dir.display()))?;

    let plan = build_plan(&repo_root, smoke)?;
    let allowlist = load_allowlist(&repo_root.join(layout::ALLOWLIST));
    let mut summary = Summary {
        smoke: plan.smoke,
        ..Summary::default()
    };
    let mut gate = ExitGate::default();

    for lang in LANGUAGES {
        let outcome = verify_language(&repo_root, lang, &plan, &allowlist)?;
        let report_path = report_dir.join(format!("diff-report-{lang}.json"));
        let mut f = fs::File::create(&report_path)
            .map_err(|e| format!("cannot write {}: {e}", report_path.display()))?;
        f.write_all(diff_report_json(lang, &outcome).as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", report_path.display()))?;
        gate.push(lang, &outcome);
        summary.push(lang, &outcome);
    }

    let summary_path = report_dir.join("summary.json");
    fs::write(&summary_path, summary.to_json())
        .map_err(|e| format!("cannot write {}: {e}", summary_path.display()))?;

    let gate_path = report_dir.join("phase-a-exit-gate.json");
    fs::write(&gate_path, gate.to_json())
        .map_err(|e| format!("cannot write {}: {e}", gate_path.display()))?;

    // Human summary to stderr, matching the gate table in the module
    // docs so a reader can eyeball-check both.
    eprintln!(
        "xtask verify: {} language(s) checked, {} test(s) executed, \
         {} pass, {} divergence(s) ({} allow-listed), smoke={}",
        summary.languages_checked,
        gate.total_executed(),
        gate.total_pass(),
        gate.total_divergences(),
        gate.total_allowlisted(),
        summary.smoke,
    );
    for (lang, counts) in &gate.per_language {
        if counts.total == 0 {
            continue;
        }
        eprintln!(
            "xtask verify: [{lang}] total={} pass={} divergences={} allow-listed={}",
            counts.total, counts.pass, counts.divergences, counts.allowlisted,
        );
    }

    // Smoke mode: infrastructure is deliberately stubbed, exit 0
    // (smoke test integration in `crates/testing/rdf-diff/tests/xtask_verify.rs`).
    if summary.smoke {
        return Ok(ExitCode::SUCCESS);
    }

    // Fail-closed: zero executed tests on a non-smoke run means neither
    // vendored manifests nor main parsers produced output. The ADR-0019
    // "zero divergences is suspicious" posture is now sharpened: an
    // *empty* run is the only suspicious zero, because when the
    // harness actually runs we have per-entry pass/fail granularity.
    if gate.total_executed() == 0 {
        eprintln!(
            "xtask verify: ERROR — no manifest entries executed on a non-smoke run. \
             Vendor `external/tests/<lang>/manifest.ttl` or pass `--smoke`."
        );
        return Ok(ExitCode::from(1));
    }

    // Unacceptable failures include accept/reject splits that aren't on
    // the allow-list and any non-allow-listed divergence.
    if summary.had_unacceptable_failure {
        return Ok(ExitCode::from(1));
    }

    if gate.total_divergences() > gate.total_allowlisted() {
        // Some divergence was neither clean nor allow-listed.
        return Ok(ExitCode::from(1));
    }

    if gate.total_allowlisted() > 0 {
        eprintln!(
            "xtask verify: WARNING — {} divergence(s) allow-listed; \
             revisit the ALLOWLIST.md before the Phase-A exit gate closes",
            gate.total_allowlisted(),
        );
    }

    Ok(ExitCode::SUCCESS)
}

fn print_help() {
    println!(
        "xtask verify — verification-v1 PR-gate runner\n\
         \n\
         USAGE:\n    cargo run -p xtask -- verify [--smoke] [--coverage]\n\
         \n\
         FLAGS:\n\
         \x20   --smoke      Use external/fact-oracles/fixtures/smoke/ when the\n\
         \x20                vendored W3C suite is absent. Auto-enabled on a\n\
         \x20                fresh checkout (no external/tests/ tree).\n\
         \x20   --coverage   Shell out to `cargo llvm-cov` to produce an\n\
         \x20                `lcov.info` and per-crate line-coverage\n\
         \x20                summaries. Mirrors the CI gate in\n\
         \x20                .github/workflows/coverage.yml. Requires the\n\
         \x20                `cargo-llvm-cov` binary to be installed.\n\
         \x20   -h, --help\n\
         \n\
         Outputs are written to `target/verification-reports/` (diff\n\
         harness) and `lcov.info` / `target/llvm-cov` (coverage)."
    );
}

/// Per-crate coverage hard gates. Kept in lockstep with
/// `.github/workflows/coverage.yml` (the `Gate <pkg>` steps) and with
/// `docs/verification/coverage-baseline.md` (the `BASELINE` table).
///
/// The thresholds differ per crate — see the runbook for the rationale.
/// A single workspace-wide `--fail-under-lines N` would either under-
/// protect the small simple crates or over-protect the large grammar,
/// so we gate each crate individually.
const COVERAGE_HARD_GATES: &[(&str, u8)] = &[
    ("rdf-diagnostics", 80),
    ("rdf-iri", 70),
    ("rdf-ntriples", 75),
    ("rdf-turtle", 70),
];

/// Path (relative to the workspace root) of the drift-guard script used
/// by CI and mirrored here.
const COVERAGE_DRIFT_SCRIPT: &str = "scripts/coverage-drift.sh";

/// Run `cargo llvm-cov` locally — the `xtask verify --coverage`
/// convenience path. We intentionally shell out rather than try to drive
/// cargo-llvm-cov as a library: the binary is the contract, it prints
/// per-crate summaries to stdout, and the exit status is load-bearing.
///
/// On a fresh checkout where `cargo-llvm-cov` is not installed, we exit
/// with a clear install hint (ADR-0020 §1.2 acceptance).
fn run_coverage() -> Result<ExitCode, String> {
    if !cargo_llvm_cov_installed() {
        eprintln!(
            "xtask verify --coverage: `cargo-llvm-cov` is not installed.\n\
             \n\
             Install it with one of:\n\
             \x20   cargo install cargo-llvm-cov --locked\n\
             \x20   # or, in CI: use `taiki-e/install-action@v2` with `tool: cargo-llvm-cov`\n\
             \n\
             You also need the `llvm-tools-preview` rustup component:\n\
             \x20   rustup component add llvm-tools-preview\n\
             \n\
             See docs/runbooks/coverage.md for the full setup."
        );
        return Ok(ExitCode::from(1));
    }

    // 1. Workspace-wide lcov report. `--locked` keeps the result
    //    reproducible on a clean clone. We do not pass `--no-clean` so
    //    stale profdata from prior runs cannot leak in.
    eprintln!("xtask verify --coverage: running cargo llvm-cov --workspace ...");
    let status = Command::new("cargo")
        .args([
            "llvm-cov",
            "--workspace",
            "--all-features",
            "--locked",
            "--lcov",
            "--output-path",
            "lcov.info",
        ])
        .status()
        .map_err(|e| format!("failed to spawn `cargo llvm-cov`: {e}"))?;
    if !status.success() {
        eprintln!("xtask verify --coverage: `cargo llvm-cov` exited {status}");
        return Ok(ExitCode::from(1));
    }

    // 2. Per-crate hard gates. A failure of any one gate fails the
    //    whole wrapper — we keep going across the rest so the developer
    //    sees every failing crate in a single run rather than one at a
    //    time. Matches the CI workflow's step-by-step `cargo llvm-cov
    //    report --package <pkg> --fail-under-lines N` sequence, where N
    //    is per-crate (see COVERAGE_HARD_GATES).
    let mut any_gate_failed = false;
    for (pkg, threshold) in COVERAGE_HARD_GATES {
        eprintln!("xtask verify --coverage: gate {pkg} at {threshold}% line coverage");
        let status = Command::new("cargo")
            .args([
                "llvm-cov",
                "report",
                "--package",
                pkg,
                "--fail-under-lines",
                &threshold.to_string(),
            ])
            .status()
            .map_err(|e| format!("failed to spawn `cargo llvm-cov report`: {e}"))?;
        if !status.success() {
            eprintln!("xtask verify --coverage: {pkg} below {threshold}% line coverage");
            any_gate_failed = true;
        }
    }

    // 3. Drift guard. Compares the measured per-crate coverage against
    //    the committed baseline in docs/verification/coverage-baseline.md.
    //    Mirrors the CI step of the same name; a green local run
    //    predicts a green CI run.
    if let Ok(root) = repo_root() {
        let script = root.join(COVERAGE_DRIFT_SCRIPT);
        if script.is_file() {
            eprintln!("xtask verify --coverage: running drift guard");
            let status = Command::new("bash")
                .arg(&script)
                .current_dir(&root)
                .status()
                .map_err(|e| format!("failed to spawn drift-guard: {e}"))?;
            if !status.success() {
                eprintln!(
                    "xtask verify --coverage: drift guard reported regression(s) \
                     — see table above"
                );
                any_gate_failed = true;
            }
        } else {
            eprintln!(
                "xtask verify --coverage: drift-guard script {} absent; skipping drift check",
                script.display()
            );
        }
    }

    // 4. Warn-only workspace summary. Informational — never fails.
    //    `cargo llvm-cov report` (no `--package`, no `--workspace`)
    //    defaults to summarising every file in the profdata set;
    //    current cargo-llvm-cov rejects `--workspace` on `report`
    //    (accepted only on the top-level `llvm-cov` subcommand).
    eprintln!("xtask verify --coverage: workspace summary (warn-only)");
    let _ = Command::new("cargo")
        .args(["llvm-cov", "report"])
        .status();

    if any_gate_failed {
        Ok(ExitCode::from(1))
    } else {
        eprintln!("xtask verify --coverage: OK — lcov.info written");
        Ok(ExitCode::SUCCESS)
    }
}

/// Probe for `cargo-llvm-cov` by asking cargo to list its subcommand
/// help. We avoid `which`/env-PATH searches because cargo resolves the
/// binary via its own plugin lookup (looks for `cargo-llvm-cov` on
/// PATH). A `cargo llvm-cov --version` that exits non-zero means the
/// plugin is missing.
fn cargo_llvm_cov_installed() -> bool {
    Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// The subset of the environment a single `verify` invocation needs.
struct Plan {
    /// Whether we are running against the smoke fixture (because the
    /// vendored W3C suite is not present or `--smoke` was passed).
    /// Controls the exit policy: in smoke mode, "no manifests found" is
    /// not a failure.
    smoke: bool,
    /// Absolute path of the vendored W3C suite root, when it exists.
    /// `None` in smoke mode.
    vendored_root: Option<PathBuf>,
    /// Allow-list file was located on disk.
    allowlist_present: bool,
}

fn build_plan(root: &Path, smoke_flag: bool) -> Result<Plan, String> {
    let vendored = root.join(layout::VENDORED_SUITE_ROOT);
    let smoke_root = root.join(layout::SMOKE_ROOT);
    let smoke = smoke_flag || !vendored.is_dir();
    if smoke && !smoke_root.is_dir() {
        return Err(format!(
            "neither vendored suite at {} nor smoke fixture at {} exists",
            vendored.display(),
            smoke_root.display()
        ));
    }
    let allowlist_present = root.join(layout::ALLOWLIST).exists();
    Ok(Plan {
        smoke,
        vendored_root: if smoke { None } else { Some(vendored) },
        allowlist_present,
    })
}

/// Per-language outcome. Kept narrow so `Summary` and `ExitGate` can aggregate.
struct LangOutcome {
    /// Paths of corpora consulted (newest-first by mtime is fine — order
    /// is informational).
    corpora: Vec<PathBuf>,
    /// Paths of oracle JSONs consulted.
    oracles: Vec<PathBuf>,
    /// Paths of manifest.ttl files discovered and executed.
    manifests: Vec<PathBuf>,
    /// Aggregated manifest-runner summary. `total == 0` means no tests
    /// ran (either we're in smoke mode or the language has no
    /// main parser registered yet).
    summary: ManifestSummary,
    /// Count of divergences whose `(language, name)` was allow-listed.
    allowlisted: usize,
    /// Human-readable reason when no tests ran for this language.
    stub_reason: Option<String>,
    /// Whether this outcome should fail the gate.
    fail_gate: bool,
}

fn verify_language(
    root: &Path,
    lang: &str,
    plan: &Plan,
    allowlist: &Allowlist,
) -> Result<LangOutcome, String> {
    let corpora = discover_corpora(root, lang, plan.smoke)?;
    let oracles = discover_oracles(root, lang)?;

    // Smoke mode: don't drive manifests. The xtask remains a
    // discovery-only sweep so the `xtask_verify_smoke_corpus_green`
    // integration test stays cheap.
    let Some(vendored_root) = plan.vendored_root.as_deref() else {
        return Ok(LangOutcome {
            corpora,
            oracles,
            manifests: Vec::new(),
            summary: ManifestSummary::default(),
            allowlisted: 0,
            stub_reason: Some("smoke mode — W3C manifests not consulted".into()),
            fail_gate: false,
        });
    };

    // Languages without a main parser yet (rdfxml, sparql) run in
    // discovery-only mode. The report still ships so the gate sees
    // which formats are pending.
    if !language_has_main_parser(lang) {
        return Ok(LangOutcome {
            corpora,
            oracles,
            manifests: Vec::new(),
            summary: ManifestSummary::default(),
            allowlisted: 0,
            stub_reason: Some(format!(
                "no main parser registered for {lang}; pending Phase-B / Phase-C"
            )),
            fail_gate: false,
        });
    }

    let manifests = manifest::discover_manifests_for_language(vendored_root, lang);
    if manifests.is_empty() {
        return Ok(LangOutcome {
            corpora,
            oracles,
            manifests,
            summary: ManifestSummary::default(),
            allowlisted: 0,
            stub_reason: Some(format!(
                "no manifest.ttl found under {} for language {lang}",
                vendored_root.display()
            )),
            fail_gate: false,
        });
    }

    let mut aggregate = ManifestSummary::default();
    for m in &manifests {
        match manifest::run_manifest(m, lang) {
            Ok(s) => aggregate.extend(s),
            Err(e) => {
                eprintln!(
                    "xtask verify: WARNING — manifest {} failed to run: {e}",
                    m.display()
                );
            }
        }
    }

    // Tag divergences against the allow-list. We don't suppress them in
    // the per-language JSON — the report still shows every entry — but
    // we decrement the gate-critical count.
    let mut allowlisted = 0usize;
    for entry in &aggregate.entries {
        if !entry.pass && allowlist.permits(lang, &entry.name) {
            allowlisted += 1;
        }
    }

    // Fail the gate when *any* non-allow-listed divergence exists, OR
    // when a positive/negative-syntax test disagreed (those are
    // always load-bearing). allow-listed positive/negative splits are
    // still permitted per the gate matrix (the ALLOWLIST is the
    // only exception knob).
    let hard_failures = aggregate
        .entries
        .iter()
        .filter(|e| !e.pass && !allowlist.permits(lang, &e.name))
        .count();
    let fail_gate = hard_failures > 0;

    Ok(LangOutcome {
        corpora,
        oracles,
        manifests,
        summary: aggregate,
        allowlisted,
        stub_reason: if plan.allowlist_present {
            None
        } else {
            Some("ALLOWLIST.md absent — divergences cannot be silenced".into())
        },
        fail_gate,
    })
}

/// Map language tag → whether we have a main parser wired. Discovery
/// for the other languages runs but they never contribute test
/// executions.
fn language_has_main_parser(lang: &str) -> bool {
    matches!(lang, "nt" | "nq" | "ttl" | "trig")
}

fn discover_corpora(root: &Path, lang: &str, smoke: bool) -> Result<Vec<PathBuf>, String> {
    let base = if smoke {
        root.join(layout::SMOKE_ROOT).join(lang)
    } else {
        root.join(layout::VENDORED_SUITE_ROOT).join(lang)
    };
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    walk_files(&base)
}

fn discover_oracles(root: &Path, lang: &str) -> Result<Vec<PathBuf>, String> {
    let base = root.join(layout::ORACLE_ROOT).join(lang);
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&base).map_err(|e| format!("cannot read {}: {e}", base.display()))? {
        let entry = entry.map_err(|e| format!("read_dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) == Some("json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Depth-first walk collecting regular files. Symlinks not followed.
fn walk_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let rd = fs::read_dir(&dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
        for entry in rd {
            let entry = entry.map_err(|e| format!("read_dir entry: {e}"))?;
            let ft = entry.file_type().map_err(|e| format!("file_type: {e}"))?;
            let path = entry.path();
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Locate the repository root by walking up until we see a `Cargo.toml`
/// that is a workspace (contains `[workspace]`). Keeps the xtask
/// runnable from any subdirectory without a CLI flag.
fn repo_root() -> Result<PathBuf, String> {
    let start = env::current_dir().map_err(|e| format!("getcwd: {e}"))?;
    let mut cur: &Path = &start;
    loop {
        let candidate = cur.join("Cargo.toml");
        if candidate.is_file() {
            // Read a small prefix and look for `[workspace]`. Avoids
            // grabbing a leaf crate's Cargo.toml.
            if let Ok(s) = fs::read_to_string(&candidate)
                && s.contains("[workspace]")
            {
                return Ok(cur.to_path_buf());
            }
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => {
                return Err(format!(
                    "no workspace Cargo.toml found above {}",
                    start.display()
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------
// Allow-list parsing. Intentionally minimal — the ALLOWLIST.md schema
// is described at crates/testing/rdf-diff/ALLOWLIST.md. Each active
// entry is `<language>:<test-name>` on its own line, case-sensitive.
// ---------------------------------------------------------------------

/// Parsed allow-list entries keyed by `(language, test-name)`. A hit on
/// this set means a divergence is intentional and does not fail the
/// gate.
#[derive(Debug, Default)]
struct Allowlist {
    entries: BTreeSet<(String, String)>,
}

impl Allowlist {
    fn permits(&self, lang: &str, test_name: &str) -> bool {
        self.entries
            .contains(&(lang.to_owned(), test_name.to_owned()))
    }
}

fn load_allowlist(path: &Path) -> Allowlist {
    let Ok(contents) = fs::read_to_string(path) else {
        return Allowlist::default();
    };
    let mut out = Allowlist::default();
    let mut in_fence = false;
    for raw in contents.lines() {
        let line = raw.trim();
        if line.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((language, rest)) = line.split_once(':') else {
            continue;
        };
        // Strip an optional trailing ":<variant>" hint — the allow-list
        // key is (language, test-name) only; the variant is documented
        // for humans.
        let name = rest.split(':').next().unwrap_or(rest).trim();
        let language = language.trim();
        if language.is_empty() || name.is_empty() {
            continue;
        }
        out.entries.insert((language.to_owned(), name.to_owned()));
    }
    out
}

// ---------------------------------------------------------------------
// Minimal JSON emission. We deliberately avoid `serde_json` to keep the
// xtask dependency graph empty except for `rdf-diff`. All writers below
// handle only the shapes defined in this file.
// ---------------------------------------------------------------------

fn diff_report_json(lang: &str, o: &LangOutcome) -> String {
    let mut s = String::new();
    s.push('{');
    push_kv(&mut s, "language", &json_string(lang), true);
    push_kv(&mut s, "corpora_count", &o.corpora.len().to_string(), false);
    push_kv(&mut s, "oracles_count", &o.oracles.len().to_string(), false);
    push_kv(
        &mut s,
        "manifests_count",
        &o.manifests.len().to_string(),
        false,
    );
    push_kv(&mut s, "tests_executed", &o.summary.total.to_string(), false);
    push_kv(&mut s, "pass", &o.summary.pass.to_string(), false);
    push_kv(
        &mut s,
        "divergences",
        &o.summary.divergences.to_string(),
        false,
    );
    push_kv(&mut s, "skipped", &o.summary.skipped.to_string(), false);
    push_kv(&mut s, "allowlisted", &o.allowlisted.to_string(), false);
    push_kv(&mut s, "fail_gate", bool_lit(o.fail_gate), false);
    push_kv(
        &mut s,
        "clean",
        bool_lit(o.summary.divergences == 0),
        false,
    );
    if let Some(reason) = &o.stub_reason {
        push_kv(&mut s, "stub_reason", &json_string(reason), false);
    }
    s.push_str(",\"entries\":");
    s.push_str(&entries_json(&o.summary.entries));
    s.push_str(",\"manifests\":");
    s.push_str(&json_path_array(&o.manifests));
    s.push_str(",\"corpora\":");
    s.push_str(&json_path_array(&o.corpora));
    s.push_str(",\"oracles\":");
    s.push_str(&json_path_array(&o.oracles));
    s.push('}');
    s
}

fn entries_json(entries: &[EntryOutcome]) -> String {
    let mut s = String::from("[");
    for (i, e) in entries.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('{');
        push_kv(&mut s, "name", &json_string(&e.name), true);
        push_kv(&mut s, "kind", &json_string(kind_label(e.kind)), false);
        push_kv(&mut s, "pass", bool_lit(e.pass), false);
        if let Some(variant) = e.divergence {
            push_kv(&mut s, "divergence", &json_string(variant), false);
        }
        push_kv(&mut s, "message", &json_string(&e.message), false);
        s.push('}');
    }
    s.push(']');
    s
}

const fn kind_label(kind: TestKind) -> &'static str {
    match kind {
        TestKind::PositiveSyntax => "PositiveSyntax",
        TestKind::NegativeSyntax => "NegativeSyntax",
        TestKind::Eval => "Eval",
        TestKind::NegativeEval => "NegativeEval",
    }
}

fn json_path_array(paths: &[PathBuf]) -> String {
    let mut s = String::from("[");
    for (i, p) in paths.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&json_string(&p.display().to_string()));
    }
    s.push(']');
    s
}

fn push_kv(s: &mut String, key: &str, value: &str, first: bool) {
    if !first {
        s.push(',');
    }
    s.push_str(&json_string(key));
    s.push(':');
    s.push_str(value);
}

const fn bool_lit(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

fn json_string(input: &str) -> String {
    let mut s = String::with_capacity(input.len() + 2);
    s.push('"');
    for c in input.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                // Unwrap: write! to a String cannot fail.
                let _ = write!(s, "\\u{:04x}", c as u32);
            }
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

#[derive(Default)]
struct Summary {
    smoke: bool,
    languages_checked: usize,
    total_divergences: usize,
    had_unacceptable_failure: bool,
    per_language: Vec<(String, usize, bool, Option<String>)>,
}

impl Summary {
    fn push(&mut self, lang: &str, o: &LangOutcome) {
        self.languages_checked += 1;
        self.total_divergences += o.summary.divergences;
        if o.fail_gate {
            self.had_unacceptable_failure = true;
        }
        self.per_language.push((
            lang.to_owned(),
            o.summary.divergences,
            o.fail_gate,
            o.stub_reason.clone(),
        ));
    }

    fn to_json(&self) -> String {
        let mut s = String::new();
        s.push('{');
        push_kv(&mut s, "smoke", bool_lit(self.smoke), true);
        push_kv(
            &mut s,
            "languages_checked",
            &self.languages_checked.to_string(),
            false,
        );
        push_kv(
            &mut s,
            "total_divergences",
            &self.total_divergences.to_string(),
            false,
        );
        push_kv(
            &mut s,
            "had_unacceptable_failure",
            bool_lit(self.had_unacceptable_failure),
            false,
        );
        s.push_str(",\"per_language\":[");
        for (i, (lang, div, fail, stub)) in self.per_language.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push('{');
            push_kv(&mut s, "language", &json_string(lang), true);
            push_kv(&mut s, "divergences", &div.to_string(), false);
            push_kv(&mut s, "fail_gate", bool_lit(*fail), false);
            if let Some(reason) = stub {
                push_kv(&mut s, "stub_reason", &json_string(reason), false);
            }
            s.push('}');
        }
        s.push_str("]}");
        s
    }
}

/// Per-language count bucket for the Phase-A exit-gate report.
#[derive(Default, Clone)]
struct GateCounts {
    total: usize,
    pass: usize,
    divergences: usize,
    allowlisted: usize,
}

/// Phase-A exit-gate aggregator. Serialises to
/// `target/verification-reports/phase-a-exit-gate.json` so CI can gate
/// on per-language pass rates without re-parsing every per-format
/// report.
#[derive(Default)]
struct ExitGate {
    per_language: Vec<(String, GateCounts)>,
}

impl ExitGate {
    fn push(&mut self, lang: &str, o: &LangOutcome) {
        self.per_language.push((
            lang.to_owned(),
            GateCounts {
                total: o.summary.total,
                pass: o.summary.pass,
                divergences: o.summary.divergences,
                allowlisted: o.allowlisted,
            },
        ));
    }

    fn total_executed(&self) -> usize {
        self.per_language.iter().map(|(_, c)| c.total).sum()
    }

    fn total_pass(&self) -> usize {
        self.per_language.iter().map(|(_, c)| c.pass).sum()
    }

    fn total_divergences(&self) -> usize {
        self.per_language.iter().map(|(_, c)| c.divergences).sum()
    }

    fn total_allowlisted(&self) -> usize {
        self.per_language.iter().map(|(_, c)| c.allowlisted).sum()
    }

    fn to_json(&self) -> String {
        let mut s = String::new();
        s.push('{');
        let mut first = true;
        for (lang, counts) in &self.per_language {
            if !first {
                s.push(',');
            }
            first = false;
            s.push_str(&json_string(lang));
            s.push_str(":{");
            push_kv(&mut s, "total", &counts.total.to_string(), true);
            push_kv(&mut s, "pass", &counts.pass.to_string(), false);
            push_kv(
                &mut s,
                "divergences",
                &counts.divergences.to_string(),
                false,
            );
            push_kv(
                &mut s,
                "allowlisted",
                &counts.allowlisted.to_string(),
                false,
            );
            s.push('}');
        }
        s.push('}');
        s
    }
}
