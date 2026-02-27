/**
 * FlowRunner: Step-by-step flow execution with DAG highlighting.
 *
 * Selects a flow and persona, runs the WASM simulation, then lets the user
 * step through the path one node at a time with the FlowDag highlighting
 * the current step.  Entity state changes and step history are shown below.
 */
import React, { useCallback, useState } from "react";
import { useContractStore } from "@/store/contract";
import { useSimulationStore } from "@/store/simulation";
import { FlowDag } from "@/components/visualizations/FlowDag";
import { ProvenanceView } from "./ProvenanceView";
import type { StepResult } from "@/store/simulation";

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
    OperationStep: "bg-blue-100 text-blue-700",
    BranchStep: "bg-orange-100 text-orange-700",
    HandoffStep: "bg-green-100 text-green-700",
    SubFlowStep: "bg-purple-100 text-purple-700",
    ParallelStep: "bg-indigo-100 text-indigo-700",
  };
  const typeClass =
    stepTypeColor[step.step_type] ?? "bg-gray-100 text-gray-600";

  return (
    <div
      className={`flex items-center gap-3 rounded border px-3 py-2 ${
        isActive
          ? "border-purple-300 bg-purple-50"
          : "border-gray-100 bg-white"
      }`}
    >
      <div className={`shrink-0 rounded px-1.5 py-0.5 text-xs font-medium ${typeClass}`}>
        {step.step_type.replace("Step", "")}
      </div>
      <div className="min-w-0 flex-1">
        <div className="font-mono text-sm text-gray-800">{step.step_id}</div>
        <div className="text-xs text-gray-400">result: {step.result}</div>
        {step.instance_bindings && Object.keys(step.instance_bindings).length > 0 && (
          <div className="text-xs text-gray-400">
            instances:{" "}
            {Object.entries(step.instance_bindings)
              .map(([e, id]) => `${e}=${id}`)
              .join(", ")}
          </div>
        )}
      </div>
      <button
        onClick={() => onProvenanceClick(step)}
        className="shrink-0 text-xs text-gray-300 hover:text-blue-500"
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
    <div className="flex items-center gap-2 rounded border border-gray-100 bg-white px-3 py-1.5 text-sm">
      <span className="font-mono font-medium text-gray-700">{entityId}</span>
      {instanceId !== "_default" && (
        <span className="text-xs text-gray-400">[{instanceId}]</span>
      )}
      <span className="text-gray-400">:</span>
      <span className="rounded bg-gray-100 px-1.5 text-xs text-gray-600">
        {fromState}
      </span>
      <span className="text-gray-400">→</span>
      <span className="rounded bg-blue-100 px-1.5 text-xs text-blue-700">
        {toState}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// FlowRunner main component
// ---------------------------------------------------------------------------

export function FlowRunner() {
  const flows = useContractStore((s) => s.flows());
  const personas = useContractStore((s) => s.personas());

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
      <div className="flex flex-wrap items-center gap-3 border-b border-gray-200 bg-white px-3 py-2">
        {/* Flow selector */}
        <div className="flex items-center gap-1">
          <label className="text-xs font-medium text-gray-600">Flow:</label>
          <select
            value={selectedFlowId}
            onChange={(e) => setSelectedFlowId(e.target.value)}
            className="rounded border border-gray-300 px-2 py-1 text-sm"
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
          <label className="text-xs font-medium text-gray-600">Persona:</label>
          <select
            value={selectedPersona}
            onChange={(e) => setSelectedPersona(e.target.value)}
            className="rounded border border-gray-300 px-2 py-1 text-sm"
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
          <button
            onClick={handleStart}
            disabled={!selectedFlowId || !selectedPersona || isRunning}
            className="rounded bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-40"
          >
            {isRunning ? "Running..." : "Start"}
          </button>

          {flowExecution && !isRunning && (
            <>
              <button
                onClick={handleStep}
                disabled={isComplete}
                className="rounded border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-40"
              >
                Step
              </button>
              <button
                onClick={handleRunToEnd}
                disabled={isComplete}
                className="rounded border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-40"
              >
                Run to End
              </button>
              <button
                onClick={handleReset}
                className="rounded border border-gray-200 bg-gray-50 px-3 py-1.5 text-sm text-gray-500 hover:bg-gray-100"
              >
                Reset
              </button>
            </>
          )}
        </div>

        {/* Progress */}
        {flowExecution && !isRunning && (
          <span className="ml-auto text-xs text-gray-400">
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
            <div className="flex flex-1 items-center justify-center text-sm text-gray-400">
              Select a flow to see the DAG.
            </div>
          )}

          {/* Error */}
          {flowExecution?.error && (
            <div className="mx-3 mb-2 rounded border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
              {flowExecution.error}
            </div>
          )}

          {/* Step history */}
          {flowExecution && !isRunning && (
            <div className="border-t border-gray-200 bg-gray-50">
              <div className="flex items-center justify-between px-3 py-2">
                <h3 className="text-xs font-semibold text-gray-500">
                  Step History
                </h3>
                {isComplete && flowExecution.outcome && (
                  <span
                    className={`rounded px-2 py-0.5 text-xs font-bold ${
                      flowExecution.outcome === "success" ||
                      flowExecution.outcome === "completed"
                        ? "bg-green-100 text-green-700"
                        : "bg-red-100 text-red-600"
                    }`}
                  >
                    Outcome: {flowExecution.outcome}
                  </span>
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
                  <div className="text-xs text-gray-400 italic">
                    No steps executed yet.
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Right sidebar: current state + entity transitions */}
        {flowExecution && !isRunning && (
          <div className="w-56 shrink-0 overflow-y-auto border-l border-gray-200 bg-white p-3">
            {/* Current step info */}
            {currentStepId && (
              <section className="mb-4">
                <h4 className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-gray-400">
                  Current Step
                </h4>
                <div className="rounded border border-purple-200 bg-purple-50 p-2">
                  <div className="font-mono text-sm font-semibold text-purple-800">
                    {currentStepId}
                  </div>
                  {flowExecution.stepsExecuted.find(
                    (s) => s.step_id === currentStepId
                  ) && (
                    <div className="mt-0.5 text-xs text-purple-600">
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
                <h4 className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-gray-400">
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
                <h4 className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-gray-400">
                  Final Outcome
                </h4>
                <div
                  className={`rounded border px-3 py-2 text-center text-sm font-bold ${
                    flowExecution.outcome === "success" ||
                    flowExecution.outcome === "completed"
                      ? "border-green-200 bg-green-50 text-green-700"
                      : "border-red-200 bg-red-50 text-red-600"
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
