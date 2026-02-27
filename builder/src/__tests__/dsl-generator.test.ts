/**
 * DSL generator correctness tests.
 *
 * Verifies that generateDsl() produces syntactically correct .tenor source
 * for all construct types and expression forms.
 */
import { describe, it, expect } from "vitest";
import { generateDsl } from "../utils/dsl-generator";
import type {
  InterchangeBundle,
  FactConstruct,
  EntityConstruct,
  RuleConstruct,
  OperationConstruct,
  FlowConstruct,
  PersonaConstruct,
} from "../types/interchange";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TV = "1.0";
const BV = "1.0.0";

function bundle(constructs: InterchangeBundle["constructs"]): InterchangeBundle {
  return { constructs, id: "test", kind: "Bundle", tenor: TV, tenor_version: BV };
}

function prov() {
  return { file: "test.tenor", line: 1 };
}

// ---------------------------------------------------------------------------
// Fact declarations
// ---------------------------------------------------------------------------

describe("DSL generator — fact declarations", () => {
  it("generates Bool fact with default", () => {
    const fact: FactConstruct = {
      id: "my_flag",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Bool" },
      default: false,
    };
    const dsl = generateDsl(bundle([fact]));
    expect(dsl).toContain("fact my_flag");
    expect(dsl).toContain("type:");
    expect(dsl).toContain("Bool");
    expect(dsl).toContain("default:");
    expect(dsl).toContain("false");
  });

  it("generates Int fact without default", () => {
    const fact: FactConstruct = {
      id: "count",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Int" },
    };
    const dsl = generateDsl(bundle([fact]));
    expect(dsl).toContain("fact count");
    expect(dsl).toContain("Int");
    expect(dsl).not.toContain("default:");
  });

  it("generates Int fact with min/max constraints", () => {
    const fact: FactConstruct = {
      id: "age",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Int", min: 0, max: 150 },
    };
    const dsl = generateDsl(bundle([fact]));
    expect(dsl).toContain("Int(min: 0, max: 150)");
  });

  it("generates Money fact with default", () => {
    const fact: FactConstruct = {
      id: "price",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Money", currency: "USD" },
      default: {
        kind: "money_value",
        amount: { kind: "decimal_value", precision: 10, scale: 2, value: "100.00" },
        currency: "USD",
      },
    };
    const dsl = generateDsl(bundle([fact]));
    expect(dsl).toContain("fact price");
    expect(dsl).toContain('Money(currency: "USD")');
    expect(dsl).toContain("default:");
    expect(dsl).toContain("100.00");
  });

  it("generates Enum fact with values list", () => {
    const fact: FactConstruct = {
      id: "status",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Enum", values: ["pending", "active", "closed"] },
      default: "pending",
    };
    const dsl = generateDsl(bundle([fact]));
    expect(dsl).toContain("fact status");
    expect(dsl).toContain('Enum(values: ["pending", "active", "closed"])');
  });

  it("generates List fact with element_type and max", () => {
    const fact: FactConstruct = {
      id: "tags",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "List", element_type: { base: "Text" }, max: 5 },
    };
    const dsl = generateDsl(bundle([fact]));
    expect(dsl).toContain("fact tags");
    expect(dsl).toContain("List(element_type: Text, max: 5)");
  });

  it("generates Record fact with field definitions", () => {
    const fact: FactConstruct = {
      id: "address",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: {
        base: "Record",
        fields: {
          street: { base: "Text" },
          zip: { base: "Int" },
        },
      },
    };
    const dsl = generateDsl(bundle([fact]));
    expect(dsl).toContain("fact address");
    expect(dsl).toContain("street: Text");
    expect(dsl).toContain("zip: Int");
  });

  it("generates Decimal fact", () => {
    const fact: FactConstruct = {
      id: "rate",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Decimal", precision: 10, scale: 4 },
    };
    const dsl = generateDsl(bundle([fact]));
    expect(dsl).toContain("Decimal(precision: 10, scale: 4)");
  });
});

// ---------------------------------------------------------------------------
// Entity declarations
// ---------------------------------------------------------------------------

