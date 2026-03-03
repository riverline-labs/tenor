/**
 * Zustand store for simulation state.
 *
 * Manages fact values, entity states, evaluation results, and flow stepping.
 * Uses the WASM evaluator via the contract handle from the elaboration store.
 * Reads declared facts/entities from the contract store for initialization.
 */

import { create } from "zustand";
import { evaluatorApi } from "@/wasm/evaluator";
import type { VerdictEntry, SimulateFlowResult, ActionSpaceResult } from "@/wasm/evaluator";
import { useElaborationStore } from "./elaboration";
import { useContractStore } from "./contract";
import type { FactConstruct, EntityConstruct, FactDefault, BaseType } from "@/types/interchange";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type FactValue =
  | string
  | number
  | boolean
  | null
  | Record<string, unknown>
  | unknown[];

export interface VerdictResult {
  verdict_type: string;
  payload?: unknown;
  provenance?: unknown;
}

export interface StepResult {
  step_id: string;
  step_type: string;
  result: string;
  instance_bindings?: Record<string, string>;
}

export interface EntityStateChange {
  entity_id: string;
  instance_id: string;
  from_state: string;
  to_state: string;
}

export interface FlowExecutionState {
  flowId: string;
  persona: string;
  currentStepId: string | null;
  stepsExecuted: StepResult[];
  entityStateChanges: EntityStateChange[];
  outcome: string | null;
  isComplete: boolean;
  running: boolean;
  error: string | null;
  // Full simulation result retained for replay
  fullResult: SimulateFlowResult | null;
}

export interface SimulationState {
  // Input state
  factValues: Record<string, FactValue>;
  entityStates: Record<string, string>;
  selectedPersona: string | null;

  // Results
  verdicts: VerdictResult[] | null;
  actionSpace: ActionSpaceResult | null;
  flowExecution: FlowExecutionState | null;

  // Evaluation errors / status
  evaluationError: string | null;
  isEvaluating: boolean;

  // Actions
  initFromContract: () => void;
  setFactValue: (factId: string, value: FactValue) => void;
  setEntityState: (entityId: string, state: string) => void;
  setSelectedPersona: (persona: string | null) => void;
  evaluate: () => Promise<void>;
  computeActionSpace: (persona: string) => Promise<void>;
  simulateFlow: (flowId: string, persona: string) => Promise<void>;
  resetSimulation: () => void;
  // Stepwise playback helpers (client-side replay of stored simulation)
  stepFlowForward: () => void;
  resetFlowPlayback: () => void;
}

// ---------------------------------------------------------------------------
// Helpers: derive default JSON value from a Tenor base type + optional default
// ---------------------------------------------------------------------------

function defaultForType(type: BaseType, declared: FactDefault | undefined): FactValue {
  // Use declared default if present
  if (declared !== undefined) {
    // BoolLiteral
    if (typeof declared === "object" && declared !== null && "kind" in declared) {
      const k = (declared as { kind: string }).kind;
      if (k === "bool_literal") return (declared as { kind: string; value: boolean }).value;
      if (k === "decimal_value") return (declared as { kind: string; value: string }).value;
      if (k === "money_value") {
        const m = declared as { kind: string; amount: { value: string }; currency: string };
        return { amount: m.amount.value, currency: m.currency };
      }
    }
    // boolean / number / string primitives
    if (typeof declared === "boolean" || typeof declared === "number" || typeof declared === "string") {
      return declared;
    }
  }

  // Fall back to sensible zero values per type
  switch (type.base) {
    case "Bool":
      return false;
    case "Int":
      return 0;
    case "Decimal":
      return "0." + "0".repeat((type as { base: "Decimal"; scale: number }).scale);
    case "Text":
      return "";
    case "Date":
      return "2024-01-01";
    case "DateTime":
      return "2024-01-01T00:00:00Z";
    case "Duration":
      return 0;
    case "Money": {
      const mt = type as { base: "Money"; currency: string };
      return { amount: "0.00", currency: mt.currency };
    }
    case "Enum": {
      const et = type as { base: "Enum"; values: string[] };
      return et.values[0] ?? "";
    }
    case "List":
      return [];
    case "Record": {
      const rt = type as { base: "Record"; fields: Record<string, BaseType> };
      const obj: Record<string, FactValue> = {};
      for (const [k, v] of Object.entries(rt.fields)) {
        obj[k] = defaultForType(v, undefined);
      }
      return obj;
    }
    case "TaggedUnion":
      return null;
    default:
      return null;
  }
}

