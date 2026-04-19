#!/usr/bin/env node
// cohort-guard.mjs — ADR-0019 §6 mechanical enforcement.
//
// Wraps `memory_store` / `memory_search`-style calls. Consults
// docs/agent-cohorts.md (via cohort-registry.mjs) and fails closed on
// cross-cohort reads. This is the load-bearing guard: every cohort-A agent
// in the verification-v1 sweep depends on it.
//
// CLI:
//   cohort-guard check --cohort=cohort-a --action=read --namespace=verification-v1
//   cohort-guard check --agent=v1-adv-nt --action=read --namespace=verification-v1
//
// Exit codes:
//   0  → allowed
//   2  → denied (with reason on stderr)
//   3  → input error
//
// Library usage:
//   import { guard } from './cohort-guard.mjs';
//   const result = guard({ cohort, action, namespace });

import { loadRegistry, evaluate } from './cohort-registry.mjs';

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

export function guard({ cohort, agent, action, namespace, registryPath }) {
  let registry;
  try {
    registry = loadRegistry(registryPath);
  } catch (err) {
    return { allow: false, reason: `registry load failed: ${err.message}` };
  }

  let resolvedCohort = cohort;
  if (!resolvedCohort && agent) {
    resolvedCohort = registry.agentCohort.get(agent) ?? null;
    if (!resolvedCohort) {
      return { allow: false, reason: `agent "${agent}" not in cohort registry` };
    }
  }

  if (!resolvedCohort) return { allow: false, reason: 'no cohort provided' };
  if (!action) return { allow: false, reason: 'no action provided' };
  if (!namespace) return { allow: false, reason: 'no namespace provided' };

  return evaluate({ cohort: resolvedCohort, action, namespace, registry });
}

// CLI entrypoint
const isMain = (() => {
  try {
    return import.meta.url === `file://${process.argv[1]}`;
  } catch {
    return false;
  }
})();

if (isMain) {
  const args = parseArgs(process.argv.slice(2));
  const sub = args._[0];
  if (sub !== 'check') {
    console.error('usage: cohort-guard check --cohort=<c> --action=<a> --namespace=<n>');
    process.exit(3);
  }
  const result = guard({
    cohort: args.cohort,
    agent: args.agent,
    action: args.action,
    namespace: args.namespace,
    registryPath: args.registry,
  });
  if (result.allow) {
    if (!args.quiet) console.log(`allow: ${result.reason}`);
    process.exit(0);
  } else {
    console.error(`deny: ${result.reason}`);
    process.exit(2);
  }
}
