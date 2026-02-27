/**
 * VerdictPanel: Stratum-organized verdict display for simulation results.
 *
 * Groups verdicts by stratum (derived from the producing rule in the contract),
 * shows green/red indicators for truthy/falsy payloads, and opens ProvenanceView
 * when a verdict card is clicked.
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import { useContractStore } from "@/store/contract";
import { useSimulationStore } from "@/store/simulation";
import type { VerdictResult } from "@/store/simulation";
import { ProvenanceView } from "./ProvenanceView";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatPayload(payload: unknown): string {
  if (payload === null || payload === undefined) return "—";
  if (typeof payload === "boolean") return payload ? "true" : "false";
  if (typeof payload === "number" || typeof payload === "string") return String(payload);
  if (typeof payload === "object") {
    const obj = payload as Record<string, unknown>;
    if ("amount" in obj && "currency" in obj) {
      return `${obj.amount} ${obj.currency}`;
    }
    if ("value" in obj) return String(obj.value);
    return JSON.stringify(payload);
  }
  return JSON.stringify(payload);
}

function isTruthyPayload(payload: unknown): boolean {
  if (payload === null || payload === undefined) return false;
  if (typeof payload === "boolean") return payload;
  if (typeof payload === "number") return payload !== 0;
  if (typeof payload === "string") return payload !== "" && payload !== "false";
  return true;
}

function getRuleForVerdict(
  verdictType: string,
  rules: ReturnType<typeof useContractStore.getState>["bundle"]["constructs"]
): { stratum: number; ruleId: string } | null {
  for (const c of rules) {
    if (c.kind === "Rule") {
      if (c.body.produce.verdict_type === verdictType) {
        return { stratum: c.stratum, ruleId: c.id };
      }
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Verdict card
// ---------------------------------------------------------------------------

interface VerdictCardProps {
  verdict: VerdictResult;
  ruleInfo: { stratum: number; ruleId: string } | null;
  onClick: () => void;
}

function VerdictCard({ verdict, ruleInfo, onClick }: VerdictCardProps) {
  const truthy = isTruthyPayload(verdict.payload);
  return (
    <button
      onClick={onClick}
      className="group flex w-full items-start gap-3 rounded border border-gray-100 bg-white p-3 text-left hover:border-blue-200 hover:bg-blue-50 transition-colors"
    >
      {/* Indicator */}
      <div
        className={`mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-xs font-bold ${
          truthy
            ? "bg-green-100 text-green-700"
            : "bg-red-100 text-red-600"
        }`}
      >
        {truthy ? "✓" : "✗"}
      </div>

      {/* Content */}
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-2">
          <span className="font-mono text-sm font-medium text-gray-800 truncate">
            {verdict.verdict_type}
          </span>
          <span
            className={`shrink-0 text-sm font-medium ${
              truthy ? "text-green-700" : "text-red-600"
            }`}
          >
            {formatPayload(verdict.payload)}
          </span>
        </div>
        {ruleInfo && (
          <div className="mt-0.5 text-xs text-gray-400">
            rule: {ruleInfo.ruleId}
          </div>
        )}
      </div>

      {/* Provenance hint */}
      <span className="shrink-0 text-xs text-gray-300 group-hover:text-blue-400">
        trace →
      </span>
    </button>
  );
}

// ---------------------------------------------------------------------------
// Auto-evaluate debounce hook
// ---------------------------------------------------------------------------

function useAutoEvaluate(enabled: boolean) {
  const evaluate = useSimulationStore((s) => s.evaluate);
  const factValues = useSimulationStore((s) => s.factValues);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!enabled) return;
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      void evaluate();
    }, 600);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [factValues, enabled]);
}

// ---------------------------------------------------------------------------
// VerdictPanel
// ---------------------------------------------------------------------------