function getContractHandle(): number {
  const handle = useElaborationStore.getState().contractHandle;
  if (handle === null) {
    throw new Error(
      "No contract loaded. Validate the contract before simulating."
    );
  }
  return handle;
}

function verdictEntriesToResults(entries: VerdictEntry[]): VerdictResult[] {
  return entries.map((v) => ({
    verdict_type: v.verdict_type,
    payload: v.payload,
    provenance: v.provenance,
  }));
}

// ---------------------------------------------------------------------------
// Store implementation
// ---------------------------------------------------------------------------

export const useSimulationStore = create<SimulationState>()((set, get) => ({
  factValues: {},
  entityStates: {},
  selectedPersona: null,
  verdicts: null,
  actionSpace: null,
  flowExecution: null,
  evaluationError: null,
  isEvaluating: false,

  initFromContract() {
    const contractState = useContractStore.getState();
    const facts: FactConstruct[] = contractState.facts();
    const entities: EntityConstruct[] = contractState.entities();

    // Build fact defaults
    const factValues: Record<string, FactValue> = {};
    for (const fact of facts) {
      factValues[fact.id] = defaultForType(fact.type, fact.default);
    }

    // Build entity initial states
    const entityStates: Record<string, string> = {};
    for (const entity of entities) {
      entityStates[entity.id] = entity.initial;
    }

    set({
      factValues,
      entityStates,
      verdicts: null,
      actionSpace: null,
      flowExecution: null,
      evaluationError: null,
      isEvaluating: false,
    });
  },

  setFactValue(factId: string, value: FactValue) {
    set((state) => ({
      factValues: { ...state.factValues, [factId]: value },
      // Clear stale results when inputs change
      verdicts: null,
      actionSpace: null,
    }));
  },

  setEntityState(entityId: string, state: string) {
    set((prev) => ({
      entityStates: { ...prev.entityStates, [entityId]: state },
      verdicts: null,
      actionSpace: null,
    }));
  },

  setSelectedPersona(persona: string | null) {
    set({ selectedPersona: persona, actionSpace: null });
  },

  async evaluate() {
    set({ evaluationError: null, isEvaluating: true });
    try {
      const handle = getContractHandle();
      const { factValues } = get();
      const result = evaluatorApi.evaluate(handle, JSON.stringify(factValues));

      // Result may be { verdicts: [...] } or an array
      const verdictList: VerdictEntry[] = Array.isArray(result)
        ? result
        : Array.isArray(result.verdicts)
          ? result.verdicts
          : [];

      set({ verdicts: verdictEntriesToResults(verdictList), isEvaluating: false });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      set({ evaluationError: message, verdicts: null, isEvaluating: false });
    }
  },

  async computeActionSpace(persona: string) {
    set({ evaluationError: null });
    try {
      const handle = getContractHandle();
      const { factValues, entityStates } = get();
      const result = evaluatorApi.computeActionSpace(
        handle,
        JSON.stringify(factValues),
        JSON.stringify(entityStates),
        persona
      );
      set({ actionSpace: result, selectedPersona: persona });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      set({ evaluationError: message, actionSpace: null });
    }
  },

  async simulateFlow(flowId: string, persona: string) {
    // Initialize execution state as running
    set({
      flowExecution: {
        flowId,
        persona,
        currentStepId: null,
        stepsExecuted: [],
        entityStateChanges: [],
        outcome: null,
        isComplete: false,
        running: true,
        error: null,
        fullResult: null,
      },
      evaluationError: null,
    });

    try {
      const handle = getContractHandle();
      const { factValues, entityStates } = get();
      const result = evaluatorApi.simulateFlow(
        handle,
        flowId,
        persona,
        JSON.stringify(factValues),
        JSON.stringify(entityStates)
      );

      const stepsExecuted: StepResult[] = (result.path ?? []).map((s) => ({
        step_id: s.step_id,
        step_type: s.step_type,
        result: s.result,
        instance_bindings: s.instance_bindings,
      }));

      const entityStateChanges: EntityStateChange[] = (result.would_transition ?? []).map((t) => ({
        entity_id: t.entity_id,
        instance_id: t.instance_id,
        from_state: t.from_state,
        to_state: t.to_state,
      }));

      // Start playback at step 0 (or null if empty)
      const firstStepId = stepsExecuted.length > 0 ? stepsExecuted[0].step_id : null;

      set({
        flowExecution: {
          flowId,
          persona,
          currentStepId: firstStepId,
          stepsExecuted: stepsExecuted.slice(0, 1),
          entityStateChanges,
          outcome: result.outcome,
          isComplete: stepsExecuted.length <= 1,
          running: false,
          error: null,
          fullResult: result,
        },
      });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      set({
        flowExecution: {
          flowId,
          persona,
          currentStepId: null,
          stepsExecuted: [],
          entityStateChanges: [],
          outcome: null,
          isComplete: false,
          running: false,
          error: message,
          fullResult: null,
        },
        evaluationError: message,
      });
    }
  },

  stepFlowForward() {
    const { flowExecution } = get();
    if (!flowExecution || !flowExecution.fullResult) return;
    const allSteps: StepResult[] = (flowExecution.fullResult.path ?? []).map((s) => ({
      step_id: s.step_id,
      step_type: s.step_type,
      result: s.result,
      instance_bindings: s.instance_bindings,
    }));
    const nextIndex = flowExecution.stepsExecuted.length;
    if (nextIndex >= allSteps.length) return;
    const next = allSteps[nextIndex];
    const newSteps = [...flowExecution.stepsExecuted, next];
    set({
      flowExecution: {
        ...flowExecution,
        stepsExecuted: newSteps,
        currentStepId: next.step_id,
        isComplete: newSteps.length >= allSteps.length,
      },
    });
  },

  resetFlowPlayback() {
    const { flowExecution } = get();
    if (!flowExecution || !flowExecution.fullResult) return;
    const allSteps: StepResult[] = (flowExecution.fullResult.path ?? []).map((s) => ({
      step_id: s.step_id,
      step_type: s.step_type,
      result: s.result,
      instance_bindings: s.instance_bindings,
    }));
    const firstStepId = allSteps.length > 0 ? allSteps[0].step_id : null;
    set({
      flowExecution: {
        ...flowExecution,
        currentStepId: firstStepId,
        stepsExecuted: allSteps.slice(0, 1),
        isComplete: allSteps.length <= 1,
      },
    });
  },

  resetSimulation() {
    set({
      factValues: {},
      entityStates: {},
      selectedPersona: null,
      verdicts: null,
      actionSpace: null,
      flowExecution: null,
      evaluationError: null,
      isEvaluating: false,
    });
  },
}));

// ---------------------------------------------------------------------------
// Selectors
// ---------------------------------------------------------------------------

export const selectFactValues = (state: SimulationState) => state.factValues;
export const selectEntityStates = (state: SimulationState) =>
  state.entityStates;
export const selectVerdicts = (state: SimulationState) => state.verdicts;
export const selectActionSpace = (state: SimulationState) => state.actionSpace;
export const selectFlowExecution = (state: SimulationState) =>
  state.flowExecution;
export const selectSelectedPersona = (state: SimulationState) =>
  state.selectedPersona;
export const selectEvaluationError = (state: SimulationState) =>
  state.evaluationError;
export const selectIsEvaluating = (state: SimulationState) =>
  state.isEvaluating;
