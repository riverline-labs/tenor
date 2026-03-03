/**
 * Elaborator shim for the Tenor Builder.
 *
 * The Builder's internal model IS the interchange JSON. The "elaborator"
 * in the Builder context means structural validation without a separate
 * compilation step — the visual editors construct interchange JSON directly.
 *
 * Two validation paths:
 * 1. quickValidate(): synchronous structural checks (no WASM)
 * 2. validateBundle(): WASM-based contract loading validation
 */

import type { InterchangeBundle, InterchangeConstruct } from "@/types/interchange";
import { evaluatorApi } from "./evaluator";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

export interface ValidationError {
  construct_id?: string;
  construct_kind?: string;
  field?: string;
  message: string;
  severity: "error" | "warning";
}

// ---------------------------------------------------------------------------
// Quick validate — synchronous structural checks, no WASM
// ---------------------------------------------------------------------------

/**
 * Run fast structural invariant checks without the WASM evaluator.
 * Returns errors for any violations found.
 */
export function quickValidate(bundle: InterchangeBundle): ValidationError[] {
  const errors: ValidationError[] = [];

  // Build an index of all construct IDs per kind
  const idsByKind: Map<string, Set<string>> = new Map();
  const allIds = new Set<string>();

  for (const c of bundle.constructs) {
    const kindSet = idsByKind.get(c.kind) ?? new Set();
    if (kindSet.has(c.id)) {
      errors.push({
        construct_id: c.id,
        construct_kind: c.kind,
        message: `Duplicate construct id '${c.id}' for kind '${c.kind}'`,
        severity: "error",
      });
    }
    kindSet.add(c.id);
    idsByKind.set(c.kind, kindSet);

    if (allIds.has(`${c.kind}:${c.id}`)) {
      // Already reported above
    } else {
      allIds.add(`${c.kind}:${c.id}`);
    }
  }

  const personaIds = idsByKind.get("Persona") ?? new Set<string>();
  const operationIds = idsByKind.get("Operation") ?? new Set<string>();
  const factIds = idsByKind.get("Fact") ?? new Set<string>();
  const flowIds = idsByKind.get("Flow") ?? new Set<string>();

  for (const c of bundle.constructs) {
    if (c.kind === "Entity") {
      const stateSet = new Set(c.states);

      // Validate initial state
      if (!stateSet.has(c.initial)) {
        errors.push({
          construct_id: c.id,
          construct_kind: "Entity",
          field: "initial",
          message: `Initial state '${c.initial}' not in states list`,
          severity: "error",
        });
      }

      // Validate transitions
      for (const t of c.transitions) {
        if (!stateSet.has(t.from)) {
          errors.push({
            construct_id: c.id,
            construct_kind: "Entity",
            field: "transitions",
            message: `Transition from state '${t.from}' not in states list`,
            severity: "error",
          });
        }
        if (!stateSet.has(t.to)) {
          errors.push({
            construct_id: c.id,
            construct_kind: "Entity",
            field: "transitions",
            message: `Transition to state '${t.to}' not in states list`,
            severity: "error",
          });
        }
      }
    }

    if (c.kind === "Rule") {
      // Validate non-negative stratum
      if (c.stratum < 0 || !Number.isInteger(c.stratum)) {
        errors.push({
          construct_id: c.id,
          construct_kind: "Rule",
          field: "stratum",
          message: `Stratum must be a non-negative integer, got ${c.stratum}`,
          severity: "error",
        });
      }
    }

    if (c.kind === "Operation") {
      // Validate personas
      for (const p of c.allowed_personas) {
        if (personaIds.size > 0 && !personaIds.has(p)) {
          errors.push({
            construct_id: c.id,
            construct_kind: "Operation",
            field: "allowed_personas",
            message: `Persona '${p}' not declared`,
            severity: "warning",
          });
        }
      }
    }

    if (c.kind === "Flow") {
      // Validate entry step exists
      const stepIds = new Set(c.steps.map((s) => s.id));

      if (!stepIds.has(c.entry)) {
        errors.push({
          construct_id: c.id,
          construct_kind: "Flow",
          field: "entry",
          message: `Entry step '${c.entry}' not found in steps`,
          severity: "error",
        });
      }

      // Validate operation steps reference declared operations
      for (const step of c.steps) {
        if (step.kind === "OperationStep") {
          if (operationIds.size > 0 && !operationIds.has(step.op)) {
            errors.push({
              construct_id: c.id,
              construct_kind: "Flow",
              field: "steps",
              message: `Step '${step.id}' references undeclared operation '${step.op}'`,
              severity: "warning",
            });
          }
        }
        if (step.kind === "SubFlowStep") {
          if (flowIds.size > 0 && !flowIds.has(step.flow)) {
            errors.push({
              construct_id: c.id,
              construct_kind: "Flow",
              field: "steps",
              message: `Sub-flow step '${step.id}' references undeclared flow '${step.flow}'`,
              severity: "warning",
            });
          }
        }
      }
    }
  }

  // Validate bundle-level fields
  if (!bundle.id || bundle.id.trim() === "") {
    errors.push({
      message: "Bundle must have a non-empty id",
      severity: "error",
    });
  }

  // Check for empty constructs is not an error but worth noting
  if (bundle.constructs.length === 0) {
    errors.push({
      message: "Contract has no constructs",
      severity: "warning",
    });
  }

  return errors;
}

// ---------------------------------------------------------------------------
// WASM-based bundle validation
// ---------------------------------------------------------------------------

/**
 * Validate a bundle by loading it into the WASM evaluator.
 * This catches deeper issues beyond structural checks.
 *
 * Requires the WASM evaluator to be initialized first (call initEvaluator()).
 */
export function validateBundle(bundle: InterchangeBundle): ValidationResult {
  // First run quick validate
  const quickErrors = quickValidate(bundle);
  const hasBlockingErrors = quickErrors.some((e) => e.severity === "error");

  if (hasBlockingErrors) {
    return { valid: false, errors: quickErrors };
  }

  // Try loading into WASM evaluator
  let handle: number | null = null;
  try {
    const bundleJson = JSON.stringify(bundle);
    const result = evaluatorApi.loadContract(bundleJson);
    handle = result.handle;
    // If load_contract succeeds, bundle is structurally valid for evaluation
    return { valid: true, errors: quickErrors };
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    return {
      valid: false,
      errors: [
        ...quickErrors,
        {
          message: `WASM validation failed: ${message}`,
          severity: "error",
        },
      ],
    };
  } finally {
    if (handle !== null) {
      try {
        evaluatorApi.freeContract(handle);
      } catch {
        // ignore cleanup errors
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Helper: check if a bundle passes all validations
// ---------------------------------------------------------------------------

export function isBundleValid(bundle: InterchangeBundle): boolean {
  const errors = quickValidate(bundle);
  return !errors.some((e) => e.severity === "error");
}

// ---------------------------------------------------------------------------
// Helper: get constructs by kind from a bundle
// ---------------------------------------------------------------------------

export function getConstructsByKind<K extends InterchangeConstruct["kind"]>(
  bundle: InterchangeBundle,
  kind: K
): Extract<InterchangeConstruct, { kind: K }>[] {
  return bundle.constructs.filter(
    (c): c is Extract<InterchangeConstruct, { kind: K }> => c.kind === kind
  );
}
