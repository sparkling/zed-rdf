---
agent: expert-d-respawn
cohort: hive-adr-0007
role: dependency-policy
date: 2026-04-20
---

# Brief D — Dependency / policy alignment

## §1 Compatibility matrix

| Option | Licence | Transitive count (runtime, order-of-mag) | MSRV | C compiler needed? | WASM-clean? | ADR-0004 delta | Exit cost |
|---|---|---|---|---|---|---|---|
| hand-roll | n/a | 0 | workspace default | no | yes | delete the "chumsky OR winnow" row | zero — nothing to rip out |
| chumsky (default features) | Apache-2.0 OR MIT | ~6–10 (pulls `stacker` → `psm` + `cc` build-dep + `libc` + `windows-sys`) | stable | **yes** (`cc` runs on `psm`; `psm` ships asm + C shims) | **no** for `psm` path; stack-growing is a no-op on wasm32 but the crate still compiles — risk is toolchain / C-toolchain requirement on contributor machines and CI | row narrows: "chumsky (no-default-features) ADR-0007"; plus a features pin note | medium — combinators infect parser sites; ripping out = rewrite each grammar |
| chumsky (`default-features = false`, recursion feature off) | Apache-2.0 OR MIT | ~3–5 (no `stacker`, no `psm`, no `cc`) | stable | no | yes | allow-list row stays; `[workspace.dependencies]` must pin `default-features = false` and the feature set | medium (same as above) |
| winnow | Apache-2.0 OR MIT | ~1–3 (effectively leaf; nom author's successor, deliberately small) | stable | no | yes | row narrows: "winnow (ADR-0007)" | low — winnow style is closer to hand-written recursive descent, easier to back out |

## §2 Critical finding — chumsky `stacker`

- **What it is.** chumsky's *default* feature set includes a `stacker`-backed recursion guard. `stacker` depends on `psm` (platform-specific stack-manipulation, ships assembly + C), which in turn pulls `cc` as a build-dependency and `libc` / `windows-sys` as runtime. That is a C toolchain requirement for any downstream that activates default features.
- **Why it matters here.** This workspace ships a Zed extension targeted at wasm32. `psm` has no meaningful role in wasm, and forcing contributors (and CI) to provide a working C toolchain for a library we adopted specifically to *avoid* infrastructure work is a net supply-chain loss. It also widens the attack surface unnecessarily: `cc`/`psm`/`windows-sys` are all exempt from the "leaf IETF-RFC" admission criteria in ADR-0004 §"Runtime IETF-RFC carve-out".
- **Recommendation.** If chumsky is admitted, admit it as `default-features = false` in `[workspace.dependencies]`, with an explicit feature list (e.g. only `std`, never `stacker`). Enforce mechanically: pin the features in `[workspace.dependencies]` and add a comment in `deny.toml` near the `[bans]` section referencing ADR-0007. If a future bump re-enables the recursion feature upstream, `deny-regression/` should fail on the reappearance of `psm` in the normal dep closure.

## §3 ADR-0004 edit proposals

**Choice 1 — hand-roll (delete the deferred row):**

```diff
-| `chumsky` **or** `winnow` (ADR-0007) | Parser combinators for complex grammars     | Writing Turtle/SPARQL by hand is viable but slower | Hand-written recursive descent |
```

**Choice 2 — adopt chumsky (feature-restricted):**

```diff
-| `chumsky` **or** `winnow` (ADR-0007) | Parser combinators for complex grammars     | Writing Turtle/SPARQL by hand is viable but slower | Hand-written recursive descent |
+| `chumsky` (ADR-0007, `default-features = false`, no `stacker`) | Parser combinators for complex grammars     | Writing Turtle/SPARQL by hand is viable but slower. Default features disabled to keep the dep graph pure-Rust and WASM-clean. | Hand-written recursive descent |
```

**Choice 3 — adopt winnow:**

```diff
-| `chumsky` **or** `winnow` (ADR-0007) | Parser combinators for complex grammars     | Writing Turtle/SPARQL by hand is viable but slower | Hand-written recursive descent |
+| `winnow` (ADR-0007) | Parser combinators for complex grammars     | nom's successor; small pure-Rust dep graph; writing Turtle/SPARQL by hand is viable but slower | Hand-written recursive descent |
```

## §4 deny.toml note

Adopting a combinator library does **not** require any `[bans]` edit in `deny.toml`: neither chumsky nor winnow parses RDF, so neither belongs on the banned-RDF list, and `deny.toml` does not maintain a positive allow-list (that lives in ADR-0004). What **is** required is a `[workspace.dependencies]` pin — and if chumsky is chosen, a `default-features = false` + explicit feature list there, so that `cargo deny check` (with `exclude-dev = true` already set) sees the reduced graph. Optionally, extend `crates/testing/deny-regression/` to assert `psm` / `stacker` are absent from the runtime closure as a belt-and-braces guard against default-feature re-enablement on version bumps.

## §5 Risk scoring

- **hand-roll — Low.** Zero new supply-chain surface; compromise impact bounded to the workspace itself.
- **chumsky (default features) — High.** `cc` build-dep means an upstream `psm`/`cc` compromise can execute on every contributor and CI build; also drags `windows-sys` / `libc` into the runtime edge. WASM portability story becomes conditional.
- **chumsky (`default-features = false`) — Medium.** Reduces attack surface to the chumsky crate itself plus its small pure-Rust transitives, but trust in chumsky's maintainer set is still centralised and the crate is larger than winnow.
- **winnow — Low/Medium.** Tiny pure-Rust dep graph, maintained by the nom lineage; compromise impact limited to one well-scoped upstream. Effectively the lowest-risk "admit a combinator lib" option.

## §6 Open questions for queen

- Do we accept chumsky's ergonomic error-recovery machinery as worth the Medium-risk scoring, or does winnow's lower-risk profile dominate given the Zed/WASM shipping target?
- If chumsky is chosen, is ADR-0004 the right home for the feature pin, or should ADR-0007 carry the normative "chumsky without `stacker`" rule and ADR-0004 merely reference it?
- Should `crates/testing/deny-regression/` gain an explicit negative assertion on `psm` / `cc` / `stacker` in the runtime closure, regardless of which combinator (if any) is admitted?
