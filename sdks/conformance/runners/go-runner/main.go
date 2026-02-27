// Go SDK conformance runner.
//
// Compares Go SDK output against expected fixtures generated from the Rust evaluator.
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"sort"

	tenor "github.com/riverline-labs/tenor-go"
)

func main() {
	fixturesDir := "fixtures"
	if len(os.Args) > 1 {
		fixturesDir = os.Args[1]
	}

	bundle := mustRead(fixturesDir + "/escrow-bundle.json")
	facts := mustReadObj(fixturesDir + "/escrow-facts.json")
	entityStates := mustReadObj(fixturesDir + "/escrow-entity-states.json")
	factsInactive := mustReadObj(fixturesDir + "/escrow-facts-inactive.json")

	eval, err := tenor.NewEvaluatorFromBundle([]byte(bundle))
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to load contract: %v\n", err)
		os.Exit(1)
	}
	defer eval.Close()

	passed, failed := 0, 0

	// Test 1: Evaluate (active)
	verdicts, err := eval.Evaluate(toFactSet(facts))
	if err != nil {
		fmt.Printf("FAIL: evaluate (active) — error: %v\n", err)
		failed++
	} else {
		expected := mustReadObj(fixturesDir + "/expected-verdicts.json")
		got := toObj(verdicts)
		if jsonEqual(got, expected) {
			fmt.Println("PASS: evaluate (active)")
			passed++
		} else {
			fmt.Println("FAIL: evaluate (active)")
			fmt.Printf("  expected: %s\n", mustMarshal(expected))
			fmt.Printf("  actual:   %s\n", mustMarshal(got))
			failed++
		}
	}

	// Test 2: Evaluate (inactive)
	verdictsInactive, err := eval.Evaluate(toFactSet(factsInactive))
	if err != nil {
		fmt.Printf("FAIL: evaluate (inactive) — error: %v\n", err)
		failed++
	} else {
		expected := mustReadObj(fixturesDir + "/expected-verdicts-inactive.json")
		got := toObj(verdictsInactive)
		if jsonEqual(got, expected) {
			fmt.Println("PASS: evaluate (inactive)")
			passed++
		} else {
			fmt.Println("FAIL: evaluate (inactive)")
			fmt.Printf("  expected: %s\n", mustMarshal(expected))
			fmt.Printf("  actual:   %s\n", mustMarshal(got))
			failed++
		}
	}

	// Test 3: ComputeActionSpace (active, admin)
	entityStateFlat := toEntityStateMap(entityStates)
	actionSpace, err := eval.ComputeActionSpace(toFactSet(facts), entityStateFlat, "admin")
	if err != nil {
		fmt.Printf("FAIL: computeActionSpace — error: %v\n", err)
		failed++
	} else {
		expected := mustReadObj(fixturesDir + "/expected-action-space.json")
		got := toObj(actionSpace)
		if jsonEqual(got, expected) {
			fmt.Println("PASS: computeActionSpace")
			passed++
		} else {
			fmt.Println("FAIL: computeActionSpace")
			fmt.Printf("  expected: %s\n", mustMarshal(expected))
			fmt.Printf("  actual:   %s\n", mustMarshal(got))
			failed++
		}
	}

	// Test 4: ComputeActionSpace (blocked — admin + inactive)
	actionSpaceBlocked, err := eval.ComputeActionSpace(toFactSet(factsInactive), entityStateFlat, "admin")
	if err != nil {
		fmt.Printf("FAIL: computeActionSpace (blocked) — error: %v\n", err)
		failed++
	} else {
		expected := mustReadObj(fixturesDir + "/expected-action-space-blocked.json")
		got := toObj(actionSpaceBlocked)
		if jsonEqual(got, expected) {
			fmt.Println("PASS: computeActionSpace (blocked)")
			passed++
		} else {
			fmt.Println("FAIL: computeActionSpace (blocked)")
			fmt.Printf("  expected: %s\n", mustMarshal(expected))
			fmt.Printf("  actual:   %s\n", mustMarshal(got))
			failed++
		}
	}

	// Test 5: ExecuteFlow
	flowResult, err := eval.ExecuteFlow("approval_flow", toFactSet(facts), entityStateFlat, "admin")
	if err != nil {
		fmt.Printf("FAIL: executeFlow — error: %v\n", err)
		failed++
	} else {
		expected := mustReadObj(fixturesDir + "/expected-flow-result.json")
		got := toObj(flowResult)
		if jsonEqual(got, expected) {
			fmt.Println("PASS: executeFlow")
			passed++
		} else {
			fmt.Println("FAIL: executeFlow")
			fmt.Printf("  expected: %s\n", mustMarshal(expected))
			fmt.Printf("  actual:   %s\n", mustMarshal(got))
			failed++
		}
	}

	fmt.Printf("\nGo SDK: %d passed, %d failed\n", passed, failed)
	if failed > 0 {
		os.Exit(1)
	}
}

// ── Helpers ───────────────────────────────────────────────────────────────────

func mustRead(path string) string {
	data, err := os.ReadFile(path)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to read %s: %v\n", path, err)
		os.Exit(1)
	}
	return string(data)
}

func mustReadObj(path string) map[string]interface{} {
	data, err := os.ReadFile(path)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to read %s: %v\n", path, err)
		os.Exit(1)
	}
	var obj map[string]interface{}
	if err := json.Unmarshal(data, &obj); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to parse JSON in %s: %v\n", path, err)
		os.Exit(1)
	}
	return obj
}

func mustMarshal(v interface{}) string {
	data, err := json.Marshal(sortKeysDeep(v))
	if err != nil {
		return fmt.Sprintf("<marshal error: %v>", err)
	}
	return string(data)
}

// toFactSet converts a generic map to a tenor.FactSet.
func toFactSet(m map[string]interface{}) tenor.FactSet {
	return tenor.FactSet(m)
}

// toEntityStateMap converts a generic map to a tenor.EntityStateMap.
// Assumes flat string format: {"Order": "pending"}.
func toEntityStateMap(m map[string]interface{}) tenor.EntityStateMap {
	result := make(tenor.EntityStateMap, len(m))
	for k, v := range m {
		if s, ok := v.(string); ok {
			result[k] = s
		}
	}
	return result
}

// toObj marshals a value to JSON then unmarshals to map[string]interface{}.
// This normalises the types (e.g. bool stays bool, numbers become float64).
func toObj(v interface{}) map[string]interface{} {
	data, err := json.Marshal(v)
	if err != nil {
		return nil
	}
	var obj map[string]interface{}
	if err := json.Unmarshal(data, &obj); err != nil {
		return nil
	}
	return obj
}

// jsonEqual compares two values for structural JSON equality with sorted keys.
func jsonEqual(a, b interface{}) bool {
	aJSON, err := json.Marshal(sortKeysDeep(a))
	if err != nil {
		return false
	}
	bJSON, err := json.Marshal(sortKeysDeep(b))
	if err != nil {
		return false
	}
	return string(aJSON) == string(bJSON)
}

// sortKeysDeep recursively sorts map keys in a JSON-compatible value.
func sortKeysDeep(v interface{}) interface{} {
	switch val := v.(type) {
	case map[string]interface{}:
		keys := make([]string, 0, len(val))
		for k := range val {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		sorted := make(map[string]interface{}, len(val))
		for _, k := range keys {
			sorted[k] = sortKeysDeep(val[k])
		}
		return sorted
	case []interface{}:
		result := make([]interface{}, len(val))
		for i, item := range val {
			result[i] = sortKeysDeep(item)
		}
		return result
	default:
		return v
	}
}
