import { describe, it, expect } from 'vitest';
import {
  TenorClient,
  ConnectionError,
  ContractNotFoundError,
  EvaluationError,
  ElaborationError,
  TenorError,
} from '../src/index';
import type { EvalResult } from '../src/index';

// ──────────────────────────────────────────────
// Unit tests (no server needed)
// ──────────────────────────────────────────────

describe('TenorClient', () => {
  describe('constructor', () => {
    it('uses default base URL', () => {
      const client = new TenorClient();
      // Access private via any to verify default
      expect((client as any).baseUrl).toBe('http://localhost:8080');
    });

    it('accepts custom base URL', () => {
      const client = new TenorClient({ baseUrl: 'http://example.com:9090' });
      expect((client as any).baseUrl).toBe('http://example.com:9090');
    });

    it('strips trailing slash from base URL', () => {
      const client = new TenorClient({ baseUrl: 'http://example.com:9090/' });
      expect((client as any).baseUrl).toBe('http://example.com:9090');
    });

    it('uses default timeout of 30000ms', () => {
      const client = new TenorClient();
      expect((client as any).timeout).toBe(30000);
    });

    it('accepts custom timeout', () => {
      const client = new TenorClient({ timeout: 5000 });
      expect((client as any).timeout).toBe(5000);
    });
  });

  describe('error classes', () => {
    it('TenorError has correct name and message', () => {
      const err = new TenorError('test error');
      expect(err.name).toBe('TenorError');
      expect(err.message).toBe('test error');
      expect(err).toBeInstanceOf(Error);
    });

    it('ConnectionError includes URL', () => {
      const err = new ConnectionError('http://localhost:9090');
      expect(err.name).toBe('ConnectionError');
      expect(err.url).toBe('http://localhost:9090');
      expect(err.message).toMatch(/Failed to connect/);
      expect(err).toBeInstanceOf(TenorError);
    });

    it('ConnectionError includes cause', () => {
      const cause = new Error('ECONNREFUSED');
      const err = new ConnectionError('http://localhost:9090', cause);
      expect(err.cause).toBe(cause);
    });

    it('EvaluationError includes details', () => {
      const err = new EvaluationError('bad input', { field: 'facts' });
      expect(err.name).toBe('EvaluationError');
      expect(err.message).toBe('bad input');
      expect(err.details).toEqual({ field: 'facts' });
      expect(err).toBeInstanceOf(TenorError);
    });

    it('ElaborationError includes details', () => {
      const err = new ElaborationError('parse error', { line: 5 });
      expect(err.name).toBe('ElaborationError');
      expect(err.message).toBe('parse error');
      expect(err.details).toEqual({ line: 5 });
      expect(err).toBeInstanceOf(TenorError);
    });

    it('ContractNotFoundError includes contractId', () => {
      const err = new ContractNotFoundError('my_contract');
      expect(err.name).toBe('ContractNotFoundError');
      expect(err.contractId).toBe('my_contract');
      expect(err.message).toMatch(/my_contract/);
      expect(err).toBeInstanceOf(TenorError);
    });
  });

  describe('connection errors', () => {
    it('throws ConnectionError when server is unreachable', async () => {
      const client = new TenorClient({
        baseUrl: 'http://127.0.0.1:19999',
        timeout: 1000,
      });
      await expect(client.health()).rejects.toThrow(ConnectionError);
    });
  });
});

// ──────────────────────────────────────────────
// Integration tests (require running server)
// ──────────────────────────────────────────────

const serverUrl = process.env.TENOR_SERVE_URL;

