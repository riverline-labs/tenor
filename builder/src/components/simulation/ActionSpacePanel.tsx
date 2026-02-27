/**
 * ActionSpacePanel: Per-persona action space display for simulation.
 *
 * Shows available, blocked, and unauthorized actions for the selected persona.
 * Allows triggering flow simulation for operations that appear in flows.
 */
import React, { useCallback, useEffect, useState } from "react";
import { useContractStore } from "@/store/contract";
import { useSimulationStore } from "@/store/simulation";
import type { ActionEntry, BlockedActionEntry } from "@/wasm/evaluator";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Get all operation IDs a persona is authorized for (via allowed_personas). */
function getPersonaOperations(
  persona: string,
  constructs: ReturnType<typeof useContractStore.getState>["bundle"]["constructs"]
): string[] {
  return constructs
    .filter(
      (c) =>
        c.kind === "Operation" &&
        Array.isArray(c.allowed_personas) &&
        c.allowed_personas.includes(persona)
    )
    .map((c) => c.id);
}

/** Find flows that include a given operation ID as a step. */
function findFlowsForOperation(
  operationId: string,
  constructs: ReturnType<typeof useContractStore.getState>["bundle"]["constructs"]
): string[] {
  const flowIds: string[] = [];
  for (const c of constructs) {
    if (c.kind === "Flow") {
      const hasOp = c.steps.some(
        (step) => step.kind === "OperationStep" && step.op === operationId
      );
      if (hasOp) flowIds.push(c.id);
    }
  }
  return flowIds;
}

// ---------------------------------------------------------------------------
// Available action card
// ---------------------------------------------------------------------------

interface AvailableCardProps {
  entry: ActionEntry;
  persona: string;
  onSimulate: (flowId: string) => void;
}

