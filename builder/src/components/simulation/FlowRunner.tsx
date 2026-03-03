/**
 * FlowRunner: Step-by-step flow execution with DAG highlighting.
 *
 * Selects a flow and persona, runs the WASM simulation, then lets the user
 * step through the path one node at a time with the FlowDag highlighting
 * the current step.  Entity state changes and step history are shown below.
 */
import React, { useCallback, useState } from "react";
import { useFlows, usePersonas } from "@/store/contract";
import { useSimulationStore } from "@/store/simulation";
import { FlowDag } from "@/components/visualizations/FlowDag";
import { ProvenanceView } from "./ProvenanceView";
import type { StepResult } from "@/store/simulation";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

// ---------------------------------------------------------------------------
// Step history row
// ---------------------------------------------------------------------------

interface StepRowProps {
  step: StepResult;
  isActive: boolean;
  onProvenanceClick: (step: StepResult) => void;
}

function StepRow({ step, isActive, onProvenanceClick }: StepRowProps) {
  const stepTypeColor: Record<string, string> = {
    OperationStep: "bg-muted text-primary",
    BranchStep: "bg-muted text-primary",
    HandoffStep: "bg-primary/10 text-primary",
    SubFlowStep: "bg-primary/10 text-primary",
    ParallelStep: "bg-muted text-primary",
  };
  const typeClass =
    stepTypeColor[step.step_type] ?? "bg-muted text-muted-foreground";

  return (
    <div
      className={`flex items-center gap-3 rounded-md border px-3 py-2 ${
        isActive
          ? "border-primary/50 bg-muted"
          : "border-border bg-card"
      }`}
    >
      <div className={`shrink-0 rounded-md px-1.5 py-0.5 text-xs font-medium ${typeClass}`}>
        {step.step_type.replace("Step", "")}
      </div>
      <div className="min-w-0 flex-1">
        <div className="font-mono text-sm text-foreground">{step.step_id}</div>
        <div className="text-xs text-muted-foreground">result: {step.result}</div>
        {step.instance_bindings && Object.keys(step.instance_bindings).length > 0 && (
          <div className="text-xs text-muted-foreground">
            instances:{" "}
            {Object.entries(step.instance_bindings)
              .map(([e, id]) => `${e}=${id}`)
              .join(", ")}
          </div>
        )}
      </div>
      <button
        onClick={() => onProvenanceClick(step)}
        className="shrink-0 text-xs text-muted-foreground hover:text-primary"
        title="View provenance"
      >
        trace
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// EntityTransition summary row
// ---------------------------------------------------------------------------

function TransitionRow({
  entityId,
  instanceId,
  fromState,
  toState,
}: {
  entityId: string;
  instanceId: string;
  fromState: string;
  toState: string;
}) {
  return (
    <div className="flex items-center gap-2 rounded-md border border-border bg-card px-3 py-1.5 text-sm">
      <span className="font-mono font-medium text-foreground">{entityId}</span>
      {instanceId !== "_default" && (
        <span className="text-xs text-muted-foreground">[{instanceId}]</span>
      )}
      <span className="text-muted-foreground">:</span>
      <span className="rounded-md bg-muted px-1.5 text-xs text-muted-foreground">
        {fromState}
      </span>
      <span className="text-muted-foreground">→</span>
      <span className="rounded-md bg-muted px-1.5 text-xs text-primary">
        {toState}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// FlowRunner main component
// ---------------------------------------------------------------------------

export function FlowRunner() {
  const flows = useFlows();
  const personas = usePersonas();

  const flowExecution = useSimulationStore((s) => s.flowExecution);
  const simulateFlow = useSimulationStore((s) => s.simulateFlow);
  const stepFlowForward = useSimulationStore((s) => s.stepFlowForward);
  const resetFlowPlayback = useSimulationStore((s) => s.resetFlowPlayback);
  const verdicts = useSimulationStore((s) => s.verdicts);

  const [selectedFlowId, setSelectedFlowId] = useState<string>("");
  const [selectedPersona, setSelectedPersona] = useState<string>("");
  const [provenanceStep, setProvenanceStep] = useState<StepResult | null>(null);

  const selectedFlow = flows.find((f) => f.id === selectedFlowId);

  const handleStart = useCallback(async () => {
    if (!selectedFlowId || !selectedPersona) return;
    await simulateFlow(selectedFlowId, selectedPersona);
  }, [selectedFlowId, selectedPersona, simulateFlow]);

  const handleStep = useCallback(() => {
    stepFlowForward();
  }, [stepFlowForward]);

  const handleRunToEnd = useCallback(() => {
    if (!flowExecution?.fullResult) return;
    const total = flowExecution.fullResult.path?.length ?? 0;
    let remaining = total - (flowExecution.stepsExecuted.length ?? 0);
    while (remaining > 0) {
      stepFlowForward();
      remaining--;
    }
  }, [flowExecution, stepFlowForward]);

  const handleReset = useCallback(() => {
    resetFlowPlayback();
  }, [resetFlowPlayback]);

  const currentStepId = flowExecution?.currentStepId ?? undefined;
  const isRunning = flowExecution?.running ?? false;
  const isComplete = flowExecution?.isComplete ?? false;
  const allStepsTotal = flowExecution?.fullResult?.path?.length ?? 0;
  const stepsShown = flowExecution?.stepsExecuted.length ?? 0;

  return (
    <div className="flex h-full flex-col">
      {/* Controls header */}
      <div className="flex flex-wrap items-center gap-3 border-b border-border bg-card px-3 py-2">
        {/* Flow selector */}
        <div className="flex items-center gap-1">
          <label className="text-xs font-medium text-muted-foreground">Flow:</label>
          <select
            value={selectedFlowId}
            onChange={(e) => setSelectedFlowId(e.target.value)}
            className="rounded-md border border-border px-2 py-1 text-sm"
            disabled={isRunning}
          >
            <option value="">Select flow...</option>
            {flows.map((f) => (
              <option key={f.id} value={f.id}>
                {f.id}
              </option>
            ))}
          </select>
        </div>

        {/* Persona selector */}
        <div className="flex items-center gap-1">
          <label className="text-xs font-medium text-muted-foreground">Persona:</label>
          <select
            value={selectedPersona}
            onChange={(e) => setSelectedPersona(e.target.value)}
            className="rounded-md border border-border px-2 py-1 text-sm"
            disabled={isRunning}
          >
            <option value="">Select persona...</option>
            {personas.map((p) => (
              <option key={p.id} value={p.id}>
                {p.id}
              </option>
            ))}
          </select>
        </div>

        {/* Action buttons */}
        <div className="flex items-center gap-1">
          <Button
            onClick={handleStart}
            disabled={!selectedFlowId || !selectedPersona || isRunning}
            size="sm"
          >
            {isRunning ? "Running..." : "Start"}
          </Button>

          {flowExecution && !isRunning && (
            <>
              <Button
                onClick={handleStep}
                disabled={isComplete}
                variant="outline"
                size="sm"
              >
                Step
              </Button>
              <Button
                onClick={handleRunToEnd}
                disabled={isComplete}
                variant="outline"
                size="sm"
              >
                Run to End
              </Button>
              <Button
                onClick={handleReset}
                variant="ghost"
                size="sm"
              >
                Reset
              </Button>
            </>
          )}
        </div>

        {/* Progress */}
        {flowExecution && !isRunning && (
          <span className="ml-auto text-xs text-muted-foreground">
            {stepsShown}/{allStepsTotal} steps
          </span>
        )}
      </div>

      {/* Main body: DAG + sidebar */}
      <div className="flex flex-1 overflow-hidden">
        {/* DAG visualization */}
        <div className="flex flex-1 flex-col overflow-hidden">
          {selectedFlow ? (
            <div className="flex-1 overflow-auto p-3">
              <FlowDag
                steps={selectedFlow.steps}
                entry={selectedFlow.entry}
                highlightedStep={currentStepId}
                editable={false}
              />
            </div>
          ) : (
            <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
              Select a flow to see the DAG.
            </div>
          )}

          {/* Error */}
          {flowExecution?.error && (
            <div className="mx-3 mb-2 rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {flowExecution.error}
            </div>
          )}

          {/* Step history */}
          {flowExecution && !isRunning && (
            <div className="border-t border-border bg-muted">
              <div className="flex items-center justify-between px-3 py-2">
                <h3 className="text-xs font-semibold text-muted-foreground">
                  Step History
                </h3>
                {isComplete && flowExecution.outcome && (
                  <Badge
                    variant={
                      flowExecution.outcome === "success" ||
                      flowExecution.outcome === "completed"
                        ? "default"
                        : "destructive"
                    }
                  >
                    Outcome: {flowExecution.outcome}
                  </Badge>
                )}
              </div>
              <div className="max-h-40 space-y-1 overflow-y-auto px-3 pb-2">
                {flowExecution.stepsExecuted.map((step, i) => (
                  <StepRow
                    key={`${step.step_id}-${i}`}
                    step={step}
                    isActive={step.step_id === currentStepId}
                    onProvenanceClick={(s) => setProvenanceStep(s)}
                  />
                ))}
                {flowExecution.stepsExecuted.length === 0 && (
                  <div className="text-xs text-muted-foreground italic">
                    No steps executed yet.
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Right sidebar: current state + entity transitions */}
        {flowExecution && !isRunning && (
          <div className="w-56 shrink-0 overflow-y-auto border-l border-border bg-card p-3">
            {/* Current step info */}
            {currentStepId && (
              <section className="mb-4">
                <h4 className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Current Step
                </h4>
                <div className="rounded-md border border-primary/50 bg-muted p-2">
                  <div className="font-mono text-sm font-semibold text-primary">
                    {currentStepId}
                  </div>
                  {flowExecution.stepsExecuted.find(
                    (s) => s.step_id === currentStepId
                  ) && (
                    <div className="mt-0.5 text-xs text-primary">
                      {
                        flowExecution.stepsExecuted.find(
                          (s) => s.step_id === currentStepId
                        )?.step_type
                      }
                    </div>
                  )}
                </div>
              </section>
            )}

            {/* Entity transitions */}
            {flowExecution.entityStateChanges.length > 0 && (
              <section className="mb-4">
                <h4 className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Entity Transitions
                </h4>
                <div className="space-y-1">
                  {flowExecution.entityStateChanges.map((t, i) => (
                    <TransitionRow
                      key={i}
                      entityId={t.entity_id}
                      instanceId={t.instance_id}
                      fromState={t.from_state}
                      toState={t.to_state}
                    />
                  ))}
                </div>
              </section>
            )}

            {/* Outcome */}
            {isComplete && flowExecution.outcome && (
              <section>
                <h4 className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Final Outcome
                </h4>
                <div
                  className={`rounded-md border px-3 py-2 text-center text-sm font-bold ${
                    flowExecution.outcome === "success" ||
                    flowExecution.outcome === "completed"
                      ? "border-primary/30 bg-primary/10 text-primary"
                      : "border-destructive/50 bg-destructive/10 text-destructive"
                  }`}
                >
                  {flowExecution.outcome}
                </div>
              </section>
            )}
          </div>
        )}
      </div>

      {/* Provenance modal for step */}
      {provenanceStep && (
        <ProvenanceView
          stepInfo={{
            stepId: provenanceStep.step_id,
            stepType: provenanceStep.step_type,
            result: provenanceStep.result,
          }}
          allVerdicts={verdicts ?? []}
          onClose={() => setProvenanceStep(null)}
        />
      )}
    </div>
  );
}
