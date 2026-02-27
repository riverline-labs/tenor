/**
 * Import utilities for Tenor contracts.
 *
 * Handles three import sources:
 * 1. importInterchangeJson — parse a raw interchange JSON string
 * 2. importTenorFile — parse a .tenor DSL file (WASM elaborator or fallback message)
 * 3. importFromUrl — fetch an interchange bundle from a URL
 *
 * All functions return a typed InterchangeBundle or throw a descriptive error.
 */

import type { InterchangeBundle } from "@/types/interchange";

// ---------------------------------------------------------------------------
// Validation result
// ---------------------------------------------------------------------------

export interface ImportValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// ---------------------------------------------------------------------------
// validateImportedBundle — structural sanity check
// ---------------------------------------------------------------------------

/**
 * Validate an imported bundle for structural correctness and version compat.
 */
export function validateImportedBundle(
  bundle: InterchangeBundle
): ImportValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Check required top-level fields
  if (!bundle.id || bundle.id.trim() === "") {
    errors.push("Bundle is missing a non-empty 'id' field.");
  }

  if (bundle.kind !== "Bundle") {
    errors.push(`Expected kind 'Bundle', got '${String(bundle.kind)}'.`);
  }

  if (!bundle.tenor) {
    errors.push("Bundle is missing 'tenor' version field.");
  } else if (bundle.tenor !== "1.0") {
    warnings.push(
      `Bundle uses Tenor version '${bundle.tenor}'. This builder targets '1.0'.`
    );
  }

  if (!bundle.constructs) {
    errors.push("Bundle is missing 'constructs' array.");
  } else if (!Array.isArray(bundle.constructs)) {
    errors.push("Bundle 'constructs' must be an array.");
  } else {
    // Check for duplicate construct IDs
    const seen = new Map<string, string>();
    for (const c of bundle.constructs) {
      if (!c.id) {
        errors.push(`Construct of kind '${String(c.kind)}' is missing an 'id' field.`);
        continue;
      }
      const key = `${c.kind}:${c.id}`;
      if (seen.has(key)) {
        errors.push(`Duplicate construct id '${c.id}' for kind '${c.kind}'.`);
      } else {
        seen.set(key, c.id);
      }
    }

    // Warn if empty
    if (bundle.constructs.length === 0) {
      warnings.push("Bundle contains no constructs.");
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

// ---------------------------------------------------------------------------
// importInterchangeJson
// ---------------------------------------------------------------------------

/**
 * Parse an interchange JSON string into a typed InterchangeBundle.
 *
 * Throws a descriptive error if the JSON is malformed or structurally invalid.
 */
export function importInterchangeJson(jsonString: string): InterchangeBundle {
  if (!jsonString || jsonString.trim() === "") {
    throw new Error("Empty JSON string provided.");
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(jsonString);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    throw new Error(`Invalid JSON: ${msg}`);
  }

  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error("Expected a JSON object at the top level.");
  }

  const obj = parsed as Record<string, unknown>;

  // Minimal required fields check before returning as InterchangeBundle
  if (!("constructs" in obj)) {
    throw new Error(
      "Not a valid interchange bundle: missing 'constructs' field. " +
        "Make sure you are importing an interchange JSON file, not a .tenor source file."
    );
  }

  if (!("kind" in obj) || obj.kind !== "Bundle") {
    throw new Error(
      `Not a valid interchange bundle: expected kind 'Bundle', ` +
        `got '${String(obj.kind ?? "missing")}'.`
    );
  }

  return obj as unknown as InterchangeBundle;
}

// ---------------------------------------------------------------------------
// importTenorFile
// ---------------------------------------------------------------------------

/**
 * Import a .tenor DSL file into an InterchangeBundle.
 *
 * Strategy:
 * 1. If a WASM elaborator is available (TenorEvaluator.elaborateBundle), use it.
 * 2. Otherwise, inform the user to export to JSON first and use importInterchangeJson.
 *
 * The browser cannot run the Rust elaborator natively. The WASM evaluator
 * only supports contract evaluation, not parsing DSL source. Until a full
 * WASM elaborator is available, this falls back to a helpful error message.
 */
export function importTenorFile(_dslContent: string): InterchangeBundle {
  // Check if the WASM evaluator exports an elaboration function
  // (future: tenor-eval-wasm may expose elaborateDsl())
  throw new Error(
    "Direct .tenor file import is not yet supported in the browser. " +
      "To import a .tenor contract:\n" +
      "  1. Run: tenor elaborate your-contract.tenor > contract.json\n" +
      "  2. Import the generated contract.json file instead.\n\n" +
      "Alternatively, use the URL import tab if your contract is served at a " +
      "/.well-known/tenor endpoint."
  );
}

// ---------------------------------------------------------------------------
// importFromUrl
// ---------------------------------------------------------------------------

/**
 * Fetch an interchange bundle from a URL.
 *
 * Tries:
 * 1. The given URL directly (if it ends in .json or contains interchange bundle)
 * 2. URL + "/.well-known/tenor" (if URL does not look like a JSON endpoint)
 *
 * Throws a descriptive error on network failure, CORS issues, or invalid content.
 */
export async function importFromUrl(url: string): Promise<InterchangeBundle> {
  if (!url || url.trim() === "") {
    throw new Error("URL is empty. Please enter a valid URL.");
  }

  // Normalize the URL
  let resolvedUrl = url.trim();

  // Determine fetch strategy: direct vs well-known endpoint
  const isJsonEndpoint =
    resolvedUrl.endsWith(".json") ||
    resolvedUrl.includes("/.well-known/tenor");

  if (!isJsonEndpoint) {
    // Append the well-known path
    resolvedUrl = resolvedUrl.replace(/\/$/, "") + "/.well-known/tenor";
  }

  let response: Response;
  try {
    response = await fetch(resolvedUrl, {
      headers: {
        Accept: "application/json",
      },
    });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    // Common case: CORS or network failure
    if (msg.toLowerCase().includes("cors") || msg.toLowerCase().includes("failed to fetch")) {
      throw new Error(
        `Network error fetching '${resolvedUrl}'.\n\n` +
          "If the server is on a different origin, ensure it sets " +
          "Access-Control-Allow-Origin headers.\n\n" +
          `Original error: ${msg}`
      );
    }
    throw new Error(`Failed to fetch '${resolvedUrl}': ${msg}`);
  }

  if (!response.ok) {
    if (response.status === 404 && !isJsonEndpoint) {
      throw new Error(
        `No /.well-known/tenor endpoint found at '${url}' (404). ` +
          "Please provide a direct URL to an interchange JSON file instead."
      );
    }
    throw new Error(
      `HTTP ${response.status} ${response.statusText} from '${resolvedUrl}'.`
    );
  }

  let text: string;
  try {
    text = await response.text();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    throw new Error(`Failed to read response body from '${resolvedUrl}': ${msg}`);
  }

  return importInterchangeJson(text);
}

// ---------------------------------------------------------------------------
// Construct count summary (for import preview)
// ---------------------------------------------------------------------------

export interface ConstructSummary {
  facts: number;
  entities: number;
  rules: number;
  operations: number;
  flows: number;
  personas: number;
  sources: number;
  systems: number;
  total: number;
}

/**
 * Summarize the construct counts in a bundle for import preview.
 */
export function summarizeBundle(bundle: InterchangeBundle): ConstructSummary {
  const counts = {
    facts: 0,
    entities: 0,
    rules: 0,
    operations: 0,
    flows: 0,
    personas: 0,
    sources: 0,
    systems: 0,
    total: bundle.constructs.length,
  };

  for (const c of bundle.constructs) {
    switch (c.kind) {
      case "Fact": counts.facts++; break;
      case "Entity": counts.entities++; break;
      case "Rule": counts.rules++; break;
      case "Operation": counts.operations++; break;
      case "Flow": counts.flows++; break;
      case "Persona": counts.personas++; break;
      case "Source": counts.sources++; break;
      case "System": counts.systems++; break;
    }
  }

  return counts;
}
