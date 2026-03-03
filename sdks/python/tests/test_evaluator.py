"""Tests for TenorEvaluator Python SDK.

Mirrors the WASM test suite from crates/tenor-eval-wasm/tests/wasm.rs
using the same BASIC_BUNDLE fixture to prove cross-SDK consistency.
"""

import json
import pytest
from tenor import TenorEvaluator


# Same bundle as crates/tenor-eval-wasm/tests/wasm.rs BASIC_BUNDLE
BASIC_BUNDLE = json.dumps({
    "constructs": [
        {
            "id": "is_active",
            "kind": "Fact",
            "provenance": {"file": "test.tenor", "line": 11},
            "source": {"field": "active", "system": "account"},
            "tenor": "1.0",
            "type": {"base": "Bool"},
        },
        {
            "id": "Order",
            "initial": "pending",
            "kind": "Entity",
            "provenance": {"file": "test.tenor", "line": 3},
            "states": ["pending", "approved"],
            "tenor": "1.0",
            "transitions": [{"from": "pending", "to": "approved"}],
        },
        {
            "body": {
                "produce": {
                    "payload": {"type": {"base": "Bool"}, "value": True},
                    "verdict_type": "account_active",
                },
                "when": {
                    "left": {"fact_ref": "is_active"},
                    "op": "=",
                    "right": {"literal": True, "type": {"base": "Bool"}},
                },
            },
            "id": "check_active",
            "kind": "Rule",
            "provenance": {"file": "test.tenor", "line": 16},
            "stratum": 0,
            "tenor": "1.0",
        },
        {
            "allowed_personas": ["admin"],
            "effects": [{"entity_id": "Order", "from": "pending", "to": "approved"}],
            "error_contract": ["precondition_failed"],
            "id": "approve_order",
            "kind": "Operation",
            "precondition": {"verdict_present": "account_active"},
            "provenance": {"file": "test.tenor", "line": 22},
            "tenor": "1.0",
        },
        {
            "entry": "step_approve",
            "id": "approval_flow",
            "kind": "Flow",
            "provenance": {"file": "test.tenor", "line": 29},
            "snapshot": "at_initiation",
            "steps": [
                {
                    "id": "step_approve",
                    "kind": "OperationStep",
                    "on_failure": {"kind": "Terminate", "outcome": "approval_failed"},
                    "op": "approve_order",
                    "outcomes": {
                        "success": {"kind": "Terminal", "outcome": "order_approved"}
                    },
                    "persona": "admin",
                }
            ],
            "tenor": "1.0",
        },
    ],
    "id": "entity_operation_basic",
    "kind": "Bundle",
    "tenor": "1.0",
    "tenor_version": "1.0.0",
})


