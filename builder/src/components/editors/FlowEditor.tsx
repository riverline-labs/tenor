/**
 * FlowEditor: CRUD editor for Flow constructs.
 *
 * Layout:
 * - Left panel: flow list with add button
 * - Right panel (flow selected):
 *   - Flow metadata (ID, snapshot, entry)
 *   - DAG visualization (FlowDag)
 *   - Step management toolbar
 *   - Step detail editor (shown when step selected)
 *   - Validation panel
 *
 * Supports all five step types:
 * OperationStep, BranchStep, ParallelStep, SubFlowStep, HandoffStep
 */
import React, { useState, useMemo } from "react";
import {
  useContractStore,
  selectFlows,
  selectOperations,
  selectPersonas,
  selectFacts,
  selectRules,
} from "@/store/contract";
import type {
  FlowConstruct,
  FlowStep,
  OperationStep,
  BranchStep,
  HandoffStep,
  SubFlowStep,
  ParallelStep,
  StepTarget,
  TerminalTarget,
  FailureHandler,
  TerminateHandler,
  CompensateHandler,
  EscalateHandler,
  CompensationStep,
  ParallelBranch,
  JoinPolicy,
  PredicateExpression,
  OperationConstruct,
  PersonaConstruct,
} from "@/types/interchange";
import { FlowDag } from "@/components/visualizations/FlowDag";
import { PredicateBuilder } from "@/components/shared/PredicateBuilder";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function newFlow(id: string): FlowConstruct {
  return {
    kind: "Flow",
    id,
    snapshot: "at_initiation",
    entry: "",
    steps: [],
    tenor: TENOR_VERSION,
    provenance: { file: "builder", line: 0 },
  };
}

function defaultTerminate(outcome = "failure"): TerminateHandler {
  return { kind: "Terminate", outcome };
}

function defaultOperationStep(id: string, operations: OperationConstruct[]): OperationStep {
  const firstOp = operations[0];
  return {
    kind: "OperationStep",
    id,
    op: firstOp?.id ?? "",
    persona: firstOp?.allowed_personas[0] ?? "",
    outcomes: { success: { kind: "Terminal", outcome: "success" } },
    on_failure: defaultTerminate("failure"),
  };
}

function defaultBranchStep(id: string, facts: { id: string }[]): BranchStep {
  const firstFact = facts[0];
  return {
    kind: "BranchStep",
    id,
    condition: {
      left: { fact_ref: firstFact?.id ?? "fact" },
      op: "=",
      right: { literal: true, type: { base: "Bool" } },
    } as PredicateExpression,
    persona: "",
    if_true: { kind: "Terminal", outcome: "success" },
    if_false: { kind: "Terminal", outcome: "failure" },
  };
}

function defaultHandoffStep(id: string): HandoffStep {
  return {
    kind: "HandoffStep",
    id,
    from_persona: "",
    to_persona: "",
    next: "",
  };
}

function defaultSubFlowStep(id: string, flows: FlowConstruct[]): SubFlowStep {
  return {
    kind: "SubFlowStep",
    id,
    flow: flows[0]?.id ?? "",
    persona: "",
    on_success: { kind: "Terminal", outcome: "success" },
    on_failure: defaultTerminate("failure"),
  };
}

function defaultParallelStep(id: string): ParallelStep {
  const branch1: ParallelBranch = { id: "branch_a", entry: "", steps: [] };
  const branch2: ParallelBranch = { id: "branch_b", entry: "", steps: [] };
  const join: JoinPolicy = {
    on_all_success: { kind: "Terminal", outcome: "success" },
    on_any_failure: defaultTerminate("failure"),
  };
  return {
    kind: "ParallelStep",
    id,
    branches: [branch1, branch2],
    join,
  };
}

