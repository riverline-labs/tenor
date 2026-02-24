/**
 * Slack slash command and interaction handlers for Tenor contracts.
 *
 * Implements:
 * - /tenor-list    -- List available contracts
 * - /tenor-eval    -- Evaluate a contract (opens modal for fact input)
 * - /tenor-explain -- Explain a contract in plain language
 */

import type { App } from '@slack/bolt';
import {
  TenorClient,
  ContractNotFoundError,
  ConnectionError,
  EvaluationError,
} from '../../../sdk/typescript/src/index.ts';

/**
 * Register all Tenor slash command handlers with the Bolt app.
 */
export function registerHandlers(app: App, tenorClient: TenorClient): void {
  // /tenor-list -- List available contracts
  app.command('/tenor-list', async ({ ack, respond }) => {
    await ack();
    try {
      const contracts = await tenorClient.listContracts();
      if (contracts.length === 0) {
        await respond({
          response_type: 'ephemeral',
          blocks: [
            {
              type: 'section',
              text: {
                type: 'mrkdwn',
                text: ':information_source: No contracts loaded. Start `tenor serve` with contract files.',
              },
            },
          ],
        });
        return;
      }

      const blocks: Record<string, unknown>[] = [
        {
          type: 'header',
          text: { type: 'plain_text', text: 'Loaded Contracts' },
        },
      ];

      for (const contract of contracts) {
        blocks.push({
          type: 'section',
          text: {
            type: 'mrkdwn',
            text: [
              `*${contract.id}*`,
              `Facts: ${contract.facts.length} | Operations: ${contract.operations.length} | Flows: ${contract.flows.length}`,
              contract.operations.length > 0
                ? `Operations: \`${contract.operations.join('`, `')}\``
                : '',
            ]
              .filter(Boolean)
              .join('\n'),
          },
        });
        blocks.push({ type: 'divider' });
      }

      await respond({ response_type: 'ephemeral', blocks });
    } catch (err) {
      await respond({ response_type: 'ephemeral', text: formatError(err) });
    }
  });

  // /tenor-eval <contract_id> -- Open modal for fact input
  app.command('/tenor-eval', async ({ ack, body, client, respond }) => {
    await ack();
    const contractId = body.text?.trim();
    if (!contractId) {
      await respond({
        response_type: 'ephemeral',
        text: 'Usage: `/tenor-eval <contract_id>`\nExample: `/tenor-eval saas_subscription`',
      });
      return;
    }

    try {
      // Validate contract exists and get its facts
      const contracts = await tenorClient.listContracts();
      const contract = contracts.find((c) => c.id === contractId);
      if (!contract) {
        await respond({
          response_type: 'ephemeral',
          text: `:x: Contract \`${contractId}\` not found. Use \`/tenor-list\` to see available contracts.`,
        });
        return;
      }

      // Build modal with input fields for each fact
      const modalBlocks: Record<string, unknown>[] = [
        {
          type: 'section',
          text: {
            type: 'mrkdwn',
            text: `Enter fact values for *${contractId}*. Use JSON syntax for complex values (objects, arrays). Leave blank to skip optional facts.`,
          },
        },
      ];

      // Add a text input for each fact (up to 25, Slack block limit)
      const factsToShow = contract.facts.slice(0, 25);
      for (const factId of factsToShow) {
        modalBlocks.push({
          type: 'input',
          optional: true,
          block_id: `fact_${factId}`,
          element: {
            type: 'plain_text_input',
            action_id: `input_${factId}`,
            placeholder: { type: 'plain_text', text: `Value for ${factId}` },
          },
          label: { type: 'plain_text', text: factId },
        });
      }

      await client.views.open({
        trigger_id: body.trigger_id,
        view: {
          type: 'modal',
          callback_id: 'tenor_eval_submit',
          private_metadata: contractId,
          title: { type: 'plain_text', text: 'Evaluate Contract' },
          submit: { type: 'plain_text', text: 'Evaluate' },
          blocks: modalBlocks,
        },
      });
    } catch (err) {
      await respond({ response_type: 'ephemeral', text: formatError(err) });
    }
  });

  // Handle modal submission for /tenor-eval
  app.view('tenor_eval_submit', async ({ ack, view, respond }) => {
    await ack();
    const contractId = view.private_metadata;
    const values = view.state.values;

    // Extract fact values from modal state
    const facts: Record<string, unknown> = {};
    for (const [blockId, fields] of Object.entries(values)) {
      if (!blockId.startsWith('fact_')) continue;
      const factId = blockId.replace('fact_', '');
      const actionId = `input_${factId}`;
      const rawValue = (fields[actionId] as { value?: string })?.value?.trim();

      if (!rawValue) continue;

      // Parse the value: try JSON first, fall back to bare value
      try {
        facts[factId] = JSON.parse(rawValue);
      } catch {
        // Not valid JSON -- try boolean, number, then string
        if (rawValue === 'true') facts[factId] = true;
        else if (rawValue === 'false') facts[factId] = false;
        else if (/^-?\d+$/.test(rawValue)) facts[factId] = parseInt(rawValue, 10);
        else if (/^-?\d+\.\d+$/.test(rawValue)) facts[factId] = parseFloat(rawValue);
        else facts[factId] = rawValue;
      }
    }

    try {
      const result = await tenorClient.invoke(contractId, facts);
      const resultJson = JSON.stringify(result, null, 2);

      // Truncate if too long for Slack
      const maxLen = 2900;
      const display =
        resultJson.length > maxLen ? resultJson.slice(0, maxLen) + '\n...(truncated)' : resultJson;

      if (typeof respond === 'function') {
        await respond({
          response_type: 'ephemeral',
          blocks: [
            {
              type: 'header',
              text: { type: 'plain_text', text: `Evaluation: ${contractId}` },
            },
            {
              type: 'section',
              text: { type: 'mrkdwn', text: `\`\`\`${display}\`\`\`` },
            },
          ],
        });
      }
    } catch (err) {
      if (typeof respond === 'function') {
        await respond({
          response_type: 'ephemeral',
          text: formatError(err),
        });
      }
    }
  });

  // /tenor-explain <contract_id> -- Explain a contract
  app.command('/tenor-explain', async ({ ack, body, respond }) => {
    await ack();
    const contractId = body.text?.trim();
    if (!contractId) {
      await respond({
        response_type: 'ephemeral',
        text: 'Usage: `/tenor-explain <contract_id>`\nExample: `/tenor-explain saas_subscription`',
      });
      return;
    }

    try {
      const explanation = await tenorClient.explain(contractId);

      // Slack uses mrkdwn, not standard markdown. The SDK returns markdown
      // which is close enough for most cases.
      let summary = explanation.summary;

      // Truncate if over Slack block limit (3000 chars)
      if (summary.length > 2900) {
        summary = summary.slice(0, 2900) + '\n\n_(truncated)_';
      }

      await respond({
        response_type: 'ephemeral',
        blocks: [
          {
            type: 'header',
            text: { type: 'plain_text', text: `Contract: ${contractId}` },
          },
          {
            type: 'section',
            text: { type: 'mrkdwn', text: summary },
          },
        ],
      });
    } catch (err) {
      await respond({ response_type: 'ephemeral', text: formatError(err) });
    }
  });
}

/**
 * Format an error for display as a user-friendly Slack message.
 * Never shows raw stack traces.
 */
function formatError(err: unknown): string {
  if (err instanceof ContractNotFoundError) {
    return `:x: Contract not found: ${err.contractId}`;
  }
  if (err instanceof ConnectionError) {
    return ':warning: Cannot reach Tenor evaluator. Is `tenor serve` running?';
  }
  if (err instanceof EvaluationError) {
    return `:x: Evaluation error: ${err.message}`;
  }
  if (err instanceof Error) {
    return `:x: Something went wrong: ${err.message}`;
  }
  return ':x: An unexpected error occurred.';
}
