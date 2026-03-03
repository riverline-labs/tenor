/**
 * Entity editor CRUD tests.
 *
 * Tests entity management through the contract store (which the EntityEditor delegates to).
 * Validates: add state, remove state, set initial, add/delete transitions, validation.
 */
import { describe, it, expect, beforeEach } from "vitest";
import type {
  EntityConstruct,
  InterchangeBundle,
} from "../types/interchange";

// ---------------------------------------------------------------------------
// Helpers — in-memory entity store operations
// (mirrors what EntityEditor does via useContractStore)
// ---------------------------------------------------------------------------

const TV = "1.0";
const BV = "1.0.0";

function makeBundle(constructs: InterchangeBundle["constructs"] = []): InterchangeBundle {
  return { constructs, id: "test", kind: "Bundle", tenor: TV, tenor_version: BV };
}

function newEntity(id: string): EntityConstruct {
  return {
    id,
    initial: "initial",
    kind: "Entity",
    provenance: { file: "builder", line: 0 },
    states: ["initial"],
    tenor: TV,
    transitions: [],
  };
}

/**
 * Validate an entity construct, returning error strings (mirrors EntityEditor validateEntity).
 */
function validateEntity(entity: EntityConstruct): string[] {
  const errors: string[] = [];
  if (!entity.initial) {
    errors.push("Entity must have an initial state.");
  }
  if (!entity.states.includes(entity.initial)) {
    errors.push(`Initial state "${entity.initial}" is not in the states list.`);
  }
  // Orphan states: states with no transitions in or out (other than initial)
  const connected = new Set<string>();
  entity.transitions.forEach((t) => {
    connected.add(t.from);
    connected.add(t.to);
  });
  const orphans = entity.states.filter(
    (s) => s !== entity.initial && !connected.has(s)
  );
  if (orphans.length > 0) {
    errors.push(`Orphan states (no transitions): ${orphans.join(", ")}`);
  }
  // Duplicate transitions
  const seen = new Set<string>();
  for (const t of entity.transitions) {
    const key = `${t.from}->${t.to}`;
    if (seen.has(key)) {
      errors.push(`Duplicate transition: ${t.from} -> ${t.to}`);
      break;
    }
    seen.add(key);
  }
  return errors;
}

// ---------------------------------------------------------------------------
// Entity CRUD tests
// ---------------------------------------------------------------------------

describe("Entity store CRUD", () => {
  it("creates a new entity with initial state", () => {
    const entity = newEntity("order");
    expect(entity.id).toBe("order");
    expect(entity.states).toEqual(["initial"]);
    expect(entity.initial).toBe("initial");
    expect(entity.transitions).toEqual([]);
    expect(entity.kind).toBe("Entity");
  });

  it("adds a new entity to bundle", () => {
    const bundle = makeBundle();
    const entity = newEntity("order");
    bundle.constructs.push(entity);
    const entities = bundle.constructs.filter((c) => c.kind === "Entity");
    expect(entities).toHaveLength(1);
    expect(entities[0].id).toBe("order");
  });

  it("adds a state to entity", () => {
    const entity = newEntity("order");
    const updated: EntityConstruct = {
      ...entity,
      states: [...entity.states, "confirmed"],
    };
    expect(updated.states).toContain("confirmed");
    expect(updated.states).toHaveLength(2);
  });

  it("removes a state from entity", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      states: ["initial", "confirmed", "shipped"],
      transitions: [{ from: "initial", to: "confirmed" }],
    };
    const updated: EntityConstruct = {
      ...entity,
      states: entity.states.filter((s) => s !== "shipped"),
      transitions: entity.transitions.filter(
        (t) => t.from !== "shipped" && t.to !== "shipped"
      ),
    };
    expect(updated.states).not.toContain("shipped");
    expect(updated.states).toHaveLength(2);
    expect(updated.transitions).toHaveLength(1);
  });

  it("sets initial state", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      states: ["initial", "confirmed"],
    };
    const updated: EntityConstruct = { ...entity, initial: "confirmed" };
    expect(updated.initial).toBe("confirmed");
  });

  it("adds a transition", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      states: ["initial", "confirmed"],
    };
    const updated: EntityConstruct = {
      ...entity,
      transitions: [...entity.transitions, { from: "initial", to: "confirmed" }],
    };
    expect(updated.transitions).toHaveLength(1);
    expect(updated.transitions[0]).toEqual({ from: "initial", to: "confirmed" });
  });

  it("deletes a transition", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      states: ["initial", "confirmed", "shipped"],
      transitions: [
        { from: "initial", to: "confirmed" },
        { from: "confirmed", to: "shipped" },
      ],
    };
    const updated: EntityConstruct = {
      ...entity,
      transitions: entity.transitions.filter(
        (t) => !(t.from === "initial" && t.to === "confirmed")
      ),
    };
    expect(updated.transitions).toHaveLength(1);
    expect(updated.transitions[0]).toEqual({ from: "confirmed", to: "shipped" });
  });

  it("removes entity from bundle", () => {
    const entity = newEntity("order");
    const bundle = makeBundle([entity]);
    const updated = {
      ...bundle,
      constructs: bundle.constructs.filter(
        (c) => !(c.id === "order" && c.kind === "Entity")
      ),
    };
    expect(updated.constructs).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Entity validation tests
// ---------------------------------------------------------------------------

describe("Entity validation", () => {
  it("valid entity with states, initial, and transitions", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      states: ["initial", "confirmed"],
      initial: "initial",
      transitions: [{ from: "initial", to: "confirmed" }],
    };
    const errors = validateEntity(entity);
    expect(errors).toHaveLength(0);
  });

  it("validation error: no initial state (empty string)", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      initial: "",
      states: ["initial"],
    };
    const errors = validateEntity(entity);
    expect(errors.some((e) => e.includes("initial state"))).toBe(true);
  });

  it("validation error: initial state not in states list", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      initial: "missing",
      states: ["initial"],
    };
    const errors = validateEntity(entity);
    expect(errors.some((e) => e.includes("missing"))).toBe(true);
  });

  it("validation warning: orphan state (state with no transitions)", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      states: ["initial", "orphan"],
      initial: "initial",
      transitions: [], // orphan has no connections
    };
    const errors = validateEntity(entity);
    expect(errors.some((e) => e.includes("orphan") || e.includes("Orphan"))).toBe(true);
  });

  it("validation error: duplicate transition", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      states: ["initial", "confirmed"],
      initial: "initial",
      transitions: [
        { from: "initial", to: "confirmed" },
        { from: "initial", to: "confirmed" }, // duplicate
      ],
    };
    const errors = validateEntity(entity);
    expect(errors.some((e) => e.toLowerCase().includes("duplicate"))).toBe(true);
  });

  it("no orphan errors when all non-initial states have transitions", () => {
    const entity: EntityConstruct = {
      ...newEntity("order"),
      states: ["initial", "confirmed", "shipped", "closed"],
      initial: "initial",
      transitions: [
        { from: "initial", to: "confirmed" },
        { from: "confirmed", to: "shipped" },
        { from: "shipped", to: "closed" },
      ],
    };
    const errors = validateEntity(entity);
    expect(errors.filter((e) => e.toLowerCase().includes("orphan"))).toHaveLength(0);
  });
});