class TestLoadContract:
    def test_load_valid_bundle(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        assert evaluator is not None

    def test_load_invalid_json(self):
        with pytest.raises(ValueError):
            TenorEvaluator.from_bundle_json("not json")

    def test_load_invalid_bundle(self):
        with pytest.raises(ValueError):
            TenorEvaluator.from_bundle_json('{"not": "a bundle"}')

    def test_load_from_dict(self):
        bundle_dict = json.loads(BASIC_BUNDLE)
        evaluator = TenorEvaluator.from_bundle(bundle_dict)
        assert evaluator is not None


class TestEvaluate:
    def test_produces_verdicts(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.evaluate({"is_active": True})
        verdicts = result["verdicts"]
        assert len(verdicts) == 1
        assert verdicts[0]["type"] == "account_active"
        assert verdicts[0]["provenance"]["rule"] == "check_active"
        assert verdicts[0]["provenance"]["stratum"] == 0
        assert "is_active" in verdicts[0]["provenance"]["facts_used"]

    def test_no_verdict_when_false(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.evaluate({"is_active": False})
        assert len(result["verdicts"]) == 0

    def test_missing_required_fact(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        with pytest.raises(RuntimeError):
            evaluator.evaluate({})


class TestComputeActionSpace:
    def test_action_available(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.compute_action_space(
            {"is_active": True},
            {"Order": "pending"},
            "admin",
        )
        assert result["persona_id"] == "admin"
        assert len(result["actions"]) == 1
        assert result["actions"][0]["flow_id"] == "approval_flow"
        assert len(result["blocked_actions"]) == 0

    def test_blocked_persona(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.compute_action_space(
            {"is_active": True},
            {"Order": "pending"},
            "guest",
        )
        assert len(result["actions"]) == 0
        assert len(result["blocked_actions"]) == 1
        assert result["blocked_actions"][0]["reason"]["type"] == "PersonaNotAuthorized"

    def test_blocked_precondition(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.compute_action_space(
            {"is_active": False},
            {"Order": "pending"},
            "admin",
        )
        assert len(result["actions"]) == 0
        assert len(result["blocked_actions"]) == 1
        assert result["blocked_actions"][0]["reason"]["type"] == "PreconditionNotMet"

    def test_blocked_entity_state(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.compute_action_space(
            {"is_active": True},
            {"Order": "approved"},
            "admin",
        )
        assert len(result["actions"]) == 0
        assert len(result["blocked_actions"]) == 1
        assert result["blocked_actions"][0]["reason"]["type"] == "EntityNotInSourceState"

    def test_current_verdicts(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        # Empty entity states — current_verdicts does not need entity state
        result = evaluator.compute_action_space(
            {"is_active": True},
            {},
            "admin",
        )
        verdicts = result["current_verdicts"]
        assert len(verdicts) == 1
        assert verdicts[0]["verdict_type"] == "account_active"

    def test_action_includes_entry_operation_id(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.compute_action_space(
            {"is_active": True},
            {"Order": "pending"},
            "admin",
        )
        assert result["actions"][0]["entry_operation_id"] == "approve_order"


class TestExecuteFlow:
    def test_success(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        # Empty entity states — execute_flow uses contract defaults (Order starts as "pending")
        result = evaluator.execute_flow(
            "approval_flow",
            {"is_active": True},
            {},
            "admin",
        )
        assert result["outcome"] == "order_approved"
        assert len(result["path"]) > 0
        assert len(result["would_transition"]) > 0

    def test_success_explicit_state(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.execute_flow(
            "approval_flow",
            {"is_active": True},
            {"Order": "pending"},
            "admin",
        )
        assert result["outcome"] == "order_approved"
        assert result["would_transition"][0]["entity_id"] == "Order"
        assert result["would_transition"][0]["from_state"] == "pending"
        assert result["would_transition"][0]["to_state"] == "approved"

    def test_precondition_fails(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.execute_flow(
            "approval_flow",
            {"is_active": False},
            {},
            "admin",
        )
        assert result["outcome"] == "approval_failed"

    def test_flow_not_found(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        with pytest.raises(RuntimeError):
            evaluator.execute_flow(
                "nonexistent_flow",
                {"is_active": True},
                {},
                "admin",
            )

    def test_already_approved_state(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        # Entity already in "approved" state — cannot transition from "pending" to "approved" again
        result = evaluator.execute_flow(
            "approval_flow",
            {"is_active": True},
            {"Order": "approved"},
            "admin",
        )
        assert result["outcome"] == "approval_failed"


class TestCrossSDKConsistency:
    """Verify Python SDK produces identical results to Rust evaluator.

    These exact values are verified against the Rust evaluator output
    (identical to crates/tenor-eval-wasm/tests/wasm.rs assertions).
    """

    def test_verdicts_match_rust(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.evaluate({"is_active": True})
        # Identical to WASM test: test_evaluate_produces_verdicts
        assert len(result["verdicts"]) == 1
        v = result["verdicts"][0]
        assert v["type"] == "account_active"
        assert v["provenance"]["rule"] == "check_active"
        assert v["provenance"]["stratum"] == 0
        assert v["provenance"]["facts_used"] == ["is_active"]

    def test_no_verdicts_match_rust(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.evaluate({"is_active": False})
        # Identical to WASM test: test_evaluate_no_verdict_when_false
        assert len(result["verdicts"]) == 0

    def test_action_space_matches_rust(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.compute_action_space(
            {"is_active": True},
            {"Order": "pending"},
            "admin",
        )
        # Identical to WASM test: test_compute_action_space_available
        assert result["persona_id"] == "admin"
        actions = result["actions"]
        assert len(actions) == 1
        assert actions[0]["flow_id"] == "approval_flow"
        assert actions[0]["entry_operation_id"] == "approve_order"
        assert len(result["blocked_actions"]) == 0

    def test_flow_execution_matches_rust(self):
        evaluator = TenorEvaluator.from_bundle_json(BASIC_BUNDLE)
        result = evaluator.execute_flow(
            "approval_flow",
            {"is_active": True},
            {},
            "admin",
        )
        # Identical to WASM test: test_simulate_flow_success
        assert result["flow_id"] == "approval_flow"
        assert result["outcome"] == "order_approved"
        assert len(result["path"]) > 0
        assert len(result["would_transition"]) > 0
