#!/usr/bin/env node
// ttl-sweep.mjs — ADR-0019 §6 memory hygiene.
//
// Walks `phase-*` and `verification-*` namespaces. Purges non-pinned
// entries older than the TTL (default 7 days). Emits an audit log to
// .claude-flow/audit/memory-hygiene/<date>.json.
//
// Uses a namespace listing / metadata interface exposed by the local
// memory backend. For portability the sweep talks to the backend via the
// same `memory_list` / `memory_delete` MCP tools used elsewhere, by
// shelling out to the project CLI in non-dry-run mode. In dry-run mode
// (the default) no mutations are performed — the sweep prints/audits the
// purge list only.
//
// CLI:
//   ttl-sweep [--dry-run] [--ttl-days=7] [--fixture=<path>]
//             [--audit-dir=<path>] [--date=<YYYY-MM-DD>]
//
// In fixture mode, an explicit JSON file supplies the entry list:
//   [ { namespace, key, stored_at, pinned?, tags? }, ... ]

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, '..', '..');
const DEFAULT_AUDIT_DIR = resolve(REPO_ROOT, '.claude-flow', 'audit', 'memory-hygiene');
const DEFAULT_TTL_DAYS = 7;
const NAMESPACE_PATTERNS = [/^phase-/, /^verification-/];

function parseArgs(argv) {
  const out = { _: [] };
  for (const a of argv) {
    if (a.startsWith('--')) {
      const [k, v] = a.slice(2).split('=');
      out[k] = v === undefined ? true : v;
    } else {
      out._.push(a);
    }
  }
  return out;
}

function inScope(namespace) {
  return NAMESPACE_PATTERNS.some((re) => re.test(namespace));
}

function isPinned(entry) {
  if (entry.pinned === true) return true;
  const tags = entry.tags || [];
  return Array.isArray(tags) && tags.includes('pinned');
}

export function classifyEntries(entries, { now = Date.now(), ttlDays = DEFAULT_TTL_DAYS } = {}) {
  const horizonMs = ttlDays * 24 * 60 * 60 * 1000;
  const purge = [];
  const keep = [];
  const skipped = [];
  for (const e of entries) {
    if (!inScope(e.namespace)) {
      skipped.push({ ...e, reason: 'out-of-scope' });
      continue;
    }
    if (isPinned(e)) {
      keep.push({ ...e, reason: 'pinned' });
      continue;
    }
    const storedAt = typeof e.stored_at === 'number' ? e.stored_at : Date.parse(e.stored_at);
    if (!Number.isFinite(storedAt)) {
      skipped.push({ ...e, reason: 'missing-timestamp' });
      continue;
    }
    const ageMs = now - storedAt;
    if (ageMs > horizonMs) {
      purge.push({ ...e, age_ms: ageMs });
    } else {
      keep.push({ ...e, age_ms: ageMs, reason: 'within-ttl' });
    }
  }
  return { purge, keep, skipped };
}

function loadFixture(path) {
  const data = JSON.parse(readFileSync(path, 'utf8'));
  if (!Array.isArray(data)) throw new Error('fixture must be an array');
  return data;
}

function writeAudit({ auditDir, date, payload }) {
  mkdirSync(auditDir, { recursive: true });
  const file = resolve(auditDir, `${date}.json`);
  writeFileSync(file, JSON.stringify(payload, null, 2) + '\n', 'utf8');
  return file;
}

function todayStamp(date) {
  const d = date ? new Date(date) : new Date();
  const y = d.getUTCFullYear();
  const m = String(d.getUTCMonth() + 1).padStart(2, '0');
  const day = String(d.getUTCDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

async function loadLiveEntries() {
  // Placeholder for the live path: enumeration against the ruflo memory
  // backend is driven by the hook runner which has the credentials/context
  // to call MCP tools. In this agent-authored scaffold we default to
  // fixture mode; the hook registration wires ttl-sweep to receive a
  // pre-materialised listing via --fixture=<tmpfile>. Keeping this
  // boundary explicit prevents the guard script from racing with the
  // memory daemon.
  throw new Error('ttl-sweep: live mode requires --fixture=<path>; wire via ruflo hook.');
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const dryRun = args['dry-run'] !== false; // default true
  const ttlDays = Number(args['ttl-days'] ?? DEFAULT_TTL_DAYS);
  const auditDir = args['audit-dir'] ? resolve(args['audit-dir']) : DEFAULT_AUDIT_DIR;
  const date = todayStamp(args['date']);

  const entries = args.fixture
    ? loadFixture(resolve(args.fixture))
    : await loadLiveEntries();

  const now = args.now ? Number(args.now) : Date.now();
  const { purge, keep, skipped } = classifyEntries(entries, { now, ttlDays });

  const payload = {
    version: 1,
    run_id: `${date}-${process.pid}`,
    ttl_days: ttlDays,
    dry_run: dryRun,
    scope_patterns: NAMESPACE_PATTERNS.map((r) => r.source),
    counts: { purge: purge.length, keep: keep.length, skipped: skipped.length },
    purge,
    keep,
    skipped,
  };

  const auditFile = writeAudit({ auditDir, date, payload });

  if (!args.quiet) {
    console.log(`ttl-sweep ${dryRun ? '(dry-run)' : '(LIVE)'}: purge=${purge.length} keep=${keep.length} skipped=${skipped.length}`);
    console.log(`audit: ${auditFile}`);
    for (const p of purge) {
      console.log(`  PURGE ${p.namespace}/${p.key} (age ${Math.round(p.age_ms / 86400000)}d)`);
    }
  }

  if (!dryRun) {
    // Live purge path deliberately staged behind the ruflo hook runner,
    // which owns MCP credentials. Emit a manifest file alongside the audit
    // log for the runner to consume.
    const manifestFile = resolve(auditDir, `${date}.purge-manifest.json`);
    writeFileSync(manifestFile, JSON.stringify(purge, null, 2) + '\n', 'utf8');
    if (!args.quiet) console.log(`purge manifest (for hook runner): ${manifestFile}`);
  }
}

const isMain = (() => {
  try { return import.meta.url === `file://${process.argv[1]}`; } catch { return false; }
})();

if (isMain) {
  main().catch((err) => {
    console.error(`ttl-sweep: ${err.message}`);
    process.exit(1);
  });
}
