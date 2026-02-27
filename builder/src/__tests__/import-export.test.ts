/**
 * Import/export tests.
 *
 * Tests file handling, validation, and format correctness for
 * importInterchangeJson, validateImportedBundle, and generateDsl.
 */
import { describe, it, expect } from "vitest";
import {
  importInterchangeJson,
  importTenorFile,
  validateImportedBundle,
} from "../utils/import";
import { generateDsl } from "../utils/dsl-generator";
import type {
  InterchangeBundle,
  FactConstruct,
  EntityConstruct,
} from "../types/interchange";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TV = "1.0";
const BV = "1.0.0";

function makeBundle(
  id: string,
  constructs: InterchangeBundle["constructs"] = []
): InterchangeBundle {
  return { constructs, id, kind: "Bundle", tenor: TV, tenor_version: BV };
}

function prov() {
  return { file: "test.tenor", line: 1 };
}

// ---------------------------------------------------------------------------
// importInterchangeJson tests
// ---------------------------------------------------------------------------

describe("importInterchangeJson", () => {
  it("imports valid interchange JSON string", () => {
    const bundle = makeBundle("test-contract");
    const json = JSON.stringify(bundle);
    const imported = importInterchangeJson(json);

    expect(imported.id).toBe("test-contract");
    expect(imported.kind).toBe("Bundle");
    expect(imported.tenor).toBe("1.0");
    expect(Array.isArray(imported.constructs)).toBe(true);
  });

  it("imports bundle with constructs", () => {
    const fact: FactConstruct = {
      id: "is_active",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Bool" },
    };
    const bundle = makeBundle("fact-contract", [fact]);
    const imported = importInterchangeJson(JSON.stringify(bundle));

    expect(imported.constructs).toHaveLength(1);
    expect(imported.constructs[0].id).toBe("is_active");
    expect(imported.constructs[0].kind).toBe("Fact");
  });

  it("throws on invalid JSON (malformed)", () => {
    expect(() => importInterchangeJson("{not valid json}")).toThrow();
  });

  it("throws on empty JSON string", () => {
    expect(() => importInterchangeJson("")).toThrow();
  });

  it("throws on whitespace-only string", () => {
    expect(() => importInterchangeJson("   ")).toThrow();
  });

  it("throws on JSON array (not object)", () => {
    expect(() => importInterchangeJson("[1, 2, 3]")).toThrow();
  });

  it("throws on JSON missing constructs array", () => {
    const missingConstructs = JSON.stringify({
      id: "test",
      kind: "Bundle",
      tenor: "1.0",
    });
    expect(() => importInterchangeJson(missingConstructs)).toThrow(/constructs/i);
  });

  it("throws when kind is not Bundle", () => {
    const wrongKind = JSON.stringify({
      id: "test",
      kind: "NotABundle",
      constructs: [],
      tenor: "1.0",
      tenor_version: "1.0.0",
    });
    expect(() => importInterchangeJson(wrongKind)).toThrow(/Bundle/i);
  });

  it("error message mentions constructs field for missing constructs", () => {
    const json = JSON.stringify({ id: "x", kind: "Bundle", tenor: "1.0", tenor_version: "1.0.0" });
    let errorMessage = "";
    try {
      importInterchangeJson(json);
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    }
    expect(errorMessage).toContain("constructs");
  });
});

// ---------------------------------------------------------------------------
// importTenorFile tests
// ---------------------------------------------------------------------------

