/**
 * Wrapper around the tenor-eval-wasm WASM module.
 * Provides a typed async API for contract loading and evaluation.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface LoadContractResult {
  handle: number;
}

export interface EvaluateResult {
  verdicts: VerdictEntry[];
  [key: string]: unknown;
}

export interface VerdictEntry {
  verdict_type: string;
  payload?: unknown;
  provenance?: unknown;
}

export interface SimulateFlowResult {
  simulation: boolean;
  flow_id: string;
  persona: string;
  outcome: string;
  path: FlowPathStep[];
  would_transition: EntityTransition[];
  verdicts: VerdictEntry[];
  instance_bindings?: Record<string, string>;
}

export interface FlowPathStep {
  step_id: string;
  step_type: string;
  result: string;
  instance_bindings?: Record<string, string>;
}

export interface EntityTransition {
  entity_id: string;
  instance_id: string;
  from_state: string;
  to_state: string;
}

export interface ActionSpaceResult {
  allowed: ActionEntry[];
  blocked: BlockedActionEntry[];
}

export interface ActionEntry {
  operation_id: string;
  persona: string;
  instance_bindings?: Record<string, string[]>;
}

export interface BlockedActionEntry {
  operation_id: string;
  reason: string;
}

export interface InspectContractResult {
  facts?: unknown[];
  entities?: unknown[];
  rules?: unknown[];
  operations?: unknown[];
  flows?: unknown[];
  [key: string]: unknown;
}

export interface EvaluatorError {
  error: string;
}

export interface EvaluatorApi {
  loadContract(interchangeJson: string): LoadContractResult;
  freeContract(handle: number): void;
  evaluate(handle: number, factsJson: string): EvaluateResult;
  simulateFlow(
    handle: number,
    flowId: string,
    personaId: string,
    factsJson: string,
    entityStatesJson: string
  ): SimulateFlowResult;
  simulateFlowWithBindings(
    handle: number,
    flowId: string,
    personaId: string,
    factsJson: string,
    entityStatesJson: string,
    instanceBindingsJson: string
  ): SimulateFlowResult;
  computeActionSpace(
    handle: number,
    factsJson: string,
    entityStatesJson: string,
    personaId: string
  ): ActionSpaceResult;
  inspectContract(handle: number): InspectContractResult;
}

// ---------------------------------------------------------------------------
// WASM module singleton
// ---------------------------------------------------------------------------

let wasmModule: typeof import("./pkg/tenor_eval_wasm") | null = null;
let initPromise: Promise<void> | null = null;

/**
 * Initialize the WASM module. Safe to call multiple times.
 */
export async function initEvaluator(): Promise<void> {
  if (wasmModule) return;
  if (initPromise) return initPromise;

  initPromise = (async () => {
    const mod = await import("./pkg/tenor_eval_wasm");
    // The default export is the init function for the web target.
    // vite-plugin-wasm handles the loading, so we just await init.
    await mod.default();
    wasmModule = mod;
    console.log("[Tenor Builder] WASM evaluator initialized");
  })();

  return initPromise;
}

/**
 * Get the WASM module, throwing if not initialized.
 */
function getWasm(): typeof import("./pkg/tenor_eval_wasm") {
  if (!wasmModule) {
    throw new Error("WASM evaluator not initialized. Call initEvaluator() first.");
  }
  return wasmModule;
}

/**
 * Parse a JSON string result from WASM, handling error responses.
 */
function parseResult<T>(json: string): T {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    throw new Error(`WASM returned invalid JSON: ${json}`);
  }
  const obj = parsed as Record<string, unknown>;
  if (typeof obj === "object" && obj !== null && "error" in obj) {
    throw new Error(`WASM error: ${obj.error}`);
  }
  return parsed as T;
}

// ---------------------------------------------------------------------------
// EvaluatorApi implementation
// ---------------------------------------------------------------------------

export const evaluatorApi: EvaluatorApi = {
  loadContract(interchangeJson: string): LoadContractResult {
    const wasm = getWasm();
    const result = wasm.load_contract(interchangeJson);
    return parseResult<LoadContractResult>(result);
  },

  freeContract(handle: number): void {
    const wasm = getWasm();
    wasm.free_contract(handle);
  },

  evaluate(handle: number, factsJson: string): EvaluateResult {
    const wasm = getWasm();
    const result = wasm.evaluate(handle, factsJson);
    return parseResult<EvaluateResult>(result);
  },

  simulateFlow(
    handle: number,
    flowId: string,
    personaId: string,
    factsJson: string,
    entityStatesJson: string
  ): SimulateFlowResult {
    const wasm = getWasm();
    const result = wasm.simulate_flow(
      handle,
      flowId,
      personaId,
      factsJson,
      entityStatesJson
    );
    return parseResult<SimulateFlowResult>(result);
  },

  simulateFlowWithBindings(
    handle: number,
    flowId: string,
    personaId: string,
    factsJson: string,
    entityStatesJson: string,
    instanceBindingsJson: string
  ): SimulateFlowResult {
    const wasm = getWasm();
    const result = wasm.simulate_flow_with_bindings(
      handle,
      flowId,
      personaId,
      factsJson,
      entityStatesJson,
      instanceBindingsJson
    );
    return parseResult<SimulateFlowResult>(result);
  },

  computeActionSpace(
    handle: number,
    factsJson: string,
    entityStatesJson: string,
    personaId: string
  ): ActionSpaceResult {
    const wasm = getWasm();
    const result = wasm.compute_action_space(
      handle,
      factsJson,
      entityStatesJson,
      personaId
    );
    return parseResult<ActionSpaceResult>(result);
  },

  inspectContract(handle: number): InspectContractResult {
    const wasm = getWasm();
    const result = wasm.inspect_contract(handle);
    return parseResult<InspectContractResult>(result);
  },
};
