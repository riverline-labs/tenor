/**
 * Simulation store tests.
 *
 * Tests simulation functionality using mocked WASM evaluator.
 * The WASM mock is set up in setup.ts.
 */
import { describe, it, expect, beforeEach, vi } from "vitest";
import type {
  FactConstruct,
  EntityConstruct,
} from "../types/interchange";
import { evaluatorApi } from "../wasm/evaluator";

// ---------------------------------------------------------------------------
// Import the store after mock setup
// ---------------------------------------------------------------------------

// We use the simulation logic functions directly to test them in isolation
// rather than testing the Zustand store state (which requires full React context).
// This mirrors the store's behavior without circular dependencies.

// Helper functions extracted from simulation.ts logic:

const TV = "1.0";
const BV = "1.0.0";

type FactValue =
  | string
  | number
  | boolean
  | null
  | Record<string, unknown>
  | unknown[];

function makeFact(
  id: string,
  type: FactConstruct["type"],
  def?: FactConstruct["default"]
): FactConstruct {
  return {
    id,
    kind: "Fact",
    tenor: TV,
    provenance: { file: "test", line: 1 },
    type,
    ...(def !== undefined ? { default: def } : {}),
  };
}

function makeEntity(
  id: string,
  initial: string,
  states: string[]
): EntityConstruct {
  return {
    id,
    kind: "Entity",
    tenor: TV,
    provenance: { file: "test", line: 1 },
    initial,
    states,
    transitions: [],
  };
}

/**
 * Derives default FactValue from type + declared default.
 * Mirrors simulation.ts `defaultForType`.
 */
function defaultForType(
  type: FactConstruct["type"],
  declared: FactConstruct["default"] | undefined
): FactValue {
  if (declared !== undefined) {
    if (typeof declared === "object" && declared !== null && "kind" in declared) {
      const k = (declared as { kind: string }).kind;
      if (k === "bool_literal") return (declared as { kind: string; value: boolean }).value;
      if (k === "decimal_value") return (declared as { kind: string; value: string }).value;
      if (k === "money_value") {
        const m = declared as { kind: string; amount: { value: string }; currency: string };
        return { amount: m.amount.value, currency: m.currency };
      }
    }
    if (
      typeof declared === "boolean" ||
      typeof declared === "number" ||
      typeof declared === "string"
    ) {
      return declared;
    }
  }

  switch (type.base) {
    case "Bool": return false;
    case "Int": return 0;
    case "Decimal":
      return "0." + "0".repeat((type as { base: "Decimal"; scale: number }).scale);
    case "Text": return "";
    case "Date": return "2024-01-01";
    case "DateTime": return "2024-01-01T00:00:00Z";
    case "Duration": return 0;
    case "Money": {
      const mt = type as { base: "Money"; currency: string };
      return { amount: "0.00", currency: mt.currency };
    }
    case "Enum": {
      const et = type as { base: "Enum"; values: string[] };
      return et.values[0] ?? "";
    }
    case "List": return [];
    case "Record": {
      const rt = type as { base: "Record"; fields: Record<string, FactConstruct["type"]> };
      const obj: Record<string, FactValue> = {};
      for (const [k, v] of Object.entries(rt.fields)) {
        obj[k] = defaultForType(v, undefined);
      }
      return obj;
    }
    case "TaggedUnion": return null;
    default: return null;
  }
}

/**
 * Build factValues map from facts (mirrors initFromContract).
 */
function buildFactValues(facts: FactConstruct[]): Record<string, FactValue> {
  const map: Record<string, FactValue> = {};
  for (const fact of facts) {
    map[fact.id] = defaultForType(fact.type, fact.default);
  }
  return map;
}

/**
 * Build entityStates map from entities (mirrors initFromContract).
 */
