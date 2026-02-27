import { readFileSync } from "fs";
import { TenorEvaluator } from "../../typescript/src/index";

const fixturesDir = process.argv[2] || "fixtures";

async function main() {
  const bundle = readFileSync(`${fixturesDir}/escrow-bundle.json`, "utf-8");
  const facts = JSON.parse(
    readFileSync(`${fixturesDir}/escrow-facts.json`, "utf-8"),
  );
  const entityStates = JSON.parse(
    readFileSync(`${fixturesDir}/escrow-entity-states.json`, "utf-8"),
  );
  const factsInactive = JSON.parse(
    readFileSync(`${fixturesDir}/escrow-facts-inactive.json`, "utf-8"),
  );

  const evaluator = TenorEvaluator.fromJson(bundle);

  let passed = 0;
  let failed = 0;

  // Test 1: evaluate (active)
  const verdicts = evaluator.evaluate(facts);
  const expectedVerdicts = JSON.parse(
    readFileSync(`${fixturesDir}/expected-verdicts.json`, "utf-8"),
  );
  if (jsonEqual(verdicts, expectedVerdicts)) {
    console.log("PASS: evaluate (active)");
    passed++;
  } else {
    console.log("FAIL: evaluate (active)");
    console.log("  expected:", JSON.stringify(sortKeys(expectedVerdicts)));
    console.log("  actual:  ", JSON.stringify(sortKeys(verdicts)));
    failed++;
  }

  // Test 2: evaluate (inactive)
  const verdictsInactive = evaluator.evaluate(factsInactive);
  const expectedVerdictsInactive = JSON.parse(
    readFileSync(`${fixturesDir}/expected-verdicts-inactive.json`, "utf-8"),
  );
  if (jsonEqual(verdictsInactive, expectedVerdictsInactive)) {
    console.log("PASS: evaluate (inactive)");
    passed++;
  } else {
    console.log("FAIL: evaluate (inactive)");
    console.log(
      "  expected:",
      JSON.stringify(sortKeys(expectedVerdictsInactive)),
    );
    console.log("  actual:  ", JSON.stringify(sortKeys(verdictsInactive)));
    failed++;
  }

  // Test 3: computeActionSpace (active, admin)
  const actionSpace = evaluator.computeActionSpace(facts, entityStates, "admin");
  const expectedActionSpace = JSON.parse(
    readFileSync(`${fixturesDir}/expected-action-space.json`, "utf-8"),
  );
  if (jsonEqual(actionSpace, expectedActionSpace)) {
    console.log("PASS: computeActionSpace");
    passed++;
  } else {
    console.log("FAIL: computeActionSpace");
    console.log("  expected:", JSON.stringify(sortKeys(expectedActionSpace)));
    console.log("  actual:  ", JSON.stringify(sortKeys(actionSpace)));
    failed++;
  }

  // Test 4: computeActionSpace (blocked — admin + inactive)
  const actionSpaceBlocked = evaluator.computeActionSpace(
    factsInactive,
    entityStates,
    "admin",
  );
  const expectedBlocked = JSON.parse(
    readFileSync(`${fixturesDir}/expected-action-space-blocked.json`, "utf-8"),
  );
  if (jsonEqual(actionSpaceBlocked, expectedBlocked)) {
    console.log("PASS: computeActionSpace (blocked)");
    passed++;
  } else {
    console.log("FAIL: computeActionSpace (blocked)");
    console.log("  expected:", JSON.stringify(sortKeys(expectedBlocked)));
    console.log("  actual:  ", JSON.stringify(sortKeys(actionSpaceBlocked)));
    failed++;
  }

  // Test 5: executeFlow
  const flowResult = evaluator.executeFlow(
    "approval_flow",
    facts,
    entityStates,
    "admin",
  );
  const expectedFlow = JSON.parse(
    readFileSync(`${fixturesDir}/expected-flow-result.json`, "utf-8"),
  );
  if (jsonEqual(flowResult, expectedFlow)) {
    console.log("PASS: executeFlow");
    passed++;
  } else {
    console.log("FAIL: executeFlow");
    console.log("  expected:", JSON.stringify(sortKeys(expectedFlow)));
    console.log("  actual:  ", JSON.stringify(sortKeys(flowResult)));
    failed++;
  }

  evaluator.free();

  console.log(`\nTypeScript SDK: ${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

function jsonEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(sortKeys(a)) === JSON.stringify(sortKeys(b));
}

function sortKeys(obj: unknown): unknown {
  if (Array.isArray(obj)) return obj.map(sortKeys);
  if (obj !== null && typeof obj === "object") {
    return Object.keys(obj as Record<string, unknown>)
      .sort()
      .reduce(
        (acc: Record<string, unknown>, key: string) => {
          acc[key] = sortKeys((obj as Record<string, unknown>)[key]);
          return acc;
        },
        {} as Record<string, unknown>,
      );
  }
  return obj;
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
