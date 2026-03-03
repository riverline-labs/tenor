/**
 * Zustand store for real-time validation state.
 *
 * Wraps both the quick (synchronous) and WASM-based validation pipelines.
 * Components subscribe to this store for live error feedback.
 */

import { create } from "zustand";
import { initEvaluator, evaluatorApi } from "@/wasm/evaluator";
import { quickValidate } from "@/wasm/elaborator";
import type { InterchangeBundle } from "@/types/interchange";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ValidationError {
  construct_id?: string;
  construct_kind?: string;
  field?: string;
  message: string;
  severity: "error" | "warning";
}

export interface ElaborationState {
  // WASM state
  wasmReady: boolean;
  contractHandle: number | null;

  // Validation state
  errors: ValidationError[];
  isValidating: boolean;
  lastValidatedAt: number | null;

  // Actions
  initWasm: () => Promise<void>;
  validate: (bundle: InterchangeBundle) => Promise<void>;
  clearErrors: () => void;
}

// ---------------------------------------------------------------------------
// Store implementation
// ---------------------------------------------------------------------------

export const useElaborationStore = create<ElaborationState>()((set, get) => ({
  wasmReady: false,
  contractHandle: null,
  errors: [],
  isValidating: false,
  lastValidatedAt: null,

  async initWasm() {
    if (get().wasmReady) return;
    try {
      await initEvaluator();
      set({ wasmReady: true });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      set({
        errors: [
          {
            message: `Failed to initialize WASM evaluator: ${message}`,
            severity: "error",
          },
        ],
      });
    }
  },

  async validate(bundle: InterchangeBundle) {
    set({ isValidating: true });

    // Step 1: Quick structural validation (synchronous)
    const quickErrors = quickValidate(bundle);

    // If there are structural errors, report them and skip WASM
    if (quickErrors.some((e) => e.severity === "error")) {
      set({
        errors: quickErrors,
        isValidating: false,
        lastValidatedAt: Date.now(),
      });
      return;
    }

    // Step 2: WASM-based validation (if WASM is ready)
    const { wasmReady, contractHandle } = get();
    if (!wasmReady) {
      // Just report quick errors if WASM not ready
      set({
        errors: quickErrors,
        isValidating: false,
        lastValidatedAt: Date.now(),
      });
      return;
    }

    // Release previous handle if any
    if (contractHandle !== null) {
      try {
        evaluatorApi.freeContract(contractHandle);
      } catch {
        // ignore cleanup errors
      }
      set({ contractHandle: null });
    }

    try {
      const bundleJson = JSON.stringify(bundle);
      const result = evaluatorApi.loadContract(bundleJson);

      set({
        contractHandle: result.handle,
        errors: quickErrors, // propagate any warnings from quick validate
        isValidating: false,
        lastValidatedAt: Date.now(),
      });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      set({
        errors: [
          ...quickErrors,
          {
            message: `Contract validation failed: ${message}`,
            severity: "error",
          },
        ],
        isValidating: false,
        lastValidatedAt: Date.now(),
      });
    }
  },

  clearErrors() {
    set({ errors: [] });
  },
}));

// ---------------------------------------------------------------------------
// Selectors
// ---------------------------------------------------------------------------

export const selectErrors = (state: ElaborationState) => state.errors;
export const selectErrorCount = (state: ElaborationState) =>
  state.errors.filter((e) => e.severity === "error").length;
export const selectWarningCount = (state: ElaborationState) =>
  state.errors.filter((e) => e.severity === "warning").length;
export const selectIsValidating = (state: ElaborationState) =>
  state.isValidating;
export const selectWasmReady = (state: ElaborationState) => state.wasmReady;
export const selectContractHandle = (state: ElaborationState) =>
  state.contractHandle;
