//! Integration test: exercise `xtask verify` on the smoke corpus.
//!
//! Tracked as invariant `I1` in `docs/verification/tests/catalogue.md`.
//!
//! `xtask verify` is claimed by `v1-ci-wiring` and does not exist on
//! disk yet. This test locates the workspace root and invokes
//! `cargo xtask verify --smoke`; when the binary is missing the test
//! `return`s cleanly so `cargo test --workspace` stays green before the
//! CI crate lands. Once `v1-ci-wiring` completes, the `#[ignore]` comes
//! off the strict assertion path.
//!
//! ADR references: ADR-0019 §2 (harness scope), ADR-0020 §5 (integration
//! pass order).

#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` for this crate is .../crates/testing/rdf-diff.
    // Walk up three levels to the workspace root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .ancestors()
        .nth(3)
        .expect("workspace root three levels above rdf-diff manifest")
        .to_path_buf()
}

fn xtask_crate_exists(root: &Path) -> bool {
    root.join("xtask").join("Cargo.toml").exists()
}

/// **I1 — `cargo xtask verify --smoke` exits zero on the smoke corpus.**
///
/// Skips silently until `v1-ci-wiring` lands the `xtask` crate. When it
/// does, remove the early-return and the test becomes a hard gate.
#[test]
#[ignore = "unignore once v1-ci-wiring lands xtask/verify"]
fn xtask_verify_smoke_corpus_green() {
    let root = workspace_root();
    if !xtask_crate_exists(&root) {
        eprintln!("xtask crate not present yet; skipping I1");
        return;
    }

    let status = Command::new(env!("CARGO"))
        .current_dir(&root)
        .args(["run", "--quiet", "-p", "xtask", "--", "verify", "--smoke"])
        .status()
        .expect("spawn cargo xtask verify");

    assert!(
        status.success(),
        "xtask verify --smoke exited non-zero: {status:?}"
    );
}

/// Always-on cheap sanity: the workspace root resolves and the
/// verification catalogue is present. Prevents silent drift of the path
/// constants above.
#[test]
fn catalogue_is_discoverable_from_workspace_root() {
    let root = workspace_root();
    let cat = root
        .join("docs")
        .join("verification")
        .join("tests")
        .join("catalogue.md");
    assert!(
        cat.exists(),
        "docs/verification/tests/catalogue.md missing at {}",
        cat.display()
    );
}