describe("DSL generator — entity declarations", () => {
  it("generates entity with states, initial, and transitions", () => {
    const entity: EntityConstruct = {
      id: "order",
      kind: "Entity",
      tenor: TV,
      provenance: prov(),
      states: ["pending", "confirmed", "shipped"],
      initial: "pending",
      transitions: [
        { from: "pending", to: "confirmed" },
        { from: "confirmed", to: "shipped" },
      ],
    };
    const dsl = generateDsl(bundle([entity]));
    expect(dsl).toContain("entity order");
    expect(dsl).toContain("states:");
    expect(dsl).toContain("pending");
    expect(dsl).toContain("confirmed");
    expect(dsl).toContain("shipped");
    expect(dsl).toContain("initial: pending");
    expect(dsl).toContain("transitions:");
    expect(dsl).toContain("(pending, confirmed)");
    expect(dsl).toContain("(confirmed, shipped)");
  });

  it("generates entity with no transitions", () => {
    const entity: EntityConstruct = {
      id: "simple",
      kind: "Entity",
      tenor: TV,
      provenance: prov(),
      states: ["open"],
      initial: "open",
      transitions: [],
    };
    const dsl = generateDsl(bundle([entity]));
    expect(dsl).toContain("entity simple");
    expect(dsl).toContain("initial: open");
  });
});

// ---------------------------------------------------------------------------
// Rule declarations
// ---------------------------------------------------------------------------

