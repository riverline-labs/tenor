# Pass 5 Negative Tests

One test per Pass 5 validation bullet from §12.2.

## Multi-file tests

`flow_reference_cycle_a.tenor` and `flow_reference_cycle_b.tenor` are a bundle.
Elaborate with `flow_reference_cycle_a.tenor` as root.
Expected error in `flow_reference_cycle_a.expected-error.json`.

## Test index

| File | Construct | Bullet violated |
|------|-----------|-----------------|
| entity_initial_not_in_states | Entity | initial ∈ states |
| entity_transition_unknown_endpoint | Entity | transition endpoints ∈ states |
| entity_hierarchy_cycle | Entity | hierarchy acyclic |
| operation_empty_personas | Operation | allowed_personas non-empty |
| operation_effect_unknown_entity | Operation | effect entity_ids resolve |
| operation_effect_unknown_transition | Operation | effects ⊆ entity.transitions |
| rule_negative_stratum | Rule | stratum ≥ 0 |
| rule_forward_stratum_ref | Rule | verdict_refs reference strata < this rule's stratum |
| flow_missing_entry | Flow | entry exists |
| flow_unresolved_step_ref | Flow | all step refs resolve |
| flow_step_cycle | Flow | step graph acyclic |
| flow_reference_cycle_a | Flow | flow reference graph acyclic |
| flow_missing_failure_handler | Flow | all OperationSteps declare FailureHandlers |

## Not covered here

Parallel entity conflict tests are in tenor-conformance/parallel/.
Pass 5 Parallel bullet: "no overlapping entity effect sets across branches."
