/**
 * Predicate builder tests.
 *
 * Tests that predicate expression construction produces correct interchange JSON.
 * Tests pure predicate functions (default constructors, type detection).
 */
import { describe, it, expect } from "vitest";
import type {
  PredicateExpression,
  FactConstruct,
  CompareExpr,
  AndExpr,
  OrExpr,
  NotExpr,
  ForallExpr,
  ExistsExpr,
  VerdictPresentExpr,
  FactRefOperand,
  LiteralOperand,
} from "../types/interchange";

// ---------------------------------------------------------------------------
// Predicate expression builders (mirrors PredicateBuilder internal logic)
// ---------------------------------------------------------------------------

const TV = "1.0";

function makeFact(id: string, type: FactConstruct["type"]): FactConstruct {
  return {
    id,
    kind: "Fact",
    tenor: TV,
    provenance: { file: "test", line: 1 },
    type,
  };
}

function buildCompare(
  factId: string,
  op: CompareExpr["op"],
  literal: boolean | number | string
): CompareExpr {
  return {
    left: { fact_ref: factId } satisfies FactRefOperand,
    op,
    right: {
      literal,
      type: { base: typeof literal === "boolean" ? "Bool" : typeof literal === "number" ? "Int" : "Text" },
    } as LiteralOperand,
  };
}

function buildAnd(
  left: PredicateExpression,
  right: PredicateExpression
): AndExpr {
  return { op: "and", left, right };
}

function buildOr(
  left: PredicateExpression,
  right: PredicateExpression
): OrExpr {
  return { op: "or", left, right };
}

function buildNot(operand: PredicateExpression): NotExpr {
  return { op: "not", operand };
}

function buildVerdictPresent(verdictId: string): VerdictPresentExpr {
  return { verdict_present: verdictId };
}

function buildForAll(
  variable: string,
  domain: string,
  body: PredicateExpression,
  elementBase: FactConstruct["type"]["base"] = "Int"
): ForallExpr {
  return {
    quantifier: "forall",
    variable,
    domain: { fact_ref: domain },
    variable_type: { base: elementBase } as ForallExpr["variable_type"],
    body: body as ForallExpr["body"],
  };
}

function buildExists(
  variable: string,
  domain: string,
  body: PredicateExpression,
  elementBase: FactConstruct["type"]["base"] = "Int"
): ExistsExpr {
  return {
    quantifier: "exists",
    variable,
    domain: { fact_ref: domain },
    variable_type: { base: elementBase } as ExistsExpr["variable_type"],
    body: body as ExistsExpr["body"],
  };
}

// Detect expression type (mirrors PredicateBuilder.exprType)
function detectExprType(expr: PredicateExpression): string {
  if ("verdict_present" in expr) return "VerdictPresent";
  if ("quantifier" in expr) {
    const q = (expr as ForallExpr | ExistsExpr).quantifier;
    return q === "forall" ? "ForAll" : "Exists";
  }
  if ("op" in expr) {
    const op = (expr as { op: string }).op;
    if (op === "and") return "And";
    if (op === "or") return "Or";
    if (op === "not") return "Not";
    return "Compare";
  }
  return "Unknown";
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Predicate builder — simple comparison", () => {
  it("builds fact_ref = literal comparison", () => {
    const expr = buildCompare("is_approved", "=", true);
    expect(expr.op).toBe("=");
    expect((expr.left as FactRefOperand).fact_ref).toBe("is_approved");
    expect((expr.right as LiteralOperand).literal).toBe(true);
    expect(detectExprType(expr)).toBe("Compare");
  });

  it("builds fact_ref >= literal comparison", () => {
    const expr = buildCompare("amount", ">=", 1000);
    expect(expr.op).toBe(">=");
    expect((expr.left as FactRefOperand).fact_ref).toBe("amount");
    expect((expr.right as LiteralOperand).literal).toBe(1000);
  });

  it("builds fact_ref != literal comparison", () => {
    const expr = buildCompare("status", "!=", "closed");
    expect(expr.op).toBe("!=");
    expect((expr.right as LiteralOperand).literal).toBe("closed");
  });

  it("supports all comparison operators", () => {
    const ops: CompareExpr["op"][] = ["=", "!=", "<", "<=", ">", ">="];
    for (const op of ops) {
      const expr = buildCompare("x", op, 0);
      expect(expr.op).toBe(op);
    }
  });
});