function uniqueStepId(base: string, steps: FlowStep[]): string {
  const ids = new Set(steps.map((s) => s.id));
  let id = base;
  let i = 1;
  while (ids.has(id)) id = `${base}_${i++}`;
  return id;
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

interface FlowValidation {
  errors: string[];
  warnings: string[];
}

function detectCycle(steps: FlowStep[], entry: string): boolean {
  // DFS cycle detection
  const adj = new Map<string, string[]>();
  for (const step of steps) {
    const nexts: string[] = [];
    if (step.kind === "OperationStep") {
      for (const t of Object.values(step.outcomes)) {
        if (typeof t === "string") nexts.push(t);
      }
    } else if (step.kind === "BranchStep") {
      if (typeof step.if_true === "string") nexts.push(step.if_true);
      if (typeof step.if_false === "string") nexts.push(step.if_false);
    } else if (step.kind === "HandoffStep") {
      if (step.next) nexts.push(step.next);
    } else if (step.kind === "SubFlowStep") {
      if (typeof step.on_success === "string") nexts.push(step.on_success);
    } else if (step.kind === "ParallelStep") {
      if (typeof step.join.on_all_success === "string") nexts.push(step.join.on_all_success);
    }
    adj.set(step.id, nexts.filter((id) => steps.some((s) => s.id === id)));
  }

  const visited = new Set<string>();
  const stack = new Set<string>();

  function dfs(node: string): boolean {
    if (stack.has(node)) return true;
    if (visited.has(node)) return false;
    visited.add(node);
    stack.add(node);
    for (const next of adj.get(node) ?? []) {
      if (dfs(next)) return true;
    }
    stack.delete(node);
    return false;
  }

  return entry ? dfs(entry) : false;
}

function validateFlow(flow: FlowConstruct, operations: OperationConstruct[], personas: PersonaConstruct[]): FlowValidation {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (!flow.entry) {
    errors.push("Entry step must be set.");
  } else if (!flow.steps.some((s) => s.id === flow.entry)) {
    errors.push(`Entry step "${flow.entry}" does not exist in this flow.`);
  }

  if (detectCycle(flow.steps, flow.entry)) {
    errors.push("Flow contains a cycle. DAG must be acyclic.");
  }

  // Check all steps reachable from entry
  if (flow.entry) {
    const reachable = new Set<string>();
    const queue = [flow.entry];
    while (queue.length > 0) {
      const cur = queue.shift()!;
      if (reachable.has(cur)) continue;
      reachable.add(cur);
      const step = flow.steps.find((s) => s.id === cur);
      if (!step) continue;
      const nexts: string[] = [];
      if (step.kind === "OperationStep") {
        for (const t of Object.values(step.outcomes)) {
          if (typeof t === "string") nexts.push(t);
        }
      } else if (step.kind === "BranchStep") {
        if (typeof step.if_true === "string") nexts.push(step.if_true);
        if (typeof step.if_false === "string") nexts.push(step.if_false);
      } else if (step.kind === "HandoffStep") {
        if (step.next) nexts.push(step.next);
      } else if (step.kind === "SubFlowStep") {
        if (typeof step.on_success === "string") nexts.push(step.on_success);
      } else if (step.kind === "ParallelStep") {
        if (typeof step.join.on_all_success === "string") nexts.push(step.join.on_all_success);
      }
      for (const n of nexts) queue.push(n);
    }
    const unreachable = flow.steps.filter((s) => !reachable.has(s.id));
    if (unreachable.length > 0) {
      warnings.push(`Unreachable steps: ${unreachable.map((s) => s.id).join(", ")}`);
    }
  }

  // Check OperationStep references
  for (const step of flow.steps) {
    if (step.kind === "OperationStep") {
      if (!operations.some((o) => o.id === step.op)) {
        errors.push(`Step "${step.id}": operation "${step.op}" not found in contract.`);
      } else {
        const op = operations.find((o) => o.id === step.op)!;
        if (step.persona && !op.allowed_personas.includes(step.persona)) {
          warnings.push(`Step "${step.id}": persona "${step.persona}" not in operation's allowed_personas.`);
        }
      }
    }
  }

  return { errors, warnings };
}

// ---------------------------------------------------------------------------
// StepTarget editor (select next step or terminal)
// ---------------------------------------------------------------------------

interface StepTargetEditorProps {
  value: StepTarget;
  onChange: (t: StepTarget) => void;
  stepIds: string[];
  label: string;
  selfId?: string;
}

function StepTargetEditor({ value, onChange, stepIds, label, selfId }: StepTargetEditorProps) {
  const isTerminal = typeof value === "object" && value !== null;
  const terminalOutcome = isTerminal ? (value as TerminalTarget).outcome : "";
  const stepId = typeof value === "string" ? value : "";
  const mode = isTerminal ? "terminal" : "step";

  return (
    <div className="flex items-center gap-1">
      <span className="w-20 flex-shrink-0 text-xs text-gray-500">{label}:</span>
      <select
        value={mode}
        onChange={(e) => {
          if (e.target.value === "terminal") {
            onChange({ kind: "Terminal", outcome: "success" } as TerminalTarget);
          } else {
            onChange(stepIds.find((id) => id !== selfId) ?? stepIds[0] ?? "");
          }
        }}
        className="rounded border border-gray-300 px-1 py-0.5 text-xs"
      >
        <option value="step">Step</option>
        <option value="terminal">Terminal</option>
      </select>
      {mode === "step" && (
        <select
          value={stepId}
          onChange={(e) => onChange(e.target.value)}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          {stepIds.filter((id) => id !== selfId).length === 0 && (
            <option value="">— no other steps —</option>
          )}
          {stepIds.filter((id) => id !== selfId).map((id) => (
            <option key={id} value={id}>{id}</option>
          ))}
        </select>
      )}
      {mode === "terminal" && (
        <select
          value={terminalOutcome}
          onChange={(e) => onChange({ kind: "Terminal", outcome: e.target.value } as TerminalTarget)}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          <option value="success">success</option>
          <option value="failure">failure</option>
          <option value="completed">completed</option>
          <option value="cancelled">cancelled</option>
          <option value="rejected">rejected</option>
        </select>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// FailureHandler editor
// ---------------------------------------------------------------------------

interface FailureHandlerEditorProps {
  value: FailureHandler;
  onChange: (h: FailureHandler) => void;
  stepIds: string[];
  selfId?: string;
  personas: PersonaConstruct[];
}

function FailureHandlerEditor({ value, onChange, stepIds, selfId, personas }: FailureHandlerEditorProps) {
  const kind = value.kind;

  return (
    <div className="space-y-1">
      <div className="flex items-center gap-1">
        <span className="text-xs text-gray-500">on_failure:</span>
        <select
          value={kind}
          onChange={(e) => {
            const k = e.target.value as FailureHandler["kind"];
            if (k === "Terminate") onChange({ kind: "Terminate", outcome: "failure" });
            else if (k === "Compensate") {
              onChange({ kind: "Compensate", steps: [], then: { kind: "Terminal", outcome: "failure" } } as CompensateHandler);
            } else if (k === "Escalate") {
              onChange({ kind: "Escalate", next: stepIds.find((id) => id !== selfId) ?? "", to_persona: personas[0]?.id ?? "" } as EscalateHandler);
            }
          }}
          className="rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          <option value="Terminate">Terminate</option>
          <option value="Compensate">Compensate</option>
          <option value="Escalate">Escalate</option>
        </select>
      </div>

      {kind === "Terminate" && (
        <div className="flex items-center gap-1 pl-2">
          <span className="text-xs text-gray-400">outcome:</span>
          <select
            value={(value as TerminateHandler).outcome}
            onChange={(e) => onChange({ kind: "Terminate", outcome: e.target.value })}
            className="rounded border border-gray-300 px-1 py-0.5 text-xs"
          >
            <option value="failure">failure</option>
            <option value="cancelled">cancelled</option>
            <option value="rejected">rejected</option>
          </select>
        </div>
      )}

      {kind === "Escalate" && (
        <div className="space-y-0.5 pl-2">
          <div className="flex items-center gap-1">
            <span className="text-xs text-gray-400">next:</span>
            <select
              value={(value as EscalateHandler).next}
              onChange={(e) => onChange({ ...(value as EscalateHandler), next: e.target.value })}
              className="rounded border border-gray-300 px-1 py-0.5 text-xs"
            >
              {stepIds.filter((id) => id !== selfId).map((id) => (
                <option key={id} value={id}>{id}</option>
              ))}
            </select>
          </div>
          <div className="flex items-center gap-1">
            <span className="text-xs text-gray-400">to_persona:</span>
            <select
              value={(value as EscalateHandler).to_persona}
              onChange={(e) => onChange({ ...(value as EscalateHandler), to_persona: e.target.value })}
              className="rounded border border-gray-300 px-1 py-0.5 text-xs"
            >
              {personas.map((p) => (
                <option key={p.id} value={p.id}>{p.id}</option>
              ))}
            </select>
          </div>
        </div>
      )}

      {kind === "Compensate" && (
        <CompensateHandlerEditor
          value={value as CompensateHandler}
          onChange={onChange}
          stepIds={stepIds}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// CompensateHandler editor
// ---------------------------------------------------------------------------

interface CompensateHandlerEditorProps {
  value: CompensateHandler;
  onChange: (h: FailureHandler) => void;
  stepIds: string[];
}

function CompensateHandlerEditor({ value, onChange, stepIds }: CompensateHandlerEditorProps) {
  function addCompStep() {
    const newStep: CompensationStep = {
      op: "",
      persona: "",
      on_failure: { kind: "Terminal", outcome: "failure" },
    };
    onChange({ ...value, steps: [...value.steps, newStep] });
  }

  function updateCompStep(idx: number, updated: CompensationStep) {
    const next = [...value.steps];
    next[idx] = updated;
    onChange({ ...value, steps: next });
  }

  function removeCompStep(idx: number) {
    onChange({ ...value, steps: value.steps.filter((_, i) => i !== idx) });
  }

  return (
    <div className="space-y-1 pl-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-gray-600">Compensation steps</span>
        <button
          onClick={addCompStep}
          className="rounded border border-dashed border-gray-300 px-1.5 py-0.5 text-xs text-gray-500 hover:bg-gray-50"
        >
          + Add
        </button>
      </div>
      {value.steps.map((cs, idx) => (
        <div key={idx} className="flex items-center gap-1 rounded border border-gray-100 bg-gray-50 p-1">
          <input
            type="text"
            value={cs.op}
            onChange={(e) => updateCompStep(idx, { ...cs, op: e.target.value })}
            className="w-24 rounded border border-gray-300 px-1 py-0.5 text-xs font-mono"
            placeholder="op_id"
          />
          <input
            type="text"
            value={cs.persona}
            onChange={(e) => updateCompStep(idx, { ...cs, persona: e.target.value })}
            className="w-20 rounded border border-gray-300 px-1 py-0.5 text-xs"
            placeholder="persona"
          />
          <button
            onClick={() => removeCompStep(idx)}
            className="rounded px-1 py-0.5 text-xs text-red-400 hover:bg-red-50"
          >
            ×
          </button>
        </div>
      ))}
      <div className="flex items-center gap-1">
        <span className="text-xs text-gray-400">then:</span>
        <select
          value={value.then.outcome}
          onChange={(e) => onChange({ ...value, then: { kind: "Terminal", outcome: e.target.value } })}
          className="rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          <option value="failure">failure</option>
          <option value="cancelled">cancelled</option>
          <option value="success">success</option>
        </select>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step detail editors (per type)
// ---------------------------------------------------------------------------

interface StepDetailProps {
  step: FlowStep;
  allStepIds: string[];
  operations: OperationConstruct[];
  personas: PersonaConstruct[];
  facts: ReturnType<typeof selectFacts>;
  verdicts: string[];
  flows: FlowConstruct[];
  onChange: (updated: FlowStep) => void;
  onDelete: () => void;
}

function StepDetail({ step, allStepIds, operations, personas, facts, verdicts, flows, onChange, onDelete }: StepDetailProps) {
  if (step.kind === "OperationStep") {
    return <OperationStepDetail step={step} allStepIds={allStepIds} operations={operations} personas={personas} onChange={onChange} onDelete={onDelete} />;
  }
  if (step.kind === "BranchStep") {
    return <BranchStepDetail step={step} allStepIds={allStepIds} personas={personas} facts={facts} verdicts={verdicts} onChange={onChange} onDelete={onDelete} />;
  }
  if (step.kind === "HandoffStep") {
    return <HandoffStepDetail step={step} allStepIds={allStepIds} personas={personas} onChange={onChange} onDelete={onDelete} />;
  }
  if (step.kind === "SubFlowStep") {
    return <SubFlowStepDetail step={step} allStepIds={allStepIds} personas={personas} flows={flows} onChange={onChange} onDelete={onDelete} />;
  }
  if (step.kind === "ParallelStep") {
    return <ParallelStepDetail step={step} allStepIds={allStepIds} onChange={onChange} onDelete={onDelete} />;
  }
  return null;
}

// -- OperationStep detail --

interface OperationStepDetailProps {
  step: OperationStep;
  allStepIds: string[];
  operations: OperationConstruct[];
  personas: PersonaConstruct[];
  onChange: (s: FlowStep) => void;
  onDelete: () => void;
}

function OperationStepDetail({ step, allStepIds, operations, personas, onChange, onDelete }: OperationStepDetailProps) {
  const selectedOp = operations.find((o) => o.id === step.op);
  const allowedPersonas = selectedOp?.allowed_personas ?? [];
  const outcomeKeys = Object.keys(step.outcomes);

  function addOutcomeRoute() {
    const label = `outcome_${outcomeKeys.length + 1}`;
    onChange({
      ...step,
      outcomes: { ...step.outcomes, [label]: { kind: "Terminal", outcome: "success" } },
    });
  }

  function removeOutcomeRoute(label: string) {
    const next = { ...step.outcomes };
    delete next[label];
    onChange({ ...step, outcomes: next });
  }

  function updateOutcomeRoute(oldLabel: string, newLabel: string, target: StepTarget) {
    const next: Record<string, StepTarget> = {};
    for (const [k, v] of Object.entries(step.outcomes)) {
      if (k === oldLabel) {
        next[newLabel] = target;
      } else {
        next[k] = v;
      }
    }
    onChange({ ...step, outcomes: next });
  }

  return (
    <div className="space-y-3 p-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-blue-700 uppercase tracking-wide">OperationStep</span>
        <button onClick={onDelete} className="rounded px-2 py-0.5 text-xs text-red-400 hover:bg-red-50">Delete</button>
      </div>

      <div className="flex items-center gap-2">
        <label className="w-16 text-xs text-gray-500">Step ID</label>
        <input
          type="text"
          value={step.id}
          onChange={(e) => onChange({ ...step, id: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-2 py-0.5 font-mono text-xs"
        />
      </div>

      <div className="flex items-center gap-2">
        <label className="w-16 text-xs text-gray-500">Operation</label>
        <select
          value={step.op}
          onChange={(e) => {
            const op = operations.find((o) => o.id === e.target.value);
            onChange({ ...step, op: e.target.value, persona: op?.allowed_personas[0] ?? "" });
          }}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          {operations.length === 0 && <option value="">— define operations first —</option>}
          {operations.map((o) => (
            <option key={o.id} value={o.id}>{o.id}</option>
          ))}
        </select>
      </div>

      <div className="flex items-center gap-2">
        <label className="w-16 text-xs text-gray-500">Persona</label>
        <select
          value={step.persona}
          onChange={(e) => onChange({ ...step, persona: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          {allowedPersonas.length === 0 && <option value="">— operation has no personas —</option>}
          {allowedPersonas.map((p) => (
            <option key={p} value={p}>{p}</option>
          ))}
          {!allowedPersonas.includes(step.persona) && step.persona && (
            <option value={step.persona}>{step.persona} (invalid)</option>
          )}
        </select>
      </div>

      {/* Outcome routing */}
      <div>
        <div className="mb-1 flex items-center justify-between">
          <span className="text-xs font-medium text-gray-600">Outcome routing</span>
          <button onClick={addOutcomeRoute} className="rounded border border-dashed border-gray-300 px-1.5 py-0.5 text-xs text-gray-500 hover:bg-gray-50">
            + Add
          </button>
        </div>
        {outcomeKeys.map((label) => (
          <div key={label} className="mb-1 flex items-center gap-1">
            <input
              type="text"
              value={label}
              onChange={(e) => updateOutcomeRoute(label, e.target.value, step.outcomes[label])}
              className="w-24 rounded border border-gray-300 px-1 py-0.5 font-mono text-xs"
            />
            <span className="text-xs text-gray-400">→</span>
            <StepTargetEditor
              value={step.outcomes[label]}
              onChange={(t) => updateOutcomeRoute(label, label, t)}
              stepIds={allStepIds}
              selfId={step.id}
              label=""
            />
            <button onClick={() => removeOutcomeRoute(label)} className="rounded px-1 py-0.5 text-xs text-red-400 hover:bg-red-50">×</button>
          </div>
        ))}
      </div>

      {/* Failure handler */}
      <FailureHandlerEditor
        value={step.on_failure}
        onChange={(h) => onChange({ ...step, on_failure: h })}
        stepIds={allStepIds}
        selfId={step.id}
        personas={personas}
      />
    </div>
  );
}

// -- BranchStep detail --

interface BranchStepDetailProps {
  step: BranchStep;
  allStepIds: string[];
  personas: PersonaConstruct[];
  facts: ReturnType<typeof selectFacts>;
  verdicts: string[];
  onChange: (s: FlowStep) => void;
  onDelete: () => void;
}

function BranchStepDetail({ step, allStepIds, personas, facts, verdicts, onChange, onDelete }: BranchStepDetailProps) {
  return (
    <div className="space-y-3 p-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-orange-700 uppercase tracking-wide">BranchStep</span>
        <button onClick={onDelete} className="rounded px-2 py-0.5 text-xs text-red-400 hover:bg-red-50">Delete</button>
      </div>

      <div className="flex items-center gap-2">
        <label className="w-16 text-xs text-gray-500">Step ID</label>
        <input
          type="text"
          value={step.id}
          onChange={(e) => onChange({ ...step, id: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-2 py-0.5 font-mono text-xs"
        />
      </div>

      <div className="flex items-center gap-2">
        <label className="w-16 text-xs text-gray-500">Persona</label>
        <select
          value={step.persona}
          onChange={(e) => onChange({ ...step, persona: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          {personas.map((p) => (
            <option key={p.id} value={p.id}>{p.id}</option>
          ))}
        </select>
      </div>

      <div>
        <label className="mb-1 block text-xs font-medium text-gray-600">Condition</label>
        <PredicateBuilder
          value={step.condition}
          onChange={(condition) => onChange({ ...step, condition })}
          availableFacts={facts}
          availableVerdicts={verdicts}
          mode="operation"
        />
      </div>

      <StepTargetEditor
        value={step.if_true}
        onChange={(t) => onChange({ ...step, if_true: t })}
        stepIds={allStepIds}
        selfId={step.id}
        label="if_true"
      />
      <StepTargetEditor
        value={step.if_false}
        onChange={(t) => onChange({ ...step, if_false: t })}
        stepIds={allStepIds}
        selfId={step.id}
        label="if_false"
      />
    </div>
  );
}

// -- HandoffStep detail --

interface HandoffStepDetailProps {
  step: HandoffStep;
  allStepIds: string[];
  personas: PersonaConstruct[];
  onChange: (s: FlowStep) => void;
  onDelete: () => void;
}

function HandoffStepDetail({ step, allStepIds, personas, onChange, onDelete }: HandoffStepDetailProps) {
  return (
    <div className="space-y-3 p-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-green-700 uppercase tracking-wide">HandoffStep</span>
        <button onClick={onDelete} className="rounded px-2 py-0.5 text-xs text-red-400 hover:bg-red-50">Delete</button>
      </div>

      <div className="flex items-center gap-2">
        <label className="w-20 text-xs text-gray-500">Step ID</label>
        <input
          type="text"
          value={step.id}
          onChange={(e) => onChange({ ...step, id: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-2 py-0.5 font-mono text-xs"
        />
      </div>

      <div className="flex items-center gap-2">
        <label className="w-20 text-xs text-gray-500">from_persona</label>
        <select
          value={step.from_persona}
          onChange={(e) => onChange({ ...step, from_persona: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          {personas.map((p) => <option key={p.id} value={p.id}>{p.id}</option>)}
        </select>
      </div>

      <div className="flex items-center gap-2">
        <label className="w-20 text-xs text-gray-500">to_persona</label>
        <select
          value={step.to_persona}
          onChange={(e) => onChange({ ...step, to_persona: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          {personas.map((p) => <option key={p.id} value={p.id}>{p.id}</option>)}
        </select>
      </div>

      <div className="flex items-center gap-2">
        <label className="w-20 text-xs text-gray-500">next</label>
        <select
          value={step.next}
          onChange={(e) => onChange({ ...step, next: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          {allStepIds.filter((id) => id !== step.id).map((id) => (
            <option key={id} value={id}>{id}</option>
          ))}
        </select>
      </div>
    </div>
  );
}

// -- SubFlowStep detail --

interface SubFlowStepDetailProps {
  step: SubFlowStep;
  allStepIds: string[];
  personas: PersonaConstruct[];
  flows: FlowConstruct[];
  onChange: (s: FlowStep) => void;
  onDelete: () => void;
}

function SubFlowStepDetail({ step, allStepIds, personas, flows, onChange, onDelete }: SubFlowStepDetailProps) {
  return (
    <div className="space-y-3 p-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-purple-700 uppercase tracking-wide">SubFlowStep</span>
        <button onClick={onDelete} className="rounded px-2 py-0.5 text-xs text-red-400 hover:bg-red-50">Delete</button>
      </div>

      <div className="flex items-center gap-2">
        <label className="w-16 text-xs text-gray-500">Step ID</label>
        <input
          type="text"
          value={step.id}
          onChange={(e) => onChange({ ...step, id: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-2 py-0.5 font-mono text-xs"
        />
      </div>

      <div className="flex items-center gap-2">
        <label className="w-16 text-xs text-gray-500">Flow</label>
        <select
          value={step.flow}
          onChange={(e) => onChange({ ...step, flow: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          {flows.map((f) => <option key={f.id} value={f.id}>{f.id}</option>)}
        </select>
      </div>

      <div className="flex items-center gap-2">
        <label className="w-16 text-xs text-gray-500">Persona</label>
        <select
          value={step.persona}
          onChange={(e) => onChange({ ...step, persona: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-1 py-0.5 text-xs"
        >
          {personas.map((p) => <option key={p.id} value={p.id}>{p.id}</option>)}
        </select>
      </div>

      <StepTargetEditor
        value={step.on_success}
        onChange={(t) => onChange({ ...step, on_success: t })}
        stepIds={allStepIds}
        selfId={step.id}
        label="on_success"
      />

      <FailureHandlerEditor
        value={step.on_failure}
        onChange={(h) => onChange({ ...step, on_failure: h })}
        stepIds={allStepIds}
        selfId={step.id}
        personas={personas}
      />
    </div>
  );
}

// -- ParallelStep detail --

interface ParallelStepDetailProps {
  step: ParallelStep;
  allStepIds: string[];
  onChange: (s: FlowStep) => void;
  onDelete: () => void;
}

function ParallelStepDetail({ step, allStepIds, onChange, onDelete }: ParallelStepDetailProps) {
  function addBranch() {
    const branch: ParallelBranch = {
      id: `branch_${step.branches.length + 1}`,
      entry: "",
      steps: [],
    };
    onChange({ ...step, branches: [...step.branches, branch] });
  }

  function removeBranch(idx: number) {
    onChange({ ...step, branches: step.branches.filter((_, i) => i !== idx) });
  }

  function updateBranch(idx: number, updated: ParallelBranch) {
    const next = [...step.branches];
    next[idx] = updated;
    onChange({ ...step, branches: next });
  }

  return (
    <div className="space-y-3 p-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-blue-700 uppercase tracking-wide">ParallelStep</span>
        <button onClick={onDelete} className="rounded px-2 py-0.5 text-xs text-red-400 hover:bg-red-50">Delete</button>
      </div>

      <div className="flex items-center gap-2">
        <label className="w-16 text-xs text-gray-500">Step ID</label>
        <input
          type="text"
          value={step.id}
          onChange={(e) => onChange({ ...step, id: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-2 py-0.5 font-mono text-xs"
        />
      </div>

      <div>
        <div className="mb-1 flex items-center justify-between">
          <span className="text-xs font-medium text-gray-600">Branches ({step.branches.length})</span>
          <button onClick={addBranch} className="rounded border border-dashed border-gray-300 px-1.5 py-0.5 text-xs text-gray-500 hover:bg-gray-50">
            + Branch
          </button>
        </div>
        {step.branches.map((branch, idx) => (
          <div key={branch.id} className="mb-1 flex items-center gap-1 rounded border border-gray-200 bg-gray-50 p-1">
            <input
              type="text"
              value={branch.id}
              onChange={(e) => updateBranch(idx, { ...branch, id: e.target.value })}
              className="w-24 rounded border border-gray-300 px-1 py-0.5 font-mono text-xs"
              placeholder="branch_id"
            />
            <button onClick={() => removeBranch(idx)} className="rounded px-1 py-0.5 text-xs text-red-400 hover:bg-red-50">×</button>
          </div>
        ))}
      </div>

      <StepTargetEditor
        value={step.join.on_all_success}
        onChange={(t) => onChange({ ...step, join: { ...step.join, on_all_success: t } })}
        stepIds={allStepIds}
        selfId={step.id}
        label="on_all_success"
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// FlowEditor main component
// ---------------------------------------------------------------------------

export function FlowEditor() {
  const flows = useContractStore(selectFlows);
  const operations = useContractStore(selectOperations);
  const personas = useContractStore(selectPersonas);
  const facts = useContractStore(selectFacts);
  const rules = useContractStore(selectRules);

  const addConstruct = useContractStore((s) => s.addConstruct);
  const removeConstruct = useContractStore((s) => s.removeConstruct);

  const [selectedFlowId, setSelectedFlowId] = useState<string | null>(null);
  const [selectedStepId, setSelectedStepId] = useState<string | null>(null);

  const selectedFlow = flows.find((f) => f.id === selectedFlowId) ?? null;
  const allFlowIds = flows.map((f) => f.id);

  const availableVerdicts = useMemo(
    () => rules.map((r) => r.body.produce.verdict_type).filter(Boolean),
    [rules]
  );

  const validation = useMemo(
    () => selectedFlow ? validateFlow(selectedFlow, operations, personas) : null,
    [selectedFlow, operations, personas]
  );

  const selectedStep = selectedFlow?.steps.find((s) => s.id === selectedStepId) ?? null;
  const allStepIds = selectedFlow?.steps.map((s) => s.id) ?? [];

  // Step type options
  type StepKind = "OperationStep" | "BranchStep" | "HandoffStep" | "SubFlowStep" | "ParallelStep";
  const [addStepKind, setAddStepKind] = useState<StepKind>("OperationStep");
  const [showAddStepDropdown, setShowAddStepDropdown] = useState(false);

  function handleAddFlow() {
    const base = "new_flow";
    let id = base;
    let i = 1;
    while (allFlowIds.includes(id)) id = `${base}_${i++}`;
    const flow = newFlow(id);
    addConstruct(flow);
    setSelectedFlowId(id);
    setSelectedStepId(null);
  }

  function handleDeleteFlow(id: string) {
    if (confirm(`Delete flow "${id}"?`)) {
      removeConstruct(id, "Flow");
      if (selectedFlowId === id) {
        setSelectedFlowId(null);
        setSelectedStepId(null);
      }
    }
  }

  function updateFlow(updates: Partial<FlowConstruct>) {
    if (!selectedFlow) return;
    if (updates.id && updates.id !== selectedFlow.id) {
      removeConstruct(selectedFlow.id, "Flow");
      addConstruct({ ...selectedFlow, ...updates } as FlowConstruct);
      setSelectedFlowId(updates.id);
    } else {
      useContractStore.getState().updateConstruct(selectedFlow.id, "Flow", updates);
    }
  }

  function addStep(kind: StepKind) {
    if (!selectedFlow) return;
    const baseId = kind.replace("Step", "").toLowerCase();
    const newId = uniqueStepId(baseId, selectedFlow.steps);
    let newStep: FlowStep;
    if (kind === "OperationStep") newStep = defaultOperationStep(newId, operations);
    else if (kind === "BranchStep") newStep = defaultBranchStep(newId, facts);
    else if (kind === "HandoffStep") newStep = defaultHandoffStep(newId);
    else if (kind === "SubFlowStep") newStep = defaultSubFlowStep(newId, flows);
    else newStep = defaultParallelStep(newId);

    const newSteps = [...selectedFlow.steps, newStep];
    const newEntry = selectedFlow.entry || newId;
    updateFlow({ steps: newSteps, entry: newEntry });
    setSelectedStepId(newId);
    setShowAddStepDropdown(false);
  }

  function deleteStep(stepId: string) {
    if (!selectedFlow) return;
    const ref = confirm(`Delete step "${stepId}"? Other steps referencing it will break.`);
    if (!ref) return;
    const newSteps = selectedFlow.steps.filter((s) => s.id !== stepId);
    const newEntry = selectedFlow.entry === stepId ? (newSteps[0]?.id ?? "") : selectedFlow.entry;
    updateFlow({ steps: newSteps, entry: newEntry });
    if (selectedStepId === stepId) setSelectedStepId(null);
  }

  function updateStep(updated: FlowStep) {
    if (!selectedFlow) return;
    const newSteps = selectedFlow.steps.map((s) => (s.id === selectedStepId ? updated : s));
    // If step ID changed, update entry and selectedStepId
    if (updated.id !== selectedStepId) {
      const newEntry = selectedFlow.entry === selectedStepId ? updated.id : selectedFlow.entry;
      updateFlow({ steps: newSteps, entry: newEntry });
      setSelectedStepId(updated.id);
    } else {
      updateFlow({ steps: newSteps });
    }
  }

  const STEP_KINDS: { kind: StepKind; label: string; color: string }[] = [
    { kind: "OperationStep", label: "Operation Step", color: "text-blue-700" },
    { kind: "BranchStep", label: "Branch Step", color: "text-orange-700" },
    { kind: "HandoffStep", label: "Handoff Step", color: "text-green-700" },
    { kind: "SubFlowStep", label: "Sub-Flow Step", color: "text-purple-700" },
    { kind: "ParallelStep", label: "Parallel Step", color: "text-cyan-700" },
  ];

  return (
    <div className="flex h-full overflow-hidden">
      {/* Flow list sidebar */}
      <aside className="flex w-48 shrink-0 flex-col border-r border-gray-200 bg-gray-50">
        <div className="flex items-center justify-between border-b border-gray-200 px-3 py-2">
          <span className="text-xs font-semibold uppercase tracking-wide text-gray-600">Flows</span>
          <button
            onClick={handleAddFlow}
            className="rounded bg-blue-500 px-2 py-0.5 text-xs text-white hover:bg-blue-600"
          >
            +
          </button>
        </div>
        <div className="flex-1 overflow-y-auto">
          {flows.length === 0 ? (
            <div className="p-3 text-center text-xs text-gray-400">No flows yet</div>
          ) : (
            flows.map((flow) => (
              <button
                key={flow.id}
                onClick={() => { setSelectedFlowId(flow.id); setSelectedStepId(null); }}
                className={`w-full px-3 py-2 text-left text-xs transition-colors ${
                  selectedFlowId === flow.id
                    ? "bg-blue-100 font-medium text-blue-700"
                    : "text-gray-600 hover:bg-gray-100"
                }`}
              >
                <div className="font-mono">{flow.id}</div>
                <div className="text-gray-400">{flow.steps.length} step{flow.steps.length !== 1 ? "s" : ""}</div>
              </button>
            ))
          )}
        </div>
      </aside>

      {/* Main area */}
      {!selectedFlow ? (
        <div className="flex flex-1 items-center justify-center text-sm text-gray-400">
          {flows.length === 0
            ? 'Click "+" to create your first flow.'
            : "Select a flow to edit"}
        </div>
      ) : (
        <div className="flex flex-1 overflow-hidden">
          {/* DAG + metadata area */}
          <div className="flex flex-1 flex-col overflow-y-auto">
            {/* Flow metadata */}
            <div className="shrink-0 border-b border-gray-200 bg-white p-3">
              <div className="flex flex-wrap items-center gap-3">
                {/* Flow ID */}
                <div className="flex items-center gap-1">
                  <label className="text-xs text-gray-500">Flow ID:</label>
                  <input
                    type="text"
                    value={selectedFlow.id}
                    onChange={(e) => updateFlow({ id: e.target.value })}
                    onBlur={(e) => {
                      const v = e.target.value.trim();
                      if (!v) updateFlow({ id: selectedFlow.id });
                    }}
                    className="rounded border border-gray-300 px-2 py-0.5 font-mono text-xs"
                  />
                </div>

                {/* Snapshot */}
                <div className="flex items-center gap-1">
                  <label className="text-xs text-gray-500">Snapshot:</label>
                  <select
                    value={selectedFlow.snapshot ?? "at_initiation"}
                    onChange={(e) => updateFlow({ snapshot: e.target.value as "at_initiation" })}
                    className="rounded border border-gray-300 px-1 py-0.5 text-xs"
                  >
                    <option value="at_initiation">at_initiation</option>
                    <option value="live">live</option>
                  </select>
                </div>

                {/* Entry step */}
                <div className="flex items-center gap-1">
                  <label className="text-xs text-gray-500">Entry:</label>
                  <select
                    value={selectedFlow.entry}
                    onChange={(e) => updateFlow({ entry: e.target.value })}
                    className="rounded border border-gray-300 px-1 py-0.5 text-xs"
                  >
                    {allStepIds.length === 0 && <option value="">— no steps —</option>}
                    {allStepIds.map((id) => (
                      <option key={id} value={id}>{id}</option>
                    ))}
                  </select>
                </div>

                {/* Delete flow */}
                <button
                  onClick={() => handleDeleteFlow(selectedFlow.id)}
                  className="ml-auto rounded border border-red-200 bg-red-50 px-2 py-0.5 text-xs text-red-600 hover:bg-red-100"
                >
                  Delete flow
                </button>
              </div>
            </div>

            {/* Validation */}
            {validation && (validation.errors.length > 0 || validation.warnings.length > 0) && (
              <div className="shrink-0 border-b border-gray-200 bg-white px-3 py-2">
                {validation.errors.map((e, i) => (
                  <p key={i} className="text-xs text-red-600">
                    Error: {e}
                  </p>
                ))}
                {validation.warnings.map((w, i) => (
                  <p key={i} className="text-xs text-amber-600">
                    Warning: {w}
                  </p>
                ))}
              </div>
            )}

            {/* Step toolbar */}
            <div className="shrink-0 flex items-center gap-2 border-b border-gray-200 bg-gray-50 px-3 py-2">
              {/* Add step dropdown */}
              <div className="relative">
                <button
                  onClick={() => setShowAddStepDropdown((v) => !v)}
                  className="rounded bg-blue-500 px-2 py-1 text-xs text-white hover:bg-blue-600"
                >
                  + Add Step
                </button>
                {showAddStepDropdown && (
                  <div className="absolute left-0 top-full z-10 mt-1 w-44 rounded border border-gray-200 bg-white shadow-lg">
                    {STEP_KINDS.map(({ kind, label, color }) => (
                      <button
                        key={kind}
                        onClick={() => addStep(kind)}
                        className={`block w-full px-3 py-1.5 text-left text-xs hover:bg-gray-50 ${color}`}
                      >
                        {label}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Delete selected step */}
              {selectedStepId && (
                <>
                  <button
                    onClick={() => deleteStep(selectedStepId)}
                    className="rounded border border-red-200 bg-red-50 px-2 py-1 text-xs text-red-600 hover:bg-red-100"
                  >
                    Delete Step
                  </button>
                  <button
                    onClick={() => updateFlow({ entry: selectedStepId })}
                    className="rounded border border-blue-200 bg-blue-50 px-2 py-1 text-xs text-blue-600 hover:bg-blue-100"
                  >
                    Set as Entry
                  </button>
                </>
              )}

              <span className="ml-auto text-xs text-gray-400">
                {allStepIds.length} step{allStepIds.length !== 1 ? "s" : ""}
                {selectedStepId && ` • selected: ${selectedStepId}`}
              </span>
            </div>

            {/* DAG visualization */}
            <div className="flex-1 p-3">
              <FlowDag
                steps={selectedFlow.steps}
                entry={selectedFlow.entry}
                editable={true}
                highlightedStep={undefined}
                onStepClick={(stepId) => setSelectedStepId(stepId)}
              />
            </div>
          </div>

          {/* Step detail side panel */}
          {selectedStep && (
            <aside className="flex w-72 shrink-0 flex-col overflow-y-auto border-l border-gray-200 bg-white">
              <div className="border-b border-gray-100 px-3 py-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-gray-700">Step Editor</span>
                  <button
                    onClick={() => setSelectedStepId(null)}
                    className="rounded px-1.5 py-0.5 text-xs text-gray-400 hover:bg-gray-100"
                  >
                    ×
                  </button>
                </div>
              </div>
              <div className="flex-1 overflow-y-auto">
                <StepDetail
                  step={selectedStep}
                  allStepIds={allStepIds}
                  operations={operations}
                  personas={personas}
                  facts={facts}
                  verdicts={availableVerdicts}
                  flows={flows.filter((f) => f.id !== selectedFlow.id)}
                  onChange={updateStep}
                  onDelete={() => {
                    deleteStep(selectedStep.id);
                  }}
                />
              </div>
            </aside>
          )}
        </div>
      )}
    </div>
  );
}
