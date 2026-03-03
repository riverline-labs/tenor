// Package tenor provides a Go client for the Tenor contract evaluator.
//
// The evaluator is implemented in Rust and compiled to WebAssembly. This
// package embeds the WASM binary and uses wazero (a pure-Go WASM runtime)
// to execute it without CGo or any native dependencies.
//
// # Quick start
//
//	eval, err := tenor.NewEvaluatorFromBundle(bundleJSON)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer eval.Close()
//
//	verdicts, err := eval.Evaluate(tenor.FactSet{"is_active": true})
//	// ...
//
//	space, err := eval.ComputeActionSpace(
//	    tenor.FactSet{"is_active": true},
//	    tenor.EntityStateMap{"Order": "pending"},
//	    "admin",
//	)
//	// ...
//
//	result, err := eval.ExecuteFlow(
//	    "approval_flow",
//	    tenor.FactSet{"is_active": true},
//	    tenor.EntityStateMap{"Order": "pending"},
//	    "admin",
//	)
package tenor

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/riverline-labs/tenor-go/internal/wasm"
)

// Evaluator wraps the Tenor contract evaluator running in a WASM module.
// It is safe to call multiple methods concurrently; the underlying WASM
// runtime serialises calls with a mutex.
//
// Close() must be called when the Evaluator is no longer needed.
type Evaluator struct {
	runtime *wasm.Runtime
	handle  uint32
}

// NewEvaluatorFromBundle creates a new Evaluator from an interchange bundle
// JSON byte slice. The bundle must be a valid Tenor interchange bundle.
//
// Each call creates a new isolated WASM runtime instance. For applications
// that evaluate many contracts concurrently, create one Evaluator per goroutine
// or use a pool.
func NewEvaluatorFromBundle(bundleJSON []byte) (*Evaluator, error) {
	ctx := context.Background()
	rt, err := wasm.NewRuntime(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create WASM runtime: %w", err)
	}

	result, err := rt.CallOneArg("load_contract", string(bundleJSON))
	if err != nil {
		_ = rt.Close()
		return nil, fmt.Errorf("failed to call load_contract: %w", err)
	}

	var loadResult struct {
		Handle *uint32 `json:"handle"`
		Error  *string `json:"error"`
	}
	if err := json.Unmarshal([]byte(result), &loadResult); err != nil {
		_ = rt.Close()
		return nil, fmt.Errorf("failed to parse load_contract result: %w", err)
	}
	if loadResult.Error != nil {
		_ = rt.Close()
		return nil, fmt.Errorf("contract load error: %s", *loadResult.Error)
	}
	if loadResult.Handle == nil {
		_ = rt.Close()
		return nil, fmt.Errorf("load_contract returned neither handle nor error")
	}

	return &Evaluator{
		runtime: rt,
		handle:  *loadResult.Handle,
	}, nil
}

// Evaluate runs stratified rule evaluation against the provided facts.
// Returns the complete VerdictSet with provenance for each verdict.
func (e *Evaluator) Evaluate(facts FactSet) (*VerdictSet, error) {
	factsJSON, err := json.Marshal(facts)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal facts: %w", err)
	}

	result, err := e.runtime.CallHandleOneArg("evaluate", e.handle, string(factsJSON))
	if err != nil {
		return nil, fmt.Errorf("evaluate WASM call failed: %w", err)
	}

	if errMsg := extractError(result); errMsg != "" {
		return nil, fmt.Errorf("evaluation error: %s", errMsg)
	}

	var verdicts VerdictSet
	if err := json.Unmarshal([]byte(result), &verdicts); err != nil {
		return nil, fmt.Errorf("failed to parse VerdictSet: %w", err)
	}

	return &verdicts, nil
}

// ComputeActionSpace computes the set of available and blocked actions for a
// persona given the current facts and entity states.
//
// entityStates maps entity IDs to their current state using the single-instance
// (flat) format: map[entity_id]state. For multi-instance contracts, use
// ComputeActionSpaceNested.
func (e *Evaluator) ComputeActionSpace(
	facts FactSet,
	entityStates EntityStateMap,
	persona string,
) (*ActionSpace, error) {
	factsJSON, err := json.Marshal(facts)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal facts: %w", err)
	}

	statesJSON, err := json.Marshal(entityStates)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal entity states: %w", err)
	}

	// compute_action_space(handle, facts_ptr, facts_len, states_ptr, states_len, persona_ptr, persona_len)
	result, err := e.runtime.CallHandleThreeArgs(
		"compute_action_space",
		e.handle,
		string(factsJSON),
		string(statesJSON),
		persona,
	)
	if err != nil {
		return nil, fmt.Errorf("compute_action_space WASM call failed: %w", err)
	}

	if errMsg := extractError(result); errMsg != "" {
		return nil, fmt.Errorf("action space error: %s", errMsg)
	}

	var actionSpace ActionSpace
	if err := json.Unmarshal([]byte(result), &actionSpace); err != nil {
		return nil, fmt.Errorf("failed to parse ActionSpace: %w", err)
	}

	return &actionSpace, nil
}

