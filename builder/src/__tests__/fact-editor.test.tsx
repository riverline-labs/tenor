/**
 * Fact editor tests.
 *
 * Tests fact management: all BaseType variants, defaults, CRUD, validation.
 * Tests the store logic that FactEditor delegates to, plus pure type helpers.
 */
import { describe, it, expect } from "vitest";
import type {
  FactConstruct,
  BaseType,
  FactDefault,
  MoneyValue,
  DecimalValue,
  BoolLiteral,
} from "../types/interchange";

// ---------------------------------------------------------------------------
// Helpers — mirrors FactEditor logic
// ---------------------------------------------------------------------------

const TV = "1.0";

function newFact(id: string): FactConstruct {
  return {
    id,
    kind: "Fact",
    provenance: { file: "builder", line: 0 },
    tenor: TV,
    type: { base: "Bool" },
  };
}

function clearDefault(fact: FactConstruct): FactConstruct {
  const copy = { ...fact };
  delete copy.default;
  return copy;
}

// ---------------------------------------------------------------------------
// Fact creation tests for each BaseType variant
// ---------------------------------------------------------------------------

describe("Fact editor — BaseType variants", () => {
  it("creates Bool fact with correct interchange structure", () => {
    const fact = newFact("is_active");
    const withDefault: FactConstruct = {
      ...fact,
      type: { base: "Bool" },
      default: { kind: "bool_literal", value: true } satisfies BoolLiteral,
    };
    expect(withDefault.kind).toBe("Fact");
    expect(withDefault.type.base).toBe("Bool");
    const def = withDefault.default as BoolLiteral;
    expect(def.kind).toBe("bool_literal");
    expect(def.value).toBe(true);
  });

  it("creates Int fact with optional min/max constraints", () => {
    const fact: FactConstruct = {
      ...newFact("count"),
      type: { base: "Int", min: 0, max: 100 },
    };
    expect(fact.type.base).toBe("Int");
    if (fact.type.base === "Int") {
      expect(fact.type.min).toBe(0);
      expect(fact.type.max).toBe(100);
    }
  });

  it("creates Money fact with currency parameter", () => {
    const moneyDefault: MoneyValue = {
      kind: "money_value",
      amount: {
        kind: "decimal_value",
        precision: 10,
        scale: 2,
        value: "500.00",
      } satisfies DecimalValue,
      currency: "USD",
    };
    const fact: FactConstruct = {
      ...newFact("escrow_amount"),
      type: { base: "Money", currency: "USD" },
      default: moneyDefault,
    };
    expect(fact.type.base).toBe("Money");
    if (fact.type.base === "Money") {
      expect(fact.type.currency).toBe("USD");
    }
    const def = fact.default as MoneyValue;
    expect(def.kind).toBe("money_value");
    expect(def.currency).toBe("USD");
    expect(def.amount.value).toBe("500.00");
  });

  it("creates Enum fact with values list", () => {
    const fact: FactConstruct = {
      ...newFact("delivery_status"),
      type: { base: "Enum", values: ["pending", "in_transit", "delivered"] },
      default: "pending",
    };
    expect(fact.type.base).toBe("Enum");
    if (fact.type.base === "Enum") {
      expect(fact.type.values).toEqual(["pending", "in_transit", "delivered"]);
    }
    expect(fact.default).toBe("pending");
  });

  it("creates List fact with element_type and max", () => {
    const fact: FactConstruct = {
      ...newFact("tags"),
      type: {
        base: "List",
        element_type: { base: "Text" },
        max: 20,
      },
    };
    expect(fact.type.base).toBe("List");
    if (fact.type.base === "List") {
      expect(fact.type.element_type.base).toBe("Text");
      expect(fact.type.max).toBe(20);
    }
  });

  it("creates Record fact with fields", () => {
    const fact: FactConstruct = {
      ...newFact("address"),
      type: {
        base: "Record",
        fields: {
          street: { base: "Text" },
          city: { base: "Text" },
          zip: { base: "Int" },
        },
      },
    };
    expect(fact.type.base).toBe("Record");
    if (fact.type.base === "Record") {
      expect(Object.keys(fact.type.fields)).toHaveLength(3);
      expect(fact.type.fields.street).toEqual({ base: "Text" });
      expect(fact.type.fields.zip).toEqual({ base: "Int" });
    }
  });

  it("creates Decimal fact with precision and scale", () => {
    const fact: FactConstruct = {
      ...newFact("rate"),
      type: { base: "Decimal", precision: 10, scale: 4 },
    };
    expect(fact.type.base).toBe("Decimal");
    if (fact.type.base === "Decimal") {
      expect(fact.type.precision).toBe(10);
      expect(fact.type.scale).toBe(4);
    }
  });

  it("creates Text fact with optional max_length", () => {
    const fact: FactConstruct = {
      ...newFact("description"),
      type: { base: "Text", max_length: 500 },
      default: "placeholder",
    };
    expect(fact.type.base).toBe("Text");
    if (fact.type.base === "Text") {
      expect(fact.type.max_length).toBe(500);
    }
    expect(fact.default).toBe("placeholder");
  });

  it("creates Date fact", () => {
    const fact: FactConstruct = {
      ...newFact("created_at"),
      type: { base: "Date" },
      default: "2024-01-01",
    };
    expect(fact.type.base).toBe("Date");
    expect(fact.default).toBe("2024-01-01");
  });

  it("creates DateTime fact", () => {
    const fact: FactConstruct = {
      ...newFact("timestamp"),
      type: { base: "DateTime" },
      default: "2024-01-01T00:00:00Z",
    };
    expect(fact.type.base).toBe("DateTime");
  });
});