describe("DSL generator — rule declarations", () => {
  it("generates rule with stratum and simple comparison predicate", () => {
    const rule: RuleConstruct = {
      id: "approve",
      kind: "Rule",
      tenor: TV,
      provenance: prov(),
      stratum: 0,
      body: {
        when: {
          left: { fact_ref: "amount" },
          op: ">",
          right: { literal: 0, type: { base: "Int" } },
        },
        produce: {
          verdict_type: "Approved",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const dsl = generateDsl(bundle([rule]));
    expect(dsl).toContain("rule approve");
    expect(dsl).toContain("stratum: 0");
    expect(dsl).toContain("when:");
    expect(dsl).toContain("produce:");
    expect(dsl).toContain("verdict Approved");
  });

  it("generates rule with stratum 1 and verdict_present predicate", () => {
    const rule: RuleConstruct = {
      id: "final_check",
      kind: "Rule",
      tenor: TV,
      provenance: prov(),
      stratum: 1,
      body: {
        when: { verdict_present: "Approved" },
        produce: {
          verdict_type: "Finalized",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const dsl = generateDsl(bundle([rule]));
    expect(dsl).toContain("stratum: 1");
    expect(dsl).toContain("verdict_present(Approved)");
    expect(dsl).toContain("verdict Finalized");
  });
});

// ---------------------------------------------------------------------------
// Operation declarations
// ---------------------------------------------------------------------------

describe("DSL generator — operation declarations", () => {
  it("generates operation with allowed_personas, precondition, effects, error_contract", () => {
    const op: OperationConstruct = {
      id: "release_funds",
      kind: "Operation",
      tenor: TV,
      provenance: prov(),
      allowed_personas: ["arbiter"],
      precondition: {
        left: { fact_ref: "is_approved" },
        op: "=",
        right: { literal: true, type: { base: "Bool" } },
      },
      effects: [{ entity_id: "escrow", from: "held", to: "released" }],
      error_contract: ["InsufficientFunds"],
    };
    const dsl = generateDsl(bundle([op]));
    expect(dsl).toContain("operation release_funds");
    expect(dsl).toContain("allowed_personas: [arbiter]");
    expect(dsl).toContain("precondition:");
    expect(dsl).toContain("effects:");
    expect(dsl).toContain("(escrow, held, released)");
    expect(dsl).toContain("error_contract:");
    expect(dsl).toContain("InsufficientFunds");
  });

  it("generates operation with empty effects", () => {
    const op: OperationConstruct = {
      id: "check_status",
      kind: "Operation",
      tenor: TV,
      provenance: prov(),
      allowed_personas: ["auditor"],
      precondition: { fact_ref: "is_active" } as unknown as OperationConstruct["precondition"],
      effects: [],
      error_contract: [],
    };
    const dsl = generateDsl(bundle([op]));
    expect(dsl).toContain("effects:          []");
  });
});

// ---------------------------------------------------------------------------
// Flow declarations
// ---------------------------------------------------------------------------

describe("DSL generator — flow declarations", () => {
  it("generates flow with OperationStep", () => {
    const flow: FlowConstruct = {
      id: "escrow_flow",
      kind: "Flow",
      tenor: TV,
      provenance: prov(),
      entry: "step_deposit",
      steps: [
        {
          id: "step_deposit",
          kind: "OperationStep",
          op: "deposit",
          persona: "buyer",
          outcomes: {
            success: { kind: "Terminal", outcome: "completed" },
          },
          on_failure: { kind: "Terminate", outcome: "failed" },
        },
      ],
    };
    const dsl = generateDsl(bundle([flow]));
    expect(dsl).toContain("flow escrow_flow");
    expect(dsl).toContain("entry:");
    expect(dsl).toContain("step_deposit");
    expect(dsl).toContain("OperationStep");
    expect(dsl).toContain("op:");
    expect(dsl).toContain("deposit");
    expect(dsl).toContain("on_failure:");
    expect(dsl).toContain("Terminate");
  });

  it("generates flow with BranchStep", () => {
    const flow: FlowConstruct = {
      id: "branch_flow",
      kind: "Flow",
      tenor: TV,
      provenance: prov(),
      entry: "check",
      steps: [
        {
          id: "check",
          kind: "BranchStep",
          persona: "system",
          condition: { fact_ref: "is_active" } as unknown as FlowConstruct["steps"][0]["condition" & keyof FlowConstruct["steps"][0]],
          if_true: "step_next",
          if_false: { kind: "Terminal", outcome: "rejected" },
        } as unknown as FlowConstruct["steps"][0],
      ],
    };
    const dsl = generateDsl(bundle([flow]));
    expect(dsl).toContain("BranchStep");
    expect(dsl).toContain("if_true:");
    expect(dsl).toContain("if_false:");
    expect(dsl).toContain("Terminal(rejected)");
  });

  it("generates flow with HandoffStep", () => {
    const flow: FlowConstruct = {
      id: "handoff_flow",
      kind: "Flow",
      tenor: TV,
      provenance: prov(),
      entry: "hand",
      steps: [
        {
          id: "hand",
          kind: "HandoffStep",
          from_persona: "buyer",
          to_persona: "seller",
          next: "confirm_step",
        },
      ],
    };
    const dsl = generateDsl(bundle([flow]));
    expect(dsl).toContain("HandoffStep");
    expect(dsl).toContain("from_persona: buyer");
    expect(dsl).toContain("to_persona:   seller");
    expect(dsl).toContain("next:         confirm_step");
  });
});

// ---------------------------------------------------------------------------
// Predicate expression serialization
// ---------------------------------------------------------------------------

describe("DSL generator — predicate expression serialization", () => {
  it("serializes AND with unicode conjunction symbol", () => {
    const rule: RuleConstruct = {
      id: "and_rule",
      kind: "Rule",
      tenor: TV,
      provenance: prov(),
      stratum: 0,
      body: {
        when: {
          op: "and",
          left: { fact_ref: "a" },
          right: { fact_ref: "b" },
        },
        produce: {
          verdict_type: "Test",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const dsl = generateDsl(bundle([rule]));
    expect(dsl).toContain("∧");
  });

  it("serializes OR with unicode disjunction symbol", () => {
    const rule: RuleConstruct = {
      id: "or_rule",
      kind: "Rule",
      tenor: TV,
      provenance: prov(),
      stratum: 0,
      body: {
        when: {
          op: "or",
          left: { fact_ref: "a" },
          right: { fact_ref: "b" },
        },
        produce: {
          verdict_type: "Test",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const dsl = generateDsl(bundle([rule]));
    expect(dsl).toContain("∨");
  });

  it("serializes NOT with unicode negation symbol", () => {
    const rule: RuleConstruct = {
      id: "not_rule",
      kind: "Rule",
      tenor: TV,
      provenance: prov(),
      stratum: 0,
      body: {
        when: {
          op: "not",
          operand: { fact_ref: "a" },
        },
        produce: {
          verdict_type: "Test",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const dsl = generateDsl(bundle([rule]));
    expect(dsl).toContain("¬");
  });

  it("serializes ForAll quantifier with unicode universal quantifier", () => {
    const rule: RuleConstruct = {
      id: "forall_rule",
      kind: "Rule",
      tenor: TV,
      provenance: prov(),
      stratum: 0,
      body: {
        when: {
          quantifier: "forall",
          variable: "item",
          variable_type: { base: "Int" },
          domain: { fact_ref: "items" },
          body: {
            left: { fact_ref: "item" },
            op: ">",
            right: { literal: 0, type: { base: "Int" } },
          },
        },
        produce: {
          verdict_type: "AllPositive",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const dsl = generateDsl(bundle([rule]));
    expect(dsl).toContain("∀");
    expect(dsl).toContain("item");
    expect(dsl).toContain("items");
  });

  it("serializes Exists quantifier with unicode existential quantifier", () => {
    const rule: RuleConstruct = {
      id: "exists_rule",
      kind: "Rule",
      tenor: TV,
      provenance: prov(),
      stratum: 0,
      body: {
        when: {
          quantifier: "exists",
          variable: "x",
          variable_type: { base: "Int" },
          domain: { fact_ref: "xs" },
          body: {
            left: { fact_ref: "x" },
            op: ">",
            right: { literal: 10, type: { base: "Int" } },
          },
        },
        produce: {
          verdict_type: "AnyBig",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const dsl = generateDsl(bundle([rule]));
    expect(dsl).toContain("∃");
  });
});

// ---------------------------------------------------------------------------
// Complete contract generation
// ---------------------------------------------------------------------------

describe("DSL generator — complete contract", () => {
  it("uses lowercase keywords throughout", () => {
    const persona: PersonaConstruct = {
      id: "buyer",
      kind: "Persona",
      tenor: TV,
      provenance: prov(),
    };
    const fact: FactConstruct = {
      id: "amount",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Money", currency: "USD" },
    };

    const dsl = generateDsl(bundle([persona, fact]));

    // Lowercase keywords in generated DSL
    expect(dsl).toContain("persona buyer");
    expect(dsl).toContain("fact amount");

    // Keyword strings should be lowercase (not "Fact amount" or "Persona buyer")
    expect(dsl).not.toMatch(/^Fact /m);
    expect(dsl).not.toMatch(/^Entity /m);
    expect(dsl).not.toMatch(/^Rule /m);
    expect(dsl).not.toMatch(/^Operation /m);
    expect(dsl).not.toMatch(/^Flow /m);
    expect(dsl).not.toMatch(/^Persona /m);
  });

  it("places Personas section before Facts section", () => {
    const persona: PersonaConstruct = {
      id: "buyer",
      kind: "Persona",
      tenor: TV,
      provenance: prov(),
    };
    const fact: FactConstruct = {
      id: "escrow_amount",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Money", currency: "USD" },
    };

    const dsl = generateDsl(bundle([fact, persona])); // Facts before Personas in input
    const personaIdx = dsl.indexOf("persona buyer");
    const factIdx = dsl.indexOf("fact escrow_amount");
    expect(personaIdx).toBeGreaterThan(-1);
    expect(factIdx).toBeGreaterThan(-1);
    expect(personaIdx).toBeLessThan(factIdx);
  });

  it("groups rules by stratum in ascending order", () => {
    const rules: RuleConstruct[] = [
      {
        id: "rule_s1",
        kind: "Rule",
        tenor: TV,
        provenance: prov(),
        stratum: 1,
        body: {
          when: { verdict_present: "BaseApproved" },
          produce: { verdict_type: "FinalApproved", payload: { type: { base: "Bool" }, value: true } },
        },
      },
      {
        id: "rule_s0",
        kind: "Rule",
        tenor: TV,
        provenance: prov(),
        stratum: 0,
        body: {
          when: { fact_ref: "is_valid" } as unknown as RuleConstruct["body"]["when"],
          produce: { verdict_type: "BaseApproved", payload: { type: { base: "Bool" }, value: true } },
        },
      },
    ];

    const dsl = generateDsl(bundle(rules));
    const s0Idx = dsl.indexOf("rule_s0");
    const s1Idx = dsl.indexOf("rule_s1");
    expect(s0Idx).toBeGreaterThan(-1);
    expect(s1Idx).toBeGreaterThan(-1);
    expect(s0Idx).toBeLessThan(s1Idx);
  });
});