// ComputeActionSpaceNested is like ComputeActionSpace but accepts entity states
// in the multi-instance nested format: map[entity_id]map[instance_id]state.
func (e *Evaluator) ComputeActionSpaceNested(
	facts FactSet,
	entityStates EntityStateMapNested,
	persona string,
) (*ActionSpace, error) {
	factsJSON, err := json.Marshal(facts)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal facts: %w", err)
	}

	statesJSON, err := json.Marshal(entityStates)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal entity states: %w", err)
	}

	result, err := e.runtime.CallHandleThreeArgs(
		"compute_action_space",
		e.handle,
		string(factsJSON),
		string(statesJSON),
		persona,
	)
	if err != nil {
		return nil, fmt.Errorf("compute_action_space WASM call failed: %w", err)
	}

	if errMsg := extractError(result); errMsg != "" {
		return nil, fmt.Errorf("action space error: %s", errMsg)
	}

	var actionSpace ActionSpace
	if err := json.Unmarshal([]byte(result), &actionSpace); err != nil {
		return nil, fmt.Errorf("failed to parse ActionSpace: %w", err)
	}

	return &actionSpace, nil
}

// ExecuteFlow simulates a flow execution, returning the outcome, path,
// entity state changes (would_transition), and current verdicts.
//
// entityStates uses the single-instance flat format. For multi-instance
// contracts with explicit instance bindings, use ExecuteFlowWithBindings.
func (e *Evaluator) ExecuteFlow(
	flowID string,
	facts FactSet,
	entityStates EntityStateMap,
	persona string,
) (*FlowResult, error) {
	factsJSON, err := json.Marshal(facts)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal facts: %w", err)
	}

	statesJSON, err := json.Marshal(entityStates)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal entity states: %w", err)
	}

	// simulate_flow(handle, flow_id_ptr, flow_id_len, persona_ptr, persona_len,
	//               facts_ptr, facts_len, states_ptr, states_len)
	result, err := e.runtime.CallHandleFourArgs(
		"simulate_flow",
		e.handle,
		flowID,
		persona,
		string(factsJSON),
		string(statesJSON),
	)
	if err != nil {
		return nil, fmt.Errorf("simulate_flow WASM call failed: %w", err)
	}

	if errMsg := extractError(result); errMsg != "" {
		return nil, fmt.Errorf("flow execution error: %s", errMsg)
	}

	var flowResult FlowResult
	if err := json.Unmarshal([]byte(result), &flowResult); err != nil {
		return nil, fmt.Errorf("failed to parse FlowResult: %w", err)
	}

	return &flowResult, nil
}

// ExecuteFlowWithBindings simulates a flow with explicit instance bindings,
// supporting multi-instance entity contracts.
//
// bindings maps entity IDs to the specific instance ID to use for that
// entity in the flow execution.
func (e *Evaluator) ExecuteFlowWithBindings(
	flowID string,
	facts FactSet,
	entityStates EntityStateMapNested,
	persona string,
	bindings InstanceBindings,
) (*FlowResult, error) {
	factsJSON, err := json.Marshal(facts)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal facts: %w", err)
	}

	statesJSON, err := json.Marshal(entityStates)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal entity states: %w", err)
	}

	bindingsJSON, err := json.Marshal(bindings)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal instance bindings: %w", err)
	}

	// simulate_flow_with_bindings(handle,
	//   flow_id_ptr, flow_id_len,
	//   persona_ptr, persona_len,
	//   facts_ptr, facts_len,
	//   states_ptr, states_len,
	//   bindings_ptr, bindings_len)
	result, err := e.runtime.CallHandleFiveArgs(
		"simulate_flow_with_bindings",
		e.handle,
		flowID,
		persona,
		string(factsJSON),
		string(statesJSON),
		string(bindingsJSON),
	)
	if err != nil {
		return nil, fmt.Errorf("simulate_flow_with_bindings WASM call failed: %w", err)
	}

	if errMsg := extractError(result); errMsg != "" {
		return nil, fmt.Errorf("flow execution error: %s", errMsg)
	}

	var flowResult FlowResult
	if err := json.Unmarshal([]byte(result), &flowResult); err != nil {
		return nil, fmt.Errorf("failed to parse FlowResult: %w", err)
	}

	return &flowResult, nil
}

// Close releases all resources held by the Evaluator, including the WASM runtime.
// It should be called via defer after creating an Evaluator.
func (e *Evaluator) Close() error {
	return e.runtime.Close()
}

// extractError checks if the JSON response contains an "error" field.
// Returns the error string if present, or empty string if not.
func extractError(result string) string {
	var errResp struct {
		Error *string `json:"error"`
	}
	if err := json.Unmarshal([]byte(result), &errResp); err == nil && errResp.Error != nil {
		return *errResp.Error
	}
	return ""
}
