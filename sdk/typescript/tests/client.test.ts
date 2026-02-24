import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  TenorClient,
  ConnectionError,
  ContractNotFoundError,
  EvaluationError,
  ElaborationError,
  TenorError,
} from '../src/index.ts';
import type { EvalResult } from '../src/index.ts';

// ──────────────────────────────────────────────
// Unit tests (no server needed)
// ──────────────────────────────────────────────

describe('TenorClient', () => {
  describe('constructor', () => {
    it('uses default base URL', () => {
      const client = new TenorClient();
      // Access private via any to verify default
      assert.equal((client as any).baseUrl, 'http://localhost:8080');
    });

    it('accepts custom base URL', () => {
      const client = new TenorClient({ baseUrl: 'http://example.com:9090' });
      assert.equal((client as any).baseUrl, 'http://example.com:9090');
    });

    it('strips trailing slash from base URL', () => {
      const client = new TenorClient({ baseUrl: 'http://example.com:9090/' });
      assert.equal((client as any).baseUrl, 'http://example.com:9090');
    });

    it('uses default timeout of 30000ms', () => {
      const client = new TenorClient();
      assert.equal((client as any).timeout, 30000);
    });

    it('accepts custom timeout', () => {
      const client = new TenorClient({ timeout: 5000 });
      assert.equal((client as any).timeout, 5000);
    });
  });

  describe('error classes', () => {
    it('TenorError has correct name and message', () => {
      const err = new TenorError('test error');
      assert.equal(err.name, 'TenorError');
      assert.equal(err.message, 'test error');
      assert.ok(err instanceof Error);
    });

    it('ConnectionError includes URL', () => {
      const err = new ConnectionError('http://localhost:9090');
      assert.equal(err.name, 'ConnectionError');
      assert.equal(err.url, 'http://localhost:9090');
      assert.match(err.message, /Failed to connect/);
      assert.ok(err instanceof TenorError);
    });

    it('ConnectionError includes cause', () => {
      const cause = new Error('ECONNREFUSED');
      const err = new ConnectionError('http://localhost:9090', cause);
      assert.equal(err.cause, cause);
    });

    it('EvaluationError includes details', () => {
      const err = new EvaluationError('bad input', { field: 'facts' });
      assert.equal(err.name, 'EvaluationError');
      assert.equal(err.message, 'bad input');
      assert.deepEqual(err.details, { field: 'facts' });
      assert.ok(err instanceof TenorError);
    });

    it('ElaborationError includes details', () => {
      const err = new ElaborationError('parse error', { line: 5 });
      assert.equal(err.name, 'ElaborationError');
      assert.equal(err.message, 'parse error');
      assert.deepEqual(err.details, { line: 5 });
      assert.ok(err instanceof TenorError);
    });

    it('ContractNotFoundError includes contractId', () => {
      const err = new ContractNotFoundError('my_contract');
      assert.equal(err.name, 'ContractNotFoundError');
      assert.equal(err.contractId, 'my_contract');
      assert.match(err.message, /my_contract/);
      assert.ok(err instanceof TenorError);
    });
  });

  describe('connection errors', () => {
    it('throws ConnectionError when server is unreachable', async () => {
      const client = new TenorClient({
        baseUrl: 'http://127.0.0.1:19999',
        timeout: 1000,
      });
      await assert.rejects(
        () => client.health(),
        (err: unknown) => {
          assert.ok(err instanceof ConnectionError);
          assert.match(err.url, /19999/);
          return true;
        },
      );
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
      assert.equal(health.status, 'ok');
      assert.ok(typeof health.tenor_version === 'string');
      assert.ok(health.tenor_version.length > 0);
    });

    it('listContracts() returns an array', async () => {
      const contracts = await client.listContracts();
      assert.ok(Array.isArray(contracts));
      // Server should have at least one pre-loaded contract
      assert.ok(contracts.length > 0, 'expected at least one contract');
      const first = contracts[0];
      assert.ok(typeof first.id === 'string');
      assert.ok(typeof first.construct_count === 'number');
      assert.ok(Array.isArray(first.facts));
      assert.ok(Array.isArray(first.operations));
      assert.ok(Array.isArray(first.flows));
    });

    it('getOperations() returns operations for a known contract', async () => {
      const contracts = await client.listContracts();
      assert.ok(contracts.length > 0, 'need at least one contract');
      const contractId = contracts[0].id;

      const operations = await client.getOperations(contractId);
      assert.ok(Array.isArray(operations));
      // The contract should have operations
      if (operations.length > 0) {
        const op = operations[0];
        assert.ok(typeof op.id === 'string');
        assert.ok(Array.isArray(op.allowed_personas));
        assert.ok(Array.isArray(op.effects));
        assert.ok(typeof op.preconditions_summary === 'string');
      }
    });

    it('getOperations() throws ContractNotFoundError for unknown contract', async () => {
      await assert.rejects(
        () => client.getOperations('nonexistent_contract_xyz'),
        (err: unknown) => {
          assert.ok(err instanceof ContractNotFoundError);
          assert.equal(err.contractId, 'nonexistent_contract_xyz');
          return true;
        },
      );
    });

    it('invoke() evaluates rules against facts', async () => {
      const contracts = await client.listContracts();
      assert.ok(contracts.length > 0, 'need at least one contract');
      const contractId = contracts[0].id;

      // Provide all required facts for the saas_subscription contract
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
      // Rule-only evaluation returns { verdicts: [...] }
      assert.ok('verdicts' in result);
      const evalResult = result as EvalResult;
      assert.ok(Array.isArray(evalResult.verdicts));
    });

    it('invoke() throws ContractNotFoundError for unknown bundle_id', async () => {
      await assert.rejects(
        () => client.invoke('nonexistent_contract_xyz', {}),
        (err: unknown) => {
          assert.ok(err instanceof ContractNotFoundError);
          return true;
        },
      );
    });

    it('explain() returns summary and verbose for a known contract', async () => {
      const contracts = await client.listContracts();
      assert.ok(contracts.length > 0, 'need at least one contract');
      const contractId = contracts[0].id;

      const explanation = await client.explain(contractId);
      assert.ok(typeof explanation.summary === 'string');
      assert.ok(typeof explanation.verbose === 'string');
      assert.ok(explanation.summary.length > 0);
      assert.ok(explanation.verbose.length > 0);
    });

    it('explain() throws ContractNotFoundError for unknown contract', async () => {
      await assert.rejects(
        () => client.explain('nonexistent_contract_xyz'),
        (err: unknown) => {
          assert.ok(err instanceof ContractNotFoundError);
          return true;
        },
      );
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
      assert.ok(typeof bundle === 'object');
      assert.ok('id' in bundle || 'constructs' in bundle);
    });

    it('elaborate() throws ElaborationError for invalid source', async () => {
      const badSource = 'this is not valid tenor syntax @@@ {{{';
      await assert.rejects(
        () => client.elaborate(badSource),
        (err: unknown) => {
          assert.ok(err instanceof ElaborationError);
          return true;
        },
      );
    });
  });
}