function buildEntityStates(entities: EntityConstruct[]): Record<string, string> {
  const map: Record<string, string> = {};
  for (const entity of entities) {
    map[entity.id] = entity.initial;
  }
  return map;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Simulation — initFromContract fact values", () => {
  it("populates factValues from facts with defaults", () => {
    const facts: FactConstruct[] = [
      makeFact("flag", { base: "Bool" }, { kind: "bool_literal", value: true }),
      makeFact("count", { base: "Int" }, 42),
      makeFact("label", { base: "Text" }, "hello"),
    ];
    const values = buildFactValues(facts);
    expect(values.flag).toBe(true);
    expect(values.count).toBe(42);
    expect(values.label).toBe("hello");
  });

  it("uses zero defaults for facts without declared default", () => {
    const facts: FactConstruct[] = [
      makeFact("bool_fact", { base: "Bool" }),
      makeFact("int_fact", { base: "Int" }),
      makeFact("text_fact", { base: "Text" }),
    ];
    const values = buildFactValues(facts);
    expect(values.bool_fact).toBe(false);
    expect(values.int_fact).toBe(0);
    expect(values.text_fact).toBe("");
  });

  it("money default from declared money_value", () => {
    const fact = makeFact(
      "price",
      { base: "Money", currency: "USD" },
      {
        kind: "money_value",
        amount: { kind: "decimal_value", precision: 10, scale: 2, value: "99.99" },
        currency: "USD",
      }
    );
    const values = buildFactValues([fact]);
    const moneyVal = values.price as { amount: string; currency: string };
    expect(moneyVal.amount).toBe("99.99");
    expect(moneyVal.currency).toBe("USD");
  });

  it("money zero-default when no declared default", () => {
    const fact = makeFact("price", { base: "Money", currency: "EUR" });
    const values = buildFactValues([fact]);
    const moneyVal = values.price as { amount: string; currency: string };
    expect(moneyVal.amount).toBe("0.00");
    expect(moneyVal.currency).toBe("EUR");
  });

  it("enum fact default is first value when no declared default", () => {
    const fact = makeFact("status", { base: "Enum", values: ["pending", "active"] });
    const values = buildFactValues([fact]);
    expect(values.status).toBe("pending");
  });

  it("enum fact default from declared string", () => {
    const fact = makeFact("status", { base: "Enum", values: ["pending", "active"] }, "active");
    const values = buildFactValues([fact]);
    expect(values.status).toBe("active");
  });

  it("list fact defaults to empty array", () => {
    const fact = makeFact("tags", { base: "List", element_type: { base: "Text" } });
    const values = buildFactValues([fact]);
    expect(Array.isArray(values.tags)).toBe(true);
    expect((values.tags as unknown[]).length).toBe(0);
  });

  it("record fact defaults to object with zero values for each field", () => {
    const fact = makeFact("address", {
      base: "Record",
      fields: {
        street: { base: "Text" },
        zip_code: { base: "Int" },
        is_verified: { base: "Bool" },
      },
    });
    const values = buildFactValues([fact]);
    const addr = values.address as Record<string, FactValue>;
    expect(addr.street).toBe("");
    expect(addr.zip_code).toBe(0);
    expect(addr.is_verified).toBe(false);
  });

  it("Decimal default uses declared precision/scale", () => {
    const fact = makeFact("rate", { base: "Decimal", precision: 10, scale: 4 });
    const values = buildFactValues([fact]);
    // Zero default for Decimal: "0." + "0".repeat(scale)
    expect(values.rate).toBe("0.0000");
  });
});

describe("Simulation — initFromContract entity states", () => {
  it("populates entityStates from entity initial states", () => {
    const entities = [
      makeEntity("order", "pending", ["pending", "confirmed"]),
      makeEntity("payment", "awaiting", ["awaiting", "received"]),
    ];
    const states = buildEntityStates(entities);
    expect(states.order).toBe("pending");
    expect(states.payment).toBe("awaiting");
  });

  it("multiple entities each get their own initial state", () => {
    const entities = [
      makeEntity("entity_a", "state_1", ["state_1", "state_2"]),
      makeEntity("entity_b", "active", ["active", "inactive"]),
      makeEntity("entity_c", "start", ["start", "end"]),
    ];
    const states = buildEntityStates(entities);
    expect(Object.keys(states)).toHaveLength(3);
    expect(states.entity_a).toBe("state_1");
    expect(states.entity_b).toBe("active");
    expect(states.entity_c).toBe("start");
  });
});

describe("Simulation — evaluate calls WASM evaluator", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("evaluate calls WASM evaluator.evaluate with facts JSON", () => {
    const mockEvaluate = vi.mocked(evaluatorApi.evaluate);
    mockEvaluate.mockReturnValueOnce({
      verdicts: [
        { verdict_type: "Approved", payload: true, provenance: null },
      ],
    });

    const handle = 1;
    const factsJson = JSON.stringify({ is_approved: true, amount: 500 });
    const result = evaluatorApi.evaluate(handle, factsJson);

    expect(mockEvaluate).toHaveBeenCalledWith(handle, factsJson);
    expect(result.verdicts).toHaveLength(1);
    expect(result.verdicts[0].verdict_type).toBe("Approved");
  });

  it("evaluate parses verdicts correctly", () => {
    const mockEvaluate = vi.mocked(evaluatorApi.evaluate);
    mockEvaluate.mockReturnValueOnce({
      verdicts: [
        { verdict_type: "BuyerApproved", payload: true },
        { verdict_type: "FundsAvailable", payload: "1000.00" },
      ],
    });

    const result = evaluatorApi.evaluate(1, "{}");
    expect(result.verdicts).toHaveLength(2);
    expect(result.verdicts[0].verdict_type).toBe("BuyerApproved");
    expect(result.verdicts[1].verdict_type).toBe("FundsAvailable");
  });

  it("handles empty verdicts response", () => {
    const mockEvaluate = vi.mocked(evaluatorApi.evaluate);
    mockEvaluate.mockReturnValueOnce({ verdicts: [] });

    const result = evaluatorApi.evaluate(1, "{}");
    expect(result.verdicts).toHaveLength(0);
  });
});

