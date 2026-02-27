/**
 * Helper functions for working with ActionSpace objects.
 *
 * These are pure functions — no WASM required.
 */

import type { ActionSpace, Action, BlockedAction, BlockedReason } from "./types";

/**
 * Get all available (non-blocked) actions for a specific flow.
 *
 * @param actionSpace - The action space to query.
 * @param flowId - The flow ID to filter by.
 * @returns Array of available actions for the given flow.
 */
export function actionsForFlow(
  actionSpace: ActionSpace,
  flowId: string,
): Action[] {
  return actionSpace.actions.filter((a) => a.flow_id === flowId);
}

/**
 * Check if a specific flow has at least one available (non-blocked) action.
 *
 * @param actionSpace - The action space to query.
 * @param flowId - The flow ID to check.
 * @returns true if the flow has at least one available action.
 */
export function isFlowAvailable(
  actionSpace: ActionSpace,
  flowId: string,
): boolean {
  return actionSpace.actions.some((a) => a.flow_id === flowId);
}

/**
 * Check if a specific flow is blocked (has a blocked action entry).
 *
 * @param actionSpace - The action space to query.
 * @param flowId - The flow ID to check.
 * @returns true if the flow appears in blocked_actions.
 */
export function isFlowBlocked(
  actionSpace: ActionSpace,
  flowId: string,
): boolean {
  return actionSpace.blocked_actions.some((b) => b.flow_id === flowId);
}

/**
 * Get the block reason for a flow, if it is blocked.
 *
 * @param actionSpace - The action space to query.
 * @param flowId - The flow ID to look up.
 * @returns The BlockedReason if the flow is blocked, or undefined.
 */
export function getBlockReason(
  actionSpace: ActionSpace,
  flowId: string,
): BlockedReason | undefined {
  const blocked = actionSpace.blocked_actions.find(
    (b) => b.flow_id === flowId,
  );
  return blocked?.reason;
}

/**
 * Get the full BlockedAction for a flow, if it is blocked.
 *
 * @param actionSpace - The action space to query.
 * @param flowId - The flow ID to look up.
 * @returns The BlockedAction if the flow is blocked, or undefined.
 */
export function getBlockedAction(
  actionSpace: ActionSpace,
  flowId: string,
): BlockedAction | undefined {
  return actionSpace.blocked_actions.find((b) => b.flow_id === flowId);
}

/**
 * Get deduplicated list of all available flow IDs.
 *
 * @param actionSpace - The action space to query.
 * @returns Unique flow IDs that have at least one available action.
 */
export function availableFlowIds(actionSpace: ActionSpace): string[] {
  return [...new Set(actionSpace.actions.map((a) => a.flow_id))];
}

/**
 * Get deduplicated list of all blocked flow IDs.
 *
 * @param actionSpace - The action space to query.
 * @returns Unique flow IDs that have a blocked action entry.
 */
export function blockedFlowIds(actionSpace: ActionSpace): string[] {
  return [...new Set(actionSpace.blocked_actions.map((b) => b.flow_id))];
}

/**
 * Check if a specific verdict type is currently active in the action space.
 *
 * @param actionSpace - The action space to query.
 * @param verdictType - The verdict type ID to check.
 * @returns true if the verdict is in current_verdicts.
 */
export function hasVerdict(
  actionSpace: ActionSpace,
  verdictType: string,
): boolean {
  return actionSpace.current_verdicts.some(
    (v) => v.verdict_type === verdictType,
  );
}
