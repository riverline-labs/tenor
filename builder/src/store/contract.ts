/**
 * Zustand store for the contract model.
 *
 * The internal model is always an InterchangeBundle (interchange JSON).
 * The DSL is generated on demand at export time.
 *
 * Features:
 * - Full CRUD for constructs (add, update, remove)
 * - Typed selectors for each construct kind
 * - Undo/redo with 50-state history
 * - Debounced elaboration trigger
 */

import { create } from "zustand";
import { temporal } from "zundo";
import type {
  InterchangeBundle,
  InterchangeConstruct,
  FactConstruct,
  EntityConstruct,
  RuleConstruct,
  OperationConstruct,
  FlowConstruct,
  PersonaConstruct,
  SourceConstruct,
  SystemConstruct,
} from "@/types/interchange";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";
const TENOR_BUNDLE_VERSION = "1.0.0";

// ---------------------------------------------------------------------------
// State shape
// ---------------------------------------------------------------------------

export interface ContractState {
  bundle: InterchangeBundle;

  // Bundle-level actions
  initContract: (bundleId: string) => void;
  loadBundle: (bundle: InterchangeBundle) => void;

  // Construct CRUD
  addConstruct: (construct: InterchangeConstruct) => void;
  updateConstruct: (
    id: string,
    kind: InterchangeConstruct["kind"],
    updates: Partial<InterchangeConstruct>
  ) => void;
  removeConstruct: (id: string, kind: InterchangeConstruct["kind"]) => void;

  // Typed selectors
  getConstructsByKind: <K extends InterchangeConstruct["kind"]>(
    kind: K
  ) => Extract<InterchangeConstruct, { kind: K }>[];
  getConstructById: <K extends InterchangeConstruct["kind"]>(
    id: string,
    kind: K
  ) => Extract<InterchangeConstruct, { kind: K }> | undefined;

  // Convenience selectors
  facts: () => FactConstruct[];
  entities: () => EntityConstruct[];
  rules: () => RuleConstruct[];
  operations: () => OperationConstruct[];
  flows: () => FlowConstruct[];
  personas: () => PersonaConstruct[];
  sources: () => SourceConstruct[];
  systems: () => SystemConstruct[];
}

// ---------------------------------------------------------------------------
// Default empty bundle
// ---------------------------------------------------------------------------

function emptyBundle(bundleId: string): InterchangeBundle {
  return {
    constructs: [],
    id: bundleId,
    kind: "Bundle",
    tenor: TENOR_VERSION,
    tenor_version: TENOR_BUNDLE_VERSION,
  };
}

// ---------------------------------------------------------------------------
// Store implementation
// ---------------------------------------------------------------------------

export const useContractStore = create<ContractState>()(
  temporal(
    (set, get) => ({
      bundle: emptyBundle("untitled"),

      initContract(bundleId: string) {
        set({ bundle: emptyBundle(bundleId) });
      },

      loadBundle(bundle: InterchangeBundle) {
        set({ bundle });
      },

      addConstruct(construct: InterchangeConstruct) {
        set((state) => ({
          bundle: {
            ...state.bundle,
            constructs: [...state.bundle.constructs, construct],
          },
        }));
      },

      updateConstruct(
        id: string,
        kind: InterchangeConstruct["kind"],
        updates: Partial<InterchangeConstruct>
      ) {
        set((state) => ({
          bundle: {
            ...state.bundle,
            constructs: state.bundle.constructs.map((c) =>
              c.id === id && c.kind === kind
                ? ({ ...c, ...updates } as InterchangeConstruct)
                : c
            ),
          },
        }));
      },

      removeConstruct(id: string, kind: InterchangeConstruct["kind"]) {
        set((state) => ({
          bundle: {
            ...state.bundle,
            constructs: state.bundle.constructs.filter(
              (c) => !(c.id === id && c.kind === kind)
            ),
          },
        }));
      },

      getConstructsByKind<K extends InterchangeConstruct["kind"]>(
        kind: K
      ): Extract<InterchangeConstruct, { kind: K }>[] {
        return get().bundle.constructs.filter(
          (c): c is Extract<InterchangeConstruct, { kind: K }> =>
            c.kind === kind
        );
      },

      getConstructById<K extends InterchangeConstruct["kind"]>(
        id: string,
        kind: K
      ): Extract<InterchangeConstruct, { kind: K }> | undefined {
        return get().bundle.constructs.find(
          (c): c is Extract<InterchangeConstruct, { kind: K }> =>
            c.id === id && c.kind === kind
        );
      },

      facts(): FactConstruct[] {
        return get()
          .bundle.constructs.filter(
            (c): c is FactConstruct => c.kind === "Fact"
          );
      },

      entities(): EntityConstruct[] {
        return get()
          .bundle.constructs.filter(
            (c): c is EntityConstruct => c.kind === "Entity"
          );
      },

      rules(): RuleConstruct[] {
        return get()
          .bundle.constructs.filter(
            (c): c is RuleConstruct => c.kind === "Rule"
          );
      },

      operations(): OperationConstruct[] {
        return get()
          .bundle.constructs.filter(
            (c): c is OperationConstruct => c.kind === "Operation"
          );
      },

      flows(): FlowConstruct[] {
        return get()
          .bundle.constructs.filter(
            (c): c is FlowConstruct => c.kind === "Flow"
          );
      },

      personas(): PersonaConstruct[] {
        return get()
          .bundle.constructs.filter(
            (c): c is PersonaConstruct => c.kind === "Persona"
          );
      },

      sources(): SourceConstruct[] {
        return get()
          .bundle.constructs.filter(
            (c): c is SourceConstruct => c.kind === "Source"
          );
      },

      systems(): SystemConstruct[] {
        return get()
          .bundle.constructs.filter(
            (c): c is SystemConstruct => c.kind === "System"
          );
      },
    }),
    {
      // Keep last 50 states for undo/redo
      limit: 50,
      // Only track bundle state changes
      partialize: (state) => ({ bundle: state.bundle }),
    }
  )
);

// ---------------------------------------------------------------------------
// Undo/redo helpers
// ---------------------------------------------------------------------------

export function undoContract() {
  useContractStore.temporal.getState().undo();
}

export function redoContract() {
  useContractStore.temporal.getState().redo();
}

export function canUndo(): boolean {
  return useContractStore.temporal.getState().pastStates.length > 0;
}

export function canRedo(): boolean {
  return useContractStore.temporal.getState().futureStates.length > 0;
}

// ---------------------------------------------------------------------------
// Typed convenience selectors (stable references for component usage)
// ---------------------------------------------------------------------------

export const selectBundle = (state: ContractState) => state.bundle;
export const selectFacts = (state: ContractState) => state.facts();
export const selectEntities = (state: ContractState) => state.entities();
export const selectRules = (state: ContractState) => state.rules();
export const selectOperations = (state: ContractState) => state.operations();
export const selectFlows = (state: ContractState) => state.flows();
export const selectPersonas = (state: ContractState) => state.personas();
export const selectSources = (state: ContractState) => state.sources();
export const selectSystems = (state: ContractState) => state.systems();