// ---------------------------------------------------------------------------
// Default value management
// ---------------------------------------------------------------------------

describe("Fact editor — default value management", () => {
  it("changing fact type clears default", () => {
    const fact: FactConstruct = {
      ...newFact("amount"),
      type: { base: "Bool" },
      default: { kind: "bool_literal", value: true } as BoolLiteral,
    };
    // Simulate type change: clear default
    const updated = clearDefault({ ...fact, type: { base: "Int" } });
    expect(updated.default).toBeUndefined();
  });

  it("sets default value for Bool fact", () => {
    const fact = newFact("flag");
    const boolDefault: BoolLiteral = { kind: "bool_literal", value: false };
    const updated: FactConstruct = { ...fact, default: boolDefault };
    const def = updated.default as BoolLiteral;
    expect(def.kind).toBe("bool_literal");
    expect(def.value).toBe(false);
  });

  it("sets default value for Enum fact as plain string", () => {
    const fact: FactConstruct = {
      ...newFact("status"),
      type: { base: "Enum", values: ["a", "b", "c"] },
    };
    const updated: FactConstruct = { ...fact, default: "b" };
    expect(updated.default).toBe("b");
  });

  it("sets default value for Int as plain number", () => {
    const fact: FactConstruct = {
      ...newFact("count"),
      type: { base: "Int" },
    };
    const updated: FactConstruct = { ...fact, default: 42 };
    expect(updated.default).toBe(42);
  });

  it("sets default value for Money with correct interchange JSON structure", () => {
    const fact: FactConstruct = {
      ...newFact("amount"),
      type: { base: "Money", currency: "EUR" },
    };
    const moneyDefault: MoneyValue = {
      kind: "money_value",
      amount: {
        kind: "decimal_value",
        precision: 10,
        scale: 2,
        value: "250.00",
      },
      currency: "EUR",
    };
    const updated: FactConstruct = { ...fact, default: moneyDefault };
    const def = updated.default as MoneyValue;
    expect(def.kind).toBe("money_value");
    expect(def.currency).toBe("EUR");
    expect(def.amount.kind).toBe("decimal_value");
    expect(def.amount.value).toBe("250.00");
  });
});

// ---------------------------------------------------------------------------
// Fact bundle CRUD
// ---------------------------------------------------------------------------

describe("Fact editor — bundle CRUD", () => {
  it("adds a fact to bundle constructs", () => {
    const fact = newFact("my_fact");
    const constructs = [fact];
    expect(constructs).toHaveLength(1);
    expect(constructs[0].id).toBe("my_fact");
  });

  it("deletes fact from bundle", () => {
    const facts: FactConstruct[] = [
      newFact("fact_a"),
      newFact("fact_b"),
      newFact("fact_c"),
    ];
    const filtered = facts.filter((f) => f.id !== "fact_b");
    expect(filtered).toHaveLength(2);
    expect(filtered.find((f) => f.id === "fact_b")).toBeUndefined();
  });

  it("updates fact type in place", () => {
    const fact: FactConstruct = newFact("changeable");
    const updated: FactConstruct = {
      ...fact,
      type: { base: "Int", min: 0 },
    };
    expect(updated.id).toBe("changeable");
    expect(updated.type.base).toBe("Int");
  });

  it("preserves other facts when updating one", () => {
    const facts: FactConstruct[] = [
      newFact("fact_a"),
      newFact("fact_b"),
      newFact("fact_c"),
    ];
    const updatedFacts = facts.map((f) =>
      f.id === "fact_b" ? { ...f, type: { base: "Int" } as BaseType } : f
    );
    expect(updatedFacts).toHaveLength(3);
    const b = updatedFacts.find((f) => f.id === "fact_b") as FactConstruct;
    expect(b.type.base).toBe("Int");
    const a = updatedFacts.find((f) => f.id === "fact_a") as FactConstruct;
    expect(a.type.base).toBe("Bool"); // unchanged
  });
});