if (!serverUrl) {
  describe('integration (skipped)', () => {
    it('TENOR_SERVE_URL not set -- skipping integration tests', () => {
      // This test always passes; it just logs that integration tests were skipped.
    });
  });
} else {
  describe('integration', () => {
    const client = new TenorClient({ baseUrl: serverUrl });

    it('health() returns valid response', async () => {
      const health = await client.health();
      expect(health.status).toBe('ok');
      expect(typeof health.tenor_version).toBe('string');
      expect(health.tenor_version.length).toBeGreaterThan(0);
    });

    it('listContracts() returns an array', async () => {
      const contracts = await client.listContracts();
      expect(Array.isArray(contracts)).toBe(true);
      expect(contracts.length).toBeGreaterThan(0);
      const first = contracts[0];
      expect(typeof first.id).toBe('string');
      expect(typeof first.construct_count).toBe('number');
      expect(Array.isArray(first.facts)).toBe(true);
      expect(Array.isArray(first.operations)).toBe(true);
      expect(Array.isArray(first.flows)).toBe(true);
    });

    it('getOperations() returns operations for a known contract', async () => {
      const contracts = await client.listContracts();
      expect(contracts.length).toBeGreaterThan(0);
      const contractId = contracts[0].id;

      const operations = await client.getOperations(contractId);
      expect(Array.isArray(operations)).toBe(true);
      if (operations.length > 0) {
        const op = operations[0];
        expect(typeof op.id).toBe('string');
        expect(Array.isArray(op.allowed_personas)).toBe(true);
        expect(Array.isArray(op.effects)).toBe(true);
        expect(typeof op.preconditions_summary).toBe('string');
      }
    });

    it('getOperations() throws ContractNotFoundError for unknown contract', async () => {
      await expect(
        client.getOperations('nonexistent_contract_xyz'),
      ).rejects.toThrow(ContractNotFoundError);
    });

    it('invoke() evaluates rules against facts', async () => {
      const contracts = await client.listContracts();
      expect(contracts.length).toBeGreaterThan(0);
      const contractId = contracts[0].id;

      const facts = {
        current_seat_count: 5,
        subscription_plan: 'professional',
        plan_features: {
          max_seats: 50,
          api_access: true,
          sso_enabled: true,
          custom_branding: false,
        },
        payment_ok: true,
        account_age_days: 60,
        cancellation_requested: false,
      };
      const result = await client.invoke(contractId, facts);
      expect('verdicts' in result).toBe(true);
      const evalResult = result as EvalResult;
      expect(Array.isArray(evalResult.verdicts)).toBe(true);
    });

    it('invoke() throws ContractNotFoundError for unknown bundle_id', async () => {
      await expect(
        client.invoke('nonexistent_contract_xyz', {}),
      ).rejects.toThrow(ContractNotFoundError);
    });

    it('explain() returns summary and verbose for a known contract', async () => {
      const contracts = await client.listContracts();
      expect(contracts.length).toBeGreaterThan(0);
      const contractId = contracts[0].id;

      const explanation = await client.explain(contractId);
      expect(typeof explanation.summary).toBe('string');
      expect(typeof explanation.verbose).toBe('string');
      expect(explanation.summary.length).toBeGreaterThan(0);
      expect(explanation.verbose.length).toBeGreaterThan(0);
    });

    it('explain() throws ContractNotFoundError for unknown contract', async () => {
      await expect(
        client.explain('nonexistent_contract_xyz'),
      ).rejects.toThrow(ContractNotFoundError);
    });

    it('elaborate() processes valid .tenor source', async () => {
      const source = `fact is_active {
  type: Bool
  source: "system.active_status"
  default: true
}

entity Account {
  states: [active, suspended]
  initial: active
  transitions: [(active, suspended)]
}

rule check_active {
  stratum: 0
  when: is_active = true
  produce: verdict allowed { payload: Bool = true }
}
`;
      const bundle = await client.elaborate(source, 'test.tenor');
      expect(typeof bundle).toBe('object');
      expect('id' in bundle || 'constructs' in bundle).toBe(true);
    });

    it('elaborate() throws ElaborationError for invalid source', async () => {
      const badSource = 'this is not valid tenor syntax @@@ {{{';
      await expect(
        client.elaborate(badSource),
      ).rejects.toThrow(ElaborationError);
    });
  });
}
