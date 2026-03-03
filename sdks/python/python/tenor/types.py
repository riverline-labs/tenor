"""Type definitions for the Tenor SDK.

These types document the shapes of dicts returned by the evaluator.
They are for documentation and IDE support, not runtime enforcement.
"""

from typing import Any, TypedDict, Union


class VerdictProvenance(TypedDict):
    rule: str
    stratum: int
    facts_used: list[str]
    verdicts_used: list[str]


class Verdict(TypedDict):
    type: str
    payload: Any
    provenance: VerdictProvenance


class VerdictSet(TypedDict):
    verdicts: list[Verdict]


class VerdictSummary(TypedDict):
    verdict_type: str
    payload: Any
    producing_rule: str
    stratum: int


class EntitySummary(TypedDict):
    entity_id: str
    current_state: str
    possible_transitions: list[str]


class Action(TypedDict):
    flow_id: str
    persona_id: str
    entry_operation_id: str
    enabling_verdicts: list[VerdictSummary]
    affected_entities: list[EntitySummary]
    description: str


class BlockedAction(TypedDict):
    flow_id: str
    reason: dict[str, Any]


class ActionSpace(TypedDict):
    persona_id: str
    actions: list[Action]
    current_verdicts: list[VerdictSummary]
    blocked_actions: list[BlockedAction]


class StepResult(TypedDict):
    step_id: str
    step_type: str
    result: str


class EntityStateChange(TypedDict):
    entity_id: str
    from_state: str
    to_state: str


class FlowResult(TypedDict):
    flow_id: str
    persona: str
    outcome: str
    path: list[StepResult]
    would_transition: list[EntityStateChange]
    verdicts: list[Verdict]


# Type aliases for common input types
FactSet = dict[str, Any]
EntityStateMap = dict[str, str]
MoneyValue = dict[str, str]  # {"amount": "...", "currency": "..."}
