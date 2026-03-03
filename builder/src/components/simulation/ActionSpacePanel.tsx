/**
 * ActionSpacePanel: Per-persona action space display for simulation.
 *
 * Shows available, blocked, and unauthorized actions for the selected persona.
 * Allows triggering flow simulation for operations that appear in flows.
 */
import React, { useCallback, useEffect, useState } from "react";
import { useContractStore, usePersonas } from "@/store/contract";
import { useSimulationStore } from "@/store/simulation";
import type { ActionEntry, BlockedActionEntry } from "@/wasm/evaluator";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

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
    <div className="rounded-md border border-primary/30 bg-primary/10 p-3">
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="font-mono text-sm font-semibold text-primary">
            {entry.operation_id}
          </div>
          <div className="text-xs text-primary">
            persona: {entry.persona || persona}
          </div>
          {entry.instance_bindings && Object.keys(entry.instance_bindings).length > 0 && (
            <div className="mt-0.5 text-xs text-muted-foreground">
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
              <Button
                key={flowId}
                onClick={() => onSimulate(flowId)}
                variant="outline"
                size="xs"
                title={`Simulate flow: ${flowId}`}
              >
                Simulate {flowId}
              </Button>
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
    <div className="rounded-md border border-border bg-muted p-3 opacity-75">
      <div className="font-mono text-sm font-medium text-muted-foreground line-through">
        {entry.operation_id}
      </div>
      <div className="mt-0.5 text-xs text-destructive">{entry.reason}</div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Unauthorized operation card
// ---------------------------------------------------------------------------

function UnauthorizedCard({ operationId }: { operationId: string }) {
  return (
    <div className="rounded-md border border-border px-3 py-1.5 text-sm text-muted-foreground">
      <span className="font-mono">{operationId}</span>
      <span className="ml-2 text-xs">(not authorized)</span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// ActionSpacePanel
// ---------------------------------------------------------------------------

export function ActionSpacePanel() {
  const personas = usePersonas();
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
      <div className="flex items-center justify-between border-b border-border bg-card px-3 py-2">
        <h2 className="text-sm font-semibold text-foreground">Action Space</h2>
        {actionSpace && (
          <span className="text-xs text-muted-foreground">{summaryText}</span>
        )}
      </div>

      {/* Persona selector */}
      <div className="flex items-center gap-2 border-b border-border bg-muted px-3 py-2">
        <label className="text-xs font-medium text-muted-foreground">Persona:</label>
        <select
          value={selectedPersona ?? ""}
          onChange={(e) => {
            if (e.target.value) void handlePersonaChange(e.target.value);
          }}
          className="flex-1 rounded-md border border-border px-2 py-1 text-sm"
        >
          <option value="">Select a persona...</option>
          {personas.map((p) => (
            <option key={p.id} value={p.id}>
              {p.id}
            </option>
          ))}
        </select>
        <Button
          onClick={handleRefresh}
          disabled={!selectedPersona || isComputing}
          variant="outline"
          size="xs"
          title="Refresh action space"
        >
          Refresh
        </Button>
      </div>

      {/* Error */}
      {evaluationError && (
        <div className="mx-3 mt-2 rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-xs text-destructive">
          {evaluationError}
        </div>
      )}

      {/* Flow simulation result banner */}
      {flowExecution && (flowExecution.outcome || flowExecution.error) && (
        <div
          className={`mx-3 mt-2 rounded-md border px-3 py-2 text-xs ${
            flowExecution.error
              ? "border-destructive/50 bg-destructive/10 text-destructive"
              : "border-primary/50 bg-muted text-primary"
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
          <div className="flex h-32 items-center justify-center text-sm text-muted-foreground">
            Select a persona to see their action space.
          </div>
        )}

        {selectedPersona && isComputing && (
          <div className="flex h-20 items-center justify-center text-sm text-muted-foreground">
            <span className="animate-pulse">Computing action space...</span>
          </div>
        )}

        {selectedPersona && !isComputing && !actionSpace && !evaluationError && (
          <div className="flex h-32 items-center justify-center text-sm text-muted-foreground">
            Evaluate facts first, then compute action space.
          </div>
        )}

        {actionSpace && !isComputing && (
          <div className="space-y-4">
            {/* Available actions */}
            <section>
              <div className="mb-2 flex items-center gap-2">
                <Badge variant="secondary" className="bg-primary/10 text-primary">
                  Available ({available.length})
                </Badge>
              </div>
              {available.length === 0 ? (
                <div className="text-sm text-muted-foreground italic">
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
                <Badge variant="destructive">
                  Blocked ({blocked.length})
                </Badge>
              </div>
              {blocked.length === 0 ? (
                <div className="text-sm text-muted-foreground italic">
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
                  className="mb-2 flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
                >
                  <span>{showUnauthorized ? "▼" : "▶"}</span>
                  <Badge variant="secondary">
                    Unauthorized ({unauthorizedOps.length})
                  </Badge>
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
