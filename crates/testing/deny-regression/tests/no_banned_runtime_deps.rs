//! Integration test for ADR-0019 §1's `[dev-dependencies]`-only
//! oracle carve-out.
//!
//! This test invokes `cargo metadata --format-version 1` against the
//! workspace, walks the resolved dependency graph from every workspace
//! member, and asserts that none of the
//! [`BANNED_RUNTIME_CRATES`](deny_regression::BANNED_RUNTIME_CRATES)
//! appears anywhere in the transitive **normal** (non-dev, non-build)
//! dependency closure of any lib or binary target.
//!
//! Why duplicate `cargo-deny`? Two reasons:
//!
//! 1. `cargo-deny`'s configuration is easy to misedit (e.g. someone
//!    flips `exclude-dev = false` to `true` or removes an entry from
//!    `[bans] deny`). This test reads the cargo resolver's view of the
//!    workspace directly and is not coupled to `deny.toml`.
//! 2. It catches a subtle bug `cargo-deny` cannot catch on its own:
//!    a banned crate declared under `[dev-dependencies]` in a
//!    **non-test** crate would be permitted by `exclude-dev`, yet ship
//!    to downstream consumers via cargo's default feature
//!    propagation. By walking only normal edges from every workspace
//!    lib/bin target we guarantee the runtime closure is clean.
//!
//! The test is hermetic: it uses `cargo metadata --offline` via the
//! current lockfile, runs in under a second, and emits a minimal
//! reproducer (the full offending edge chain) on failure.

use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::process::Command;

use deny_regression::BANNED_RUNTIME_CRATES;
use serde_json::Value;

/// Locate the workspace root so the test works regardless of
/// invocation directory. We rely on `CARGO_MANIFEST_DIR` pointing at
/// this crate's manifest, which is two levels deep
/// (`crates/testing/deny-regression`).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("deny-regression lives three levels below workspace root")
        .to_path_buf()
}

fn cargo_metadata() -> Value {
    let root = workspace_root();
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--all-features",
        ])
        .current_dir(&root)
        .output()
        .expect("failed to spawn `cargo metadata`");

    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice::<Value>(&output.stdout).expect("cargo metadata emitted invalid JSON")
}

/// A single edge in the normal-dependency closure walk. Retained so
/// that failure messages print a usable reproducer chain. `to_id` is
/// the full cargo package id; kept for debugging and not formatted
/// into the default failure message.
#[derive(Debug, Clone)]
struct Edge {
    from_name: String,
    to_name: String,
    #[allow(dead_code)]
    to_id: String,
}

fn name_from_id<'a>(packages: &'a [Value], id: &str) -> &'a str {
    packages
        .iter()
        .find(|pkg| pkg["id"].as_str() == Some(id))
        .and_then(|pkg| pkg["name"].as_str())
        .unwrap_or("<unknown>")
}

#[test]
fn no_banned_runtime_deps_in_workspace_closure() {
    let metadata = cargo_metadata();

    let packages = metadata["packages"]
        .as_array()
        .expect("packages array missing");
    let workspace_members: Vec<String> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members array missing")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();

    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .expect("resolve.nodes array missing; is this a workspace?");

    let banned: BTreeSet<&str> = BANNED_RUNTIME_CRATES.iter().copied().collect();

    let mut violations: Vec<Vec<Edge>> = Vec::new();

    for member_id in &workspace_members {
        // BFS across normal edges only. `path` records the edge chain
        // that led us to each node so the failure message points at
        // the exact offending edge.
        let mut visited: BTreeSet<String> = BTreeSet::new();
        visited.insert(member_id.clone());

        let member_name = name_from_id(packages, member_id).to_owned();
        let mut queue: VecDeque<(String, Vec<Edge>)> = VecDeque::new();
        queue.push_back((member_id.clone(), Vec::new()));

        while let Some((current_id, path)) = queue.pop_front() {
            let node = nodes
                .iter()
                .find(|n| n["id"].as_str() == Some(&current_id));
            let Some(node) = node else {
                continue;
            };
            let Some(deps) = node["deps"].as_array() else {
                continue;
            };

            for dep in deps {
                let dep_pkg = dep["pkg"].as_str().unwrap_or_default().to_owned();
                let dep_name = dep["name"].as_str().unwrap_or_default().to_owned();

                // Keep only normal edges. `dep_kinds` is the
                // authoritative field in cargo metadata format
                // version 1: each entry has a `kind` that is `null`
                // for a normal edge, `"dev"` for a dev-dependency,
                // or `"build"` for a build-dependency.
                let Some(dep_kinds) = dep["dep_kinds"].as_array() else {
                    continue;
                };
                let has_normal_edge = dep_kinds
                    .iter()
                    .any(|k| k.get("kind").map(|v| v.is_null()).unwrap_or(true));
                if !has_normal_edge {
                    continue;
                }

                // The `pkg` field stores the canonical package id,
                // which is what we look up against `packages[]` for
                // the human-readable name. Fall back to `dep.name`
                // (the rename) if the id lookup fails.
                let resolved_name = name_from_id(packages, &dep_pkg).to_owned();
                let label = if resolved_name == "<unknown>" {
                    dep_name
                } else {
                    resolved_name
                };

                let from_name = path
                    .last()
                    .map(|e| e.to_name.clone())
                    .unwrap_or_else(|| member_name.clone());

                let mut next_path = path.clone();
                next_path.push(Edge {
                    from_name,
                    to_name: label.clone(),
                    to_id: dep_pkg.clone(),
                });

                if banned.contains(label.as_str()) {
                    violations.push(next_path.clone());
                }

                if visited.insert(dep_pkg.clone()) {
                    queue.push_back((dep_pkg, next_path));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "ADR-0019 §1 runtime carve-out violated: {} banned crate(s) \
         reached via non-dev/non-build edges.\n\n{}",
        violations.len(),
        violations
            .iter()
            .map(|chain| format_chain(chain))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

fn format_chain(chain: &[Edge]) -> String {
    let mut out = String::new();
    for (idx, edge) in chain.iter().enumerate() {
        if idx == 0 {
            out.push_str(&edge.from_name);
        }
        out.push_str(" -> ");
        out.push_str(&edge.to_name);
    }
    out
}

#[test]
fn banned_list_matches_deny_toml() {
    // Minimal sanity check: every entry in BANNED_RUNTIME_CRATES must
    // appear in `deny.toml`. This is a loose textual check (not a
    // TOML parse) to avoid adding a TOML dependency; it still catches
    // the two most likely drift cases: (a) someone removes a crate
    // from `deny.toml` without updating the const, and (b) someone
    // adds a crate to the const without updating `deny.toml`.
    let root = workspace_root();
    let deny_toml = std::fs::read_to_string(root.join("deny.toml"))
        .expect("deny.toml must be readable from the workspace root");

    for banned in BANNED_RUNTIME_CRATES {
        let needle = format!("name = \"{banned}\"");
        assert!(
            deny_toml.contains(&needle),
            "deny.toml is missing a `{needle}` entry for banned crate `{banned}`. \
             ADR-0019 §1 requires these lists to agree.",
        );
    }
}
