/**
 * Example Express server using the Tenor middleware.
 *
 * Demonstrates how to mount the middleware and expose contract
 * operations as REST endpoints.
 *
 * Prerequisites:
 *   tenor serve --port 8080 domains/saas/saas_subscription.tenor
 *
 * Usage:
 *   node --experimental-strip-types src/server.ts
 */

import express from 'express';
import { tenorMiddleware } from './middleware.ts';

const PORT = parseInt(process.env.PORT ?? '3000', 10);
const TENOR_URL = process.env.TENOR_URL ?? 'http://localhost:8080';

const app = express();
app.use(express.json());

// Mount Tenor middleware at the default /tenor prefix
const tenor = tenorMiddleware({ tenorUrl: TENOR_URL, prefix: '/tenor' });
app.use('/tenor', tenor);

// Root route: welcome message with endpoint listing
app.get('/', (_req, res) => {
  res.json({
    name: '@tenor-examples/express-middleware',
    description: 'Express middleware that auto-generates REST routes from Tenor contract operations',
    tenor_url: TENOR_URL,
    endpoints: {
      'GET /tenor/contracts': 'List loaded contracts',
      'GET /tenor/contracts/:id': 'Explain a contract',
      'GET /tenor/contracts/:id/operations': 'List operations for a contract',
      'POST /tenor/contracts/:id/evaluate': 'Evaluate contract with facts',
      'POST /tenor/contracts/:id/operations/:opId': 'Execute a specific operation',
    },
  });
});

app.listen(PORT, () => {
  console.log();
  console.log(`  Tenor Express Middleware Example`);
  console.log(`  Server:     http://localhost:${PORT}`);
  console.log(`  Tenor URL:  ${TENOR_URL}`);
  console.log();
  console.log(`  Endpoints:`);
  console.log(`    GET  /                                          Welcome`);
  console.log(`    GET  /tenor/contracts                           List contracts`);
  console.log(`    GET  /tenor/contracts/:id                       Explain contract`);
  console.log(`    GET  /tenor/contracts/:id/operations            List operations`);
  console.log(`    POST /tenor/contracts/:id/evaluate              Evaluate`);
  console.log(`    POST /tenor/contracts/:id/operations/:opId      Execute operation`);
  console.log();
  console.log(`  Prerequisites: tenor serve --port 8080 domains/saas/saas_subscription.tenor`);
  console.log();
});
