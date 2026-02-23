/**
 * Tenor Express Middleware
 *
 * Generates REST routes from Tenor contract operations.
 * The contract defines the endpoints, not the developer.
 *
 * Routes:
 * - GET  {prefix}/contracts                         List loaded contracts
 * - GET  {prefix}/contracts/:contractId              Explain a contract
 * - GET  {prefix}/contracts/:contractId/operations   List operations
 * - POST {prefix}/contracts/:contractId/evaluate     Evaluate with facts
 * - POST {prefix}/contracts/:contractId/operations/:opId  Execute operation
 */

import { Router, type Request, type Response } from 'express';
import {
  TenorClient,
  ContractNotFoundError,
  EvaluationError,
  ConnectionError,
} from '../../../sdk/typescript/src/index.ts';

/** Configuration for the Tenor middleware. */
export interface TenorMiddlewareOptions {
  /** URL of the running `tenor serve` instance. */
  tenorUrl: string;
  /** Route prefix. Default: "/tenor" */
  prefix?: string;
}

/**
 * Create an Express Router that exposes Tenor contract operations as REST endpoints.
 *
 * The middleware connects to a running `tenor serve` instance via the TenorClient SDK.
 * Each route maps directly to an SDK agent skill:
 *
 * - listContracts  -> GET  /contracts
 * - explain        -> GET  /contracts/:id
 * - getOperations  -> GET  /contracts/:id/operations
 * - invoke         -> POST /contracts/:id/evaluate
 * - invoke         -> POST /contracts/:id/operations/:opId
 */
export function tenorMiddleware(options: TenorMiddlewareOptions): Router {
  const client = new TenorClient({ baseUrl: options.tenorUrl });
  const router = Router();

  /**
   * GET /contracts
   *
   * Lists all loaded contracts with their facts, operations, and flows.
   * Maps to SDK skill: client.listContracts()
   */
  router.get('/contracts', async (_req: Request, res: Response) => {
    try {
      const contracts = await client.listContracts();
      res.json({ contracts });
    } catch (err) {
      handleError(res, err);
    }
  });

  /**
   * GET /contracts/:contractId
   *
   * Returns a plain-language explanation of the contract.
   * Maps to SDK skill: client.explain(contractId)
   */
  router.get('/contracts/:contractId', async (req: Request, res: Response) => {
    try {
      const explanation = await client.explain(req.params.contractId);
      res.json(explanation);
    } catch (err) {
      handleError(res, err);
    }
  });

  /**
   * GET /contracts/:contractId/operations
   *
   * Lists operations available in a contract with personas and effects.
   * Maps to SDK skill: client.getOperations(contractId)
   */
  router.get('/contracts/:contractId/operations', async (req: Request, res: Response) => {
    try {
      const operations = await client.getOperations(req.params.contractId);
      res.json({ operations });
    } catch (err) {
      handleError(res, err);
    }
  });

  /**
   * POST /contracts/:contractId/evaluate
   *
   * Evaluate a contract against provided facts.
   * Optionally execute a flow by providing flow_id and persona.
   *
   * Request body:
   *   { facts: {...}, flow_id?: string, persona?: string }
   *
   * Maps to SDK skill: client.invoke(contractId, facts, options)
   */
  router.post('/contracts/:contractId/evaluate', async (req: Request, res: Response) => {
    try {
      const { facts, flow_id, persona } = req.body as {
        facts?: Record<string, unknown>;
        flow_id?: string;
        persona?: string;
      };

      if (!facts || typeof facts !== 'object') {
        res.status(400).json({ error: "Missing or invalid 'facts' in request body" });
        return;
      }

      const options = flow_id ? { flow_id, persona } : undefined;
      const result = await client.invoke(req.params.contractId, facts, options);
      res.json(result);
    } catch (err) {
      handleError(res, err);
    }
  });

  /**
   * POST /contracts/:contractId/operations/:opId
   *
   * Execute a specific operation. Validates the operation exists,
   * then evaluates the contract with the appropriate flow context.
   *
   * Request body:
   *   { facts: {...}, persona: string }
   *
   * Maps to SDK skills: client.getOperations() + client.invoke()
   */
  router.post(
    '/contracts/:contractId/operations/:opId',
    async (req: Request, res: Response) => {
      try {
        const { facts, persona } = req.body as {
          facts?: Record<string, unknown>;
          persona?: string;
        };

        if (!facts || typeof facts !== 'object') {
          res.status(400).json({ error: "Missing or invalid 'facts' in request body" });
          return;
        }
        if (!persona) {
          res.status(400).json({ error: "Missing 'persona' in request body" });
          return;
        }

        // Validate the operation exists
        const operations = await client.getOperations(req.params.contractId);
        const op = operations.find((o) => o.id === req.params.opId);
        if (!op) {
          res.status(404).json({
            error: `Operation '${req.params.opId}' not found in contract '${req.params.contractId}'`,
          });
          return;
        }

        // Validate the persona is allowed
        if (!op.allowed_personas.includes(persona)) {
          res.status(403).json({
            error: `Persona '${persona}' is not authorized for operation '${req.params.opId}'`,
            allowed_personas: op.allowed_personas,
          });
          return;
        }

        // Evaluate the contract with the provided facts
        const result = await client.invoke(req.params.contractId, facts);
        res.json({
          operation: req.params.opId,
          persona,
          effects: op.effects,
          result,
        });
      } catch (err) {
        handleError(res, err);
      }
    },
  );

  return router;
}

/**
 * Map SDK error types to appropriate HTTP status codes.
 */
function handleError(res: Response, err: unknown): void {
  if (err instanceof ContractNotFoundError) {
    res.status(404).json({ error: err.message });
  } else if (err instanceof EvaluationError) {
    res.status(422).json({ error: err.message, details: err.details });
  } else if (err instanceof ConnectionError) {
    res.status(502).json({ error: 'Cannot reach Tenor evaluator', url: err.url });
  } else if (err instanceof Error) {
    res.status(500).json({ error: err.message });
  } else {
    res.status(500).json({ error: 'Unknown error' });
  }
}
