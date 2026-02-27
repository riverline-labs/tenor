# Tenor Python SDK

Python bindings for the Tenor contract evaluator, built as a native extension module via [PyO3](https://pyo3.rs/) and [maturin](https://maturin.rs/). No WASM, no subprocess — the Rust evaluator compiles directly into a Python extension for native performance.

## Installation

```bash
pip install tenor
```

## Quick Start

```python
import json
from tenor import TenorEvaluator

# Load a contract from interchange JSON
with open("my_contract.json") as f:
    bundle_json = f.read()

evaluator = TenorEvaluator.from_bundle_json(bundle_json)

# Evaluate rules against facts
result = evaluator.evaluate({"is_active": True, "balance": {"amount": "500.00", "currency": "USD"}})
for verdict in result["verdicts"]:
    print(f"Verdict: {verdict['type']} (rule: {verdict['provenance']['rule']})")

# Compute the action space for a persona
action_space = evaluator.compute_action_space(
    facts={"is_active": True},
    entity_states={"Order": "pending"},  # flat single-instance format
    persona="admin",
)
print(f"Available actions for admin: {[a['flow_id'] for a in action_space['actions']]}")
print(f"Blocked actions: {[b['flow_id'] for b in action_space['blocked_actions']]}")

# Execute (simulate) a flow
flow_result = evaluator.execute_flow(
    flow_id="approval_flow",
    facts={"is_active": True},
    entity_states={},  # empty = use contract initial states
    persona="admin",
)
print(f"Flow outcome: {flow_result['outcome']}")
print(f"Would transition: {flow_result['would_transition']}")
```

## API Reference

### `TenorEvaluator`

The main class. Holds a parsed contract for repeated evaluation.

#### `TenorEvaluator.from_bundle_json(json: str) -> TenorEvaluator`

Load a contract from an interchange JSON string.

Raises `ValueError` if the JSON is invalid or the bundle is not a valid Tenor contract.

#### `TenorEvaluator.from_bundle(bundle: dict) -> TenorEvaluator`

Load a contract from a Python dict (interchange bundle).

Raises `ValueError` if the bundle is not a valid Tenor contract.

#### `evaluator.evaluate(facts: dict) -> dict`

Evaluate rules against the provided facts.

- `facts`: dict mapping fact IDs to values. Missing facts with defaults will use the declared default.
- Returns a `VerdictSet` dict with shape: `{"verdicts": [...]}`
- Each verdict has `type`, `payload`, and `provenance` (with `rule`, `stratum`, `facts_used`, `verdicts_used`).
- Raises `RuntimeError` if a required fact is missing or evaluation fails.

#### `evaluator.compute_action_space(facts: dict, entity_states: dict, persona: str) -> dict`

Compute the set of actions available to a persona given the current state of the world.

- `facts`: dict of fact values
- `entity_states`: dict of entity states. Two formats supported:
  - Flat single-instance: `{"Order": "pending"}`
  - Multi-instance: `{"Order": {"ord-001": "pending", "ord-002": "approved"}}`
- `persona`: persona ID string
- Returns an `ActionSpace` dict with `persona_id`, `actions`, `blocked_actions`, `current_verdicts`.
- Raises `RuntimeError` on evaluation failure.

#### `evaluator.execute_flow(flow_id: str, facts: dict, entity_states: dict, persona: str) -> dict`

Execute (simulate) a named flow against the provided facts and entity states.

- `flow_id`: ID of the flow to execute
- `facts`: dict of fact values
- `entity_states`: dict of entity states (flat or multi-instance format).
  An empty dict `{}` uses contract-declared initial states for all entities.
- `persona`: persona ID recorded for provenance
- Returns a `FlowResult` dict with `flow_id`, `persona`, `outcome`, `path`, `would_transition`, `verdicts`.
- Raises `RuntimeError` if the flow is not found or execution fails.

## Type Stubs

The package includes `tenor/types.py` with `TypedDict` definitions for IDE autocomplete and mypy support:

```python
from tenor.types import ActionSpace, FlowResult, VerdictSet, FactSet, EntityStateMap
```

The package ships with a `py.typed` marker (PEP 561) to indicate full typing support.

## Build from Source

Requires [Rust](https://rustup.rs/) and [maturin](https://maturin.rs/).

```bash
# Clone the tenor repository
git clone https://github.com/riverline/tenor.git
cd tenor/sdks/python

# Create a virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install maturin and build
pip install maturin pytest
maturin develop

# Run tests
pytest tests/ -v
```

## Notes

- The SDK uses the stable ABI (`abi3-py39`) for broad Python version compatibility (Python 3.9+).
- The Python extension wraps the same Rust evaluator used by the WASM SDK and the platform executor — results are identical across all SDK surfaces.
- Multi-instance entity states are supported via the nested `{"entity_id": {"instance_id": "state"}}` format.
