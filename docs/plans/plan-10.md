# Phase 10: Hosted Platform — Complete Implementation

Multi-tenant hosted Tenor with provisioning, auth, API gateway, and billing. This is the productization phase — turning the executor infrastructure into a hosted service that anyone can sign up for and deploy contracts to.

**Repo:** Private (`~/src/riverline/tenor-platform`). This is the commercial offering.

---

## What "done" means

1. Multi-tenant: multiple organizations, each with isolated contracts, entities, and provenance
2. Auth: API key authentication, organization/user management, persona-to-identity mapping
3. Provisioning: deploy a contract from interchange bundle, get a live executor endpoint
4. API gateway: rate limiting, request routing, API key validation, CORS
5. Billing: usage metering (evaluations, flow executions, storage), plan tiers
6. Admin dashboard: organization management, contract deployment status, usage metrics
7. The existing platform-serve endpoints work unchanged — the hosted layer wraps them

---

## Step 1: Multi-tenancy

### 1A: Tenant model

```
Organization {
  id: UUID
  name: String
  plan: PlanTier
  created_at: DateTime
  api_keys: Vec<ApiKey>
}

ApiKey {
  id: UUID
  org_id: UUID
  key_hash: String     // bcrypt hash, never store plaintext
  name: String         // human label
  permissions: Permissions
  created_at: DateTime
  last_used_at: Option<DateTime>
}

Deployment {
  id: UUID
  org_id: UUID
  contract_id: String
  bundle_etag: String
  status: DeploymentStatus   // provisioning, active, suspended, archived
  created_at: DateTime
  endpoint_url: String       // org-scoped: /{org_id}/{contract_id}/...
}
```

### 1B: Tenant isolation

- **Database:** Schema-per-tenant or row-level security (RLS) with `org_id` on every table. RLS is simpler to operate. Every query must filter by `org_id`.
- **Storage:** Entity states, provenance records, flow executions are all scoped to `(org_id, contract_id)`. No cross-tenant data access.
- **Executor:** Each deployment runs against the same executor process but with tenant-scoped storage queries.

### 1C: Database migrations

Add `org_id` column to all existing tables (entity_states, provenance_records, flow_executions, etc.). Add RLS policies. Migrate existing data to a default organization.

---

## Step 2: Authentication and authorization

### 2A: API key auth

Every API request requires an `Authorization: Bearer <api-key>` header. The gateway:

1. Extracts the API key
2. Hashes and looks up in the api_keys table
3. Resolves the organization
4. Checks permissions
5. Injects `org_id` into the request context
6. Routes to the appropriate tenant-scoped handler

### 2B: Persona-to-identity mapping

The contract declares personas. The hosted platform maps real identities to personas:

```
PersonaMapping {
  org_id: UUID
  contract_id: String
  api_key_id: UUID
  persona_id: String
}
```

When an API key calls an operation, the platform checks which persona(s) the key is mapped to. The key can only invoke operations where its mapped persona is in `allowed_personas`.

### 2C: Management API

```
POST   /api/v1/organizations                  — create org
GET    /api/v1/organizations/{org_id}          — get org
PUT    /api/v1/organizations/{org_id}          — update org

POST   /api/v1/organizations/{org_id}/api-keys              — create API key
GET    /api/v1/organizations/{org_id}/api-keys               — list API keys
DELETE /api/v1/organizations/{org_id}/api-keys/{key_id}      — revoke API key

POST   /api/v1/organizations/{org_id}/deployments            — deploy contract
GET    /api/v1/organizations/{org_id}/deployments             — list deployments
GET    /api/v1/organizations/{org_id}/deployments/{deploy_id} — get deployment
DELETE /api/v1/organizations/{org_id}/deployments/{deploy_id} — archive deployment

POST   /api/v1/organizations/{org_id}/persona-mappings       — map API key to persona
GET    /api/v1/organizations/{org_id}/persona-mappings        — list mappings
DELETE /api/v1/organizations/{org_id}/persona-mappings/{id}   — remove mapping
```

---

## Step 3: Contract deployment (provisioning)

### 3A: Deployment flow

```
POST /api/v1/organizations/{org_id}/deployments
Content-Type: application/json

{
  "contract_id": "escrow_release",
  "bundle": { ... }   // interchange bundle JSON
}
```

The platform:

1. Validates the bundle (schema validation, construct references)
2. Computes etag
3. Creates database tables/partitions for the contract (entity states, provenance)
4. Loads the contract into the executor
5. Returns the live endpoint URL

### 3B: Live endpoints

Once deployed, the contract is accessible at:

