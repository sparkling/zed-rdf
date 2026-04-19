// cohort-registry.mjs
// Parse docs/agent-cohorts.md and expose the canonical cohort map used by
// cohort-guard. Deliberately minimal: we only extract the per-agent rows and
// the "May read / May NOT read" lines for each cohort.
//
// Source of truth: docs/agent-cohorts.md. This file must never diverge;
// cohort-guard fails closed on parse errors.

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const DEFAULT_REGISTRY = resolve(HERE, '..', '..', 'docs', 'agent-cohorts.md');

// Shared namespaces that any cohort may read — spec-pins are shared.
const SHARED_READABLE = new Set([
  'verification/spec-readings',
  'verification/memory-hygiene',
]);

function parseCohortSection(md, headingRegex) {
  const m = md.match(headingRegex);
  if (!m) return null;
  const start = m.index + m[0].length;
  // A section ends at the next `### ` or `## ` heading.
  const tail = md.slice(start);
  const end = tail.search(/\n#{2,3} /);
  return end === -1 ? tail : tail.slice(0, end);
}

function parseAllowList(section, label) {
  // `**May read memory namespaces:**` or `**May NOT read:**`
  const re = new RegExp(`\\*\\*${label}:\\*\\*([\\s\\S]*?)(?:\\n\\n|\\n\\*\\*|$)`, 'i');
  const m = section.match(re);
  if (!m) return [];
  return m[1]
    .split(/[,\n]/)
    .map((s) => s.replace(/[`.*]/g, '').trim())
    .filter((s) => s && !s.startsWith('('))
    // Drop prose fragments like "(pins are shared reference material)"
    .map((s) => s.split(/\s+/)[0])
    .filter(Boolean);
}

function parseAgentTable(section) {
  const rows = [];
  const lines = section.split('\n');
  for (const line of lines) {
    const t = line.trim();
    if (!t.startsWith('|') || t.startsWith('|---') || t.startsWith('| Agent')) continue;
    const cells = t.split('|').map((c) => c.trim()).filter(Boolean);
    if (cells.length < 4) continue;
    const agentCell = cells[0].replace(/`/g, '');
    const lineage = cells[2].toLowerCase();
    if (!/^v1-/.test(agentCell)) continue;
    if (lineage !== 'cohort-a' && lineage !== 'cohort-b') continue;
    rows.push({ agent: agentCell, cohort: lineage });
  }
  return rows;
}

export function loadRegistry(path = DEFAULT_REGISTRY) {
  const md = readFileSync(path, 'utf8');
  const sectionA = parseCohortSection(md, /###\s+Cohort A[^\n]*\n/);
  const sectionB = parseCohortSection(md, /###\s+Cohort B[^\n]*\n/);
  if (!sectionA || !sectionB) {
    throw new Error(`cohort-registry: failed to locate cohort sections in ${path}`);
  }

  // Per-agent tables appear under "### Cohort A (verification-v1)" and
  // "### Cohort B (verification-v1-adv)". Those are the rows we want.
  const perAgentA = parseCohortSection(md, /###\s+Cohort A\s*\(`verification-v1`\)[^\n]*\n/);
  const perAgentB = parseCohortSection(md, /###\s+Cohort B\s*\(`verification-v1-adv`\)[^\n]*\n/);

  const rows = [
    ...(perAgentA ? parseAgentTable(perAgentA) : []),
    ...(perAgentB ? parseAgentTable(perAgentB) : []),
  ];

  const agentCohort = new Map(rows.map((r) => [r.agent, r.cohort]));

  return {
    path,
    agentCohort,
    cohorts: {
      'cohort-a': {
        hive: 'verification-v1',
        allow: parseAllowList(sectionA, 'May read memory namespaces'),
        deny: parseAllowList(sectionA, 'May NOT read'),
      },
      'cohort-b': {
        hive: 'verification-v1-adv',
        allow: parseAllowList(sectionB, 'May read memory namespaces'),
        deny: parseAllowList(sectionB, 'May NOT read'),
      },
    },
  };
}

export function namespaceMatches(pattern, namespace) {
  // Glob-ish: "crate/*-shadow" matches "crate/turtle-shadow".
  if (!pattern) return false;
  if (pattern === namespace) return true;
  if (pattern.endsWith('/*')) return namespace.startsWith(pattern.slice(0, -1));
  if (pattern.includes('*')) {
    const re = new RegExp('^' + pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*') + '$');
    return re.test(namespace);
  }
  // Prefix match: "verification-v1" should match "verification-v1/sub".
  return namespace === pattern || namespace.startsWith(pattern + '/');
}

export function isSharedNamespace(ns) {
  for (const pfx of SHARED_READABLE) {
    if (ns === pfx || ns.startsWith(pfx + '/')) return true;
  }
  return false;
}

export function evaluate({ cohort, action, namespace, registry }) {
  // Fails closed: unknown cohort → deny.
  if (!cohort || !registry.cohorts[cohort]) {
    return { allow: false, reason: `unknown cohort "${cohort}"` };
  }
  const rules = registry.cohorts[cohort];

  // Writes: only into the caller's own hive or shared namespaces.
  if (action === 'write' || action === 'store') {
    if (isSharedNamespace(namespace)) return { allow: true, reason: 'shared-write' };
    if (namespaceMatches(rules.hive, namespace)) return { allow: true, reason: 'own-hive' };
    // Cohort A has legitimate writes to crate/** etc. — fall through to allow.
    if (rules.allow.some((p) => namespaceMatches(p, namespace))) {
      return { allow: true, reason: 'allow-listed' };
    }
    return { allow: false, reason: `write outside cohort scope: ${namespace}` };
  }

  // Reads: explicit deny wins over allow.
  for (const pattern of rules.deny) {
    if (namespaceMatches(pattern, namespace)) {
      return { allow: false, reason: `deny-listed for ${cohort}: ${pattern}` };
    }
  }
  if (isSharedNamespace(namespace)) return { allow: true, reason: 'shared' };
  for (const pattern of rules.allow) {
    if (namespaceMatches(pattern, namespace)) {
      return { allow: true, reason: `allow-listed: ${pattern}` };
    }
  }
  return { allow: false, reason: `not on allow-list for ${cohort}: ${namespace}` };
}
