//! Test fixtures for the executor conformance suite.
//!
//! Each fixture function returns a valid Tenor interchange JSON bundle
//! (as serde_json::Value) suitable for loading into a TestableExecutor.

use serde_json::{json, Value};

/// A minimal contract with one entity (Order: draft → submitted → approved),
/// one fact (amount: Int), one rule, one operation (submit), one persona
/// (clerk), and one flow (approval_flow).
///
/// This is the workhorse fixture used by most E1-E17 tests.
pub fn basic_contract() -> Value {
    json!({
        "id": "test.basic",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "kind": "Persona",
                "id": "clerk",
                "provenance": { "file": "fixture", "line": 1 }
            },
            {
                "kind": "Fact",
                "id": "amount",
                "type": { "base": "Int" },
                "provenance": { "file": "fixture", "line": 2 }
            },
            {
                "kind": "Entity",
                "id": "Order",
                "states": ["draft", "submitted", "approved"],
                "initial": "draft",
                "transitions": [
                    { "from": "draft", "to": "submitted" },
                    { "from": "submitted", "to": "approved" }
                ],
                "provenance": { "file": "fixture", "line": 3 }
            },
            {
                "kind": "Rule",
                "id": "amount_positive",
                "stratum": 1,
                "body": {
                    "when": { "fact_ref": "amount" },
                    "produce": {
                        "verdict_type": "Bool",
                        "payload": { "fact_ref": "amount" }
                    }
                },
                "provenance": { "file": "fixture", "line": 4 }
            },
            {
                "kind": "Operation",
                "id": "submit",
                "allowed_personas": ["clerk"],
                "precondition": null,
                "effects": [
                    { "entity_id": "Order", "from": "draft", "to": "submitted", "outcome": "submitted" }
                ],
                "outcomes": ["submitted"],
                "provenance": { "file": "fixture", "line": 5 }
            },
            {
                "kind": "Operation",
                "id": "approve",
                "allowed_personas": ["clerk"],
                "precondition": null,
                "effects": [
                    { "entity_id": "Order", "from": "submitted", "to": "approved", "outcome": "approved" }
                ],
                "outcomes": ["approved"],
                "provenance": { "file": "fixture", "line": 6 }
            },
            {
                "kind": "Flow",
                "id": "approval_flow",
                "entry": "step_submit",
                "snapshot": "at_initiation",
                "steps": [
                    {
                        "id": "step_submit",
                        "kind": "operation",
                        "operation_id": "submit",
                        "transitions": [
                            { "on": "submitted", "next": "step_approve" }
                        ]
                    },
                    {
                        "id": "step_approve",
                        "kind": "operation",
                        "operation_id": "approve",
                        "transitions": [
                            { "on": "approved", "next": null }
                        ]
                    }
                ],
                "provenance": { "file": "fixture", "line": 7 }
            }
        ]
    })
}

/// Contract with two entities (Order, Payment) for atomicity testing (E3).
pub fn multi_entity_contract() -> Value {
    json!({
        "id": "test.multi_entity",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "kind": "Persona",
                "id": "clerk",
                "provenance": { "file": "fixture", "line": 1 }
            },
            {
                "kind": "Fact",
                "id": "amount",
                "type": { "base": "Int" },
                "provenance": { "file": "fixture", "line": 2 }
            },
            {
                "kind": "Entity",
                "id": "Order",
                "states": ["draft", "submitted"],
                "initial": "draft",
                "transitions": [
                    { "from": "draft", "to": "submitted" }
                ],
                "provenance": { "file": "fixture", "line": 3 }
            },
            {
                "kind": "Entity",
                "id": "Payment",
                "states": ["pending", "processed"],
                "initial": "pending",
                "transitions": [
                    { "from": "pending", "to": "processed" }
                ],
                "provenance": { "file": "fixture", "line": 4 }
            },
            {
                "kind": "Operation",
                "id": "submit_with_payment",
                "allowed_personas": ["clerk"],
                "precondition": null,
                "effects": [
                    { "entity_id": "Order", "from": "draft", "to": "submitted", "outcome": "ok" },
                    { "entity_id": "Payment", "from": "pending", "to": "processed", "outcome": "ok" }
                ],
                "outcomes": ["ok"],
                "provenance": { "file": "fixture", "line": 5 }
            },
            {
                "kind": "Flow",
                "id": "submit_flow",
                "entry": "step_submit",
                "snapshot": "at_initiation",
                "steps": [
                    {
                        "id": "step_submit",
                        "kind": "operation",
                        "operation_id": "submit_with_payment",
                        "transitions": [
                            { "on": "ok", "next": null }
                        ]
                    }
                ],
                "provenance": { "file": "fixture", "line": 6 }
            }
        ]
    })
}

