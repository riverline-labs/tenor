package tenor_test

import (
	"testing"

	tenor "github.com/riverline-labs/tenor-go"
)

// BASIC_BUNDLE is the same fixture used in crates/tenor-eval-wasm/tests/wasm.rs.
// It contains Fact (is_active: Bool), Entity (Order), Rule (check_active),
// Operation (approve_order with precondition account_active) and Flow (approval_flow).
const basicBundle = `{
  "constructs": [
    {
      "id": "is_active",
      "kind": "Fact",
      "provenance": { "file": "test.tenor", "line": 11 },
      "source": { "field": "active", "system": "account" },
      "tenor": "1.0",
      "type": { "base": "Bool" }
    },
    {
      "id": "Order",
      "initial": "pending",
      "kind": "Entity",
      "provenance": { "file": "test.tenor", "line": 3 },
      "states": ["pending", "approved"],
      "tenor": "1.0",
      "transitions": [{ "from": "pending", "to": "approved" }]
    },
    {
      "body": {
        "produce": {
          "payload": { "type": { "base": "Bool" }, "value": true },
          "verdict_type": "account_active"
        },
        "when": {
          "left": { "fact_ref": "is_active" },
          "op": "=",
          "right": { "literal": true, "type": { "base": "Bool" } }
        }
      },
      "id": "check_active",
      "kind": "Rule",
      "provenance": { "file": "test.tenor", "line": 16 },
      "stratum": 0,
      "tenor": "1.0"
    },
    {
      "allowed_personas": ["admin"],
      "effects": [{ "entity_id": "Order", "from": "pending", "to": "approved" }],
      "error_contract": ["precondition_failed"],
      "id": "approve_order",
      "kind": "Operation",
      "precondition": { "verdict_present": "account_active" },
      "provenance": { "file": "test.tenor", "line": 22 },
      "tenor": "1.0"
    },
    {
      "entry": "step_approve",
      "id": "approval_flow",
      "kind": "Flow",
      "provenance": { "file": "test.tenor", "line": 29 },
      "snapshot": "at_initiation",
      "steps": [
        {
          "id": "step_approve",
          "kind": "OperationStep",
          "on_failure": { "kind": "Terminate", "outcome": "approval_failed" },
          "op": "approve_order",
          "outcomes": {
            "success": { "kind": "Terminal", "outcome": "order_approved" }
          },
          "persona": "admin"
        }
      ],
      "tenor": "1.0"
    }
  ],
  "id": "entity_operation_basic",
  "kind": "Bundle",
  "tenor": "1.0",
  "tenor_version": "1.0.0"
}`

// ── Contract loading ──

func TestLoadValidBundle(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("expected no error loading valid bundle, got: %v", err)
	}
	defer func() {
		if err := eval.Close(); err != nil {
			t.Errorf("Close failed: %v", err)
		}
	}()
}

func TestLoadInvalidJSON(t *testing.T) {
	_, err := tenor.NewEvaluatorFromBundle([]byte("not json"))
	if err == nil {
		t.Fatal("expected error for invalid JSON, got nil")
	}
}

func TestLoadInvalidBundle(t *testing.T) {
	_, err := tenor.NewEvaluatorFromBundle([]byte(`{"not": "a bundle"}`))
	if err == nil {
		t.Fatal("expected error for invalid bundle structure, got nil")
	}
}

// ── Evaluate ──

