/**
 * TenorEvaluator integration tests.
 *
 * These tests use the same BASIC_BUNDLE fixture as crates/tenor-eval-wasm/tests/wasm.rs
 * and verify identical results to the Rust evaluator for the same inputs.
 */

import { describe, it, expect, afterEach } from "vitest";
import { TenorEvaluator } from "../src/evaluator";
import type { InterchangeBundle } from "../src/types";

// ---------------------------------------------------------------------------
// Fixture: entity_operation_basic contract (same as wasm.rs BASIC_BUNDLE)
// ---------------------------------------------------------------------------

const BASIC_BUNDLE: InterchangeBundle = {
  id: "entity_operation_basic",
  kind: "Bundle",
  tenor: "1.0",
  tenor_version: "1.0.0",
  constructs: [
    {
      id: "is_active",
      kind: "Fact",
      provenance: { file: "test.tenor", line: 11 },
      source: { field: "active", system: "account" },
      tenor: "1.0",
      type: { base: "Bool" },
    },
    {
      id: "Order",
      initial: "pending",
      kind: "Entity",
      provenance: { file: "test.tenor", line: 3 },
      states: ["pending", "approved"],
      tenor: "1.0",
      transitions: [{ from: "pending", to: "approved" }],
    },
    {
      body: {
        produce: {
          payload: { type: { base: "Bool" }, value: true },
          verdict_type: "account_active",
        },
        when: {
          left: { fact_ref: "is_active" },
          op: "=",
          right: { literal: true, type: { base: "Bool" } },
        },
      },
      id: "check_active",
      kind: "Rule",
      provenance: { file: "test.tenor", line: 16 },
      stratum: 0,
      tenor: "1.0",
    },
    {
      allowed_personas: ["admin"],
      effects: [{ entity_id: "Order", from: "pending", to: "approved" }],
      error_contract: ["precondition_failed"],
      id: "approve_order",
      kind: "Operation",
      precondition: { verdict_present: "account_active" },
      provenance: { file: "test.tenor", line: 22 },
      tenor: "1.0",
    },
    {
      entry: "step_approve",
      id: "approval_flow",
      kind: "Flow",
      provenance: { file: "test.tenor", line: 29 },
      snapshot: "at_initiation",
      steps: [
        {
          id: "step_approve",
          kind: "OperationStep",
          on_failure: { kind: "Terminate", outcome: "approval_failed" },
          op: "approve_order",
          outcomes: {
            success: { kind: "Terminal", outcome: "order_approved" },
          },
          persona: "admin",
        },
      ],
      tenor: "1.0",
    },
  ],
};