/// Contract with parallel branches for E8/E9 testing.
pub fn parallel_flow_contract() -> Value {
    json!({
        "id": "test.parallel",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "kind": "Persona",
                "id": "clerk",
                "provenance": { "file": "fixture", "line": 1 }
            },
            {
                "kind": "Fact",
                "id": "amount",
                "type": { "base": "Int" },
                "provenance": { "file": "fixture", "line": 2 }
            },
            {
                "kind": "Entity",
                "id": "Order",
                "states": ["draft", "reviewed", "approved"],
                "initial": "draft",
                "transitions": [
                    { "from": "draft", "to": "reviewed" },
                    { "from": "reviewed", "to": "approved" }
                ],
                "provenance": { "file": "fixture", "line": 3 }
            },
            {
                "kind": "Operation",
                "id": "review",
                "allowed_personas": ["clerk"],
                "precondition": null,
                "effects": [
                    { "entity_id": "Order", "from": "draft", "to": "reviewed", "outcome": "reviewed" }
                ],
                "outcomes": ["reviewed"],
                "provenance": { "file": "fixture", "line": 4 }
            },
            {
                "kind": "Operation",
                "id": "finalize",
                "allowed_personas": ["clerk"],
                "precondition": null,
                "effects": [
                    { "entity_id": "Order", "from": "reviewed", "to": "approved", "outcome": "done" }
                ],
                "outcomes": ["done"],
                "provenance": { "file": "fixture", "line": 5 }
            },
            {
                "kind": "Flow",
                "id": "parallel_flow",
                "entry": "branch_start",
                "snapshot": "at_initiation",
                "steps": [
                    {
                        "id": "branch_start",
                        "kind": "branch",
                        "branches": ["branch_a", "branch_b"],
                        "join": "join_step"
                    },
                    {
                        "id": "branch_a",
                        "kind": "operation",
                        "operation_id": "review",
                        "transitions": [
                            { "on": "reviewed", "next": null }
                        ]
                    },
                    {
                        "id": "branch_b",
                        "kind": "operation",
                        "operation_id": "review",
                        "transitions": [
                            { "on": "reviewed", "next": null }
                        ]
                    },
                    {
                        "id": "join_step",
                        "kind": "join",
                        "transitions": [
                            { "on": "completed", "next": null }
                        ]
                    }
                ],
                "provenance": { "file": "fixture", "line": 6 }
            }
        ]
    })
}

/// Contract with Decimal and Money facts for E7 numeric model testing.
pub fn numeric_contract() -> Value {
    json!({
        "id": "test.numeric",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "kind": "Persona",
                "id": "clerk",
                "provenance": { "file": "fixture", "line": 1 }
            },
            {
                "kind": "Fact",
                "id": "rate",
                "type": { "base": "Decimal", "precision": 10, "scale": 4 },
                "provenance": { "file": "fixture", "line": 2 }
            },
            {
                "kind": "Fact",
                "id": "price",
                "type": { "base": "Money", "currency": "USD" },
                "provenance": { "file": "fixture", "line": 3 }
            },
            {
                "kind": "Entity",
                "id": "Order",
                "states": ["draft", "priced"],
                "initial": "draft",
                "transitions": [
                    { "from": "draft", "to": "priced" }
                ],
                "provenance": { "file": "fixture", "line": 4 }
            },
            {
                "kind": "Operation",
                "id": "set_price",
                "allowed_personas": ["clerk"],
                "precondition": null,
                "effects": [
                    { "entity_id": "Order", "from": "draft", "to": "priced", "outcome": "priced" }
                ],
                "outcomes": ["priced"],
                "provenance": { "file": "fixture", "line": 5 }
            },
            {
                "kind": "Flow",
                "id": "price_flow",
                "entry": "step_price",
                "snapshot": "at_initiation",
                "steps": [
                    {
                        "id": "step_price",
                        "kind": "operation",
                        "operation_id": "set_price",
                        "transitions": [
                            { "on": "priced", "next": null }
                        ]
                    }
                ],
                "provenance": { "file": "fixture", "line": 6 }
            }
        ]
    })
}

