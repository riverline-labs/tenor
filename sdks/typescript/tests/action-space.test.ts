/**
 * Action-space helper function unit tests.
 *
 * These tests use mock ActionSpace objects — no WASM required.
 */

import { describe, it, expect } from "vitest";
import {
  actionsForFlow,
  isFlowAvailable,
  isFlowBlocked,
  getBlockReason,
  getBlockedAction,
  availableFlowIds,
  blockedFlowIds,
  hasVerdict,
} from "../src/action-space";
import type {
  ActionSpace,
  Action,
  BlockedAction,
  VerdictSummary,
} from "../src/types";

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

function makeVerdictSummary(
  verdictType: string,
  producingRule = "check_rule",
): VerdictSummary {
  return {
    verdict_type: verdictType,
    payload: true,
    producing_rule: producingRule,
    stratum: 0,
  };
}

function makeAction(
  flowId: string,
  personaId = "admin",
  entryOpId = "approve_order",
): Action {
  return {
    flow_id: flowId,
    persona_id: personaId,
    entry_operation_id: entryOpId,
    enabling_verdicts: [makeVerdictSummary("account_active")],
    affected_entities: [
      {
        entity_id: "Order",
        current_state: "pending",
        possible_transitions: ["approved"],
      },
    ],
    description: `Execute ${flowId}: ${entryOpId} transitions Order from pending to approved`,
    instance_bindings: { Order: ["_default"] },
  };
}

function makeBlockedPersona(flowId: string): BlockedAction {
  return {
    flow_id: flowId,
    reason: { type: "PersonaNotAuthorized" },
    instance_bindings: {},
  };
}

function makeBlockedPrecondition(
  flowId: string,
  missingVerdicts: string[],
): BlockedAction {
  return {
    flow_id: flowId,
    reason: { type: "PreconditionNotMet", missing_verdicts: missingVerdicts },
    instance_bindings: {},
  };
}

function makeBlockedEntityState(
  flowId: string,
  entityId: string,
  currentState: string,
  requiredState: string,
): BlockedAction {
  return {
    flow_id: flowId,
    reason: {
      type: "EntityNotInSourceState",
      entity_id: entityId,
      current_state: currentState,
      required_state: requiredState,
    },
    instance_bindings: { [entityId]: ["_default"] },
  };
}

