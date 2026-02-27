"""Python SDK conformance runner.

Compares Python SDK output against expected fixtures generated from the Rust evaluator.
"""
import json
import os
import sys
from pathlib import Path

# Add the Python SDK to path so tenor can be imported without installation.
# This uses the PYTHONPATH set by run-python.sh or falls back to the .venv.
SDK_DIR = Path(__file__).parent.parent.parent / "python"
if str(SDK_DIR / "python") not in sys.path:
    sys.path.insert(0, str(SDK_DIR / "python"))

from tenor import TenorEvaluator  # noqa: E402


def main() -> None:
    fixtures_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("fixtures")

    bundle = (fixtures_dir / "escrow-bundle.json").read_text()
    facts = json.loads((fixtures_dir / "escrow-facts.json").read_text())
    entity_states = json.loads((fixtures_dir / "escrow-entity-states.json").read_text())
    facts_inactive = json.loads((fixtures_dir / "escrow-facts-inactive.json").read_text())

    evaluator = TenorEvaluator.from_bundle_json(bundle)

    passed = 0
    failed = 0

    # Test 1: evaluate (active)
    verdicts = evaluator.evaluate(facts)
    expected = json.loads((fixtures_dir / "expected-verdicts.json").read_text())
    if json_equal(verdicts, expected):
        print("PASS: evaluate (active)")
        passed += 1
    else:
        print("FAIL: evaluate (active)")
        print(f"  expected: {json.dumps(sort_keys_recursive(expected), sort_keys=True)}")
        print(f"  actual:   {json.dumps(sort_keys_recursive(verdicts), sort_keys=True)}")
        failed += 1

    # Test 2: evaluate (inactive)
    verdicts_inactive = evaluator.evaluate(facts_inactive)
    expected_inactive = json.loads((fixtures_dir / "expected-verdicts-inactive.json").read_text())
    if json_equal(verdicts_inactive, expected_inactive):
        print("PASS: evaluate (inactive)")
        passed += 1
    else:
        print("FAIL: evaluate (inactive)")
        print(f"  expected: {json.dumps(sort_keys_recursive(expected_inactive), sort_keys=True)}")
        print(f"  actual:   {json.dumps(sort_keys_recursive(verdicts_inactive), sort_keys=True)}")
        failed += 1

    # Test 3: compute_action_space (active, admin)
    action_space = evaluator.compute_action_space(facts, entity_states, "admin")
    expected_as = json.loads((fixtures_dir / "expected-action-space.json").read_text())
    if json_equal(action_space, expected_as):
        print("PASS: compute_action_space")
        passed += 1
    else:
        print("FAIL: compute_action_space")
        print(f"  expected: {json.dumps(sort_keys_recursive(expected_as), sort_keys=True)}")
        print(f"  actual:   {json.dumps(sort_keys_recursive(action_space), sort_keys=True)}")
        failed += 1

    # Test 4: compute_action_space (blocked — admin + inactive)
    action_space_blocked = evaluator.compute_action_space(facts_inactive, entity_states, "admin")
    expected_blocked = json.loads((fixtures_dir / "expected-action-space-blocked.json").read_text())
    if json_equal(action_space_blocked, expected_blocked):
        print("PASS: compute_action_space (blocked)")
        passed += 1
    else:
        print("FAIL: compute_action_space (blocked)")
        print(f"  expected: {json.dumps(sort_keys_recursive(expected_blocked), sort_keys=True)}")
        print(f"  actual:   {json.dumps(sort_keys_recursive(action_space_blocked), sort_keys=True)}")
        failed += 1

    # Test 5: execute_flow
    flow_result = evaluator.execute_flow("approval_flow", facts, entity_states, "admin")
    expected_flow = json.loads((fixtures_dir / "expected-flow-result.json").read_text())
    if json_equal(flow_result, expected_flow):
        print("PASS: execute_flow")
        passed += 1
    else:
        print("FAIL: execute_flow")
        print(f"  expected: {json.dumps(sort_keys_recursive(expected_flow), sort_keys=True)}")
        print(f"  actual:   {json.dumps(sort_keys_recursive(flow_result), sort_keys=True)}")
        failed += 1

    print(f"\nPython SDK: {passed} passed, {failed} failed")
    sys.exit(1 if failed > 0 else 0)


def json_equal(a: object, b: object) -> bool:
    return (
        json.dumps(sort_keys_recursive(a), sort_keys=True)
        == json.dumps(sort_keys_recursive(b), sort_keys=True)
    )


def sort_keys_recursive(obj: object) -> object:
    if isinstance(obj, dict):
        return {k: sort_keys_recursive(v) for k, v in sorted(obj.items())}
    if isinstance(obj, list):
        return [sort_keys_recursive(item) for item in obj]
    return obj


if __name__ == "__main__":
    main()
