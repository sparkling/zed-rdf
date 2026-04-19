//! `cargo run -p xtask -- verify` — the PR-gate entry point for the
//! verification-v1 sweep.
//!
//! Behaviour (ADR-0020 §3, `v1-ci-wiring`):
//!
//! 1. Discover format corpora — W3C manifests and edge-case inputs under
//!    `external/tests/<lang>/**` plus the smoke-fixture fallback under
//!    `external/fact-oracles/fixtures/smoke/<lang>/**` when the vendored
//!    suite is not present.
//! 2. Discover oracle JSON fact corpora under `external/fact-oracles/<lang>/*.json`
//!    (materialised out-of-process by the `fact-oracles.yml` workflow —
//!    **no JVM runs here**; JSON is consumed as data).
//! 3. Resolve the main + shadow Rust parsers registered in the
//!    `crates/testing/rdf-diff-oracles` crate (when landed by
//!    `v1-oracle-rust`) and run the diff harness from
//!    `rdf-diff::diff_many`. Until that crate lands, `xtask` runs in
//!    *harness-stub* mode: it still enumerates corpora and oracles, emits
//!    per-format `DiffReport`s, and exits 0 on a smoke fixture so the PR
//!    gate on `main` is green during the v1 sweep's own bootstrap.
//! 4. Emit one `DiffReport` JSON per format + a `summary.json` under
//!    `target/verification-reports/`. The workflow uploads this tree as a
//!    build artifact on failure (`.github/workflows/verification.yml`).
//! 5. Exit non-zero on any non-allow-listed divergence. Allow-list file
//!    path: `crates/testing/rdf-diff/ALLOWLIST.md` (ADR-0019 §2).
//!
//! Phase-A main-parser note (`phaseA-tester`, ADR-0017 §4): the main
//! parsers (`rdf-iri::Iri` / `IriParser`, `rdf-ntriples::NTriplesParser`
//! / `NQuadsParser`, `rdf-turtle::TurtleParser` / `TriGParser`) are
//! expected to be registered by `rdf-diff-oracles` alongside the shadow
//! crates and the JSON-oracle adapters. Each is still added behind a
//! separate feature flag so that any one crate's landing can be
//! integrated without blocking the others. Until all three land and
//! `rdf-diff-oracles` declares path-deps on them, `verify_language`
//! continues to run in stub mode for every language. ADR-0019
//! §Validation reminds us that zero divergences on Phase-A inputs is
//! *suspicious*; the stub-mode clean report is acceptable only because
//! `stub_reason` is emitted alongside it — a downstream reader can
//! distinguish "ran clean" from "did not run".
//!
//! Deliberate non-features:
//!
//! - No JVM invocation, ever. This binary only reads JSON that the
//!   scheduled `fact-oracles.yml` materialised.
//! - No `serde`/`serde_json` dependency — we serialise summary/report
//!   JSON by hand so the xtask graph stays minimal and does not risk
//!   pulling any banned crate transitively (ADR-0019 §1, `deny.toml`).
//! - No dependency on shadow crates at compile time: we load them via
//!   the frozen `rdf-diff::Parser` trait through the `rdf-diff-oracles`
//!   registry.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// pedantic lint carve-outs kept narrow to the bits this single-file
// driver genuinely trips on.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

// NB: during the v1 bootstrap the xtask does **not** take a path
// dependency on `rdf-diff` — see this crate's Cargo.toml for the
// rationale. The shapes below are a local, read-only mirror of the
// subset of `rdf-diff`'s public API that the gate summary surfaces.
// When `v1-diff-core` lands the real diff, the integration-pass patch
// replaces these with `use rdf_diff::{DiffReport, Divergence};` and
// drops the shim.

/// Local mirror of `rdf_diff::DiffReport`. Structural equivalence only —
/// must be kept in sync with `crates/testing/rdf-diff/src/lib.rs` until
/// xtask takes the path-dep in ADR-0020 §5's integration pass.
#[derive(Default)]
struct DiffReport {
    divergences: Vec<Divergence>,
    triage_hint: String,
}

impl DiffReport {
    const fn is_clean(&self) -> bool {
        self.divergences.is_empty()
    }
}

