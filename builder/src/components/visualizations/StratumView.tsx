/**
 * StratumView: visual representation of rule strata.
 *
 * Renders rules organized as horizontal swim lanes by stratum number.
 * Stratum 0 is at the top; higher strata are below.
 *
 * Features:
 * - Swim lanes per stratum with color coding
 * - Rule cards showing ID, abbreviated condition, and verdict type
 * - Validation overlay: same-stratum verdict_present references (error)
 * - Compact mode: just rule IDs in rows
 * - Click a rule card to navigate (calls onSelectRule)
 */
import React, { useMemo } from "react";
import type { RuleConstruct, PredicateExpression, VerdictPresentExpr } from "@/types/interchange";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface StratumViewProps {
  rules: RuleConstruct[];
  selectedRuleId?: string | null;
  compact?: boolean;
  onSelectRule?: (id: string) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const STRATUM_COLORS: string[] = [
  "border-blue-300 bg-blue-50",
  "border-green-300 bg-green-50",
  "border-purple-300 bg-purple-50",
  "border-orange-300 bg-orange-50",
  "border-pink-300 bg-pink-50",
  "border-teal-300 bg-teal-50",
];

const STRATUM_HEADER_COLORS: string[] = [
  "bg-blue-100 text-blue-800",
  "bg-green-100 text-green-800",
  "bg-purple-100 text-purple-800",
  "bg-orange-100 text-orange-800",
  "bg-pink-100 text-pink-800",
  "bg-teal-100 text-teal-800",
];

function stratumColor(stratum: number): string {
  return STRATUM_COLORS[stratum % STRATUM_COLORS.length];
}

function stratumHeaderColor(stratum: number): string {
  return STRATUM_HEADER_COLORS[stratum % STRATUM_HEADER_COLORS.length];
}

/**
 * Collect all verdict_id strings referenced via verdict_present in a predicate expression.
 */
function collectVerdictRefs(expr: PredicateExpression): Set<string> {
  const refs = new Set<string>();

  function walk(e: PredicateExpression | object) {
    if (!e || typeof e !== "object") return;

    if ("verdict_present" in e) {
      refs.add((e as VerdictPresentExpr).verdict_present);
      return;
    }
    if ("left" in e) walk((e as { left: object }).left);
    if ("right" in e) walk((e as { right: object }).right);
    if ("operand" in e) walk((e as { operand: object }).operand);
    if ("body" in e) walk((e as { body: object }).body);
  }

  walk(expr);
  return refs;
}

/**
 * Summarize a predicate expression as a short human-readable string.
 */
function summarizeExpr(expr: PredicateExpression | null | undefined): string {
  if (!expr) return "(always)";

  if ("verdict_present" in expr) {
    return `verdict_present(${expr.verdict_present})`;
  }

  if ("quantifier" in expr) {
    return `${expr.quantifier} ${expr.variable} in ...`;
  }

  if ("op" in expr) {
    const op = (expr as { op: string }).op;
    if (op === "and") return "... AND ...";
    if (op === "or") return "... OR ...";
    if (op === "not") return "NOT ...";

    // Compare
    const cmp = expr as {
      op: string;
      left: { fact_ref?: string; literal?: unknown };
      right: { fact_ref?: string; literal?: unknown };
    };
    const left = cmp.left.fact_ref ?? String(cmp.left.literal ?? "?");
    const right = cmp.right.fact_ref ?? String(cmp.right.literal ?? "?");
    return `${left} ${op} ${right}`;
  }

  return "(condition)";
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

interface ValidationIssue {
  ruleId: string;
  message: string;
  kind: "error" | "warning";
}

function computeValidationIssues(rules: RuleConstruct[]): ValidationIssue[] {
  const issues: ValidationIssue[] = [];

  // Build map: verdict_id -> producing rule
  const verdictProducers = new Map<string, RuleConstruct>();
  for (const rule of rules) {
    const verdictId = rule.body.produce.verdict_type;
    if (verdictId) {
      if (verdictProducers.has(verdictId)) {
        issues.push({
          ruleId: rule.id,
          message: `Duplicate verdict ID "${verdictId}" also produced by ${verdictProducers.get(verdictId)!.id}`,
          kind: "error",
        });
      } else {
        verdictProducers.set(verdictId, rule);
      }
    }
  }

  // Check same-stratum verdict_present references
  for (const rule of rules) {
    const refs = collectVerdictRefs(rule.body.when);
    for (const verdictId of refs) {
      const producer = verdictProducers.get(verdictId);
      if (producer && producer.stratum === rule.stratum) {
        issues.push({
          ruleId: rule.id,
          message: `Circular: references verdict "${verdictId}" in same stratum ${rule.stratum}`,
          kind: "error",
        });
      }
    }
  }

  return issues;
}

// ---------------------------------------------------------------------------
// RuleCard
// ---------------------------------------------------------------------------

interface RuleCardProps {
  rule: RuleConstruct;
  selected: boolean;
  issues: ValidationIssue[];
  compact: boolean;
  onClick: () => void;
}

function RuleCard({ rule, selected, issues, compact, onClick }: RuleCardProps) {
  const ruleIssues = issues.filter((i) => i.ruleId === rule.id);
  const hasError = ruleIssues.some((i) => i.kind === "error");

  if (compact) {
    return (
      <button
        onClick={onClick}
        className={`rounded px-2 py-1 text-xs font-mono transition-colors ${
          selected
            ? "bg-blue-500 text-white"
            : hasError
            ? "border border-red-300 bg-red-50 text-red-700 hover:bg-red-100"
            : "border border-gray-200 bg-white text-gray-700 hover:bg-gray-100"
        }`}
        title={ruleIssues.map((i) => i.message).join("\n") || rule.id}
      >
        {rule.id}
        {hasError && " ⚠"}
      </button>
    );
  }

  return (
    <button
      onClick={onClick}
      className={`w-36 rounded border p-2 text-left text-xs transition-colors ${
        selected
          ? "border-blue-500 bg-blue-100"
          : hasError
          ? "border-red-300 bg-red-50 hover:bg-red-100"
          : "border-gray-200 bg-white hover:bg-gray-50"
      }`}
      title={ruleIssues.map((i) => i.message).join("\n") || undefined}
    >
      <div className="mb-0.5 font-mono font-semibold text-gray-800 truncate">
        {rule.id}
      </div>
      <div className="text-gray-500 truncate">
        {summarizeExpr(rule.body.when)}
      </div>
      <div className="mt-0.5 truncate font-mono text-blue-600">
        → {rule.body.produce.verdict_type || "verdict"}
      </div>
      {hasError && (
        <div className="mt-0.5 text-red-500">
          {ruleIssues[0].message}
        </div>
      )}
    </button>
  );
}

// ---------------------------------------------------------------------------
// StratumView (main export)
// ---------------------------------------------------------------------------

export function StratumView({
  rules,
  selectedRuleId,
  compact = false,
  onSelectRule,
}: StratumViewProps) {
  // Group rules by stratum
  const strata = useMemo<Map<number, RuleConstruct[]>>(() => {
    const map = new Map<number, RuleConstruct[]>();
    for (const rule of rules) {
      const s = rule.stratum ?? 0;
      if (!map.has(s)) map.set(s, []);
      map.get(s)!.push(rule);
    }
    return map;
  }, [rules]);

  const sortedStrata = useMemo(
    () => Array.from(strata.keys()).sort((a, b) => a - b),
    [strata]
  );

  const validationIssues = useMemo(
    () => computeValidationIssues(rules),
    [rules]
  );

  if (rules.length === 0) {
    return (
      <div className="rounded border-2 border-dashed border-gray-200 py-6 text-center text-xs text-gray-400">
        No rules yet
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {sortedStrata.map((stratum) => {
        const stratumRules = strata.get(stratum) ?? [];
        const color = stratumColor(stratum);
        const headerColor = stratumHeaderColor(stratum);

        return (
          <div key={stratum} className={`rounded border ${color}`}>
            {/* Lane header */}
            <div
              className={`flex items-center gap-2 rounded-t px-3 py-1.5 text-xs font-semibold ${headerColor}`}
            >
              <span>Stratum {stratum}</span>
              <span className="rounded-full bg-white bg-opacity-60 px-1.5 py-0.5 text-xs font-medium">
                {stratumRules.length} rule{stratumRules.length !== 1 ? "s" : ""}
              </span>
            </div>

            {/* Rule cards */}
            <div
              className={`flex flex-wrap gap-2 p-2 ${
                compact ? "items-center" : "items-start"
              }`}
            >
              {stratumRules.map((rule) => (
                <RuleCard
                  key={rule.id}
                  rule={rule}
                  selected={selectedRuleId === rule.id}
                  issues={validationIssues}
                  compact={compact}
                  onClick={() => onSelectRule?.(rule.id)}
                />
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}