// Standard test action space: one available flow, one blocked flow
function makeStandardActionSpace(): ActionSpace {
  return {
    persona_id: "admin",
    actions: [makeAction("approval_flow"), makeAction("cancellation_flow")],
    current_verdicts: [
      makeVerdictSummary("account_active", "check_active"),
      makeVerdictSummary("high_value", "check_value"),
    ],
    blocked_actions: [
      makeBlockedPersona("restricted_flow"),
      makeBlockedPrecondition("premium_flow", ["premium_verdict"]),
    ],
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("actionsForFlow", () => {
  it("test_1_returns_matching_actions", () => {
    const space = makeStandardActionSpace();
    const result = actionsForFlow(space, "approval_flow");
    expect(result).toHaveLength(1);
    expect(result[0].flow_id).toBe("approval_flow");
  });

  it("test_2_returns_empty_for_unknown_flow", () => {
    const space = makeStandardActionSpace();
    expect(actionsForFlow(space, "unknown_flow")).toHaveLength(0);
  });

  it("test_3_returns_empty_for_blocked_flow", () => {
    const space = makeStandardActionSpace();
    // restricted_flow is blocked, not in actions
    expect(actionsForFlow(space, "restricted_flow")).toHaveLength(0);
  });

  it("test_4_returns_multiple_when_multiple_actions_for_same_flow", () => {
    // Simulate a flow with two different persona actions
    const space: ActionSpace = {
      persona_id: "admin",
      actions: [
        makeAction("multi_flow", "admin"),
        makeAction("multi_flow", "manager"),
      ],
      current_verdicts: [],
      blocked_actions: [],
    };
    const result = actionsForFlow(space, "multi_flow");
    expect(result).toHaveLength(2);
  });
});

describe("isFlowAvailable", () => {
  it("test_1_returns_true_when_action_exists", () => {
    const space = makeStandardActionSpace();
    expect(isFlowAvailable(space, "approval_flow")).toBe(true);
  });

  it("test_2_returns_false_when_no_action", () => {
    const space = makeStandardActionSpace();
    expect(isFlowAvailable(space, "unknown_flow")).toBe(false);
  });

  it("test_3_returns_false_for_blocked_flow", () => {
    const space = makeStandardActionSpace();
    // restricted_flow is in blocked_actions, not in actions
    expect(isFlowAvailable(space, "restricted_flow")).toBe(false);
  });

  it("test_4_returns_false_for_empty_action_space", () => {
    const space: ActionSpace = {
      persona_id: "guest",
      actions: [],
      current_verdicts: [],
      blocked_actions: [],
    };
    expect(isFlowAvailable(space, "any_flow")).toBe(false);
  });
});

describe("isFlowBlocked", () => {
  it("test_1_returns_true_when_blocked", () => {
    const space = makeStandardActionSpace();
    expect(isFlowBlocked(space, "restricted_flow")).toBe(true);
  });

  it("test_2_returns_false_when_available", () => {
    const space = makeStandardActionSpace();
    expect(isFlowBlocked(space, "approval_flow")).toBe(false);
  });

  it("test_3_returns_false_for_unknown_flow", () => {
    const space = makeStandardActionSpace();
    expect(isFlowBlocked(space, "unknown_flow")).toBe(false);
  });

  it("test_4_returns_false_for_empty_blocked_actions", () => {
    const space: ActionSpace = {
      persona_id: "admin",
      actions: [makeAction("approval_flow")],
      current_verdicts: [],
      blocked_actions: [],
    };
    expect(isFlowBlocked(space, "approval_flow")).toBe(false);
  });
});

describe("getBlockReason", () => {
  it("test_1_returns_PersonaNotAuthorized_reason", () => {
    const space = makeStandardActionSpace();
    const reason = getBlockReason(space, "restricted_flow");
    expect(reason).toBeDefined();
    expect(reason!.type).toBe("PersonaNotAuthorized");
  });

  it("test_2_returns_PreconditionNotMet_reason", () => {
    const space = makeStandardActionSpace();
    const reason = getBlockReason(space, "premium_flow");
    expect(reason).toBeDefined();
    expect(reason!.type).toBe("PreconditionNotMet");
    if (reason!.type === "PreconditionNotMet") {
      expect(reason!.missing_verdicts).toContain("premium_verdict");
    }
  });

  it("test_3_returns_EntityNotInSourceState_reason", () => {
    const space: ActionSpace = {
      persona_id: "admin",
      actions: [],
      current_verdicts: [],
      blocked_actions: [
        makeBlockedEntityState("approval_flow", "Order", "approved", "pending"),
      ],
    };
    const reason = getBlockReason(space, "approval_flow");
    expect(reason).toBeDefined();
    expect(reason!.type).toBe("EntityNotInSourceState");
    if (reason!.type === "EntityNotInSourceState") {
      expect(reason!.entity_id).toBe("Order");
      expect(reason!.current_state).toBe("approved");
      expect(reason!.required_state).toBe("pending");
    }
  });

  it("test_4_returns_undefined_for_unblocked_flow", () => {
    const space = makeStandardActionSpace();
    expect(getBlockReason(space, "approval_flow")).toBeUndefined();
  });

  it("test_5_returns_undefined_for_unknown_flow", () => {
    const space = makeStandardActionSpace();
    expect(getBlockReason(space, "unknown_flow")).toBeUndefined();
  });
});

describe("getBlockedAction", () => {
  it("test_1_returns_full_blocked_action", () => {
    const space = makeStandardActionSpace();
    const blocked = getBlockedAction(space, "restricted_flow");
    expect(blocked).toBeDefined();
    expect(blocked!.flow_id).toBe("restricted_flow");
    expect(blocked!.reason.type).toBe("PersonaNotAuthorized");
  });

  it("test_2_returns_undefined_for_available_flow", () => {
    const space = makeStandardActionSpace();
    expect(getBlockedAction(space, "approval_flow")).toBeUndefined();
  });
});

describe("availableFlowIds", () => {
  it("test_1_returns_deduplicated_flow_ids", () => {
    const space: ActionSpace = {
      persona_id: "admin",
      actions: [
        makeAction("flow_a"),
        makeAction("flow_b"),
        makeAction("flow_a"), // duplicate
      ],
      current_verdicts: [],
      blocked_actions: [],
    };
    const ids = availableFlowIds(space);
    expect(ids).toHaveLength(2);
    expect(ids).toContain("flow_a");
    expect(ids).toContain("flow_b");
  });

  it("test_2_returns_empty_for_no_actions", () => {
    const space: ActionSpace = {
      persona_id: "admin",
      actions: [],
      current_verdicts: [],
      blocked_actions: [makeBlockedPersona("blocked_flow")],
    };
    expect(availableFlowIds(space)).toHaveLength(0);
  });

  it("test_3_returns_correct_ids_from_standard_space", () => {
    const space = makeStandardActionSpace();
    const ids = availableFlowIds(space);
    expect(ids).toContain("approval_flow");
    expect(ids).toContain("cancellation_flow");
    expect(ids).not.toContain("restricted_flow");
  });
});

describe("blockedFlowIds", () => {
  it("test_1_returns_deduplicated_blocked_flow_ids", () => {
    const space: ActionSpace = {
      persona_id: "admin",
      actions: [],
      current_verdicts: [],
      blocked_actions: [
        makeBlockedPersona("flow_x"),
        makeBlockedPrecondition("flow_y", ["v1"]),
        makeBlockedPersona("flow_x"), // duplicate
      ],
    };
    const ids = blockedFlowIds(space);
    expect(ids).toHaveLength(2);
    expect(ids).toContain("flow_x");
    expect(ids).toContain("flow_y");
  });

  it("test_2_returns_empty_for_no_blocked_actions", () => {
    const space: ActionSpace = {
      persona_id: "admin",
      actions: [makeAction("approval_flow")],
      current_verdicts: [],
      blocked_actions: [],
    };
    expect(blockedFlowIds(space)).toHaveLength(0);
  });

  it("test_3_returns_correct_ids_from_standard_space", () => {
    const space = makeStandardActionSpace();
    const ids = blockedFlowIds(space);
    expect(ids).toContain("restricted_flow");
    expect(ids).toContain("premium_flow");
    expect(ids).not.toContain("approval_flow");
  });
});

describe("hasVerdict", () => {
  it("test_1_returns_true_when_verdict_present", () => {
    const space = makeStandardActionSpace();
    expect(hasVerdict(space, "account_active")).toBe(true);
  });

  it("test_2_returns_true_for_second_verdict", () => {
    const space = makeStandardActionSpace();
    expect(hasVerdict(space, "high_value")).toBe(true);
  });

  it("test_3_returns_false_when_verdict_absent", () => {
    const space = makeStandardActionSpace();
    expect(hasVerdict(space, "nonexistent_verdict")).toBe(false);
  });

  it("test_4_returns_false_for_empty_current_verdicts", () => {
    const space: ActionSpace = {
      persona_id: "admin",
      actions: [],
      current_verdicts: [],
      blocked_actions: [],
    };
    expect(hasVerdict(space, "account_active")).toBe(false);
  });
});