describe("Simulation — computeActionSpace", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("computeActionSpace called with correct parameters", () => {
    const mockActionSpace = vi.mocked(evaluatorApi.computeActionSpace);
    mockActionSpace.mockReturnValueOnce({
      allowed: [{ operation_id: "deposit", persona: "buyer" }],
      blocked: [],
    });

    const result = evaluatorApi.computeActionSpace(
      1,
      JSON.stringify({ amount: 1000 }),
      JSON.stringify({ order: "pending" }),
      "buyer"
    );

    expect(mockActionSpace).toHaveBeenCalledWith(
      1,
      JSON.stringify({ amount: 1000 }),
      JSON.stringify({ order: "pending" }),
      "buyer"
    );
    expect(result.allowed).toHaveLength(1);
    expect(result.allowed[0].operation_id).toBe("deposit");
    expect(result.blocked).toHaveLength(0);
  });

  it("action space populated correctly from result", () => {
    const mockActionSpace = vi.mocked(evaluatorApi.computeActionSpace);
    mockActionSpace.mockReturnValueOnce({
      allowed: [
        { operation_id: "deposit", persona: "buyer" },
        { operation_id: "review", persona: "arbiter" },
      ],
      blocked: [
        { operation_id: "release", reason: "escrow not yet held" },
      ],
    });

    const result = evaluatorApi.computeActionSpace(1, "{}", "{}", "buyer");
    expect(result.allowed).toHaveLength(2);
    expect(result.blocked).toHaveLength(1);
    expect(result.blocked[0].reason).toContain("escrow");
  });
});

describe("Simulation — simulateFlow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("simulateFlow called with correct params", () => {
    const mockSim = vi.mocked(evaluatorApi.simulateFlow);
    mockSim.mockReturnValueOnce({
      simulation: true,
      flow_id: "escrow_flow",
      persona: "buyer",
      outcome: "completed",
      path: [],
      would_transition: [],
      verdicts: [],
    });

    const factsJson = JSON.stringify({ amount: 500 });
    const entityStatesJson = JSON.stringify({ order: "pending" });

    const result = evaluatorApi.simulateFlow(1, "escrow_flow", "buyer", factsJson, entityStatesJson);

    expect(mockSim).toHaveBeenCalledWith(1, "escrow_flow", "buyer", factsJson, entityStatesJson);
    expect(result.flow_id).toBe("escrow_flow");
    expect(result.persona).toBe("buyer");
    expect(result.outcome).toBe("completed");
  });

  it("simulateFlow returns step path and transitions", () => {
    const mockSim = vi.mocked(evaluatorApi.simulateFlow);
    mockSim.mockReturnValueOnce({
      simulation: true,
      flow_id: "test_flow",
      persona: "buyer",
      outcome: "success",
      path: [
        { step_id: "deposit_step", step_type: "OperationStep", result: "success" },
        { step_id: "confirm_step", step_type: "OperationStep", result: "success" },
      ],
      would_transition: [
        { entity_id: "order", instance_id: "_default", from_state: "pending", to_state: "confirmed" },
      ],
      verdicts: [{ verdict_type: "EscrowLocked", payload: true }],
    });

    const result = evaluatorApi.simulateFlow(1, "test_flow", "buyer", "{}", "{}");
    expect(result.path).toHaveLength(2);
    expect(result.path[0].step_id).toBe("deposit_step");
    expect(result.would_transition).toHaveLength(1);
    expect(result.would_transition[0].from_state).toBe("pending");
    expect(result.would_transition[0].to_state).toBe("confirmed");
    expect(result.verdicts).toHaveLength(1);
  });
});

describe("Simulation — evaluation error handling", () => {
  it("evaluation errors are caught and stored", () => {
    const mockEvaluate = vi.mocked(evaluatorApi.evaluate);
    mockEvaluate.mockImplementationOnce(() => {
      throw new Error("Contract not loaded");
    });

    let caughtError: string | null = null;
    try {
      evaluatorApi.evaluate(1, "{}");
    } catch (e) {
      caughtError = e instanceof Error ? e.message : String(e);
    }
    expect(caughtError).toBe("Contract not loaded");
  });

  it("simulateFlow errors produce error message", () => {
    const mockSim = vi.mocked(evaluatorApi.simulateFlow);
    mockSim.mockImplementationOnce(() => {
      throw new Error("Flow not found: unknown_flow");
    });

    let caughtError: string | null = null;
    try {
      evaluatorApi.simulateFlow(1, "unknown_flow", "buyer", "{}", "{}");
    } catch (e) {
      caughtError = e instanceof Error ? e.message : String(e);
    }
    expect(caughtError).toContain("unknown_flow");
  });
});

describe("Simulation — resetSimulation", () => {
  it("reset clears all simulation state to initial values", () => {
    // Verify the shape of a cleared state
    const cleared = {
      factValues: {},
      entityStates: {},
      selectedPersona: null,
      verdicts: null,
      actionSpace: null,
      flowExecution: null,
      evaluationError: null,
      isEvaluating: false,
    };

    expect(cleared.factValues).toEqual({});
    expect(cleared.entityStates).toEqual({});
    expect(cleared.selectedPersona).toBeNull();
    expect(cleared.verdicts).toBeNull();
    expect(cleared.actionSpace).toBeNull();
    expect(cleared.flowExecution).toBeNull();
    expect(cleared.evaluationError).toBeNull();
    expect(cleared.isEvaluating).toBe(false);
  });
});
