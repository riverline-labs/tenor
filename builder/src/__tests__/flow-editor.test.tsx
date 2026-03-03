/**
 * Flow editor tests.
 *
 * Tests flow management: OperationStep, BranchStep, DAG acyclicity, outcome handling.
 */
import { describe, it, expect } from "vitest";
import type {
  FlowConstruct,
  FlowStep,
  OperationStep,
  BranchStep,
  HandoffStep,
  StepTarget,
  FailureHandler,
} from "../types/interchange";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TV = "1.0";

function newFlow(id: string): FlowConstruct {
  return {
    id,
    kind: "Flow",
    tenor: TV,
    provenance: { file: "builder", line: 0 },
    entry: "start",
    steps: [],
  };
}

function newOperationStep(
  id: string,
  op: string,
  persona: string,
  outcomes: Record<string, StepTarget> = {},
  onFailure: FailureHandler = { kind: "Terminate", outcome: "failed" }
): OperationStep {
  return {
    id,
    kind: "OperationStep",
    op,
    persona,
    outcomes,
    on_failure: onFailure,
  };
}

/**
 * Detect cycles in a flow's step graph using DFS.
 * Returns true if the flow has a cycle.
 */
function hasCycle(flow: FlowConstruct): boolean {
  const stepMap = new Map<string, FlowStep>(flow.steps.map((s) => [s.id, s]));

  function getTargetIds(target: StepTarget): string[] {
    if (typeof target === "string") return [target];
    return []; // Terminal targets don't step forward
  }

  function getNextSteps(step: FlowStep): string[] {
    const nexts: string[] = [];
    switch (step.kind) {
      case "OperationStep": {
        for (const t of Object.values(step.outcomes)) {
          nexts.push(...getTargetIds(t));
        }
        break;
      }
      case "BranchStep": {
        nexts.push(...getTargetIds(step.if_true));
        nexts.push(...getTargetIds(step.if_false));
        break;
      }
      case "HandoffStep": {
        nexts.push(step.next);
        break;
      }
      case "SubFlowStep": {
        nexts.push(...getTargetIds(step.on_success));
        break;
      }
    }
    return nexts.filter((n) => stepMap.has(n));
  }

  const visited = new Set<string>();
  const inStack = new Set<string>();

  function dfs(id: string): boolean {
    if (inStack.has(id)) return true; // cycle
    if (visited.has(id)) return false;
    visited.add(id);
    inStack.add(id);
    const step = stepMap.get(id);
    if (step) {
      for (const nextId of getNextSteps(step)) {
        if (dfs(nextId)) return true;
      }
    }
    inStack.delete(id);
    return false;
  }

  for (const step of flow.steps) {
    if (dfs(step.id)) return true;
  }
  return false;
}

/**
 * Find unhandled outcomes: OperationSteps whose outcomes map doesn't cover all operation outcomes.
 */
function findUnhandledOutcomes(flow: FlowConstruct, opOutcomes: Map<string, string[]>): string[] {
  const issues: string[] = [];
  for (const step of flow.steps) {
    if (step.kind === "OperationStep") {
      const declared = opOutcomes.get(step.op) ?? [];
      for (const outcome of declared) {
        if (!(outcome in step.outcomes)) {
          issues.push(`Step '${step.id}' missing outcome routing for '${outcome}'`);
        }
      }
    }
  }
  return issues;
}

// ---------------------------------------------------------------------------
// Flow step creation tests
// ---------------------------------------------------------------------------

describe("Flow editor — step creation", () => {
  it("creates flow with OperationStep", () => {
    const flow = newFlow("deposit_flow");
    const step = newOperationStep(
      "deposit_step",
      "deposit",
      "buyer",
      { success: { kind: "Terminal", outcome: "completed" } }
    );
    const updated: FlowConstruct = {
      ...flow,
      entry: step.id,
      steps: [step],
    };
    expect(updated.steps).toHaveLength(1);
    expect(updated.steps[0].kind).toBe("OperationStep");
    const opStep = updated.steps[0] as OperationStep;
    expect(opStep.op).toBe("deposit");
    expect(opStep.persona).toBe("buyer");
    expect(opStep.outcomes.success).toEqual({ kind: "Terminal", outcome: "completed" });
    expect(opStep.on_failure).toEqual({ kind: "Terminate", outcome: "failed" });
  });

  it("creates flow with BranchStep", () => {
    const branchStep: BranchStep = {
      id: "check_flag",
      kind: "BranchStep",
      persona: "system",
      condition: {
        left: { fact_ref: "is_approved" },
        op: "=",
        right: { literal: true, type: { base: "Bool" } },
      } as BranchStep["condition"],
      if_true: "approve_step",
      if_false: { kind: "Terminal", outcome: "rejected" },
    };
    const flow: FlowConstruct = {
      ...newFlow("check_flow"),
      entry: "check_flag",
      steps: [branchStep],
    };
    const step = flow.steps[0] as BranchStep;
    expect(step.kind).toBe("BranchStep");
    expect(typeof step.if_true).toBe("string");
    expect(step.if_false).toEqual({ kind: "Terminal", outcome: "rejected" });
  });

  it("creates flow with HandoffStep", () => {
    const handoffStep: HandoffStep = {
      id: "handoff",
      kind: "HandoffStep",
      from_persona: "buyer",
      to_persona: "seller",
      next: "confirm_step",
    };
    const flow: FlowConstruct = {
      ...newFlow("handoff_flow"),
      entry: "handoff",
      steps: [handoffStep],
    };
    const step = flow.steps[0] as HandoffStep;
    expect(step.kind).toBe("HandoffStep");
    expect(step.from_persona).toBe("buyer");
    expect(step.to_persona).toBe("seller");
    expect(step.next).toBe("confirm_step");
  });

  it("sets entry step", () => {
    const flow = newFlow("my_flow");
    const step = newOperationStep("first_step", "init_op", "buyer");
    const updated: FlowConstruct = {
      ...flow,
      entry: "first_step",
      steps: [step],
    };
    expect(updated.entry).toBe("first_step");
  });
});

