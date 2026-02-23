# Tenor Express Middleware Example

Express middleware that auto-generates REST routes from Tenor contract operations. The contract defines the API endpoints, not the developer.

## Architecture

```
Express App
  |
  +-- Tenor Middleware (Router)
        |
        +-- TenorClient SDK
              |
              +-- tenor serve (HTTP API)
                    |
                    +-- Contract (.tenor file)
```

The middleware creates a `TenorClient` pointed at a running `tenor serve` instance and exposes routes that map directly to SDK agent skills.

## Prerequisites

- **Rust toolchain** -- for building `tenor` CLI
- **Node.js 22+** -- for `--experimental-strip-types` support
- A running `tenor serve` instance with at least one contract loaded

## Quick Start

1. **Start the Tenor evaluator:**

```bash
# From the repo root
cargo run -p tenor-cli -- serve --port 8080 domains/saas/saas_subscription.tenor
```

2. **Install dependencies:**

```bash
cd examples/express-middleware
npm install
```

3. **Start the Express server:**

```bash
npm start
# or: node --experimental-strip-types src/server.ts
```

4. **Test the endpoints:**

```bash
# List contracts
curl http://localhost:3000/tenor/contracts

# Explain a contract
curl http://localhost:3000/tenor/contracts/saas_subscription

# List operations
curl http://localhost:3000/tenor/contracts/saas_subscription/operations

# Evaluate with facts
curl -X POST http://localhost:3000/tenor/contracts/saas_subscription/evaluate \
  -H "Content-Type: application/json" \
  -d '{
    "facts": {
      "current_seat_count": 5,
      "subscription_plan": "professional",
      "plan_features": {
        "max_seats": 50,
        "api_access": true,
        "sso_enabled": true,
        "custom_branding": false
      },
      "payment_ok": true,
      "account_age_days": 365,
      "cancellation_requested": false
    }
  }'

# Execute a specific operation
curl -X POST http://localhost:3000/tenor/contracts/saas_subscription/operations/activate_subscription \
  -H "Content-Type: application/json" \
  -d '{
    "facts": {
      "current_seat_count": 5,
      "subscription_plan": "professional",
      "plan_features": {
        "max_seats": 50,
        "api_access": true,
        "sso_enabled": true,
        "custom_branding": false
      },
      "payment_ok": true,
      "account_age_days": 365,
      "cancellation_requested": false
    },
    "persona": "account_manager"
  }'
```

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/tenor/contracts` | List all loaded contracts |
| GET | `/tenor/contracts/:id` | Plain-language contract explanation |
| GET | `/tenor/contracts/:id/operations` | Operations with personas and effects |
| POST | `/tenor/contracts/:id/evaluate` | Evaluate contract against facts |
| POST | `/tenor/contracts/:id/operations/:opId` | Execute a specific operation |

## Error Handling

The middleware maps SDK errors to HTTP status codes:

| SDK Error | HTTP Status | Meaning |
|-----------|-------------|---------|
| `ContractNotFoundError` | 404 | Contract not loaded in evaluator |
| `EvaluationError` | 422 | Evaluation failed (missing facts, type errors) |
| `ConnectionError` | 502 | Cannot reach tenor serve |
| Other errors | 500 | Unexpected server error |

## Extending

- **Add authentication:** Wrap the router with your auth middleware before mounting
- **Custom validators:** Add request validation before calling the SDK
- **Webhooks:** Post evaluation results to external services after `invoke`
- **Metrics:** Instrument SDK calls with your APM tool

## Note

This is a reference implementation showing the integration pattern. For production use, add authentication, rate limiting, request validation, and structured logging.
