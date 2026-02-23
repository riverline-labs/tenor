/**
 * Audit analysis engine -- provenance walking and compliance gap detection.
 *
 * Given a contract evaluation, traces each verdict back through its provenance
 * chain to build a complete audit trail showing which rules fired, what facts
 * drove each decision, and where gaps exist.
 */

import {
  TenorClient,
  type Verdict,
  type EvalResult,
  type ContractSummary,
} from '../../../sdk/typescript/src/index.ts';

// ── Types ──────────────────────────────────────────────────────────────

export interface AuditReport {
  contractId: string;
  timestamp: string;
  factInputs: FactInput[];
  verdictTraces: VerdictTrace[];
  factCoverage: FactCoverageEntry[];
  complianceGaps: ComplianceGap[];
  summary: AuditSummary;
}

export interface FactInput {
  id: string;
  value: unknown;
  type: string;
}

export interface VerdictTrace {
  verdictType: string;
  payload: unknown;
  rule: string;
  stratum: number;
  factsUsed: string[];
  verdictsUsed: string[];
  dependencyChain: string[];
}

export interface FactCoverageEntry {
  factId: string;
  usedByVerdicts: string[];
  usedByRules: string[];
}

export interface ComplianceGap {
  type: 'orphan_fact' | 'shallow_provenance' | 'single_rule_dependency';
  description: string;
  severity: 'info' | 'warning' | 'critical';
  affectedItems: string[];
}

export interface AuditSummary {
  totalFacts: number;
  totalVerdicts: number;
  factsWithCoverage: number;
  factsWithoutCoverage: number;
  gapCount: { info: number; warning: number; critical: number };
}

// ── Core audit function ────────────────────────────────────────────────

/**
 * Run a compliance audit on a contract evaluation.
 *
 * Evaluates the contract with the given facts, then traces all verdict
 * provenance chains to build a comprehensive audit report.
 */
export async function runAudit(
  tenorUrl: string,
  contractId: string,
  facts: Record<string, unknown>,
): Promise<AuditReport> {
  const client = new TenorClient({ baseUrl: tenorUrl });

  // Evaluate the contract
  const result = (await client.invoke(contractId, facts)) as EvalResult;

  // Get contract metadata for declared facts list
  const contracts = await client.listContracts();
  const contract = contracts.find((c: ContractSummary) => c.id === contractId);
  if (!contract) {
    throw new Error(`Contract '${contractId}' not found after evaluation`);
  }

  const declaredFacts = contract.facts;
  const verdicts = result.verdicts;

  // Build fact inputs record
  const factInputs = buildFactInputs(facts, declaredFacts);

  // Build verdict traces with dependency chains
  const verdictTraces = buildVerdictTraces(verdicts);

  // Build fact coverage matrix
  const factCoverage = buildFactCoverage(declaredFacts, verdicts);

  // Detect compliance gaps
  const complianceGaps = detectGaps(declaredFacts, facts, verdicts, factCoverage);

  // Compute summary
  const factsWithCoverage = factCoverage.filter((f) => f.usedByVerdicts.length > 0).length;
  const summary: AuditSummary = {
    totalFacts: declaredFacts.length,
    totalVerdicts: verdicts.length,
    factsWithCoverage,
    factsWithoutCoverage: declaredFacts.length - factsWithCoverage,
    gapCount: {
      info: complianceGaps.filter((g) => g.severity === 'info').length,
      warning: complianceGaps.filter((g) => g.severity === 'warning').length,
      critical: complianceGaps.filter((g) => g.severity === 'critical').length,
    },
  };

  return {
    contractId,
    timestamp: new Date().toISOString(),
    factInputs,
    verdictTraces,
    factCoverage,
    complianceGaps,
    summary,
  };
}

// ── Builders ───────────────────────────────────────────────────────────

/** Build fact input entries from provided facts and declared fact IDs. */
function buildFactInputs(
  providedFacts: Record<string, unknown>,
  declaredFacts: string[],
): FactInput[] {
  return declaredFacts.map((id) => {
    const value = providedFacts[id];
    return {
      id,
      value: value ?? null,
      type: value === undefined ? 'not_provided' : inferType(value),
    };
  });
}

/** Infer a human-readable type name for a fact value. */
function inferType(value: unknown): string {
  if (value === null) return 'null';
  if (typeof value === 'boolean') return 'boolean';
  if (typeof value === 'number') return Number.isInteger(value) ? 'integer' : 'decimal';
  if (typeof value === 'string') return 'string';
  if (Array.isArray(value)) return 'array';
  if (typeof value === 'object') return 'object';
  return 'unknown';
}

/** Build verdict traces with full dependency chains. */
function buildVerdictTraces(verdicts: Verdict[]): VerdictTrace[] {
  // Build index of verdicts by type for dependency resolution
  const verdictByType = new Map<string, Verdict[]>();
  for (const v of verdicts) {
    const existing = verdictByType.get(v.type) ?? [];
    existing.push(v);
    verdictByType.set(v.type, existing);
  }

  return verdicts.map((v) => {
    const chain = buildDependencyChain(v, verdictByType, new Set());
    return {
      verdictType: v.type,
      payload: v.payload,
      rule: v.provenance.rule,
      stratum: v.provenance.stratum,
      factsUsed: [...v.provenance.facts_used],
      verdictsUsed: [...v.provenance.verdicts_used],
      dependencyChain: chain,
    };
  });
}

/**
 * Build the full dependency chain for a verdict by recursively following
 * verdicts_used references.
 *
 * Produces a flat list of strings describing the chain:
 *   "verdict:approve_seat <- rule:seat_check <- fact:current_seat_count, fact:plan_features"
 */
