/**
 * Model round-trip integrity tests.
 *
 * Tests that the Builder's model survives round-trips through export/import cycles.
 * Canonical construct ordering: Personas, Sources, Facts, Entities, Rules (by stratum),
 * Operations, Flows, Systems.
 */
import { describe, it, expect, beforeEach } from "vitest";
import { importInterchangeJson, validateImportedBundle } from "../utils/import";
import { generateDsl } from "../utils/dsl-generator";
import type {
  InterchangeBundle,
  FactConstruct,
  EntityConstruct,
  RuleConstruct,
} from "../types/interchange";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";
const TENOR_BUNDLE_VERSION = "1.0.0";

function makeBundle(
  id: string,
  constructs: InterchangeBundle["constructs"] = []
): InterchangeBundle {
  return {
    constructs,
    id,
    kind: "Bundle",
    tenor: TENOR_VERSION,
    tenor_version: TENOR_BUNDLE_VERSION,
  };
}

function fakeProv(file = "test.tenor", line = 1) {
  return { file, line };
}

function exportBundleJson(bundle: InterchangeBundle): string {
  return JSON.stringify(bundle);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Model round-trip", () => {
  it("empty contract round-trip", () => {
    const bundle = makeBundle("empty-contract");
    const exported = exportBundleJson(bundle);
    const imported = importInterchangeJson(exported);

    expect(imported.id).toBe("empty-contract");
    expect(imported.kind).toBe("Bundle");
    expect(imported.tenor).toBe("1.0");
    expect(imported.constructs).toHaveLength(0);
  });

  it("fact-only contract round-trip", () => {
    const facts: FactConstruct[] = [
      {
        id: "is_active",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Bool" },
        default: { kind: "bool_literal", value: true },
      },
      {
        id: "count",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Int", min: 0, max: 100 },
      },
      {
        id: "price",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Money", currency: "USD" },
        default: {
          kind: "money_value",
          amount: { kind: "decimal_value", precision: 10, scale: 2, value: "99.99" },
          currency: "USD",
        },
      },
      {
        id: "status",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Enum", values: ["pending", "active", "closed"] },
        default: "pending",
      },
      {
        id: "label",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Text" },
        default: "hello",
      },
      {
        id: "tags",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "List", element_type: { base: "Text" }, max: 10 },
      },
    ];

    const bundle = makeBundle("fact-contract", facts);
    const exported = exportBundleJson(bundle);
    const imported = importInterchangeJson(exported);

    expect(imported.constructs).toHaveLength(6);
    const boolFact = imported.constructs.find((c) => c.id === "is_active") as FactConstruct;
    expect(boolFact.kind).toBe("Fact");
    expect(boolFact.type.base).toBe("Bool");
    expect(boolFact.default).toEqual({ kind: "bool_literal", value: true });

    const enumFact = imported.constructs.find((c) => c.id === "status") as FactConstruct;
    expect(enumFact.type.base).toBe("Enum");
    if (enumFact.type.base === "Enum") {
      expect(enumFact.type.values).toEqual(["pending", "active", "closed"]);
    }

    const priceFact = imported.constructs.find((c) => c.id === "price") as FactConstruct;
    expect(priceFact.type.base).toBe("Money");
    expect(priceFact.default).toBeDefined();

    const listFact = imported.constructs.find((c) => c.id === "tags") as FactConstruct;
    expect(listFact.type.base).toBe("List");
    if (listFact.type.base === "List") {
      expect(listFact.type.max).toBe(10);
      expect(listFact.type.element_type.base).toBe("Text");
    }
  });

  it("entity contract round-trip", () => {
    const entity: EntityConstruct = {
      id: "order",
      kind: "Entity",
      tenor: TENOR_VERSION,
      provenance: fakeProv(),
      states: ["pending", "confirmed", "shipped", "closed"],
      initial: "pending",
      transitions: [
        { from: "pending", to: "confirmed" },
        { from: "confirmed", to: "shipped" },
        { from: "shipped", to: "closed" },
      ],
    };

    const bundle = makeBundle("entity-contract", [entity]);
    const exported = exportBundleJson(bundle);
    const imported = importInterchangeJson(exported);

    expect(imported.constructs).toHaveLength(1);
    const importedEntity = imported.constructs[0] as EntityConstruct;
    expect(importedEntity.id).toBe("order");
    expect(importedEntity.initial).toBe("pending");
    expect(importedEntity.states).toEqual(["pending", "confirmed", "shipped", "closed"]);
    expect(importedEntity.transitions).toHaveLength(3);
    expect(importedEntity.transitions[0]).toEqual({ from: "pending", to: "confirmed" });
    expect(importedEntity.transitions[2]).toEqual({ from: "shipped", to: "closed" });
  });

  it("DSL export round-trip — basic facts", () => {
    const facts: FactConstruct[] = [
      {
        id: "is_active",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Bool" },
        default: false,
      },
      {
        id: "item_count",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Int" },
      },
    ];

    const bundle = makeBundle("dsl-test", facts);
    const dsl = generateDsl(bundle);

    expect(dsl).toContain("fact is_active");
    expect(dsl).toContain("fact item_count");
    expect(dsl).toContain("type:");
    // Lowercase keywords
    expect(dsl).not.toContain("Fact ");
    expect(dsl).not.toContain("Entity ");
  });

  it("construct ordering preserved — export includes all kinds in correct order sections", () => {
    const persona = {
      id: "buyer",
      kind: "Persona" as const,
      tenor: TENOR_VERSION,
      provenance: fakeProv(),
    };
    const fact: FactConstruct = {
      id: "amount",
      kind: "Fact",
      tenor: TENOR_VERSION,
      provenance: fakeProv(),
      type: { base: "Money", currency: "USD" },
    };
    const entity: EntityConstruct = {
      id: "order",
      kind: "Entity",
      tenor: TENOR_VERSION,
      provenance: fakeProv(),
      states: ["pending", "complete"],
      initial: "pending",
      transitions: [{ from: "pending", to: "complete" }],
    };
    const rule: RuleConstruct = {
      id: "approve_rule",
      kind: "Rule",
      tenor: TENOR_VERSION,
      provenance: fakeProv(),
      stratum: 0,
      body: {
        when: { fact_ref: "amount" } as unknown as RuleConstruct["body"]["when"],
        produce: {
          verdict_type: "Approved",
          payload: {
            type: { base: "Bool" },
            value: true,
          },
        },
      },
    };

    // Constructs added out of canonical order (rule, entity, fact, persona)
    const bundle = makeBundle("ordering-test", [rule, entity, fact, persona]);

    // Export and import back
    const exported = exportBundleJson(bundle);
    const imported = importInterchangeJson(exported);

    // Validate passes
    const validation = validateImportedBundle(imported);
    expect(validation.valid).toBe(true);

    // All 4 constructs present
    expect(imported.constructs).toHaveLength(4);

    // DSL should group by kind with section headers
    const dsl = generateDsl(bundle);
    expect(dsl).toContain("persona buyer");
    expect(dsl).toContain("fact amount");
    expect(dsl).toContain("entity order");
    expect(dsl).toContain("rule approve_rule");

    // Personas should appear before Facts in the DSL
    const personaIdx = dsl.indexOf("persona buyer");
    const factIdx = dsl.indexOf("fact amount");
    expect(personaIdx).toBeLessThan(factIdx);

    // Facts should appear before Entities in the DSL
    const entityIdx = dsl.indexOf("entity order");
    expect(factIdx).toBeLessThan(entityIdx);
  });

  it("validateImportedBundle passes on a valid bundle", () => {
    const bundle = makeBundle("valid-test", [
      {
        id: "my_fact",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Bool" },
      },
    ]);

    const result = validateImportedBundle(bundle);
    expect(result.valid).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it("validateImportedBundle catches duplicate IDs within same kind", () => {
    const bundle = makeBundle("dup-test", [
      {
        id: "same_id",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Bool" },
      },
      {
        id: "same_id",
        kind: "Fact",
        tenor: TENOR_VERSION,
        provenance: fakeProv(),
        type: { base: "Int" },
      },
    ]);

    const result = validateImportedBundle(bundle);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("same_id"))).toBe(true);
  });
});