/// Local mirror of `rdf_diff::Divergence`. Only the variant *names* are
/// surfaced in the report JSON; the full payload lands with the harness
/// integration in §5. Kept `#[allow(dead_code)]` because stub-mode never
/// constructs any variant.
#[allow(dead_code)]
enum Divergence {
    FactOnlyIn,
    ObjectMismatch,
    AcceptRejectSplit,
}

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
    let mut summary = Summary {
        smoke: plan.smoke,
        ..Summary::default()
    };

    for lang in LANGUAGES {
        let outcome = verify_language(&repo_root, lang, &plan)?;
        let report_path = report_dir.join(format!("diff-report-{lang}.json"));
        let mut f = fs::File::create(&report_path)
            .map_err(|e| format!("cannot write {}: {e}", report_path.display()))?;
        f.write_all(diff_report_json(lang, &outcome).as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", report_path.display()))?;
        summary.push(lang, &outcome);
    }

    let summary_path = report_dir.join("summary.json");
    fs::write(&summary_path, summary.to_json())
        .map_err(|e| format!("cannot write {}: {e}", summary_path.display()))?;

    eprintln!(
        "xtask verify: {} language(s) checked, {} divergence(s), smoke={}",
        summary.languages_checked, summary.total_divergences, summary.smoke
    );

    if summary.had_unacceptable_failure {
        return Ok(ExitCode::from(1));
    }

    // ADR-0019 §Validation fail-closed: zero divergences on a real
    // (non-smoke) run is *suspicious*, not success. Zero happens when
    // either (a) the main parsers + shadows + oracles are not all
    // wired, or (b) the fixture corpus is empty. Either way, declaring
    // green would let a broken harness claim the green flag. In smoke
    // mode zero is fine because the infrastructure is deliberately
    // stubbed.
    if !summary.smoke && summary.total_divergences == 0 {
        eprintln!(
            "xtask verify: ERROR — zero divergences on a non-smoke run; \
             per ADR-0019 §Validation this is suspicious, not success. \
             Check that main parsers + shadows + oracles are wired and \
             the fixture corpus is non-empty."
        );
        return Ok(ExitCode::from(1));
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
    /// vendored W3C suite is not present). Controls the exit policy:
    /// in smoke mode, "harness not yet wired" is not a failure.
    smoke: bool,
    /// Whether the Rust-side oracle registry (`rdf-diff-oracles`) is
    /// available. Until `v1-oracle-rust` lands, this is `false` and the
    /// harness runs in stub mode.
    rust_oracles_available: bool,
    /// Allow-list file was located (not parsed — divergence allow-list
    /// shape is owned by `v1-diff-core`).
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
    let rust_oracles_available = root
        .join("crates/testing/rdf-diff-oracles/src/lib.rs")
        .exists();
    let allowlist_present = root.join(layout::ALLOWLIST).exists();
    Ok(Plan {
        smoke,
        rust_oracles_available,
        allowlist_present,
    })
}

/// Per-language outcome. Kept narrow so `Summary` can aggregate.
struct LangOutcome {
    /// Paths of corpora consulted (newest-first by mtime is fine — order
    /// is informational).
    corpora: Vec<PathBuf>,
    /// Paths of oracle JSONs consulted.
    oracles: Vec<PathBuf>,
    /// The `DiffReport` produced by the harness. `None` when the harness
    /// is not yet wired.
    report: Option<DiffReport>,
    /// Human-readable reason when the report is `None`.
    stub_reason: Option<String>,
    /// Whether this outcome should fail the gate.
    fail_gate: bool,
}

fn verify_language(root: &Path, lang: &str, plan: &Plan) -> Result<LangOutcome, String> {
    let corpora = discover_corpora(root, lang, plan.smoke)?;
    let oracles = discover_oracles(root, lang)?;

    // Without the Rust oracle registry we cannot run the real diff. The
    // `Parser` trait is frozen in `rdf-diff` but its implementers live
    // in `rdf-diff-oracles` + the shadow crates — both landed by sibling
    // agents in the same sweep. Emit an advisory report until then.
    if !plan.rust_oracles_available {
        return Ok(LangOutcome {
            corpora,
            oracles,
            report: None,
            stub_reason: Some(
                "rdf-diff-oracles registry not yet present; harness in stub mode".into(),
            ),
            // In smoke mode this is expected bootstrap; in non-smoke it
            // is also expected during the v1 sweep bootstrap because the
            // registry-landing agent runs concurrently. The sweep's
            // *integration pass* (ADR-0020 §5) flips this to real.
            fail_gate: false,
        });
    }

    // The `rdf-diff-oracles` crate exists but we deliberately don't
    // link it here yet — see `xtask/verify/Cargo.toml`'s dependency
    // stanza for why. The integration-pass patch in ADR-0020 §5 swaps
    // the body below for a real call into `rdf_diff::diff_many` driven
    // by the registry. For now we surface a transparent stub that is
    // never clean-by-mistake: we emit an empty `DiffReport` but tag
    // the outcome with a `stub_reason` so the summary JSON (and the
    // reviewer) can see the gate is not yet load-bearing.
    // `XTASK_VERIFY_FAIL=1` forces an injected divergence. Used only to
    // validate the gate's failure path end-to-end during the v1 sweep
    // ("deliberately-broken shadow" acceptance, ADR-0020 §Acceptance)
    // before the real harness is wired. Never set in production.
    let force_fail = env::var_os("XTASK_VERIFY_FAIL").is_some();
    let mut report = DiffReport::default();
    if force_fail {
        report.divergences.push(Divergence::AcceptRejectSplit);
        report.triage_hint =
            "XTASK_VERIFY_FAIL=1 set — injected divergence for gate-wiring acceptance test"
                .to_string();
    }
    let stub_reason = if force_fail {
        Some("injected failure via XTASK_VERIFY_FAIL=1".to_string())
    } else {
        Some(
            "rdf-diff-oracles present but xtask path-dep deferred to ADR-0020 §5 integration pass"
                .to_string(),
        )
    };
    let fail_gate = !report.is_clean() && !allowlisted_equivalent(plan, &report);

    Ok(LangOutcome {
        corpora,
        oracles,
        report: Some(report),
        stub_reason,
        fail_gate,
    })
}

/// Placeholder allow-list predicate. The real one is owned by
/// `v1-diff-core` + the `ALLOWLIST.md` format; for the PR gate we accept
/// an empty divergence list only.
const fn allowlisted_equivalent(plan: &Plan, report: &DiffReport) -> bool {
    plan.allowlist_present && report.is_clean()
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
    for entry in fs::read_dir(&base)
        .map_err(|e| format!("cannot read {}: {e}", base.display()))?
    {
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
            let ft = entry
                .file_type()
                .map_err(|e| format!("file_type: {e}"))?;
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
    push_kv(&mut s, "fail_gate", bool_lit(o.fail_gate), false);
    if let Some(reason) = &o.stub_reason {
        push_kv(&mut s, "stub_reason", &json_string(reason), false);
    }
    if let Some(report) = &o.report {
        s.push_str(",\"divergences\":");
        s.push_str(&diff_divergences_json(report));
        push_kv(
            &mut s,
            "triage_hint",
            &json_string(&report.triage_hint),
            false,
        );
        push_kv(&mut s, "clean", bool_lit(report.is_clean()), false);
    } else {
        s.push_str(",\"divergences\":[]");
        push_kv(&mut s, "clean", bool_lit(true), false);
    }
    s.push_str(",\"corpora\":");
    s.push_str(&json_path_array(&o.corpora));
    s.push_str(",\"oracles\":");
    s.push_str(&json_path_array(&o.oracles));
    s.push('}');
    s
}

fn diff_divergences_json(report: &DiffReport) -> String {
    // We only surface *count* and *variant names* here — the rich shape
    // belongs to `v1-diff-core` and is rendered by the harness-side
    // report emitter once the real diff runs. This keeps xtask decoupled
    // from the internal enum shape.
    let mut s = String::from("[");
    for (i, d) in report.divergences.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let variant = match d {
            Divergence::FactOnlyIn => "FactOnlyIn",
            Divergence::ObjectMismatch => "ObjectMismatch",
            Divergence::AcceptRejectSplit => "AcceptRejectSplit",
        };
        s.push_str("{\"variant\":");
        s.push_str(&json_string(variant));
        s.push('}');
    }
    s.push(']');
    s
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
        let divergences = o.report.as_ref().map_or(0, |r| r.divergences.len());
        self.total_divergences += divergences;
        if o.fail_gate {
            self.had_unacceptable_failure = true;
        }
        self.per_language.push((
            lang.to_string(),
            divergences,
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