func TestEvaluate(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	result, err := eval.Evaluate(tenor.FactSet{"is_active": true})
	if err != nil {
		t.Fatalf("Evaluate failed: %v", err)
	}
	if len(result.Verdicts) != 1 {
		t.Fatalf("expected 1 verdict, got %d", len(result.Verdicts))
	}
	if result.Verdicts[0].Type != "account_active" {
		t.Errorf("expected verdict type 'account_active', got %q", result.Verdicts[0].Type)
	}
	if result.Verdicts[0].Provenance.Rule != "check_active" {
		t.Errorf("expected rule 'check_active', got %q", result.Verdicts[0].Provenance.Rule)
	}
	if result.Verdicts[0].Provenance.Stratum != 0 {
		t.Errorf("expected stratum 0, got %d", result.Verdicts[0].Provenance.Stratum)
	}
	// Verify facts_used includes is_active
	found := false
	for _, f := range result.Verdicts[0].Provenance.FactsUsed {
		if f == "is_active" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected facts_used to include 'is_active', got %v", result.Verdicts[0].Provenance.FactsUsed)
	}
}

func TestEvaluateNoVerdictWhenFalse(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	result, err := eval.Evaluate(tenor.FactSet{"is_active": false})
	if err != nil {
		t.Fatalf("Evaluate failed: %v", err)
	}
	if len(result.Verdicts) != 0 {
		t.Errorf("expected 0 verdicts when is_active=false, got %d", len(result.Verdicts))
	}
}

func TestEvaluateMissingRequiredFact(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	_, err = eval.Evaluate(tenor.FactSet{})
	if err == nil {
		t.Fatal("expected error for missing required fact, got nil")
	}
}

// ── ComputeActionSpace ──

func TestComputeActionSpaceAvailable(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	space, err := eval.ComputeActionSpace(
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMap{"Order": "pending"},
		"admin",
	)
	if err != nil {
		t.Fatalf("ComputeActionSpace failed: %v", err)
	}

	if space.PersonaID != "admin" {
		t.Errorf("expected persona_id 'admin', got %q", space.PersonaID)
	}
	if len(space.Actions) != 1 {
		t.Fatalf("expected 1 available action, got %d", len(space.Actions))
	}
	if space.Actions[0].FlowID != "approval_flow" {
		t.Errorf("expected flow_id 'approval_flow', got %q", space.Actions[0].FlowID)
	}
	if space.Actions[0].EntryOperationID != "approve_order" {
		t.Errorf("expected entry_operation_id 'approve_order', got %q", space.Actions[0].EntryOperationID)
	}
	if len(space.BlockedActions) != 0 {
		t.Errorf("expected 0 blocked actions, got %d", len(space.BlockedActions))
	}
}

func TestComputeActionSpaceBlockedPersona(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	space, err := eval.ComputeActionSpace(
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMap{"Order": "pending"},
		"guest",
	)
	if err != nil {
		t.Fatalf("ComputeActionSpace failed: %v", err)
	}

	if len(space.Actions) != 0 {
		t.Errorf("expected 0 actions for guest persona, got %d", len(space.Actions))
	}
	if len(space.BlockedActions) != 1 {
		t.Fatalf("expected 1 blocked action, got %d", len(space.BlockedActions))
	}
	if space.BlockedActions[0].Reason.Type != "PersonaNotAuthorized" {
		t.Errorf("expected PersonaNotAuthorized, got %q", space.BlockedActions[0].Reason.Type)
	}
}

func TestComputeActionSpaceBlockedPrecondition(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	space, err := eval.ComputeActionSpace(
		tenor.FactSet{"is_active": false},
		tenor.EntityStateMap{"Order": "pending"},
		"admin",
	)
	if err != nil {
		t.Fatalf("ComputeActionSpace failed: %v", err)
	}

	if len(space.Actions) != 0 {
		t.Errorf("expected 0 actions when precondition not met, got %d", len(space.Actions))
	}
	if len(space.BlockedActions) != 1 {
		t.Fatalf("expected 1 blocked action, got %d", len(space.BlockedActions))
	}
	if space.BlockedActions[0].Reason.Type != "PreconditionNotMet" {
		t.Errorf("expected PreconditionNotMet, got %q", space.BlockedActions[0].Reason.Type)
	}
	found := false
	for _, v := range space.BlockedActions[0].Reason.MissingVerdicts {
		if v == "account_active" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected missing_verdicts to include 'account_active', got %v",
			space.BlockedActions[0].Reason.MissingVerdicts)
	}
}

