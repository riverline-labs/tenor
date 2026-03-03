/**
 * ProvenanceView: Renders the provenance chain from a verdict or flow step
 * back through the rule chain to source facts.
 *
 * Displayed as a modal/drawer overlay.
 */
import React, { useCallback } from "react";
import { useContractStore } from "@/store/contract";
import type { VerdictResult } from "@/store/simulation";
import type { PredicateExpression } from "@/types/interchange";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ProvenanceEntry {
  verdict_type?: string;
  rule_id?: string;
  stratum?: number;
  fact_refs?: string[];
  verdict_refs?: string[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Extract all fact_ref IDs and verdict_present IDs from a predicate expression.
 */
function extractRefs(
  expr: PredicateExpression | undefined | null,
  factRefs: Set<string>,
  verdictRefs: Set<string>
): void {
  if (!expr) return;

  const e = expr as unknown as Record<string, unknown>;

  if ("fact_ref" in e && typeof e.fact_ref === "string") {
    factRefs.add(e.fact_ref);
    return;
  }
  if ("verdict_present" in e && typeof e.verdict_present === "string") {
    verdictRefs.add(e.verdict_present);
    return;
  }
  if ("left" in e) extractRefs(e.left as PredicateExpression, factRefs, verdictRefs);
  if ("right" in e) extractRefs(e.right as PredicateExpression, factRefs, verdictRefs);
  if ("operand" in e) extractRefs(e.operand as PredicateExpression, factRefs, verdictRefs);
  if ("body" in e) extractRefs(e.body as PredicateExpression, factRefs, verdictRefs);
  if ("domain" in e) extractRefs(e.domain as PredicateExpression, factRefs, verdictRefs);
}

/**
 * Summarize a predicate expression into a human-readable string.
 */
function summarizeExpr(expr: PredicateExpression | undefined | null, depth = 0): string {
  if (!expr) return "—";
  if (depth > 4) return "...";

  const e = expr as unknown as Record<string, unknown>;

  if ("fact_ref" in e) return `fact(${e.fact_ref})`;
  if ("verdict_present" in e) return `verdict_present(${e.verdict_present})`;
  if ("literal" in e) return String(e.literal);

  if ("op" in e) {
    const op = e.op as string;
    if (op === "not") return `NOT ${summarizeExpr(e.operand as PredicateExpression, depth + 1)}`;
    if (op === "and") return `(${summarizeExpr(e.left as PredicateExpression, depth + 1)} AND ${summarizeExpr(e.right as PredicateExpression, depth + 1)})`;
    if (op === "or") return `(${summarizeExpr(e.left as PredicateExpression, depth + 1)} OR ${summarizeExpr(e.right as PredicateExpression, depth + 1)})`;
    if (["=", "!=", "<", "<=", ">", ">=", "*"].includes(op)) {
      return `${summarizeExpr(e.left as PredicateExpression, depth + 1)} ${op} ${summarizeExpr(e.right as PredicateExpression, depth + 1)}`;
    }
  }
  if ("quantifier" in e) {
    return `${e.quantifier}(${e.variable} in ${summarizeExpr(e.domain as PredicateExpression, depth + 1)})`;
  }
  return JSON.stringify(expr).slice(0, 40);
}

function formatPayload(payload: unknown): string {
  if (payload === null || payload === undefined) return "—";
  if (typeof payload === "boolean") return payload ? "true" : "false";
  if (typeof payload === "number" || typeof payload === "string") return String(payload);
  if (typeof payload === "object") {
    const obj = payload as Record<string, unknown>;
    if ("amount" in obj && "currency" in obj) return `${obj.amount} ${obj.currency}`;
    if ("value" in obj) return String(obj.value);
    return JSON.stringify(payload);
  }
  return JSON.stringify(payload);
}

// ---------------------------------------------------------------------------
// ProvenanceNode component (recursive)
// ---------------------------------------------------------------------------

interface ProvenanceNodeProps {
  verdictType: string;
  depth?: number;
  allVerdicts: VerdictResult[];
  onFactClick?: (factId: string) => void;
}

function ProvenanceNode({ verdictType, depth = 0, allVerdicts, onFactClick }: ProvenanceNodeProps) {
  const constructs = useContractStore((s) => s.bundle.constructs);

  // Find the rule that produces this verdict
  const rule = constructs.find(
    (c) => c.kind === "Rule" && c.body.produce.verdict_type === verdictType
  );

  // Find the verdict value from simulation results
  const verdictEntry = allVerdicts.find((v) => v.verdict_type === verdictType);

  // Extract fact refs and verdict refs from the rule condition
  const factRefs = new Set<string>();
  const verdictRefs = new Set<string>();
  if (rule && rule.kind === "Rule") {
    extractRefs(rule.body.when, factRefs, verdictRefs);
  }

  const indent = depth * 16;

  return (
    <div style={{ marginLeft: indent }} className="mb-2">
      {/* Connector line */}
      {depth > 0 && (
        <div
          className="absolute -left-4 top-3 h-px w-4 bg-border"
          style={{ marginLeft: indent - 16 }}
        />
      )}

      {/* Verdict node */}
      <div className="rounded-md border border-primary/50 bg-muted p-2">
        <div className="flex items-center justify-between gap-2">
          <span className="font-mono text-sm font-semibold text-primary">
            {verdictType}
          </span>
          {verdictEntry && (
            <span className="text-sm text-primary">
              = {formatPayload(verdictEntry.payload)}
            </span>
          )}
        </div>
        {rule && rule.kind === "Rule" && (
          <div className="mt-1.5 text-xs text-muted-foreground">
            <span className="font-medium">Rule:</span>{" "}
            <span className="font-mono text-foreground">{rule.id}</span>
            <Badge variant="secondary" className="ml-2 bg-primary/10 text-primary">
              stratum {rule.stratum}
            </Badge>
          </div>
        )}
        {rule && rule.kind === "Rule" && (
          <div className="mt-1 text-xs text-muted-foreground">
            <span className="font-medium">Condition:</span>{" "}
            <code className="rounded-md bg-muted px-1 text-foreground">
              {summarizeExpr(rule.body.when)}
            </code>
          </div>
        )}
        {!rule && (
          <div className="mt-1 text-xs text-muted-foreground italic">
            No rule found for this verdict
          </div>
        )}
      </div>

      {/* Fact dependencies */}
      {factRefs.size > 0 && (
        <div className="ml-4 mt-1 space-y-0.5">
          {Array.from(factRefs).map((factId) => (
            <button
              key={factId}
              onClick={() => onFactClick?.(factId)}
              className="flex items-center gap-1 rounded-md border border-primary/50 bg-muted px-2 py-0.5 text-xs text-primary hover:bg-muted"
            >
              <span className="text-muted-foreground">fact</span>
              <span className="font-mono font-medium">{factId}</span>
            </button>
          ))}
        </div>
      )}

      {/* Verdict_present dependencies (recursive) */}
      {depth < 4 && verdictRefs.size > 0 && (
        <div className="ml-4 mt-1 space-y-1">
          {Array.from(verdictRefs).map((ref) => (
            <ProvenanceNode
              key={ref}
              verdictType={ref}
              depth={depth + 1}
              allVerdicts={allVerdicts}
              onFactClick={onFactClick}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// ProvenanceView modal
// ---------------------------------------------------------------------------

export interface ProvenanceViewProps {
  verdict?: VerdictResult;
  stepInfo?: {
    stepId: string;
    stepType: string;
    result: string;
  };
  allVerdicts?: VerdictResult[];
  onClose: () => void;
  onFactClick?: (factId: string) => void;
}

export function ProvenanceView({
  verdict,
  stepInfo,
  allVerdicts = [],
  onClose,
  onFactClick,
}: ProvenanceViewProps) {
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) onClose();
    },
    [onClose]
  );

  const verdictList = verdict
    ? [verdict, ...allVerdicts.filter((v) => v.verdict_type !== verdict.verdict_type)]
    : allVerdicts;

  return (
    <div
      className="fixed inset-0 z-50 flex items-end justify-end bg-black/30"
      onClick={handleBackdropClick}
    >
      <div className="flex h-full w-96 max-w-full flex-col bg-card shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border px-4 py-3">
          <h3 className="font-semibold text-foreground">Provenance</h3>
          <Button
            onClick={onClose}
            variant="ghost"
            size="icon-xs"
          >
            ×
          </Button>
        </div>

        {/* Content */}
        <div className="relative flex-1 overflow-y-auto p-4">
          {verdict && (
            <>
              <div className="mb-3 rounded-md border border-border bg-muted p-2">
                <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Selected verdict
                </div>
                <div className="mt-1 font-mono text-sm font-bold text-foreground">
                  {verdict.verdict_type}
                </div>
                <div className="text-sm text-muted-foreground">
                  = {formatPayload(verdict.payload)}
                </div>
              </div>

              <ProvenanceNode
                verdictType={verdict.verdict_type}
                allVerdicts={verdictList}
                onFactClick={onFactClick}
              />
            </>
          )}

          {stepInfo && (
            <div className="rounded-md border border-border bg-muted p-3">
              <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Step
              </div>
              <div className="mt-1 font-mono text-sm font-bold text-foreground">
                {stepInfo.stepId}
              </div>
              <div className="text-sm text-muted-foreground">
                Type: {stepInfo.stepType} — Result: {stepInfo.result}
              </div>
            </div>
          )}

          {!verdict && !stepInfo && (
            <div className="text-sm text-muted-foreground italic">
              No provenance data available.
            </div>
          )}
        </div>

        {/* Footer hint */}
        <div className="border-t border-border px-4 py-2 text-xs text-muted-foreground">
          Click a fact to highlight it in the input panel. Click outside to close.
        </div>
      </div>
    </div>
  );
}
