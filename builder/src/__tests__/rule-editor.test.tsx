/**
 * Rule editor tests.
 *
 * Tests rule management: stratum ordering, predicate validity, validation.
 */
import { describe, it, expect } from "vitest";
import type {
  RuleConstruct,
  PredicateExpression,
  CompareExpr,
  AndExpr,
  OrExpr,
  VerdictPresentExpr,
} from "../types/interchange";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TV = "1.0";

function newRule(id: string, stratum: number): RuleConstruct {
  return {
    id,
    kind: "Rule",
    provenance: { file: "builder", line: 0 },
    stratum,
    tenor: TV,
    body: {
      when: {
        left: { fact_ref: "fact" },
        op: "=",
        right: { literal: true, type: { base: "Bool" } },
      } as PredicateExpression,
      produce: {
        verdict_type: `${id}_verdict`,
        payload: {
          type: { base: "Bool" },
          value: true,
        },
      },
    },
  };
}

/**
 * Validate a rule for cross-stratum references.
 * Returns errors if a verdict_present references same or higher stratum.
 */
function validateRuleVerdictPresent(
  rule: RuleConstruct,
  allRules: RuleConstruct[]
): string[] {
  const errors: string[] = [];

  function checkExpr(expr: PredicateExpression): void {
    if ("verdict_present" in expr) {
      const vp = expr as VerdictPresentExpr;
      // Find which rule produces this verdict
      const producingRule = allRules.find(
        (r) => r.body.produce.verdict_type === vp.verdict_present
      );
      if (producingRule && producingRule.stratum >= rule.stratum) {
        errors.push(
          `verdict_present(${vp.verdict_present}) references stratum ${producingRule.stratum} from stratum ${rule.stratum} — must reference lower stratum only.`
        );
      }
    }
    if ("op" in expr) {
      const op = (expr as { op: string }).op;
      if (op === "and" || op === "or") {
        const binary = expr as AndExpr | OrExpr;
        checkExpr(binary.left as PredicateExpression);
        checkExpr(binary.right as PredicateExpression);
      }
    }
  }

  checkExpr(rule.body.when);
  return errors;
}

// ---------------------------------------------------------------------------
// Rule stratum tests
// ---------------------------------------------------------------------------

describe("Rule editor — stratum ordering", () => {
  it("adds rule at stratum 0 with correct fields", () => {
    const rule = newRule("approve", 0);
    expect(rule.id).toBe("approve");
    expect(rule.stratum).toBe(0);
    expect(rule.body.produce.verdict_type).toBe("approve_verdict");
    expect(rule.body.produce.payload.type.base).toBe("Bool");
    expect(rule.body.produce.payload.value).toBe(true);
  });

  it("adds rule at stratum 1 with verdict_present condition", () => {
    const rule: RuleConstruct = {
      ...newRule("final", 1),
      body: {
        when: { verdict_present: "approve_verdict" } as PredicateExpression,
        produce: {
          verdict_type: "final_verdict",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    expect(rule.stratum).toBe(1);
    const when = rule.body.when as VerdictPresentExpr;
    expect("verdict_present" in when).toBe(true);
    expect(when.verdict_present).toBe("approve_verdict");
  });

  it("sorts rules by stratum in ascending order", () => {
    const rules = [
      newRule("rule_s2", 2),
      newRule("rule_s0", 0),
      newRule("rule_s1", 1),
    ];
    const sorted = [...rules].sort((a, b) => a.stratum - b.stratum);
    expect(sorted[0].id).toBe("rule_s0");
    expect(sorted[1].id).toBe("rule_s1");
    expect(sorted[2].id).toBe("rule_s2");
  });

  it("multiple rules can share the same stratum", () => {
    const rules = [
      newRule("rule_a", 0),
      newRule("rule_b", 0),
      newRule("rule_c", 0),
    ];
    const strata = rules.map((r) => r.stratum);
    expect(new Set(strata).size).toBe(1);
    expect(strata[0]).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Predicate expression validity tests
// ---------------------------------------------------------------------------

describe("Rule editor — predicate expressions", () => {
  it("simple comparison predicate — fact_ref = literal", () => {
    const rule: RuleConstruct = {
      ...newRule("check_amount", 0),
      body: {
        when: {
          left: { fact_ref: "escrow_amount" },
          op: ">=",
          right: { literal: 100, type: { base: "Int" } },
        } as CompareExpr,
        produce: {
          verdict_type: "AmountOk",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const pred = rule.body.when as CompareExpr;
    expect(pred.op).toBe(">=");
    expect((pred.left as { fact_ref: string }).fact_ref).toBe("escrow_amount");
    expect((pred.right as { literal: number }).literal).toBe(100);
  });

  it("AND predicate composed of two comparisons", () => {
    const andExpr: AndExpr = {
      op: "and",
      left: {
        left: { fact_ref: "a" },
        op: ">",
        right: { literal: 0, type: { base: "Int" } },
      } as CompareExpr,
      right: {
        left: { fact_ref: "b" },
        op: "=",
        right: { literal: true, type: { base: "Bool" } },
      } as CompareExpr,
    };
    const rule: RuleConstruct = {
      ...newRule("and_rule", 0),
      body: {
        when: andExpr,
        produce: {
          verdict_type: "BothMet",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const pred = rule.body.when as AndExpr;
    expect(pred.op).toBe("and");
    expect("left" in pred).toBe(true);
    expect("right" in pred).toBe(true);
  });

  it("OR predicate produces correct interchange structure", () => {
    const orExpr: OrExpr = {
      op: "or",
      left: { fact_ref: "flag_a" } as unknown as PredicateExpression,
      right: { fact_ref: "flag_b" } as unknown as PredicateExpression,
    };
    expect(orExpr.op).toBe("or");
    expect("left" in orExpr).toBe(true);
    expect("right" in orExpr).toBe(true);
  });

  it("verdict_present in rule body references correct verdict id", () => {
    const when: VerdictPresentExpr = { verdict_present: "ApprovedByBuyer" };
    const rule: RuleConstruct = {
      ...newRule("escalation_rule", 1),
      body: {
        when,
        produce: {
          verdict_type: "EscalatedVerdict",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const pred = rule.body.when as VerdictPresentExpr;
    expect(pred.verdict_present).toBe("ApprovedByBuyer");
  });
});

// ---------------------------------------------------------------------------
// Validation tests
// ---------------------------------------------------------------------------

describe("Rule editor — validation", () => {
  it("no error when stratum 1 rule references stratum 0 verdict", () => {
    const baseRule = newRule("base", 0);
    const finalRule: RuleConstruct = {
      ...newRule("final", 1),
      body: {
        when: { verdict_present: "base_verdict" } as PredicateExpression,
        produce: {
          verdict_type: "final_verdict",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const errors = validateRuleVerdictPresent(finalRule, [baseRule, finalRule]);
    expect(errors).toHaveLength(0);
  });

  it("validation error when same-stratum verdict_present used", () => {
    const ruleA = newRule("rule_a", 0);
    const ruleB: RuleConstruct = {
      ...newRule("rule_b", 0),
      body: {
        when: { verdict_present: "rule_a_verdict" } as PredicateExpression,
        produce: {
          verdict_type: "rule_b_verdict",
          payload: { type: { base: "Bool" }, value: true },
        },
      },
    };
    const errors = validateRuleVerdictPresent(ruleB, [ruleA, ruleB]);
    expect(errors).toHaveLength(1);
    // Error message should reference stratum conflict
    expect(errors[0]).toMatch(/stratum/i);
  });
});