```
/{org_id}/{contract_id}/.well-known/tenor
/{org_id}/{contract_id}/actions
/{org_id}/{contract_id}/flows/{flow_id}/execute
/{org_id}/{contract_id}/flows/{flow_id}/simulate
/{org_id}/{contract_id}/evaluate
/{org_id}/{contract_id}/entities/{entity_id}
...
```

These are the same endpoints as platform-serve, scoped by `org_id` and `contract_id`. The existing handler code is reused — the hosted layer adds auth and tenant scoping.

### 3C: Contract updates

```
PUT /api/v1/organizations/{org_id}/deployments/{deploy_id}
Content-Type: application/json

{
  "bundle": { ... }   // new version
  "migration_policy": "blue-green"   // required if breaking changes
}
```

This triggers the migration flow from Phase 1. The platform runs `tenor migrate` internally.

---

## Step 4: API gateway

### 4A: Rate limiting

Per-API-key rate limits based on plan tier:

| Plan       | Evaluations/min | Executions/min | Storage (entities) |
| ---------- | --------------- | -------------- | ------------------ |
| Free       | 100             | 10             | 1,000              |
| Pro        | 10,000          | 1,000          | 100,000            |
| Enterprise | Unlimited       | Unlimited      | Unlimited          |

Rate limiting via token bucket (Redis or in-memory). Return `429 Too Many Requests` with `Retry-After` header.

### 4B: Request routing

The gateway routes requests to the correct executor instance for the target org + contract. For v1, a single executor process handles all tenants (with RLS isolation). The gateway validates auth and injects tenant context.

### 4C: CORS

Configure CORS for browser-based clients (the Automatic UI from Phase 8). Allow configurable origins per organization.

### 4D: Request/response logging

Log every API request (method, path, org_id, contract_id, status code, latency). Do NOT log request/response bodies (may contain sensitive facts).

---

## Step 5: Billing and metering

### 5A: Usage metering

Track per-organization usage:

```
UsageRecord {
  org_id: UUID
  period: Date        // daily
  evaluations: u64
  flow_executions: u64
  simulations: u64
  entity_instances: u64   // peak count
  storage_bytes: u64      // provenance + entity state
}
```

Meter at the gateway level (increment counters per request type).

### 5B: Plan enforcement

When an organization exceeds their plan limits:

- **Free:** Hard limit. Return `402 Payment Required` with upgrade instructions.
- **Pro:** Soft limit with overage billing.
- **Enterprise:** No limits.

### 5C: Billing integration

For v1, metering data is exported to a billing system (Stripe or similar) via a periodic job. The platform does not handle payments directly — it produces usage data that the billing system consumes.

---

## Step 6: Admin dashboard

A web UI for platform operators (not end users — end users get the Automatic UI from Phase 8):

- Organization list: name, plan, deployment count, usage summary
- Organization detail: API keys, deployments, persona mappings, usage graph
- Deployment detail: contract summary, entity counts, execution history, migration history
- System health: executor status, database connections, request rate, error rate
- Usage reports: per-org, per-contract, per-day

The admin dashboard is a separate React app served at `/admin/`.

---

## Step 7: Tests

- Integration test: create org → create API key → deploy contract → execute flow → verify tenant isolation
- Integration test: two orgs deploy same contract → data is isolated
- Integration test: API key without persona mapping → operation rejected
- Integration test: rate limit exceeded → 429 returned
- Integration test: deploy v2 contract with migration → migration runs correctly within tenant scope
- Integration test: archive deployment → endpoint returns 404
- Unit test: tenant scoping on all database queries
- Unit test: API key hash/verify
- Unit test: rate limiter

---

## Final Report

```
## Phase 10: Hosted Platform — COMPLETE

### Multi-tenancy
- Tenant model: Organization, ApiKey, Deployment, PersonaMapping
- Isolation: RLS with org_id on all tables
- Database migrations: org_id added to all existing tables

### Auth
- API key authentication
- Persona-to-identity mapping
- Management API for orgs, keys, deployments, mappings

### Provisioning
- Deploy contract from bundle
- Live endpoints at /{org_id}/{contract_id}/...
- Contract updates with migration support

### Gateway
- Rate limiting per plan tier
- CORS configuration
- Request routing and logging

### Billing
- Usage metering: evaluations, executions, entities, storage
- Plan enforcement (Free, Pro, Enterprise)
- Billing data export

### Admin dashboard
- Organization management
- Deployment monitoring
- Usage reports

### Tests
- Integration: [N] passing
- Tenant isolation verified
- Rate limiting verified

### Commits
- [hash] [message]
- ...
```

Phase 10 is done when the hosted platform supports multi-tenant contract deployment with auth, rate limiting, metering, and an admin dashboard, and every checkbox above is checked. Not before.