func TestComputeActionSpaceBlockedEntityState(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	space, err := eval.ComputeActionSpace(
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMap{"Order": "approved"},
		"admin",
	)
	if err != nil {
		t.Fatalf("ComputeActionSpace failed: %v", err)
	}

	if len(space.Actions) != 0 {
		t.Errorf("expected 0 actions when entity in wrong state, got %d", len(space.Actions))
	}
	if len(space.BlockedActions) != 1 {
		t.Fatalf("expected 1 blocked action, got %d", len(space.BlockedActions))
	}
	if space.BlockedActions[0].Reason.Type != "EntityNotInSourceState" {
		t.Errorf("expected EntityNotInSourceState, got %q", space.BlockedActions[0].Reason.Type)
	}
}

func TestComputeActionSpaceCurrentVerdicts(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	space, err := eval.ComputeActionSpace(
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMap{},
		"admin",
	)
	if err != nil {
		t.Fatalf("ComputeActionSpace failed: %v", err)
	}

	if len(space.CurrentVerdicts) != 1 {
		t.Fatalf("expected 1 current verdict, got %d", len(space.CurrentVerdicts))
	}
	if space.CurrentVerdicts[0].VerdictType != "account_active" {
		t.Errorf("expected verdict_type 'account_active', got %q", space.CurrentVerdicts[0].VerdictType)
	}
	if space.CurrentVerdicts[0].ProducingRule != "check_active" {
		t.Errorf("expected producing_rule 'check_active', got %q", space.CurrentVerdicts[0].ProducingRule)
	}
}

// ── ExecuteFlow ──

func TestExecuteFlowSuccess(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	result, err := eval.ExecuteFlow(
		"approval_flow",
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMap{},
		"admin",
	)
	if err != nil {
		t.Fatalf("ExecuteFlow failed: %v", err)
	}

	if !result.Simulation {
		t.Error("expected simulation=true")
	}
	if result.FlowID != "approval_flow" {
		t.Errorf("expected flow_id 'approval_flow', got %q", result.FlowID)
	}
	if result.Outcome != "order_approved" {
		t.Errorf("expected outcome 'order_approved', got %q", result.Outcome)
	}
	if len(result.Path) == 0 {
		t.Error("expected non-empty path")
	}
	if len(result.WouldTransition) == 0 {
		t.Error("expected non-empty would_transition")
	}
}

func TestExecuteFlowPreconditionFails(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	result, err := eval.ExecuteFlow(
		"approval_flow",
		tenor.FactSet{"is_active": false},
		tenor.EntityStateMap{},
		"admin",
	)
	if err != nil {
		t.Fatalf("ExecuteFlow failed: %v", err)
	}

	if result.Outcome != "approval_failed" {
		t.Errorf("expected outcome 'approval_failed', got %q", result.Outcome)
	}
}

func TestExecuteFlowNotFound(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	_, err = eval.ExecuteFlow(
		"nonexistent_flow",
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMap{},
		"admin",
	)
	if err == nil {
		t.Fatal("expected error for nonexistent flow, got nil")
	}
}

// ── Results match Rust evaluator ──