/// Contract for E15-E17 multi-instance entity tests.
pub fn multi_instance_contract() -> Value {
    json!({
        "id": "test.multi_instance",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "kind": "Persona",
                "id": "clerk",
                "provenance": { "file": "fixture", "line": 1 }
            },
            {
                "kind": "Fact",
                "id": "amount",
                "type": { "base": "Int" },
                "provenance": { "file": "fixture", "line": 2 }
            },
            {
                "kind": "Entity",
                "id": "Order",
                "states": ["draft", "submitted"],
                "initial": "draft",
                "transitions": [
                    { "from": "draft", "to": "submitted" }
                ],
                "provenance": { "file": "fixture", "line": 3 }
            },
            {
                "kind": "Operation",
                "id": "submit",
                "allowed_personas": ["clerk"],
                "precondition": null,
                "effects": [
                    { "entity_id": "Order", "from": "draft", "to": "submitted", "outcome": "submitted" }
                ],
                "outcomes": ["submitted"],
                "provenance": { "file": "fixture", "line": 4 }
            },
            {
                "kind": "Flow",
                "id": "instance_flow",
                "entry": "step_submit",
                "snapshot": "at_initiation",
                "steps": [
                    {
                        "id": "step_submit",
                        "kind": "operation",
                        "operation_id": "submit",
                        "transitions": [
                            { "on": "submitted", "next": null }
                        ]
                    }
                ],
                "provenance": { "file": "fixture", "line": 5 }
            }
        ]
    })
}

/// Contract for E18-E20 trust tests.
pub fn trust_contract() -> Value {
    json!({
        "id": "test.trust",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "kind": "Persona",
                "id": "clerk",
                "provenance": { "file": "fixture", "line": 1 }
            },
            {
                "kind": "Fact",
                "id": "amount",
                "type": { "base": "Int" },
                "provenance": { "file": "fixture", "line": 2 }
            },
            {
                "kind": "Entity",
                "id": "Order",
                "states": ["draft", "submitted"],
                "initial": "draft",
                "transitions": [
                    { "from": "draft", "to": "submitted" }
                ],
                "provenance": { "file": "fixture", "line": 3 }
            },
            {
                "kind": "Operation",
                "id": "submit",
                "allowed_personas": ["clerk"],
                "precondition": null,
                "effects": [
                    { "entity_id": "Order", "from": "draft", "to": "submitted", "outcome": "submitted" }
                ],
                "outcomes": ["submitted"],
                "provenance": { "file": "fixture", "line": 4 }
            },
            {
                "kind": "Flow",
                "id": "trust_flow",
                "entry": "step_submit",
                "snapshot": "at_initiation",
                "steps": [
                    {
                        "id": "step_submit",
                        "kind": "operation",
                        "operation_id": "submit",
                        "transitions": [
                            { "on": "submitted", "next": null }
                        ]
                    }
                ],
                "provenance": { "file": "fixture", "line": 5 }
            }
        ]
    })
}

/// Standard fact set for the basic contract.
pub fn basic_facts() -> Value {
    json!({
        "amount": 100
    })
}

/// Fact set with a DateTime value (for E6 testing).
pub fn datetime_facts() -> Value {
    json!({
        "amount": 42,
        "created_at": "2024-01-15T10:30:00+05:30"
    })
}

/// Initial entity states (Order in draft state).
pub fn initial_entity_states() -> Value {
    json!({
        "Order": "draft"
    })
}

/// Entity states for the multi-entity contract (Order and Payment both initial).
pub fn multi_entity_initial_states() -> Value {
    json!({
        "Order": "draft",
        "Payment": "pending"
    })
}

/// Entity states for the Order after submission (for E2 testing — wrong state).
pub fn order_submitted_states() -> Value {
    json!({
        "Order": "submitted"
    })
}

/// Numeric fact set with Decimal and Money values.
pub fn numeric_facts() -> Value {
    json!({
        "rate": "0.1250",
        "price": "99.99"
    })
}
