/**
 * Agent Basics -- demonstrates all three agent skills.
 *
 * Prerequisites:
 *   tenor serve --port 8080 domains/saas/saas_subscription.tenor
 *
 * Run:
 *   npx tsx examples/agent-basics.ts
 *   # or: node --experimental-strip-types examples/agent-basics.ts
 */
import { TenorClient } from '../src/index';

async function main() {
  const client = new TenorClient({ baseUrl: 'http://localhost:8080' });

  // Step 1: Check connectivity
  const health = await client.health();
  console.log(`Connected to Tenor evaluator v${health.tenor_version}\n`);

  // Step 2: Discover available contracts
  const contracts = await client.listContracts();
  console.log(`Available contracts: ${contracts.map(c => c.id).join(', ')}\n`);

  if (contracts.length === 0) {
    console.log('No contracts loaded. Start the server with a .tenor file.');
    return;
  }

  const contractId = contracts[0].id;

  // -----------------------------------------------------------------------
  // Agent skill 1: getOperations -- "What can I do with this contract?"
  // -----------------------------------------------------------------------
  const operations = await client.getOperations(contractId);
  console.log(`Operations for '${contractId}':`);
  for (const op of operations) {
    const personas = op.allowed_personas.join(', ');
    const transitions = op.effects
      .map(e => `${e.entity_id}: ${e.from} -> ${e.to}`)
      .join('; ');
    console.log(`  - ${op.id} (personas: ${personas}) [${transitions}]`);
  }
  console.log();

  // -----------------------------------------------------------------------
  // Agent skill 2: invoke -- "Run the contract with these facts"
  // -----------------------------------------------------------------------
  // Facts match the saas_subscription.tenor contract schema:
  //   current_seat_count: Int, subscription_plan: Enum, plan_features: PlanFeatures,
  //   payment_ok: Bool, account_age_days: Int, cancellation_requested: Bool
  const result = await client.invoke(contractId, {
    current_seat_count: 5,
    subscription_plan: 'professional',
    plan_features: {
      max_seats: 50,
      api_access: true,
      sso_enabled: true,
      custom_branding: false,
    },
    payment_ok: true,
    account_age_days: 365,
    cancellation_requested: false,
  });

  console.log('Evaluation result:');
  if ('verdicts' in result && Array.isArray(result.verdicts)) {
    for (const v of result.verdicts) {
      console.log(`  ${v.type} = ${JSON.stringify(v.payload)}  (rule: ${v.provenance.rule}, stratum: ${v.provenance.stratum})`);
    }
  }
  console.log();

  // -----------------------------------------------------------------------
  // Agent skill 3: explain -- "What does this contract do?"
  // -----------------------------------------------------------------------
  const explanation = await client.explain(contractId);
  console.log('Contract explanation (summary):');
  // Print first few lines of the summary
  const summaryLines = explanation.summary.split('\n').slice(0, 10);
  for (const line of summaryLines) {
    console.log(`  ${line}`);
  }
  console.log('  ...\n');

  console.log('All three agent skills demonstrated successfully.');
}

main().catch(console.error);
