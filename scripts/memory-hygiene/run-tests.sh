#!/usr/bin/env bash
# Run all memory-hygiene unit tests. Used locally and by v1-ci-wiring.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
cd "$here"

node tests/cohort-guard.test.mjs
node tests/ttl-sweep.test.mjs
node tests/falsification-hook.test.mjs

echo ""
echo "memory-hygiene: all tests passed"