// TestResultsMatchRustEvaluator verifies that the Go SDK produces identical
// results to the Rust evaluator for the same inputs. We compare the key
// fields that the Rust tests also verify (type, rule, stratum).
func TestResultsMatchRustEvaluator(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	// Test 1: evaluate(is_active=true) -> account_active verdict from check_active at stratum 0
	verdicts, err := eval.Evaluate(tenor.FactSet{"is_active": true})
	if err != nil {
		t.Fatalf("Evaluate failed: %v", err)
	}
	if len(verdicts.Verdicts) != 1 || verdicts.Verdicts[0].Type != "account_active" {
		t.Errorf("Rust evaluator produces account_active from is_active=true; Go got: %+v", verdicts.Verdicts)
	}
	if verdicts.Verdicts[0].Provenance.Rule != "check_active" {
		t.Errorf("Rust evaluator: rule=check_active; Go got: %q", verdicts.Verdicts[0].Provenance.Rule)
	}
	if verdicts.Verdicts[0].Provenance.Stratum != 0 {
		t.Errorf("Rust evaluator: stratum=0; Go got: %d", verdicts.Verdicts[0].Provenance.Stratum)
	}

	// Test 2: simulate_flow(approval_flow, is_active=true) -> order_approved
	flowResult, err := eval.ExecuteFlow(
		"approval_flow",
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMap{},
		"admin",
	)
	if err != nil {
		t.Fatalf("ExecuteFlow failed: %v", err)
	}
	if flowResult.Outcome != "order_approved" {
		t.Errorf("Rust evaluator: outcome=order_approved; Go got: %q", flowResult.Outcome)
	}
	if len(flowResult.WouldTransition) == 0 {
		t.Error("Rust evaluator: would_transition non-empty; Go got empty")
	} else {
		wt := flowResult.WouldTransition[0]
		if wt.EntityID != "Order" || wt.FromState != "pending" || wt.ToState != "approved" {
			t.Errorf("Rust evaluator: Order pending->approved; Go got: %+v", wt)
		}
	}

	// Test 3: compute_action_space(is_active=true, Order=pending, admin) -> 1 action
	space, err := eval.ComputeActionSpace(
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMap{"Order": "pending"},
		"admin",
	)
	if err != nil {
		t.Fatalf("ComputeActionSpace failed: %v", err)
	}
	if len(space.Actions) != 1 || space.Actions[0].FlowID != "approval_flow" {
		t.Errorf("Rust evaluator: 1 action (approval_flow); Go got: %d actions", len(space.Actions))
	}
}

// ── Multi-instance ──

func TestComputeActionSpaceNestedFormatAvailable(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	space, err := eval.ComputeActionSpaceNested(
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMapNested{"Order": {"ord-001": "pending"}},
		"admin",
	)
	if err != nil {
		t.Fatalf("ComputeActionSpaceNested failed: %v", err)
	}

	if len(space.Actions) != 1 {
		t.Fatalf("expected 1 action, got %d", len(space.Actions))
	}
	// instance_bindings should include ord-001
	orderBindings, ok := space.Actions[0].InstanceBindings["Order"]
	if !ok {
		t.Fatal("expected Order in instance_bindings")
	}
	found := false
	for _, iid := range orderBindings {
		if iid == "ord-001" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected 'ord-001' in instance_bindings['Order'], got %v", orderBindings)
	}
}

func TestExecuteFlowWithBindings(t *testing.T) {
	eval, err := tenor.NewEvaluatorFromBundle([]byte(basicBundle))
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	defer eval.Close()

	result, err := eval.ExecuteFlowWithBindings(
		"approval_flow",
		tenor.FactSet{"is_active": true},
		tenor.EntityStateMapNested{"Order": {"ord-001": "pending"}},
		"admin",
		tenor.InstanceBindings{"Order": "ord-001"},
	)
	if err != nil {
		t.Fatalf("ExecuteFlowWithBindings failed: %v", err)
	}

	if result.Outcome != "order_approved" {
		t.Errorf("expected outcome 'order_approved', got %q", result.Outcome)
	}
	if len(result.WouldTransition) == 0 {
		t.Fatal("expected non-empty would_transition")
	}
	wt := result.WouldTransition[0]
	if wt.EntityID != "Order" {
		t.Errorf("expected entity_id 'Order', got %q", wt.EntityID)
	}
	if wt.InstanceID != "ord-001" {
		t.Errorf("expected instance_id 'ord-001', got %q", wt.InstanceID)
	}
	if wt.FromState != "pending" || wt.ToState != "approved" {
		t.Errorf("expected pending->approved, got %q->%q", wt.FromState, wt.ToState)
	}
}
