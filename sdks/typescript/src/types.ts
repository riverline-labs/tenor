/**
 * TypeScript type definitions for the Tenor contract evaluator SDK.
 *
 * These types mirror the JSON interface of the WASM evaluator. All communication
 * between the TypeScript SDK and the WASM module is via JSON strings; these types
 * define the shape of those JSON payloads.
 */

// ---------------------------------------------------------------------------
// Fact types
// ---------------------------------------------------------------------------

/** Describes the type of a fact in the contract. */
export interface FactType {
  base: string;
  currency?: string;
  precision?: number;
  scale?: number;
  /** Record variant: field name -> field type. */
  fields?: Record<string, FactType>;
  /** List variant: element type. */
  element?: FactType;
  /** Enum variant: allowed string values. */
  values?: string[];
}

/** A money value with amount string and ISO currency code. */
export interface MoneyValue {
  amount: string;
  currency: string;
}

/**
 * A fact value. Can be a primitive, Money, Record, List, or null (missing optional fact).
 * Numbers in JSON become number in TypeScript; Decimal/Money amounts use string.
 * Uses interface-based recursion to satisfy TypeScript's alias constraints.
 */
export type FactValue =
  | boolean
  | string
  | number
  | MoneyValue
  | FactRecord
  | FactList
  | null;

/** A record fact value: field name -> fact value. */
export interface FactRecord {
  [field: string]: FactValue;
}

/** A list fact value. */
export interface FactList extends Array<FactValue> {}

/** A set of facts keyed by fact ID. */
export interface FactSet {
  [factId: string]: FactValue;
}

// ---------------------------------------------------------------------------
// Entity state types
// ---------------------------------------------------------------------------

/**
 * Entity state map.
 *
 * Old (single-instance) format: `{ "Order": "pending" }`
 * New (multi-instance) format: `{ "Order": { "ord-001": "pending" } }`
 *
 * The WASM module accepts both formats and auto-detects.
 * This TypeScript type represents the old flat format for simplicity;
 * use NestedEntityStateMap for multi-instance.
 */
export interface EntityStateMap {
  [entityId: string]: string;
}

/**
 * Multi-instance entity state map: entity_id -> instance_id -> state.
 */
export interface NestedEntityStateMap {
  [entityId: string]: { [instanceId: string]: string };
}

/**
 * Entity state input: accepts both flat (old) and nested (new) formats.
 * Use EntityStateMap for single-instance contracts, NestedEntityStateMap for multi-instance.
 */
export type EntityStateInput = EntityStateMap | NestedEntityStateMap;

/** Instance bindings: entity_id -> instance_id. Used with simulate_flow_with_bindings. */
export interface InstanceBindings {
  [entityId: string]: string;
}

// ---------------------------------------------------------------------------
// Verdict types
// ---------------------------------------------------------------------------

/** Provenance for a verdict: which rule produced it, which facts were used. */
export interface VerdictProvenance {
  rule: string;
  stratum: number;
  facts_used: string[];
  verdicts_used?: string[];
}

/** A single verdict produced by a rule. */
export interface Verdict {
  type: string;
  payload: unknown;
  provenance: VerdictProvenance;
}

/** The complete set of verdicts from rule evaluation. */
export interface VerdictSet {
  verdicts: Verdict[];
}

// ---------------------------------------------------------------------------
// Action Space types
// ---------------------------------------------------------------------------

/** Summary of a verdict for action space context. */
export interface VerdictSummary {
  verdict_type: string;
  payload: unknown;
  producing_rule: string;
  stratum: number;
}

/** Summary of an entity's current state and possible transitions. */
export interface EntitySummary {
  entity_id: string;
  current_state: string;
  possible_transitions: string[];
}

/**
 * A single executable action available to a persona.
 *
 * `instance_bindings` maps entity_id -> array of valid instance_ids that are
 * in the required source state for this action. For single-instance contracts,
 * this contains `"_default"`.
 */
export interface Action {
  flow_id: string;
  persona_id: string;
  entry_operation_id: string;
  enabling_verdicts: VerdictSummary[];
  affected_entities: EntitySummary[];
  description: string;
  /** entity_id -> array of valid instance_ids in the required source state. */
  instance_bindings: Record<string, string[]>;
}

/** Why an action is currently blocked. */
export type BlockedReason =
  | { type: "PersonaNotAuthorized" }
  | { type: "PreconditionNotMet"; missing_verdicts: string[] }
  | {
      type: "EntityNotInSourceState";
      entity_id: string;
      current_state: string;
      required_state: string;
    }
  | { type: "MissingFacts"; fact_ids: string[] };

/**
 * An action that exists but is not currently executable.
 *
 * `instance_bindings` maps entity_id -> array of blocking instance_ids (when
 * reason is EntityNotInSourceState). Empty otherwise.
 */
export interface BlockedAction {
  flow_id: string;
  reason: BlockedReason;
  /** entity_id -> array of blocking instance_ids (for EntityNotInSourceState). */
  instance_bindings: Record<string, string[]>;
}

/** The complete action space for a persona at a point in time. */
export interface ActionSpace {
  persona_id: string;
  actions: Action[];
  current_verdicts: VerdictSummary[];
  blocked_actions: BlockedAction[];
}

// ---------------------------------------------------------------------------
// Flow result types
// ---------------------------------------------------------------------------

/** A single step execution record in a flow. */
export interface StepResult {
  step_id: string;
  step_type: string;
  result: string;
  /** instance_bindings for this step (may be absent for non-operation steps). */
  instance_bindings?: Record<string, string>;
}

/** An entity state transition that would occur if the flow were applied. */
export interface EntityStateChange {
  entity_id: string;
  instance_id: string;
  from_state: string;
  to_state: string;
}

/** The result of simulating a flow. */
export interface FlowResult {
  simulation: boolean;
  flow_id: string;
  persona: string;
  outcome: string;
  path: StepResult[];
  would_transition: EntityStateChange[];
  verdicts: Verdict[];
  /** instance_bindings echoed back from simulate_flow_with_bindings. */
  instance_bindings?: Record<string, string>;
}

// ---------------------------------------------------------------------------
// Contract inspection types
// ---------------------------------------------------------------------------

/** A fact as returned by inspect(). */
export interface InspectFact {
  id: string;
  type: string;
  source?: unknown;
  has_default?: boolean;
  type_spec?: FactType;
}

/** An entity as returned by inspect(). */
export interface InspectEntity {
  id: string;
  states: string[];
  initial: string;
  transitions: Array<{ from: string; to: string }>;
}

/** A rule as returned by inspect(). */
export interface InspectRule {
  id: string;
  stratum: number;
  produces: string;
  condition_summary: string;
}

/** The full inspection result for a loaded contract. */
export interface InspectResult {
  facts: InspectFact[];
  entities: InspectEntity[];
  rules: InspectRule[];
  personas: Array<{ id: string }>;
  operations: unknown[];
  flows: Array<{ id: string; entry: string; steps: string[] }>;
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/** Error response from the WASM module. */
export interface WasmErrorResponse {
  error: string;
}

// ---------------------------------------------------------------------------
// Interchange bundle type (opaque — passed through to WASM)
// ---------------------------------------------------------------------------

/** An interchange bundle as produced by the Tenor elaborator. */
export interface InterchangeBundle {
  id: string;
  kind: "Bundle";
  tenor: string;
  tenor_version: string;
  constructs: unknown[];
}