describe("importTenorFile", () => {
  it("throws with helpful error message directing to CLI", () => {
    expect(() => importTenorFile("fact foo { type: Bool }")).toThrow();
  });

  it("error message mentions CLI command", () => {
    let errorMessage = "";
    try {
      importTenorFile("fact foo { type: Bool }");
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    }
    // Should mention either tenor CLI, elaborate, or JSON import
    expect(
      errorMessage.toLowerCase().includes("tenor") ||
      errorMessage.toLowerCase().includes("elaborate") ||
      errorMessage.toLowerCase().includes("json")
    ).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// validateImportedBundle tests
// ---------------------------------------------------------------------------

describe("validateImportedBundle", () => {
  it("passes valid bundle", () => {
    const bundle = makeBundle("valid", [
      {
        id: "fact1",
        kind: "Fact",
        tenor: TV,
        provenance: prov(),
        type: { base: "Bool" },
      },
    ]);
    const result = validateImportedBundle(bundle);
    expect(result.valid).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it("passes empty bundle with warning", () => {
    const bundle = makeBundle("empty");
    const result = validateImportedBundle(bundle);
    expect(result.valid).toBe(true); // empty is valid (no errors)
    expect(result.warnings.length).toBeGreaterThan(0); // but warns about no constructs
  });

  it("catches duplicate IDs within same kind", () => {
    const bundle = makeBundle("dup-test", [
      {
        id: "same_id",
        kind: "Fact",
        tenor: TV,
        provenance: prov(),
        type: { base: "Bool" },
      },
      {
        id: "same_id",
        kind: "Fact",
        tenor: TV,
        provenance: prov(),
        type: { base: "Int" },
      },
    ]);
    const result = validateImportedBundle(bundle);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("same_id"))).toBe(true);
  });

  it("same id in different kinds is allowed (each kind is its own namespace)", () => {
    // Fact:order and Entity:order can coexist
    const fact: FactConstruct = {
      id: "order",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Bool" },
    };
    const entity: EntityConstruct = {
      id: "order",
      kind: "Entity",
      tenor: TV,
      provenance: prov(),
      states: ["open"],
      initial: "open",
      transitions: [],
    };
    const bundle = makeBundle("mixed-ids", [fact, entity]);
    const result = validateImportedBundle(bundle);
    // Different kind = different key (Fact:order vs Entity:order) — no error
    expect(result.valid).toBe(true);
  });

  it("error when bundle kind is wrong", () => {
    const bundle = {
      ...makeBundle("test"),
      kind: "NotBundle" as InterchangeBundle["kind"],
    };
    const result = validateImportedBundle(bundle);
    expect(result.valid).toBe(false);
  });

  it("error when bundle has empty id", () => {
    const bundle = { ...makeBundle(""), id: "" };
    const result = validateImportedBundle(bundle);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("id"))).toBe(true);
  });

  it("warning when Tenor version differs from 1.0", () => {
    const bundle = { ...makeBundle("test"), tenor: "2.0" };
    const result = validateImportedBundle(bundle);
    // Version mismatch is a warning, not an error
    expect(result.warnings.some((w) => w.includes("2.0"))).toBe(true);
  });

  it("error when constructs is not an array", () => {
    const bundle = {
      ...makeBundle("test"),
      constructs: "not an array" as unknown as InterchangeBundle["constructs"],
    };
    const result = validateImportedBundle(bundle);
    expect(result.valid).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Export as JSON round-trip
// ---------------------------------------------------------------------------

describe("Export as JSON", () => {
  it("export as JSON produces valid interchange that re-imports cleanly", () => {
    const fact: FactConstruct = {
      id: "escrow_amount",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Money", currency: "USD" },
      default: {
        kind: "money_value",
        amount: { kind: "decimal_value", precision: 10, scale: 2, value: "1000.00" },
        currency: "USD",
      },
    };
    const entity: EntityConstruct = {
      id: "escrow",
      kind: "Entity",
      tenor: TV,
      provenance: prov(),
      states: ["held", "released", "refunded"],
      initial: "held",
      transitions: [
        { from: "held", to: "released" },
        { from: "held", to: "refunded" },
      ],
    };
    const bundle = makeBundle("export-test", [fact, entity]);

    // Export
    const exported = JSON.stringify(bundle);

    // Re-import
    const reimported = importInterchangeJson(exported);
    const validation = validateImportedBundle(reimported);

    expect(validation.valid).toBe(true);
    expect(reimported.constructs).toHaveLength(2);

    const reimportedFact = reimported.constructs.find((c) => c.kind === "Fact") as FactConstruct;
    expect(reimportedFact.type.base).toBe("Money");

    const reimportedEntity = reimported.constructs.find((c) => c.kind === "Entity") as EntityConstruct;
    expect(reimportedEntity.initial).toBe("held");
    expect(reimportedEntity.states).toHaveLength(3);
  });
});

// ---------------------------------------------------------------------------
// Export as .tenor DSL
// ---------------------------------------------------------------------------

describe("Export as .tenor DSL", () => {
  it("export as .tenor contains fact and entity keywords", () => {
    const fact: FactConstruct = {
      id: "is_valid",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Bool" },
    };
    const entity: EntityConstruct = {
      id: "contract_state",
      kind: "Entity",
      tenor: TV,
      provenance: prov(),
      states: ["open", "closed"],
      initial: "open",
      transitions: [{ from: "open", to: "closed" }],
    };
    const bundle = makeBundle("dsl-test", [fact, entity]);
    const dsl = generateDsl(bundle);

    expect(dsl).toContain("fact is_valid");
    expect(dsl).toContain("entity contract_state");
    expect(typeof dsl).toBe("string");
    expect(dsl.length).toBeGreaterThan(0);
  });

  it("DSL uses lowercase keywords (not Fact, Entity, Rule)", () => {
    const fact: FactConstruct = {
      id: "my_bool",
      kind: "Fact",
      tenor: TV,
      provenance: prov(),
      type: { base: "Bool" },
    };
    const dsl = generateDsl(makeBundle("case-test", [fact]));

    // Lowercase keywords
    expect(dsl).toContain("fact my_bool");
    expect(dsl).not.toMatch(/^Fact /m);
  });

  it("generated DSL is a non-empty string", () => {
    const bundle = makeBundle("non-empty", [
      {
        id: "x",
        kind: "Fact",
        tenor: TV,
        provenance: prov(),
        type: { base: "Bool" },
      },
    ]);
    const dsl = generateDsl(bundle);
    expect(typeof dsl).toBe("string");
    expect(dsl.trim().length).toBeGreaterThan(0);
  });
});
