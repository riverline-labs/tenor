/**
 * TenorEvaluator — wraps the WASM evaluator module.
 *
 * The WASM module (built by wasm-pack --target nodejs) is a synchronous
 * CommonJS module. It loads the .wasm binary via readFileSync at require-time.
 */

import type {
  FactSet,
  EntityStateInput,
  InstanceBindings,
  VerdictSet,
  ActionSpace,
  FlowResult,
  InspectResult,
  InterchangeBundle,
} from "./types";

// ---------------------------------------------------------------------------
// WASM module interface
// ---------------------------------------------------------------------------

interface WasmModule {
  load_contract(interchange_json: string): string;
  free_contract(handle: number): void;
  evaluate(handle: number, facts_json: string): string;
  simulate_flow(
    handle: number,
    flow_id: string,
    persona_id: string,
    facts_json: string,
    entity_states_json: string,
  ): string;
  simulate_flow_with_bindings(
    handle: number,
    flow_id: string,
    persona_id: string,
    facts_json: string,
    entity_states_json: string,
    instance_bindings_json: string,
  ): string;
  compute_action_space(
    handle: number,
    facts_json: string,
    entity_states_json: string,
    persona_id: string,
  ): string;
  inspect_contract(handle: number): string;
}

// ---------------------------------------------------------------------------
// WASM module loading (singleton, synchronous)
// ---------------------------------------------------------------------------

let wasmModule: WasmModule | null = null;

/**
 * Load the WASM module. The wasm-pack --target nodejs output is a synchronous
 * CommonJS module that loads the .wasm binary at require-time via readFileSync.
 * We use require() so it works from the compiled dist/ directory.
 */
