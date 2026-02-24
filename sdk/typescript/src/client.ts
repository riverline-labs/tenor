import type {
  HealthResponse,
  ContractSummary,
  ContractsResponse,
  OperationInfo,
  OperationsResponse,
  EvalResult,
  FlowEvalResult,
  EvaluateOptions,
  ExplainResult,
  InterchangeBundle,
} from './types.ts';

import {
  TenorError,
  ConnectionError,
  ContractNotFoundError,
  EvaluationError,
  ElaborationError,
} from './errors.ts';

/** Configuration options for the TenorClient. */
export interface TenorClientOptions {
  /** Base URL of the tenor serve instance. Default: http://localhost:8080 */
  baseUrl?: string;
  /** Request timeout in milliseconds. Default: 30000 */
  timeout?: number;
}

/**
 * TypeScript client for the Tenor evaluator HTTP API.
 *
 * Connects to a running `tenor serve` instance and provides typed methods
 * for all agent skills: listing contracts, querying operations, invoking
 * evaluation, and getting explanations.
 *
 * Zero runtime dependencies -- uses Node 22+ built-in `fetch`.
 */
export class TenorClient {
  private readonly baseUrl: string;
  private readonly timeout: number;

  constructor(options?: TenorClientOptions) {
    this.baseUrl = (options?.baseUrl ?? 'http://localhost:8080').replace(/\/$/, '');
    this.timeout = options?.timeout ?? 30000;
  }

  /** Check if the evaluator is reachable. */
  async health(): Promise<HealthResponse> {
    return this.request<HealthResponse>('GET', '/health');
  }

  /** List all loaded contracts. */
  async listContracts(): Promise<ContractSummary[]> {
    const response = await this.request<ContractsResponse>('GET', '/contracts');
    return response.contracts;
  }

  /**
   * Get operations available in a contract.
   * Agent skill: getOperations
   */
  async getOperations(contractId: string): Promise<OperationInfo[]> {
    const response = await this.request<OperationsResponse>(
      'GET',
      `/contracts/${encodeURIComponent(contractId)}/operations`,
    );
    return response.operations;
  }

  /**
   * Evaluate a contract against facts.
   * Agent skill: invoke
   *
   * If `options.flow_id` is provided, executes flow evaluation and returns
   * a FlowEvalResult. Otherwise performs rule-only evaluation and returns
   * an EvalResult.
   */
  async invoke(
    contractId: string,
    facts: Record<string, unknown>,
    options?: EvaluateOptions,
  ): Promise<EvalResult | FlowEvalResult> {
    return this.request<EvalResult | FlowEvalResult>('POST', '/evaluate', {
      bundle_id: contractId,
      facts,
      flow_id: options?.flow_id ?? null,
      persona: options?.persona ?? null,
    });
  }

  /**
   * Explain a contract in human-readable form.
   * Agent skill: explain
   */
  async explain(contractId: string): Promise<ExplainResult> {
    return this.request<ExplainResult>('POST', '/explain', {
      bundle_id: contractId,
    });
  }

  /** Elaborate .tenor source text into interchange JSON. */
  async elaborate(source: string, filename?: string): Promise<InterchangeBundle> {
    return this.request<InterchangeBundle>('POST', '/elaborate', {
      source,
      filename: filename ?? 'input.tenor',
    });
  }

  /**
   * Internal HTTP request helper.
   * Handles timeout, error classification, and JSON parsing.
   */
  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const init: RequestInit = {
      method,
      headers: { 'Content-Type': 'application/json' },
      signal: AbortSignal.timeout(this.timeout),
    };
    if (body !== undefined) {
      init.body = JSON.stringify(body);
    }

    let response: Response;
    try {
      response = await fetch(url, init);
    } catch (err: unknown) {
      if (err instanceof Error && err.name === 'TimeoutError') {
        throw new ConnectionError(url, err);
      }
      throw new ConnectionError(url, err instanceof Error ? err : undefined);
    }

    if (!response.ok) {
      const errorBody = await response.json().catch(() => ({
        error: response.statusText,
      })) as { error?: string; details?: Record<string, unknown> };

      if (response.status === 404) {
        // Extract contract ID from path (GET /contracts/{id}/operations)
        const pathMatch = path.match(/\/contracts\/([^/]+)/);
        if (pathMatch) {
          throw new ContractNotFoundError(decodeURIComponent(pathMatch[1]));
        }
        // Extract contract ID from error message (POST /evaluate, /explain)
        const msgMatch = errorBody.error?.match(/contract '([^']+)' not found/);
        if (msgMatch) {
          throw new ContractNotFoundError(msgMatch[1]);
        }
      }

      if (path.includes('/evaluate')) {
        throw new EvaluationError(
          errorBody.error ?? 'Evaluation failed',
          errorBody.details,
        );
      }

      if (path.includes('/elaborate')) {
        throw new ElaborationError(
          errorBody.error ?? 'Elaboration failed',
          errorBody.details,
        );
      }

      throw new TenorError(errorBody.error ?? `HTTP ${response.status}`);
    }

    return response.json() as Promise<T>;
  }
}
