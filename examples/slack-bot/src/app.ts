/**
 * Tenor Slack Bot -- contract interaction via slash commands.
 *
 * Uses Slack Bolt in Socket Mode for local development (no public URL needed).
 *
 * Environment variables:
 *   SLACK_BOT_TOKEN      Bot User OAuth Token (xoxb-...)
 *   SLACK_SIGNING_SECRET App Signing Secret
 *   SLACK_APP_TOKEN      App-Level Token with connections:write scope (xapp-...)
 *   TENOR_URL            URL of running tenor serve (default: http://localhost:8080)
 */

import { App } from '@slack/bolt';
import { TenorClient } from '../../../sdk/typescript/src/index.ts';
import { registerHandlers } from './handlers.ts';

const TENOR_URL = process.env.TENOR_URL ?? 'http://localhost:8080';

// Validate required environment variables
const requiredEnv = ['SLACK_BOT_TOKEN', 'SLACK_SIGNING_SECRET', 'SLACK_APP_TOKEN'];
const missing = requiredEnv.filter((key) => !process.env[key]);
if (missing.length > 0) {
  console.error(`Missing required environment variables: ${missing.join(', ')}`);
  console.error();
  console.error('Set these before starting the bot:');
  console.error('  SLACK_BOT_TOKEN      Bot User OAuth Token (xoxb-...)');
  console.error('  SLACK_SIGNING_SECRET App Signing Secret');
  console.error('  SLACK_APP_TOKEN      App-Level Token with connections:write (xapp-...)');
  console.error();
  console.error('See README.md for setup instructions.');
  process.exit(1);
}

// Create Bolt app in Socket Mode
const app = new App({
  token: process.env.SLACK_BOT_TOKEN,
  signingSecret: process.env.SLACK_SIGNING_SECRET,
  socketMode: true,
  appToken: process.env.SLACK_APP_TOKEN,
});

// Create TenorClient
const tenorClient = new TenorClient({ baseUrl: TENOR_URL });

// Register all handlers
registerHandlers(app, tenorClient);

// Start the app
(async () => {
  await app.start();
  console.log();
  console.log('  Tenor Slack Bot');
  console.log(`  Tenor URL: ${TENOR_URL}`);
  console.log('  Bot is running in Socket Mode');
  console.log();
  console.log('  Commands:');
  console.log('    /tenor-list              List loaded contracts');
  console.log('    /tenor-eval <contract>   Evaluate a contract (opens modal)');
  console.log('    /tenor-explain <contract> Explain a contract');
  console.log();
})();