// ---------------------------------------------------------------------------
// DAG validation tests
// ---------------------------------------------------------------------------

describe("Flow editor — DAG validation", () => {
  it("linear flow (A -> Terminal) has no cycle", () => {
    const flow: FlowConstruct = {
      ...newFlow("linear_flow"),
      entry: "step_a",
      steps: [
        newOperationStep(
          "step_a",
          "op_a",
          "buyer",
          { success: { kind: "Terminal", outcome: "done" } }
        ),
      ],
    };
    expect(hasCycle(flow)).toBe(false);
  });

  it("two-step chain (A -> B -> Terminal) has no cycle", () => {
    const flow: FlowConstruct = {
      ...newFlow("chain_flow"),
      entry: "step_a",
      steps: [
        newOperationStep("step_a", "op_a", "buyer", { success: "step_b" }),
        newOperationStep("step_b", "op_b", "seller", {
          success: { kind: "Terminal", outcome: "done" },
        }),
      ],
    };
    expect(hasCycle(flow)).toBe(false);
  });

  it("cyclic flow (A -> B -> A) is detected", () => {
    const flow: FlowConstruct = {
      ...newFlow("cyclic_flow"),
      entry: "step_a",
      steps: [
        newOperationStep("step_a", "op_a", "buyer", { success: "step_b" }),
        newOperationStep("step_b", "op_b", "seller", { success: "step_a" }), // cycle
      ],
    };
    expect(hasCycle(flow)).toBe(true);
  });

  it("self-loop is detected as cycle", () => {
    const flow: FlowConstruct = {
      ...newFlow("self_loop"),
      entry: "step_a",
      steps: [
        newOperationStep("step_a", "op_a", "buyer", { success: "step_a" }), // self-loop
      ],
    };
    expect(hasCycle(flow)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Outcome handling tests
// ---------------------------------------------------------------------------

describe("Flow editor — outcome handling", () => {
  it("no unhandled outcomes when all operation outcomes routed", () => {
    const flow: FlowConstruct = {
      ...newFlow("complete_flow"),
      entry: "deposit",
      steps: [
        newOperationStep("deposit", "deposit_op", "buyer", {
          success: { kind: "Terminal", outcome: "done" },
          disputed: { kind: "Terminal", outcome: "disputed" },
        }),
      ],
    };
    const opOutcomes = new Map([["deposit_op", ["success", "disputed"]]]);
    const issues = findUnhandledOutcomes(flow, opOutcomes);
    expect(issues).toHaveLength(0);
  });

  it("detects unhandled outcome in OperationStep", () => {
    const flow: FlowConstruct = {
      ...newFlow("incomplete_flow"),
      entry: "deposit",
      steps: [
        newOperationStep("deposit", "deposit_op", "buyer", {
          success: { kind: "Terminal", outcome: "done" },
          // missing "disputed" routing
        }),
      ],
    };
    const opOutcomes = new Map([["deposit_op", ["success", "disputed"]]]);
    const issues = findUnhandledOutcomes(flow, opOutcomes);
    expect(issues).toHaveLength(1);
    expect(issues[0]).toContain("disputed");
  });

  it("all Terminal targets are valid step targets", () => {
    const terminalSuccess: StepTarget = { kind: "Terminal", outcome: "success" };
    const terminalFailed: StepTarget = { kind: "Terminal", outcome: "failed" };
    expect(typeof terminalSuccess).toBe("object");
    expect((terminalSuccess as { kind: string }).kind).toBe("Terminal");
    expect((terminalFailed as { outcome: string }).outcome).toBe("failed");
  });

  it("on_failure Terminate routes correctly", () => {
    const step = newOperationStep(
      "step_a",
      "op_a",
      "buyer",
      { success: { kind: "Terminal", outcome: "done" } },
      { kind: "Terminate", outcome: "error_occurred" }
    );
    const handler = step.on_failure as { kind: string; outcome: string };
    expect(handler.kind).toBe("Terminate");
    expect(handler.outcome).toBe("error_occurred");
  });
});