function AvailableCard({ entry, persona, onSimulate }: AvailableCardProps) {
  const constructs = useContractStore((s) => s.bundle.constructs);
  const flows = findFlowsForOperation(entry.operation_id, constructs);

  return (
    <div className="rounded border border-green-200 bg-green-50 p-3">
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="font-mono text-sm font-semibold text-green-800">
            {entry.operation_id}
          </div>
          <div className="text-xs text-green-600">
            persona: {entry.persona || persona}
          </div>
          {entry.instance_bindings && Object.keys(entry.instance_bindings).length > 0 && (
            <div className="mt-0.5 text-xs text-gray-500">
              instances:{" "}
              {Object.entries(entry.instance_bindings)
                .map(([e, ids]) => `${e}: [${ids.join(", ")}]`)
                .join("; ")}
            </div>
          )}
        </div>
        {flows.length > 0 && (
          <div className="flex shrink-0 flex-col gap-1">
            {flows.map((flowId) => (
              <button
                key={flowId}
                onClick={() => onSimulate(flowId)}
                className="rounded border border-green-400 bg-white px-2 py-0.5 text-xs text-green-700 hover:bg-green-100"
                title={`Simulate flow: ${flowId}`}
              >
                Simulate {flowId}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Blocked action card
// ---------------------------------------------------------------------------

function BlockedCard({ entry }: { entry: BlockedActionEntry }) {
  return (
    <div className="rounded border border-gray-200 bg-gray-50 p-3 opacity-75">
      <div className="font-mono text-sm font-medium text-gray-500 line-through">
        {entry.operation_id}
      </div>
      <div className="mt-0.5 text-xs text-red-500">{entry.reason}</div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Unauthorized operation card
// ---------------------------------------------------------------------------

function UnauthorizedCard({ operationId }: { operationId: string }) {
  return (
    <div className="rounded border border-gray-100 px-3 py-1.5 text-sm text-gray-400">
      <span className="font-mono">{operationId}</span>
      <span className="ml-2 text-xs">(not authorized)</span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// ActionSpacePanel
// ---------------------------------------------------------------------------

export function ActionSpacePanel() {
  const personas = useContractStore((s) => s.personas());
  const constructs = useContractStore((s) => s.bundle.constructs);
  const selectedPersona = useSimulationStore((s) => s.selectedPersona);
  const actionSpace = useSimulationStore((s) => s.actionSpace);
  const evaluationError = useSimulationStore((s) => s.evaluationError);
  const setSelectedPersona = useSimulationStore((s) => s.setSelectedPersona);
  const computeActionSpace = useSimulationStore((s) => s.computeActionSpace);
  const simulateFlow = useSimulationStore((s) => s.simulateFlow);
  const flowExecution = useSimulationStore((s) => s.flowExecution);

  const [showUnauthorized, setShowUnauthorized] = useState(false);
  const [isComputing, setIsComputing] = useState(false);

  // When persona changes, compute action space automatically
  const handlePersonaChange = useCallback(
    async (persona: string) => {
      setSelectedPersona(persona);
      setIsComputing(true);
      await computeActionSpace(persona);
      setIsComputing(false);
    },
    [setSelectedPersona, computeActionSpace]
  );

  const handleRefresh = useCallback(async () => {
    if (!selectedPersona) return;
    setIsComputing(true);
    await computeActionSpace(selectedPersona);
    setIsComputing(false);
  }, [selectedPersona, computeActionSpace]);

  const handleSimulate = useCallback(
    async (flowId: string) => {
      if (!selectedPersona) return;
      await simulateFlow(flowId, selectedPersona);
    },
    [selectedPersona, simulateFlow]
  );

  // Compute unauthorized operations: authorized by contract but not in allowed list
  const authorized = actionSpace
    ? [
        ...actionSpace.allowed.map((a) => a.operation_id),
        ...actionSpace.blocked.map((b) => b.operation_id),
      ]
    : [];

  const personaOps = selectedPersona
    ? getPersonaOperations(selectedPersona, constructs)
    : [];

  const unauthorizedOps = personaOps.filter((op) => !authorized.includes(op));

  const available = actionSpace?.allowed ?? [];
  const blocked = actionSpace?.blocked ?? [];

  const summaryText = actionSpace
    ? `${available.length} available, ${blocked.length} blocked, ${unauthorizedOps.length} unauthorized`
    : "—";

  // Auto-select first persona if only one
  useEffect(() => {
    if (!selectedPersona && personas.length === 1) {
      void handlePersonaChange(personas[0].id);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [personas]);

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-gray-200 bg-white px-3 py-2">
        <h2 className="text-sm font-semibold text-gray-700">Action Space</h2>
        {actionSpace && (
          <span className="text-xs text-gray-400">{summaryText}</span>
        )}
      </div>

      {/* Persona selector */}
      <div className="flex items-center gap-2 border-b border-gray-100 bg-gray-50 px-3 py-2">
        <label className="text-xs font-medium text-gray-600">Persona:</label>
        <select
          value={selectedPersona ?? ""}
          onChange={(e) => {
            if (e.target.value) void handlePersonaChange(e.target.value);
          }}
          className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
        >
          <option value="">Select a persona...</option>
          {personas.map((p) => (
            <option key={p.id} value={p.id}>
              {p.id}
            </option>
          ))}
        </select>
        <button
          onClick={handleRefresh}
          disabled={!selectedPersona || isComputing}
          className="rounded border border-gray-200 bg-white px-2 py-1 text-xs text-gray-500 hover:bg-gray-100 disabled:opacity-40"
          title="Refresh action space"
        >
          Refresh
        </button>
      </div>

      {/* Error */}
      {evaluationError && (
        <div className="mx-3 mt-2 rounded border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700">
          {evaluationError}
        </div>
      )}

      {/* Flow simulation result banner */}
      {flowExecution && (flowExecution.outcome || flowExecution.error) && (
        <div
          className={`mx-3 mt-2 rounded border px-3 py-2 text-xs ${
            flowExecution.error
              ? "border-red-200 bg-red-50 text-red-700"
              : "border-blue-200 bg-blue-50 text-blue-700"
          }`}
        >
          {flowExecution.error
            ? `Flow error: ${flowExecution.error}`
            : `Flow "${flowExecution.flowId}" outcome: ${flowExecution.outcome}`}
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-3">
        {!selectedPersona && (
          <div className="flex h-32 items-center justify-center text-sm text-gray-400">
            Select a persona to see their action space.
          </div>
        )}

        {selectedPersona && isComputing && (
          <div className="flex h-20 items-center justify-center text-sm text-gray-400">
            <span className="animate-pulse">Computing action space...</span>
          </div>
        )}

        {selectedPersona && !isComputing && !actionSpace && !evaluationError && (
          <div className="flex h-32 items-center justify-center text-sm text-gray-400">
            Evaluate facts first, then compute action space.
          </div>
        )}

        {actionSpace && !isComputing && (
          <div className="space-y-4">
            {/* Available actions */}
            <section>
              <div className="mb-2 flex items-center gap-2">
                <span className="rounded bg-green-100 px-2 py-0.5 text-xs font-semibold text-green-700">
                  Available ({available.length})
                </span>
              </div>
              {available.length === 0 ? (
                <div className="text-sm text-gray-400 italic">
                  No available actions for this persona.
                </div>
              ) : (
                <div className="space-y-1.5">
                  {available.map((entry, i) => (
                    <AvailableCard
                      key={`${entry.operation_id}-${i}`}
                      entry={entry}
                      persona={selectedPersona ?? ""}
                      onSimulate={handleSimulate}
                    />
                  ))}
                </div>
              )}
            </section>

            {/* Blocked actions */}
            <section>
              <div className="mb-2 flex items-center gap-2">
                <span className="rounded bg-red-100 px-2 py-0.5 text-xs font-semibold text-red-600">
                  Blocked ({blocked.length})
                </span>
              </div>
              {blocked.length === 0 ? (
                <div className="text-sm text-gray-400 italic">
                  No blocked actions.
                </div>
              ) : (
                <div className="space-y-1.5">
                  {blocked.map((entry, i) => (
                    <BlockedCard key={`${entry.operation_id}-${i}`} entry={entry} />
                  ))}
                </div>
              )}
            </section>

            {/* Unauthorized (collapsible) */}
            {unauthorizedOps.length > 0 && (
              <section>
                <button
                  onClick={() => setShowUnauthorized((v) => !v)}
                  className="mb-2 flex items-center gap-1 text-xs text-gray-400 hover:text-gray-600"
                >
                  <span>{showUnauthorized ? "▼" : "▶"}</span>
                  <span className="rounded bg-gray-100 px-2 py-0.5 font-semibold text-gray-500">
                    Unauthorized ({unauthorizedOps.length})
                  </span>
                </button>
                {showUnauthorized && (
                  <div className="space-y-1">
                    {unauthorizedOps.map((op) => (
                      <UnauthorizedCard key={op} operationId={op} />
                    ))}
                  </div>
                )}
              </section>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
