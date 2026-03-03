/**
 * HTTP API types for the TenorClient.
 *
 * Types that overlap with the WASM SDK types (Verdict, VerdictProvenance,
 * InterchangeBundle) are re-exported from ./types. HTTP-specific types
 * (HealthResponse, ContractSummary, EvalResult, FlowEvalResult, etc.)
 * are defined here with their original shapes.
 */

import type { Verdict, VerdictProvenance, InterchangeBundle } from "./types";

// Re-export shared types so HTTP-client consumers can import from one place
export type { Verdict, VerdictProvenance, InterchangeBundle };

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

/** Rule-only evaluation result from POST /evaluate (no flow_id) */
export interface EvalResult {
  verdicts: Verdict[];
}

/** Step record from flow execution (HTTP API shape) */
export interface StepRecord {
  step_id: string;
  result: string;
}

/** Entity state change from flow execution (HTTP API shape) */
export interface HttpEntityStateChange {
  entity_id: string;
  from: string;
  to: string;
}

/** Flow evaluation result from POST /evaluate (with flow_id) */
export interface FlowEvalResult {
  flow_id: string;
  outcome: string;
  initiating_persona: string | null;
  entity_state_changes: HttpEntityStateChange[];
  steps_executed: StepRecord[];
  verdicts: {
    verdicts: Verdict[];
  };
}

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
