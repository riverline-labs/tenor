/** Health check response from GET /health */
export interface HealthResponse {
  status: string;
  tenor_version: string;
}

/** Contract summary in list response from GET /contracts */
export interface ContractSummary {
  id: string;
  construct_count: number;
  facts: string[];
  operations: string[];
  flows: string[];
}

/** List contracts response */
export interface ContractsResponse {
  contracts: ContractSummary[];
}

/** Effect within an operation (entity state transition) */
export interface OperationEffect {
  entity_id: string;
  from: string;
  to: string;
}

/**
 * Operation details from GET /contracts/{id}/operations.
 *
 * Field names match the interchange JSON exactly:
 * - `allowed_personas` (not "personas") -- from Operation struct in crates/eval/src/types.rs
 * - `effects` array (not a single "entity" field) -- operations can target multiple entities
 * Verified against: crates/cli/src/serve.rs handle_get_operations
 */
export interface OperationInfo {
  id: string;
  allowed_personas: string[];
  effects: OperationEffect[];
  preconditions_summary: string;
}

/** Operations list response */
export interface OperationsResponse {
  operations: OperationInfo[];
}

/**
 * Verdict from evaluation.
 *
 * Field names match VerdictSet::to_json() output in crates/eval/src/types.rs:
 * - `type` (not "verdict_type") -- the JSON key is "type"
 * - `payload` (not "value") -- the JSON key is "payload"
 * - `provenance.rule` (not "rule_id") -- the JSON key is "rule"
 */
export interface Verdict {
  type: string;
  payload: unknown;
  provenance: VerdictProvenance;
}

export interface VerdictProvenance {
  rule: string;
  stratum: number;
  facts_used: string[];
  verdicts_used: string[];
}

/** Rule-only evaluation result from POST /evaluate (no flow_id) */
export interface EvalResult {
  verdicts: Verdict[];
}

/** Step record from flow execution */
export interface StepRecord {
  step_id: string;
  result: string;
}

/** Entity state change from flow execution */
export interface EntityStateChange {
  entity_id: string;
  from: string;
  to: string;
}

/** Flow evaluation result from POST /evaluate (with flow_id) */
export interface FlowEvalResult {
  flow_id: string;
  outcome: string;
  initiating_persona: string | null;
  entity_state_changes: EntityStateChange[];
  steps_executed: StepRecord[];
  verdicts: {
    verdicts: Verdict[];
  };
}

/** Elaborate response (interchange JSON bundle) from POST /elaborate */
export type InterchangeBundle = Record<string, unknown>;

/** Options for evaluate requests */
export interface EvaluateOptions {
  flow_id?: string;
  persona?: string;
}

/** Explain response from POST /explain */
export interface ExplainResult {
  summary: string;
  verbose: string;
}

/** Error response shape from the server */
export interface ErrorResponse {
  error: string;
  details?: Record<string, unknown>;
}