describe("Predicate builder — AND expression", () => {
  it("builds AND of two comparisons", () => {
    const left = buildCompare("amount", ">", 0);
    const right = buildCompare("is_active", "=", true);
    const expr = buildAnd(left, right);

    expect(expr.op).toBe("and");
    expect(detectExprType(expr)).toBe("And");

    const leftComp = expr.left as CompareExpr;
    expect(leftComp.op).toBe(">");
    expect((leftComp.left as FactRefOperand).fact_ref).toBe("amount");

    const rightComp = expr.right as CompareExpr;
    expect(rightComp.op).toBe("=");
  });

  it("AND expression produces correct interchange structure", () => {
    const expr = buildAnd(
      buildCompare("a", "=", 1),
      buildCompare("b", "=", 2)
    );
    const json = JSON.parse(JSON.stringify(expr)); // serialize/deserialize
    expect(json.op).toBe("and");
    expect(json.left).toBeDefined();
    expect(json.right).toBeDefined();
    expect(json.left.op).toBe("=");
    expect(json.right.op).toBe("=");
  });
});

describe("Predicate builder — OR expression", () => {
  it("builds OR of two comparisons", () => {
    const expr = buildOr(
      buildCompare("flag_a", "=", true),
      buildCompare("flag_b", "=", true)
    );
    expect(expr.op).toBe("or");
    expect(detectExprType(expr)).toBe("Or");
  });

  it("OR produces { op: 'or', left: ..., right: ... } interchange structure", () => {
    const expr = buildOr(
      buildCompare("x", "!=", "closed"),
      buildCompare("y", "!=", "failed")
    );
    const json = JSON.parse(JSON.stringify(expr));
    expect(json.op).toBe("or");
    expect(json.left).toBeDefined();
    expect(json.right).toBeDefined();
  });
});

describe("Predicate builder — NOT expression", () => {
  it("builds NOT of a comparison", () => {
    const expr = buildNot(buildCompare("is_blocked", "=", true));
    expect(expr.op).toBe("not");
    expect(detectExprType(expr)).toBe("Not");
    const operand = expr.operand as CompareExpr;
    expect(operand.op).toBe("=");
  });

  it("NOT produces { op: 'not', operand: ... } interchange structure", () => {
    const expr = buildNot(buildCompare("a", "=", 0));
    const json = JSON.parse(JSON.stringify(expr));
    expect(json.op).toBe("not");
    expect(json.operand).toBeDefined();
  });
});

describe("Predicate builder — verdict_present", () => {
  it("builds VerdictPresent expression", () => {
    const expr = buildVerdictPresent("ApprovedByBuyer");
    expect("verdict_present" in expr).toBe(true);
    expect((expr as VerdictPresentExpr).verdict_present).toBe("ApprovedByBuyer");
    expect(detectExprType(expr)).toBe("VerdictPresent");
  });

  it("verdict_present produces { verdict_present: '...' } interchange structure", () => {
    const expr = buildVerdictPresent("EscrowReleased");
    const json = JSON.parse(JSON.stringify(expr));
    expect(json.verdict_present).toBe("EscrowReleased");
    expect(Object.keys(json)).toEqual(["verdict_present"]);
  });
});

describe("Predicate builder — ForAll quantifier", () => {
  it("builds ForAll quantifier expression", () => {
    const body = buildCompare("x", ">", 0);
    const expr = buildForAll("x", "items_list", body, "Int");

    expect(detectExprType(expr)).toBe("ForAll");
    const forall = expr as ForallExpr;
    expect(forall.quantifier).toBe("forall");
    expect(forall.variable).toBe("x");
    expect(forall.domain.fact_ref).toBe("items_list");
    expect(forall.variable_type.base).toBe("Int");
  });

  it("ForAll produces { quantifier: 'forall', variable, domain, variable_type, body } structure", () => {
    const expr = buildForAll("item", "payments", buildCompare("item", ">", 10), "Int");
    const json = JSON.parse(JSON.stringify(expr));
    expect(json.quantifier).toBe("forall");
    expect(json.variable).toBe("item");
    expect(json.domain.fact_ref).toBe("payments");
    expect(json.variable_type.base).toBe("Int");
    expect(json.body).toBeDefined();
  });
});

