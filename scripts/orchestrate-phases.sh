#!/usr/bin/env bash
# Sequential phase orchestration: C → D → E → F → G → H → I
# Each phase uses a single-shot parallel spawn per ADR-0017 §2.
# Run from the repository root.
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

log() { echo "[orchestrate] $(date -u +%H:%M:%SZ) $*"; }

check_gate() {
  local tag="$1"
  if git tag --list "$tag" | grep -q "$tag"; then
    log "Gate $tag: PASSED"
    return 0
  else
    log "Gate $tag: NOT YET MET — run the phase integration pass first"
    return 1
  fi
}

# ── Phase C: SPARQL syntax ────────────────────────────────────────────────────
phase_c() {
  log "Phase C: SPARQL syntax — pc-sparql-wiring + pc-tester + pc-reviewer"
  npx @sparkleideas/cli@latest swarm init --topology hierarchical \
    --namespace phase-c --agents 3 || true
  # Spawn all Phase C agents in ONE claude-flow command (ADR-0017 §6.1).
  # See scripts/spawn/phase-c/*.md for per-agent prompts.
  log "Phase C agents spawned. Wait for completion then run integration pass."
  log "Integration: cargo test --workspace --all-features --no-fail-fast"
  log "Exit gate: cargo run -p xtask -- verify sparql"
  log "On success: git tag phase-c/done"
}

# ── Phase D: ShEx + Datalog syntax ───────────────────────────────────────────
phase_d() {
  check_gate phase-c/done
  log "Phase D: shex-syntax + datalog-syntax — pd-shex-syntax + pd-datalog-syntax + pd-tester + pd-reviewer"
  npx @sparkleideas/cli@latest swarm init --topology mesh \
    --namespace phase-d --agents 4 || true
  log "Phase D agents spawned. See scripts/spawn/phase-d/*.md"
  log "Exit gate: ShEx syntax entries 100%; Datalog fixture corpus green"
  log "On success: git tag phase-d/done"
}

# ── Phase E: Vocab + formatters ───────────────────────────────────────────────
phase_e() {
  check_gate phase-d/done
  log "Phase E: rdf-vocab + rdf-format formatters"
  npx @sparkleideas/cli@latest swarm init --topology hierarchical \
    --namespace phase-e --agents 4 || true
  log "Phase E agents spawned. See scripts/spawn/phase-e/*.md"
  log "Exit gate: hover-docs snapshot locked; formatter idempotency green"
  log "On success: git tag phase-e/done"
}

# ── Phase F: LSP core ─────────────────────────────────────────────────────────
phase_f() {
  check_gate phase-e/done
  log "Phase F: rdf-lsp core (didOpen/didChange/diag/hover/completion/goto/docSymbol/format)"
  npx @sparkleideas/cli@latest swarm init --topology hierarchical \
    --namespace phase-f --agents 4 || true
  log "Phase F agents spawned. See scripts/spawn/phase-f/*.md"
  log "Exit gate: LSP integration harness green across all 11 languages"
  log "On success: git tag phase-f/done"
}

# ── Phase G: LSP polish ───────────────────────────────────────────────────────
phase_g() {
  check_gate phase-f/done
  log "Phase G: rename + code-actions + semantic-tokens + incremental parse"
  npx @sparkleideas/cli@latest swarm init --topology mesh \
    --namespace phase-g --agents 4 || true
  log "Phase G agents spawned. See scripts/spawn/phase-g/*.md"
  log "Exit gate: per-feature tests green; ≤100ms cold-open highlight"
  log "On success: git tag phase-g/done"
}

# ── Phase H: Zed extension ────────────────────────────────────────────────────
phase_h() {
  check_gate phase-g/done
  log "Phase H: extensions/zed-rdf"
  npx @sparkleideas/cli@latest swarm init --topology mesh \
    --namespace phase-h --agents 3 || true
  log "Phase H agents spawned. See scripts/spawn/phase-h/*.md"
  log "Exit gate: zed: install dev extension works on all 11 languages"
  log "On success: git tag phase-h/done"
}

# ── Phase I: Publish + harden ─────────────────────────────────────────────────
phase_i() {
  check_gate phase-h/done
  log "Phase I: publish crates + extension + fuzz 24h + v1.0 tag"
  npx @sparkleideas/cli@latest swarm init --topology hierarchical-mesh \
    --namespace phase-i --agents 3 || true
  log "Phase I agents spawned. See scripts/spawn/phase-i/*.md"
  log "Exit gate: v1.0 tagged; all crates published; fuzz 24h clean"
  log "On success: git tag phase-i/done && git tag v1.0"
}

# ── Entry point ───────────────────────────────────────────────────────────────
CMD="${1:-help}"
case "$CMD" in
  c) phase_c ;;
  d) phase_d ;;
  e) phase_e ;;
  f) phase_f ;;
  g) phase_g ;;
  h) phase_h ;;
  i) phase_i ;;
  all)
    phase_c
    log "Run Phase C integration pass, then: $0 d"
    ;;
  help|*)
    echo "Usage: $0 <c|d|e|f|g|h|i|all>"
    echo "  c    Phase C: SPARQL syntax (active)"
    echo "  d    Phase D: ShEx + Datalog syntax (requires phase-c/done)"
    echo "  e    Phase E: Vocab + formatters (requires phase-d/done)"
    echo "  f    Phase F: LSP core (requires phase-e/done)"
    echo "  g    Phase G: LSP polish (requires phase-f/done)"
    echo "  h    Phase H: Zed extension (requires phase-g/done)"
    echo "  i    Phase I: Publish + harden (requires phase-h/done)"
    echo "  all  Start Phase C, then prompt for each subsequent gate"
    ;;
esac