function buildDependencyChain(
  verdict: Verdict,
  verdictByType: Map<string, Verdict[]>,
  visited: Set<string>,
): string[] {
  const chain: string[] = [];
  const key = `${verdict.type}:${verdict.provenance.rule}`;

  if (visited.has(key)) return chain;
  visited.add(key);

  // This verdict's own chain entry
  const factsStr =
    verdict.provenance.facts_used.length > 0
      ? verdict.provenance.facts_used.map((f) => `fact:${f}`).join(', ')
      : '(no facts)';
  chain.push(`verdict:${verdict.type} <- rule:${verdict.provenance.rule} <- ${factsStr}`);

  // Recurse into verdicts used
  for (const usedType of verdict.provenance.verdicts_used) {
    const usedVerdicts = verdictByType.get(usedType) ?? [];
    for (const uv of usedVerdicts) {
      const subChain = buildDependencyChain(uv, verdictByType, visited);
      chain.push(...subChain);
    }
  }

  return chain;
}

/** Build fact coverage matrix: which facts influence which verdicts. */
function buildFactCoverage(
  declaredFacts: string[],
  verdicts: Verdict[],
): FactCoverageEntry[] {
  // Direct coverage: fact -> verdicts that use it directly
  const directVerdicts = new Map<string, Set<string>>();
  const directRules = new Map<string, Set<string>>();

  for (const factId of declaredFacts) {
    directVerdicts.set(factId, new Set());
    directRules.set(factId, new Set());
  }

  // Walk all verdicts
  for (const v of verdicts) {
    for (const factId of v.provenance.facts_used) {
      directVerdicts.get(factId)?.add(v.type);
      directRules.get(factId)?.add(v.provenance.rule);
    }

    // Transitive: if verdict A uses verdict B, and B uses fact F,
    // then fact F transitively influences verdict A
    if (v.provenance.verdicts_used.length > 0) {
      // Find all facts used by the verdicts this one depends on
      const transitiveFacts = getTransitiveFacts(v, verdicts, new Set());
      for (const factId of transitiveFacts) {
        directVerdicts.get(factId)?.add(v.type);
      }
    }
  }

  return declaredFacts.map((factId) => ({
    factId,
    usedByVerdicts: [...(directVerdicts.get(factId) ?? [])],
    usedByRules: [...(directRules.get(factId) ?? [])],
  }));
}

/** Get all facts transitively used by a verdict through its verdict dependencies. */
function getTransitiveFacts(
  verdict: Verdict,
  allVerdicts: Verdict[],
  visited: Set<string>,
): Set<string> {
  const facts = new Set<string>();
  const key = `${verdict.type}:${verdict.provenance.rule}`;

  if (visited.has(key)) return facts;
  visited.add(key);

  for (const f of verdict.provenance.facts_used) {
    facts.add(f);
  }

  for (const usedType of verdict.provenance.verdicts_used) {
    const usedVerdicts = allVerdicts.filter((v) => v.type === usedType);
    for (const uv of usedVerdicts) {
      const subFacts = getTransitiveFacts(uv, allVerdicts, visited);
      for (const f of subFacts) {
        facts.add(f);
      }
    }
  }

  return facts;
}

// ── Gap detection ──────────────────────────────────────────────────────

/** Detect compliance gaps in the evaluation. */
function detectGaps(
  declaredFacts: string[],
  providedFacts: Record<string, unknown>,
  verdicts: Verdict[],
  factCoverage: FactCoverageEntry[],
): ComplianceGap[] {
  const gaps: ComplianceGap[] = [];

  // Orphan facts: declared facts that no verdict references
  const orphanFacts = factCoverage
    .filter((f) => f.usedByVerdicts.length === 0 && providedFacts[f.factId] !== undefined)
    .map((f) => f.factId);

  if (orphanFacts.length > 0) {
    gaps.push({
      type: 'orphan_fact',
      description:
        'Facts provided but not referenced by any verdict. These facts had no influence on the evaluation outcome.',
      severity: 'warning',
      affectedItems: orphanFacts,
    });
  }

  // Shallow provenance: verdicts with only direct fact dependencies (no verdict chain)
  const shallowVerdicts = verdicts
    .filter(
      (v) =>
        v.provenance.verdicts_used.length === 0 && v.provenance.facts_used.length > 0,
    )
    .map((v) => `${v.type} (rule: ${v.provenance.rule})`);

  if (shallowVerdicts.length > 0) {
    gaps.push({
      type: 'shallow_provenance',
      description:
        'Verdicts with single-level provenance (facts only, no verdict dependencies). May indicate simple rules that do not participate in multi-rule reasoning.',
      severity: 'info',
      affectedItems: shallowVerdicts,
    });
  }

  // Single-rule dependency: verdict types produced by exactly one rule
  const verdictTypeRules = new Map<string, Set<string>>();
  for (const v of verdicts) {
    const rules = verdictTypeRules.get(v.type) ?? new Set();
    rules.add(v.provenance.rule);
    verdictTypeRules.set(v.type, rules);
  }

  const singleRuleVerdicts: string[] = [];
  for (const [type, rules] of verdictTypeRules) {
    if (rules.size === 1) {
      singleRuleVerdicts.push(`${type} (only rule: ${[...rules][0]})`);
    }
  }

  if (singleRuleVerdicts.length > 0) {
    gaps.push({
      type: 'single_rule_dependency',
      description:
        'Verdict types produced by exactly one rule with no redundancy. If the rule has a defect, the verdict type has no alternative source.',
      severity: 'info',
      affectedItems: singleRuleVerdicts,
    });
  }

  return gaps;
}
