/**
 * @tenor/sdk — TypeScript/JavaScript SDK for the Tenor contract evaluator.
 *
 * Two entry points:
 * - TenorEvaluator: WASM-based local contract evaluation (no server needed)
 * - TenorClient: HTTP client for a running `tenor serve` instance
 *
 * @example
 * ```typescript
 * // WASM evaluator (local, no server)
 * import { TenorEvaluator, isFlowAvailable } from '@tenor/sdk';
 *
 * const evaluator = TenorEvaluator.fromJson(bundleJson);
 * try {
 *   const verdicts = evaluator.evaluate({ is_active: true });
 *   const space = evaluator.computeActionSpace(
 *     { is_active: true },
 *     { Order: 'pending' },
 *     'admin'
 *   );
 *   if (isFlowAvailable(space, 'approval_flow')) {
 *     const result = evaluator.executeFlow('approval_flow', { is_active: true }, { Order: 'pending' }, 'admin');
 *     console.log(result.outcome);
 *   }
 * } finally {
 *   evaluator.free();
 * }
 *
 * // HTTP client (requires running `tenor serve`)
 * import { TenorClient } from '@tenor/sdk';
 *
 * const client = new TenorClient({ baseUrl: 'http://localhost:8080' });
 * const contracts = await client.listContracts();
 * const result = await client.invoke(contracts[0].id, { is_active: true });
 * ```
 */

// ---------------------------------------------------------------------------
// WASM evaluator
// ---------------------------------------------------------------------------

export { TenorEvaluator } from "./evaluator";

export type {
  // Fact types
  FactSet,
  FactValue,
  FactRecord,
  FactList,
  MoneyValue,
  FactType,
  // Entity state types
  EntityStateMap,
  NestedEntityStateMap,
  EntityStateInput,
  InstanceBindings,
  // Verdict types
  VerdictSet,
  Verdict,
  VerdictProvenance,
  // Action space types
  ActionSpace,
  Action,
  BlockedAction,
  BlockedReason,
  VerdictSummary,
  EntitySummary,
  // Flow result types
  FlowResult,
  StepResult,
  EntityStateChange,
  // Inspection types
  InspectResult,
  InspectFact,
  InspectEntity,
  InspectRule,
  // Bundle type
  InterchangeBundle,
  // WASM error type
  WasmErrorResponse,
} from "./types";

export {
  actionsForFlow,
  isFlowAvailable,
  isFlowBlocked,
  getBlockReason,
  getBlockedAction,
  availableFlowIds,
  blockedFlowIds,
  hasVerdict,
} from "./action-space";

// ---------------------------------------------------------------------------
// HTTP client
// ---------------------------------------------------------------------------

export { TenorClient } from "./client";
export type { TenorClientOptions } from "./client";

export {
  TenorError,
  ConnectionError,
  ContractNotFoundError,
  EvaluationError,
  ElaborationError,
} from "./errors";

export type {
  // HTTP API types
  HealthResponse,
  ContractSummary,
  ContractsResponse,
  OperationEffect,
  OperationInfo,
  OperationsResponse,
  EvalResult,
  StepRecord,
  HttpEntityStateChange,
  FlowEvalResult,
  EvaluateOptions,
  ExplainResult,
  ErrorResponse,
} from "./client-types";
