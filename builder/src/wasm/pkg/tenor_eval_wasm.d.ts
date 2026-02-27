/* tslint:disable */
/* eslint-disable */

export function compute_action_space(handle: number, facts_json: string, entity_states_json: string, persona_id: string): string;

export function evaluate(handle: number, facts_json: string): string;

export function free_contract(handle: number): void;

export function inspect_contract(handle: number): string;

export function load_contract(interchange_json: string): string;

export function simulate_flow(handle: number, flow_id: string, persona_id: string, facts_json: string, entity_states_json: string): string;

/**
 * Extended simulate_flow that accepts instance_bindings.
 *
 * `entity_states_json` accepts both old flat format and new nested format.
 * `instance_bindings_json` maps entity_id → instance_id; if empty/null, uses _default.
 */
export function simulate_flow_with_bindings(handle: number, flow_id: string, persona_id: string, facts_json: string, entity_states_json: string, instance_bindings_json: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compute_action_space: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly evaluate: (a: number, b: number, c: number) => [number, number];
    readonly free_contract: (a: number) => void;
    readonly inspect_contract: (a: number) => [number, number];
    readonly load_contract: (a: number, b: number) => [number, number];
    readonly simulate_flow: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number];
    readonly simulate_flow_with_bindings: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