const BASIC_BUNDLE_JSON = JSON.stringify(BASIC_BUNDLE);

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function loadBasicBundle(): TenorEvaluator {
  return TenorEvaluator.fromJson(BASIC_BUNDLE_JSON);
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe("TenorEvaluator", () => {
  // Track evaluators for cleanup
  const evaluators: TenorEvaluator[] = [];
  function track(ev: TenorEvaluator): TenorEvaluator {
    evaluators.push(ev);
    return ev;
  }

  afterEach(() => {
    for (const ev of evaluators.splice(0)) {
      if (!ev.isFreed) ev.free();
    }
  });

  // ── Test 1: Load contract ──────────────────────────────────────────────────

  it("test_1_load_contract_success", () => {
    const ev = track(TenorEvaluator.fromJson(BASIC_BUNDLE_JSON));
    expect(ev.isFreed).toBe(false);
  });

  // ── Test 2: Load invalid JSON ──────────────────────────────────────────────

  it("test_2_load_invalid_json_throws", () => {
    expect(() => TenorEvaluator.fromJson("not json")).toThrow();
  });

  // ── Test 3: Load invalid bundle ────────────────────────────────────────────

  it("test_3_load_invalid_bundle_throws", () => {
    expect(() =>
      TenorEvaluator.fromJson(JSON.stringify({ not: "a bundle" })),
    ).toThrow();
  });

  // ── Test 4: Evaluate produces verdicts ────────────────────────────────────

  it("test_4_evaluate_produces_verdicts", () => {
    const ev = track(loadBasicBundle());
    const result = ev.evaluate({ is_active: true });

    expect(result.verdicts).toHaveLength(1);
    expect(result.verdicts[0].type).toBe("account_active");

    // Verify provenance (matches Rust evaluator output)
    const prov = result.verdicts[0].provenance;
    expect(prov.rule).toBe("check_active");
    expect(prov.stratum).toBe(0);
    expect(prov.facts_used).toContain("is_active");
  });

  // ── Test 5: Evaluate — no verdict when false ───────────────────────────────

  it("test_5_evaluate_no_verdict_when_false", () => {
    const ev = track(loadBasicBundle());
    const result = ev.evaluate({ is_active: false });
    expect(result.verdicts).toHaveLength(0);
  });

  // ── Test 6: Evaluate — missing required fact ───────────────────────────────

  it("test_6_evaluate_missing_required_fact_throws", () => {
    const ev = track(loadBasicBundle());
    expect(() => ev.evaluate({})).toThrow();
  });

  // ── Test 7: Execute flow success ──────────────────────────────────────────

  it("test_7_execute_flow_success", () => {
    const ev = track(loadBasicBundle());
    const result = ev.executeFlow(
      "approval_flow",
      { is_active: true },
      {},
      "admin",
    );

    expect(result.simulation).toBe(true);
    expect(result.flow_id).toBe("approval_flow");
    expect(result.outcome).toBe("order_approved");
    expect(result.path.length).toBeGreaterThan(0);
    expect(result.would_transition.length).toBeGreaterThan(0);

    // Check transition structure
    const transition = result.would_transition[0];
    expect(transition.entity_id).toBe("Order");
    expect(transition.from_state).toBe("pending");
    expect(transition.to_state).toBe("approved");
  });

  // ── Test 8: Execute flow — precondition fails ─────────────────────────────

  it("test_8_execute_flow_precondition_fails", () => {
    const ev = track(loadBasicBundle());
    const result = ev.executeFlow(
      "approval_flow",
      { is_active: false },
      {},
      "admin",
    );

    expect(result.simulation).toBe(true);
    expect(result.outcome).toBe("approval_failed");
  });

  // ── Test 9: Execute flow — flow not found ─────────────────────────────────

  it("test_9_execute_flow_not_found_throws", () => {
    const ev = track(loadBasicBundle());
    expect(() =>
      ev.executeFlow("nonexistent_flow", { is_active: true }, {}, "admin"),
    ).toThrow();
  });

  // ── Test 10: Compute action space — available ─────────────────────────────

  it("test_10_compute_action_space_available", () => {
    const ev = track(loadBasicBundle());
    const space = ev.computeActionSpace(
      { is_active: true },
      { Order: "pending" },
      "admin",
    );

    expect(space.persona_id).toBe("admin");
    expect(space.actions).toHaveLength(1);
    expect(space.actions[0].flow_id).toBe("approval_flow");
    expect(space.actions[0].entry_operation_id).toBe("approve_order");
    expect(space.blocked_actions).toHaveLength(0);
  });

  // ── Test 11: Compute action space — blocked persona ───────────────────────

  it("test_11_compute_action_space_blocked_persona", () => {
    const ev = track(loadBasicBundle());
    const space = ev.computeActionSpace(
      { is_active: true },
      { Order: "pending" },
      "guest",
    );

    expect(space.actions).toHaveLength(0);
    expect(space.blocked_actions).toHaveLength(1);
    expect(space.blocked_actions[0].reason.type).toBe("PersonaNotAuthorized");
  });

  // ── Test 12: Compute action space — blocked precondition ──────────────────

  it("test_12_compute_action_space_blocked_precondition", () => {
    const ev = track(loadBasicBundle());
    const space = ev.computeActionSpace(
      { is_active: false },
      { Order: "pending" },
      "admin",
    );

    expect(space.actions).toHaveLength(0);
    expect(space.blocked_actions).toHaveLength(1);

    const reason = space.blocked_actions[0].reason;
    expect(reason.type).toBe("PreconditionNotMet");
    if (reason.type === "PreconditionNotMet") {
      expect(reason.missing_verdicts).toContain("account_active");
    }
  });

  // ── Test 13: Compute action space — blocked entity state ──────────────────

  it("test_13_compute_action_space_blocked_entity_state", () => {
    const ev = track(loadBasicBundle());
    const space = ev.computeActionSpace(
      { is_active: true },
      { Order: "approved" },
      "admin",
    );

    expect(space.actions).toHaveLength(0);
    expect(space.blocked_actions).toHaveLength(1);
    expect(space.blocked_actions[0].reason.type).toBe("EntityNotInSourceState");
  });

  // ── Test 14: Inspect contract ─────────────────────────────────────────────

  it("test_14_inspect_contract", () => {
    const ev = track(loadBasicBundle());
    const info = ev.inspect();

    expect(info.facts.length).toBeGreaterThan(0);
    expect(info.entities.length).toBeGreaterThan(0);
    expect(info.rules.length).toBeGreaterThan(0);
    expect(info.operations.length).toBeGreaterThan(0);
    expect(info.flows.length).toBeGreaterThan(0);

    // Verify fact details
    const fact = info.facts.find((f) => f.id === "is_active");
    expect(fact).toBeDefined();
    expect(fact!.type).toBe("Bool");

    // Verify entity details
    const entity = info.entities.find((e) => e.id === "Order");
    expect(entity).toBeDefined();
    expect(entity!.initial).toBe("pending");
  });

  // ── Test 15: Free and reuse ────────────────────────────────────────────────

  it("test_15_free_and_reuse", () => {
    const ev1 = TenorEvaluator.fromJson(BASIC_BUNDLE_JSON);
    const ev2 = track(TenorEvaluator.fromJson(BASIC_BUNDLE_JSON));

    // Both work before free
    expect(ev1.evaluate({ is_active: true }).verdicts).toHaveLength(1);
    expect(ev2.evaluate({ is_active: true }).verdicts).toHaveLength(1);

    // Free ev1
    ev1.free();
    expect(ev1.isFreed).toBe(true);

    // ev1 throws after free
    expect(() => ev1.evaluate({ is_active: true })).toThrow(
      "TenorEvaluator has been freed",
    );

    // ev2 still works
    expect(ev2.evaluate({ is_active: true }).verdicts).toHaveLength(1);

    // Double-free is a no-op
    expect(() => ev1.free()).not.toThrow();
  });

  // ── Test 16: Results match Rust evaluator ─────────────────────────────────
  //
  // This test verifies that for the BASIC_BUNDLE, given {is_active: true},
  // the TypeScript SDK produces the SAME results as the Rust evaluator.
  // The expected values are derived from the Rust WASM tests in wasm.rs.

  it("test_16_results_match_rust_evaluator", () => {
    const ev = track(loadBasicBundle());

    // (a) Verdicts match
    const verdicts = ev.evaluate({ is_active: true });
    expect(verdicts.verdicts).toHaveLength(1);
    expect(verdicts.verdicts[0].type).toBe("account_active"); // Rust: verdicts[0]["type"] == "account_active"
    expect(verdicts.verdicts[0].provenance.rule).toBe("check_active"); // Rust: prov["rule"] == "check_active"
    expect(verdicts.verdicts[0].provenance.stratum).toBe(0); // Rust: prov["stratum"] == 0
    expect(verdicts.verdicts[0].provenance.facts_used).toContain("is_active"); // Rust: facts_used contains "is_active"

    // (b) Flow result matches
    const flow = ev.executeFlow(
      "approval_flow",
      { is_active: true },
      {},
      "admin",
    );
    expect(flow.simulation).toBe(true); // Rust: parsed["simulation"] == true
    expect(flow.flow_id).toBe("approval_flow"); // Rust: parsed["flow_id"] == "approval_flow"
    expect(flow.outcome).toBe("order_approved"); // Rust: parsed["outcome"] == "order_approved"
    expect(flow.path.length).toBeGreaterThan(0); // Rust: path.length > 0
    expect(flow.would_transition.length).toBeGreaterThan(0); // Rust: would_transition.length > 0

    // (c) Action space matches
    const space = ev.computeActionSpace(
      { is_active: true },
      { Order: "pending" },
      "admin",
    );
    expect(space.persona_id).toBe("admin"); // Rust: parsed["persona_id"] == "admin"
    expect(space.actions).toHaveLength(1); // Rust: actions.len() == 1
    expect(space.actions[0].flow_id).toBe("approval_flow"); // Rust: actions[0]["flow_id"] == "approval_flow"
    expect(space.actions[0].entry_operation_id).toBe("approve_order"); // Rust: actions[0]["entry_operation_id"] == "approve_order"
    expect(space.blocked_actions).toHaveLength(0); // Rust: blocked.len() == 0

    // (d) Current verdicts match action space
    expect(space.current_verdicts).toHaveLength(1); // Rust: verdicts.len() == 1
    expect(space.current_verdicts[0].verdict_type).toBe("account_active"); // Rust: verdicts[0]["verdict_type"] == "account_active"
    expect(space.current_verdicts[0].producing_rule).toBe("check_active"); // Rust: verdicts[0]["producing_rule"] == "check_active"
  });

  // ── Additional: fromBundle ─────────────────────────────────────────────────

  it("fromBundle_works_identically_to_fromJson", () => {
    const ev1 = track(TenorEvaluator.fromBundle(BASIC_BUNDLE));
    const ev2 = track(TenorEvaluator.fromJson(BASIC_BUNDLE_JSON));

    const v1 = ev1.evaluate({ is_active: true });
    const v2 = ev2.evaluate({ is_active: true });

    expect(v1.verdicts).toHaveLength(v2.verdicts.length);
    expect(v1.verdicts[0].type).toBe(v2.verdicts[0].type);
  });
});