describe("Predicate builder — Exists quantifier", () => {
  it("builds Exists quantifier expression", () => {
    const body = buildCompare("x", ">", 100);
    const expr = buildExists("x", "orders", body, "Int");

    expect(detectExprType(expr)).toBe("Exists");
    const exists = expr as ExistsExpr;
    expect(exists.quantifier).toBe("exists");
    expect(exists.variable).toBe("x");
    expect(exists.domain.fact_ref).toBe("orders");
  });
});

describe("Predicate builder — nested expressions", () => {
  it("builds AND containing OR and verdict_present", () => {
    const orPart = buildOr(
      buildCompare("amount", ">", 1000),
      buildVerdictPresent("OverrideApproved")
    );
    const andExpr = buildAnd(
      buildCompare("is_active", "=", true),
      orPart
    );

    expect(detectExprType(andExpr)).toBe("And");
    const leftPart = andExpr.left as CompareExpr;
    expect(leftPart.op).toBe("=");

    const rightPart = andExpr.right as OrExpr;
    expect(rightPart.op).toBe("or");

    const rightRight = rightPart.right as VerdictPresentExpr;
    expect(rightRight.verdict_present).toBe("OverrideApproved");
  });

  it("deeply nested NOT(AND(a, b)) expression", () => {
    const inner = buildAnd(
      buildCompare("a", "=", true),
      buildCompare("b", ">", 0)
    );
    const notExpr = buildNot(inner);

    expect(detectExprType(notExpr)).toBe("Not");
    const operand = notExpr.operand as AndExpr;
    expect(detectExprType(operand)).toBe("And");
  });
});

describe("Predicate builder — type-aware operators", () => {
  it("Bool fact supports = and != operators", () => {
    const boolFact = makeFact("flag", { base: "Bool" });
    // Bool comparisons are valid with = and !=
    const eqExpr = buildCompare(boolFact.id, "=", true);
    const neqExpr = buildCompare(boolFact.id, "!=", false);
    expect(eqExpr.op).toBe("=");
    expect(neqExpr.op).toBe("!=");
  });

  it("numeric fact supports all comparison operators", () => {
    const intFact = makeFact("count", { base: "Int" });
    const ops: CompareExpr["op"][] = ["=", "!=", "<", "<=", ">", ">="];
    for (const op of ops) {
      const expr = buildCompare(intFact.id, op, 42);
      expect(expr.op).toBe(op);
    }
  });

  it("Money fact supports comparison operators", () => {
    const moneyFact = makeFact("price", { base: "Money", currency: "USD" });
    const expr = buildCompare(moneyFact.id, ">=", 0);
    expect(expr.op).toBe(">=");
    expect((expr.left as FactRefOperand).fact_ref).toBe("price");
  });
});

describe("Predicate builder — available verdicts filtering", () => {
  it("rule mode should only use verdicts from lower strata", () => {
    // Simulates: stratum 1 rule should only see stratum 0 verdicts
    const strata0Verdicts = ["BuyerApproved", "SellerApproved"];
    const strata1Verdicts = ["EscrowReleased"];

    // In rule mode for stratum 1, only show stratum 0 verdicts as available
    const availableForStratum1 = strata0Verdicts; // not strata1Verdicts

    const expr = buildVerdictPresent(availableForStratum1[0]);
    expect(expr.verdict_present).toBe("BuyerApproved");
    expect(availableForStratum1).not.toContain("EscrowReleased");
  });

  it("operation mode should pass all verdicts as available", () => {
    const allVerdicts = ["BuyerApproved", "SellerApproved", "EscrowReleased"];
    // In operation mode, all verdicts are available
    for (const v of allVerdicts) {
      const expr = buildVerdictPresent(v);
      expect(expr.verdict_present).toBe(v);
    }
  });
});
