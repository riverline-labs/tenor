/**
 * Vitest global setup file.
 * Configures @testing-library/jest-dom matchers and mocks WASM module.
 */
import "@testing-library/jest-dom";
import { vi } from "vitest";

// ---------------------------------------------------------------------------
// Mock the WASM evaluator module
// WASM does not run in jsdom - provide a mock implementation
// ---------------------------------------------------------------------------

const mockWasmModule = {
  load_contract: vi.fn((_json: string) => {
    return JSON.stringify({ handle: 1 });
  }),
  free_contract: vi.fn((_handle: number) => {}),
  evaluate: vi.fn((_handle: number, _factsJson: string) => {
    return JSON.stringify({ verdicts: [] });
  }),
  simulate_flow: vi.fn(
    (
      _handle: number,
      _flowId: string,
      _personaId: string,
      _factsJson: string,
      _entityStatesJson: string
    ) => {
      return JSON.stringify({
        simulation: true,
        flow_id: _flowId,
        persona: _personaId,
        outcome: "success",
        path: [],
        would_transition: [],
        verdicts: [],
        instance_bindings: {},
      });
    }
  ),
  simulate_flow_with_bindings: vi.fn(
    (
      _handle: number,
      _flowId: string,
      _personaId: string,
      _factsJson: string,
      _entityStatesJson: string,
      _instanceBindingsJson: string
    ) => {
      return JSON.stringify({
        simulation: true,
        flow_id: _flowId,
        persona: _personaId,
        outcome: "success",
        path: [],
        would_transition: [],
        verdicts: [],
        instance_bindings: {},
      });
    }
  ),
  compute_action_space: vi.fn(
    (
      _handle: number,
      _factsJson: string,
      _entityStatesJson: string,
      _personaId: string
    ) => {
      return JSON.stringify({ allowed: [], blocked: [] });
    }
  ),
  inspect_contract: vi.fn((_handle: number) => {
    return JSON.stringify({
      facts: [],
      entities: [],
      rules: [],
      operations: [],
      flows: [],
    });
  }),
  // default init function (no-op in tests)
  default: vi.fn(async () => {}),
};

vi.mock("../wasm/pkg/tenor_eval_wasm", () => mockWasmModule);

// Also mock the evaluator wrapper so tests can control WASM behavior directly
vi.mock("../wasm/evaluator", () => {
  const mockLoadContractResult = { handle: 1 };
  const mockEvaluatorApi = {
    loadContract: vi.fn((_json: string) => mockLoadContractResult),
    freeContract: vi.fn((_handle: number) => {}),
    evaluate: vi.fn((_handle: number, _factsJson: string) => ({
      verdicts: [],
    })),
    simulateFlow: vi.fn(
      (
        _handle: number,
        flowId: string,
        persona: string,
        _factsJson: string,
        _entityStatesJson: string
      ) => ({
        simulation: true,
        flow_id: flowId,
        persona,
        outcome: "success",
        path: [],
        would_transition: [],
        verdicts: [],
        instance_bindings: {},
      })
    ),
    simulateFlowWithBindings: vi.fn(
      (
        _handle: number,
        flowId: string,
        persona: string,
        _factsJson: string,
        _entityStatesJson: string,
        _instanceBindingsJson: string
      ) => ({
        simulation: true,
        flow_id: flowId,
        persona,
        outcome: "success",
        path: [],
        would_transition: [],
        verdicts: [],
        instance_bindings: {},
      })
    ),
    computeActionSpace: vi.fn(
      (
        _handle: number,
        _factsJson: string,
        _entityStatesJson: string,
        _personaId: string
      ) => ({ allowed: [], blocked: [] })
    ),
    inspectContract: vi.fn((_handle: number) => ({
      facts: [],
      entities: [],
      rules: [],
      operations: [],
      flows: [],
    })),
  };

  return {
    evaluatorApi: mockEvaluatorApi,
    initEvaluator: vi.fn(async () => {}),
  };
});
