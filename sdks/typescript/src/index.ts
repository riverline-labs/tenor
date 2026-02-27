/**
 * @tenor/sdk — TypeScript/JavaScript SDK for the Tenor contract evaluator.
 *
 * @example
 * ```typescript
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
 * ```
 */

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
  // Error type
  TenorError,
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
