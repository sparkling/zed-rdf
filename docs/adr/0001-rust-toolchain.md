# ADR-0001: Rust edition, toolchain, and MSRV (no MSRV commitment)

- **Status:** Accepted (2026-04-18)
- **Date:** 2026-04-18
- **Deciders:** Henrik Pettersen
- **Supersedes:** earlier draft of this ADR proposing an N-2 stable, 6-month-bump MSRV policy
- **Tags:** `policy`, `toolchain`

## Context and Problem Statement

We ship a Rust LSP, several parser crates, and a Zed extension targeting
`wasm32-wasip2`. We need an unambiguous written policy for:

- Rust edition,
- release channel,
- whether to commit to a minimum supported Rust version (MSRV),
- unsafe code policy.

Without a policy, contributors land code that only compiles on a very
new toolchain (bad for users on older Rust), and we pay an ongoing cost
to track and enforce an MSRV.

## Decision Drivers

- **Process minimalism.** A tiny team prefers fewer automated gates to
  maintain.
- **Access to recent Rust features.** Edition 2024, `let … else`,
  stable GATs, etc. matter for the parser and iterator code we write.
- **Compatibility with `zed_extension_api`.** The Zed extension API
  tracks recent Rust; pinning old MSRVs would fight it.
- **Nothing running in production at scale yet.** Downstream
  compatibility concerns are theoretical until the extension is live.

## Considered Options

1. **Latest stable, no MSRV commitment.** Build on whatever stable is
   current at release. No `cargo-msrv` gate. Simplest.
2. **N-2 stable, six-month cadence.** MSRV is "current stable minus
   two releases" and re-computed twice a year. Scheduled, visible.
3. **Pinned MSRV bumped only when a dependency forces it.** Cheap but
   unpredictable.
4. **Nightly.** Access to every feature; sacrifices every downstream
   user.

## Decision

**Chosen option: Option 1 — latest stable, no MSRV commitment.**

- **Edition**: Rust 2024 across all crates.
- **Channel**: **stable** only. Beta and nightly CI jobs exist and are
  **informational** (non-gating), to catch upstream breakage early.
- **MSRV**: **no commitment.** Each release builds on whatever stable
  is current at release time. We do not run a `cargo-msrv` gate.
  - The `rust-version` field in `Cargo.toml` is **informational only**;
    we keep it roughly current and bump it freely without ceremony.
  - Release notes record the Rust version used to build the release.
  - If downstream users later surface MSRV complaints, revisit via a
    new ADR that supersedes this one.
- **Toolchain file**: `rust-toolchain.toml` pins `channel = "stable"`
  for contributors and installs the `wasm32-wasip2` target.
- **Unsafe policy**: `#![forbid(unsafe_code)]` is the workspace default
  (set via `[workspace.lints.rust] unsafe_code = "forbid"`). A crate
  that legitimately needs `unsafe` must:
  - document why in an ADR amendment,
  - annotate every `unsafe` block with a `SAFETY:` comment,
  - run `miri` on the `unsafe` modules in CI.

  There are currently **no allow-listed `unsafe` crates**. The list
  grows only by ADR.

## Consequences

- **Positive**
  - Minimal process overhead — no scheduled MSRV bumps, no CI gate to
    maintain, no ADR trail of 6-month bumps.
  - Access to the newest stable features immediately.
  - Edition 2024 simplifies lifetime elision, `let … else`, and async
    trait ergonomics.
- **Negative**
  - Users on slightly older Rust may need to update to build our
    crates from source. Documented.
  - No guarantee that `cargo install rdf-lsp` succeeds on a distro's
    stable Rust older than what we build with.
- **Neutral**
  - README records the latest tested Rust version; that is our only
    "claim".

## Validation

- CI green on stable.
- Beta and nightly jobs run; their failures open issues but do not
  block merges.
- Release notes mention the exact Rust version used to build the
  release.

## Links

- `docs/sparc/01-specification.md` §8 — scope-decision session, 2026-04-18.
- `docs/sparc/05-completion.md` §1 DoD.
