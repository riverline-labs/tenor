/**
 * CLI entry point for the Tenor Audit Agent.
 *
 * Parses command-line arguments, loads facts from a JSON file, runs the
 * audit analysis, and outputs a formatted compliance report.
 *
 * Usage:
 *   node --experimental-strip-types src/cli.ts \
 *     --contract saas_subscription \
 *     --facts sample-facts/saas.json \
 *     --format terminal \
 *     --output audit-report.md
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { runAudit } from './auditor.ts';
import { formatTerminal, formatMarkdown } from './report.ts';

// ── Argument parsing ───────────────────────────────────────────────────

interface CliArgs {
  contract: string;
  facts: string;
  format: 'terminal' | 'markdown';
  output?: string;
  url: string;
}

function printUsage(): void {
  console.log(`
  Tenor Audit Agent -- compliance report from contract provenance chains

  Usage:
    node --experimental-strip-types src/cli.ts [options]

  Required:
    --contract <id>          Contract ID to audit
    --facts <path>           Path to JSON file with fact values

  Options:
    --format <format>        Output format: terminal (default) or markdown
    --output <path>          Write report to file instead of stdout
    --url <url>              Tenor evaluator URL (default: http://localhost:8080)
    --help                   Show this help message

  Examples:
    # Terminal output with SaaS sample facts
    npm run audit -- --contract saas_subscription --facts sample-facts/saas.json

    # Markdown report to file
    npm run audit -- --contract saas_subscription --facts sample-facts/saas.json \\
      --format markdown --output audit-report.md
`);
}

function parseArgs(argv: string[]): CliArgs | null {
  const args = argv.slice(2);

  if (args.includes('--help') || args.includes('-h')) {
    printUsage();
    return null;
  }

  let contract = '';
  let facts = '';
  let format: 'terminal' | 'markdown' = 'terminal';
  let output: string | undefined;
  let url = 'http://localhost:8080';

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case '--contract':
        contract = args[++i] ?? '';
        break;
      case '--facts':
        facts = args[++i] ?? '';
        break;
      case '--format':
        {
          const fmt = args[++i];
          if (fmt !== 'terminal' && fmt !== 'markdown') {
            console.error(`Invalid format: ${fmt}. Use 'terminal' or 'markdown'.`);
            process.exit(1);
          }
          format = fmt;
        }
        break;
      case '--output':
        output = args[++i];
        break;
      case '--url':
        url = args[++i] ?? url;
        break;
      default:
        console.error(`Unknown argument: ${args[i]}`);
        printUsage();
        process.exit(1);
    }
  }

  if (!contract) {
    console.error('Error: --contract is required');
    printUsage();
    process.exit(1);
  }
  if (!facts) {
    console.error('Error: --facts is required');
    printUsage();
    process.exit(1);
  }

  return { contract, facts, format, output, url };
}

// ── Main ───────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  const cliArgs = parseArgs(process.argv);
  if (!cliArgs) {
    process.exit(0);
  }

  // Load facts from JSON file
  const factsPath = resolve(cliArgs.facts);
  let factsData: Record<string, unknown>;
  try {
    const raw = readFileSync(factsPath, 'utf-8');
    factsData = JSON.parse(raw) as Record<string, unknown>;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`Failed to read facts file: ${factsPath}`);
    console.error(`  ${msg}`);
    process.exit(1);
  }

  // Run audit
  console.error(`Auditing contract '${cliArgs.contract}' against ${Object.keys(factsData).length} facts...`);
  console.error(`Evaluator: ${cliArgs.url}`);
  console.error('');

  try {
    const report = await runAudit(cliArgs.url, cliArgs.contract, factsData);

    // Format output
    const formatted =
      cliArgs.format === 'markdown' ? formatMarkdown(report) : formatTerminal(report);

    // Write output
    if (cliArgs.output) {
      const outputPath = resolve(cliArgs.output);
      writeFileSync(outputPath, formatted, 'utf-8');
      console.error(`Report written to: ${outputPath}`);
    } else {
      console.log(formatted);
    }

    // Exit with code 1 if critical gaps found
    if (report.summary.gapCount.critical > 0) {
      process.exit(1);
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`Audit failed: ${msg}`);
    process.exit(1);
  }
}

main();
