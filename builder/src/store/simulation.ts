/**
 * Zustand store for simulation state.
 *
 * Manages fact values, entity states, evaluation results, and flow stepping.
 * Uses the WASM evaluator via the contract handle from the elaboration store.
 */

import { create } from "zustand";
import { evaluatorApi } from "@/wasm/evaluator";
import type { VerdictEntry, SimulateFlowResult, ActionSpaceResult } from "@/wasm/evaluator";
import { useElaborationStore } from "./elaboration";

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

export interface FlowExecutionState {
  flowId: string;
  persona: string;
  result: SimulateFlowResult | null;
  running: boolean;
  error: string | null;
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

  // Evaluation errors
  evaluationError: string | null;

  // Actions
  setFactValue: (factId: string, value: FactValue) => void;
  setEntityState: (entityId: string, state: string) => void;
  setSelectedPersona: (persona: string | null) => void;
  evaluate: () => Promise<void>;
  computeActionSpace: (persona: string) => Promise<void>;
  startFlowSimulation: (flowId: string, persona: string) => Promise<void>;
  stepFlow: () => void; // no-op in initial implementation (full simulation not step-by-step yet)
  resetSimulation: () => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
    set({ evaluationError: null });
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

      set({ verdicts: verdictEntriesToResults(verdictList) });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      set({ evaluationError: message, verdicts: null });
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

  async startFlowSimulation(flowId: string, persona: string) {
    set({
      flowExecution: {
        flowId,
        persona,
        result: null,
        running: true,
        error: null,
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
      set({
        flowExecution: {
          flowId,
          persona,
          result,
          running: false,
          error: null,
        },
      });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      set({
        flowExecution: {
          flowId,
          persona,
          result: null,
          running: false,
          error: message,
        },
        evaluationError: message,
      });
    }
  },

  stepFlow() {
    // Step-by-step flow execution is handled server-side.
    // The current implementation runs the full simulation at once.
    // This is a no-op placeholder for future step-by-step UI.
    console.warn(
      "[SimulationStore] stepFlow() is not yet implemented; use startFlowSimulation() for full simulation."
    );
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