export function VerdictPanel() {
  const verdicts = useSimulationStore((s) => s.verdicts);
  const evaluationError = useSimulationStore((s) => s.evaluationError);
  const isEvaluating = useSimulationStore((s) => s.isEvaluating);
  const constructs = useContractStore((s) => s.bundle.constructs);

  const [selectedVerdict, setSelectedVerdict] = useState<VerdictResult | null>(null);
  const [autoEvaluate, setAutoEvaluate] = useState(false);

  useAutoEvaluate(autoEvaluate);

  // Group verdicts by stratum
  const grouped = React.useMemo(() => {
    if (!verdicts) return new Map<number, Array<{ verdict: VerdictResult; ruleInfo: { stratum: number; ruleId: string } | null }>>();
    const map = new Map<number, Array<{ verdict: VerdictResult; ruleInfo: { stratum: number; ruleId: string } | null }>>();
    for (const v of verdicts) {
      const ruleInfo = getRuleForVerdict(v.verdict_type, constructs);
      const stratum = ruleInfo?.stratum ?? 0;
      const existing = map.get(stratum) ?? [];
      existing.push({ verdict: v, ruleInfo });
      map.set(stratum, existing);
    }
    return map;
  }, [verdicts, constructs]);

  const stratumKeys = Array.from(grouped.keys()).sort((a, b) => a - b);

  const handleVerdictClick = useCallback((v: VerdictResult) => {
    setSelectedVerdict(v);
  }, []);

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-gray-200 bg-white px-3 py-2">
        <h2 className="text-sm font-semibold text-gray-700">Verdicts</h2>
        <div className="flex items-center gap-3">
          <label className="flex cursor-pointer items-center gap-1.5 text-xs text-gray-500">
            <input
              type="checkbox"
              checked={autoEvaluate}
              onChange={(e) => setAutoEvaluate(e.target.checked)}
              className="h-3 w-3"
            />
            Auto-evaluate
          </label>
          {verdicts && (
            <span className="text-xs text-gray-400">
              {verdicts.length} verdict{verdicts.length !== 1 ? "s" : ""}
            </span>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-3">
        {/* Error state */}
        {evaluationError && !isEvaluating && (
          <div className="rounded border border-red-200 bg-red-50 p-3 text-sm text-red-700">
            <strong>Evaluation error:</strong> {evaluationError}
          </div>
        )}

        {/* Loading */}
        {isEvaluating && (
          <div className="flex h-20 items-center justify-center text-sm text-gray-400">
            <span className="animate-pulse">Evaluating...</span>
          </div>
        )}

        {/* Empty state */}
        {!isEvaluating && !evaluationError && verdicts === null && (
          <div className="flex h-32 items-center justify-center text-center text-sm text-gray-400">
            Fill in fact values and click Evaluate to see verdicts.
          </div>
        )}

        {/* No verdicts produced */}
        {!isEvaluating && verdicts !== null && verdicts.length === 0 && (
          <div className="flex h-32 items-center justify-center text-center text-sm text-gray-400">
            No verdicts produced with these fact values.
          </div>
        )}

        {/* Grouped verdicts */}
        {stratumKeys.map((stratum) => {
          const entries = grouped.get(stratum) ?? [];
          return (
            <section key={stratum} className="mb-4">
              <div className="mb-2 flex items-center gap-2">
                <span className="rounded bg-purple-100 px-2 py-0.5 text-xs font-semibold text-purple-700">
                  Stratum {stratum}
                </span>
                <span className="text-xs text-gray-400">
                  {entries.length} verdict{entries.length !== 1 ? "s" : ""}
                </span>
              </div>
              <div className="space-y-1.5">
                {entries.map(({ verdict, ruleInfo }, i) => (
                  <VerdictCard
                    key={`${verdict.verdict_type}-${i}`}
                    verdict={verdict}
                    ruleInfo={ruleInfo}
                    onClick={() => handleVerdictClick(verdict)}
                  />
                ))}
              </div>
            </section>
          );
        })}
      </div>

      {/* ProvenanceView modal */}
      {selectedVerdict && (
        <ProvenanceView
          verdict={selectedVerdict}
          onClose={() => setSelectedVerdict(null)}
        />
      )}
    </div>
  );
}