function getWasmModule(): WasmModule {
  if (wasmModule !== null) return wasmModule;

  // The wasm/ directory is at the package root, two levels up from dist/
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const mod = require("../wasm/tenor_eval_wasm.js") as WasmModule;
  wasmModule = mod;
  return wasmModule;
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

function parseResult(json: string): unknown {
  try {
    return JSON.parse(json);
  } catch {
    throw new Error(`WASM returned invalid JSON: ${json}`);
  }
}

function checkError(result: unknown, context: string): void {
  if (
    result !== null &&
    typeof result === "object" &&
    "error" in result &&
    (result as { error: unknown }).error !== undefined
  ) {
    const err = (result as { error: unknown }).error;
    throw new Error(`${context}: ${String(err)}`);
  }
}

// ---------------------------------------------------------------------------
// TenorEvaluator
// ---------------------------------------------------------------------------

/**
 * TenorEvaluator wraps a loaded Tenor contract for evaluation in Node.js.
 *
 * Contracts are loaded from interchange bundle JSON (the output of `tenor elaborate`).
 * Each evaluator holds a WASM-side contract handle and must be freed when done.
 *
 * @example
 * ```typescript
 * const evaluator = await TenorEvaluator.fromJson(bundleJson);
 * try {
 *   const verdicts = evaluator.evaluate({ is_active: true });
 *   const space = evaluator.computeActionSpace({ is_active: true }, { Order: 'pending' }, 'admin');
 * } finally {
 *   evaluator.free();
 * }
 * ```
 */
export class TenorEvaluator {
  private readonly wasm: WasmModule;
  private readonly handle: number;
  private freed = false;

  private constructor(wasm: WasmModule, handle: number) {
    this.wasm = wasm;
    this.handle = handle;
  }

  /**
   * Load a contract from an interchange bundle object.
   *
   * @param bundle - The interchange bundle (output of `tenor elaborate`).
   * @returns A loaded TenorEvaluator ready for evaluation.
   */
  static fromBundle(bundle: InterchangeBundle): TenorEvaluator {
    return TenorEvaluator.fromJson(JSON.stringify(bundle));
  }

  /**
   * Load a contract from an interchange JSON string.
   *
   * @param json - The interchange bundle as a JSON string.
   * @returns A loaded TenorEvaluator ready for evaluation.
   * @throws {Error} If the JSON is invalid or not a valid contract bundle.
   */
  static fromJson(json: string): TenorEvaluator {
    const wasm = getWasmModule();
    const resultStr = wasm.load_contract(json);
    const result = parseResult(resultStr);
    checkError(result, "Failed to load contract");
    const handle = (result as { handle: number }).handle;
    return new TenorEvaluator(wasm, handle);
  }

  /**
   * Evaluate rules against the provided facts and return the verdict set.
   *
   * @param facts - Map of fact IDs to their values.
   * @returns The set of verdicts produced by rule evaluation.
   * @throws {Error} If facts are missing or invalid, or after free().
   */
  evaluate(facts: FactSet): VerdictSet {
    this.ensureNotFreed();
    const resultStr = this.wasm.evaluate(this.handle, JSON.stringify(facts));
    const result = parseResult(resultStr);
    checkError(result, "Evaluation error");
    return result as VerdictSet;
  }

  /**
   * Compute the action space for a persona given current facts and entity states.
   *
   * @param facts - Map of fact IDs to their values.
   * @param entityStates - Current entity states (flat or nested format).
   * @param persona - The persona ID to compute the action space for.
   * @returns The action space: available and blocked actions for this persona.
   * @throws {Error} If evaluation fails or after free().
   */
  computeActionSpace(
    facts: FactSet,
    entityStates: EntityStateInput,
    persona: string,
  ): ActionSpace {
    this.ensureNotFreed();
    const resultStr = this.wasm.compute_action_space(
      this.handle,
      JSON.stringify(facts),
      JSON.stringify(entityStates),
      persona,
    );
    const result = parseResult(resultStr);
    checkError(result, "Action space error");
    return result as ActionSpace;
  }

  /**
   * Simulate (execute) a flow and return the result.
   *
   * The flow is always run in simulation mode — entity states are not actually
   * mutated. The result shows what would happen if the flow were applied.
   *
   * @param flowId - The flow ID to execute.
   * @param facts - Map of fact IDs to their values.
   * @param entityStates - Current entity states (flat or nested format).
   * @param persona - The initiating persona ID.
   * @returns The flow simulation result with outcome, path, and transitions.
   * @throws {Error} If the flow is not found, or after free().
   */
  executeFlow(
    flowId: string,
    facts: FactSet,
    entityStates: EntityStateInput,
    persona: string,
  ): FlowResult {
    this.ensureNotFreed();
    const resultStr = this.wasm.simulate_flow(
      this.handle,
      flowId,
      persona,
      JSON.stringify(facts),
      JSON.stringify(entityStates),
    );
    const result = parseResult(resultStr);
    checkError(result, "Flow execution error");
    return result as FlowResult;
  }

  /**
   * Simulate a flow with explicit instance bindings for multi-instance entities.
   *
   * @param flowId - The flow ID to execute.
   * @param facts - Map of fact IDs to their values.
   * @param entityStates - Current entity states (flat or nested format).
   * @param persona - The initiating persona ID.
   * @param instanceBindings - Explicit entity_id -> instance_id bindings.
   * @returns The flow simulation result with outcome, path, transitions, and bindings.
   * @throws {Error} If the flow is not found, or after free().
   */
  executeFlowWithBindings(
    flowId: string,
    facts: FactSet,
    entityStates: EntityStateInput,
    persona: string,
    instanceBindings: InstanceBindings,
  ): FlowResult {
    this.ensureNotFreed();
    const resultStr = this.wasm.simulate_flow_with_bindings(
      this.handle,
      flowId,
      persona,
      JSON.stringify(facts),
      JSON.stringify(entityStates),
      JSON.stringify(instanceBindings),
    );
    const result = parseResult(resultStr);
    checkError(result, "Flow execution error");
    return result as FlowResult;
  }

  /**
   * Inspect the loaded contract's structure.
   *
   * @returns The contract's facts, entities, rules, operations, and flows.
   * @throws {Error} After free().
   */
  inspect(): InspectResult {
    this.ensureNotFreed();
    const resultStr = this.wasm.inspect_contract(this.handle);
    const result = parseResult(resultStr);
    checkError(result, "Inspect error");
    return result as InspectResult;
  }

  /**
   * Free the contract handle. The evaluator cannot be used after this call.
   * Calling free() on an already-freed evaluator is a no-op.
   */
  free(): void {
    if (!this.freed) {
      this.wasm.free_contract(this.handle);
      this.freed = true;
    }
  }

  /** Whether this evaluator has been freed. */
  get isFreed(): boolean {
    return this.freed;
  }

  private ensureNotFreed(): void {
    if (this.freed) {
      throw new Error("TenorEvaluator has been freed and cannot be used");
    }
  }
}
