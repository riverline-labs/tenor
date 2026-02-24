/**
 * Report formatters for audit output.
 *
 * Produces both terminal-readable (with ANSI colors) and markdown formats
 * from an AuditReport.
 */

import type { AuditReport, ComplianceGap, FactCoverageEntry, VerdictTrace } from './auditor.ts';

// ── ANSI color codes ───────────────────────────────────────────────────

const RESET = '\x1b[0m';
const BOLD = '\x1b[1m';
const DIM = '\x1b[2m';
const RED = '\x1b[31m';
const GREEN = '\x1b[32m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const WHITE = '\x1b[37m';

function severityColor(severity: string): string {
  switch (severity) {
    case 'critical':
      return RED;
    case 'warning':
      return YELLOW;
    case 'info':
      return GREEN;
    default:
      return WHITE;
  }
}

function severityIcon(severity: string): string {
  switch (severity) {
    case 'critical':
      return 'X';
    case 'warning':
      return '!';
    case 'info':
      return 'i';
    default:
      return '?';
  }
}

// ── Terminal formatter ─────────────────────────────────────────────────

/** Format an audit report for terminal output with ANSI colors. */
export function formatTerminal(report: AuditReport): string {
  const lines: string[] = [];

  // Header
  lines.push('');
  lines.push(`${BOLD}${CYAN}=== Compliance Audit Report ===${RESET}`);
  lines.push('');
  lines.push(`  ${DIM}Contract:${RESET}  ${BOLD}${report.contractId}${RESET}`);
  lines.push(`  ${DIM}Timestamp:${RESET} ${report.timestamp}`);
  lines.push(`  ${DIM}Facts:${RESET}     ${report.summary.totalFacts} declared, ${report.factInputs.filter((f) => f.type !== 'not_provided').length} provided`);
  lines.push(`  ${DIM}Verdicts:${RESET}  ${report.summary.totalVerdicts}`);
  lines.push('');

  // Verdict Audit Trail
  lines.push(`${BOLD}${CYAN}--- Verdict Audit Trail ---${RESET}`);
  lines.push('');

  for (const trace of report.verdictTraces) {
    lines.push(formatVerdictTraceTerminal(trace));
  }

  // Fact Coverage Matrix
  lines.push(`${BOLD}${CYAN}--- Fact Coverage Matrix ---${RESET}`);
  lines.push('');
  lines.push(formatFactCoverageTerminal(report.factCoverage));

  // Compliance Gaps
  lines.push(`${BOLD}${CYAN}--- Compliance Gaps ---${RESET}`);
  lines.push('');

  if (report.complianceGaps.length === 0) {
    lines.push(`  ${GREEN}No compliance gaps detected.${RESET}`);
    lines.push('');
  } else {
    for (const gap of report.complianceGaps) {
      lines.push(formatGapTerminal(gap));
    }
  }

  // Summary
  lines.push(`${BOLD}${CYAN}--- Summary ---${RESET}`);
  lines.push('');
  lines.push(`  Facts:     ${report.summary.totalFacts} total, ${report.summary.factsWithCoverage} covered, ${report.summary.factsWithoutCoverage} uncovered`);
  lines.push(`  Verdicts:  ${report.summary.totalVerdicts}`);
  lines.push(`  Gaps:      ${formatGapCountTerminal(report.summary.gapCount)}`);
  lines.push('');

  return lines.join('\n');
}

function formatVerdictTraceTerminal(trace: VerdictTrace): string {
  const lines: string[] = [];
  lines.push(`  ${BOLD}${trace.verdictType}${RESET} = ${formatPayload(trace.payload)}`);
  lines.push(`    ${DIM}Rule:${RESET}    ${trace.rule}`);
  lines.push(`    ${DIM}Stratum:${RESET} ${trace.stratum}`);
  lines.push(`    ${DIM}Facts:${RESET}   ${trace.factsUsed.length > 0 ? trace.factsUsed.join(', ') : '(none)'}`);
  if (trace.verdictsUsed.length > 0) {
    lines.push(`    ${DIM}Depends:${RESET} ${trace.verdictsUsed.join(', ')}`);
  }
  lines.push(`    ${DIM}Chain:${RESET}`);
  for (const step of trace.dependencyChain) {
    lines.push(`      ${DIM}|${RESET} ${step}`);
  }
  lines.push('');
  return lines.join('\n');
}

function formatFactCoverageTerminal(coverage: FactCoverageEntry[]): string {
  const lines: string[] = [];

  // Find max fact name length for alignment
  const maxLen = Math.max(...coverage.map((f) => f.factId.length), 4);

  lines.push(`  ${DIM}${'Fact'.padEnd(maxLen)}  Verdicts             Rules${RESET}`);
  lines.push(`  ${DIM}${''.padEnd(maxLen, '-')}  -------------------  -----${RESET}`);

  for (const entry of coverage) {
    const verdicts = entry.usedByVerdicts.length > 0 ? entry.usedByVerdicts.join(', ') : `${YELLOW}(none)${RESET}`;
    const rules = entry.usedByRules.length > 0 ? entry.usedByRules.join(', ') : '-';
    lines.push(`  ${entry.factId.padEnd(maxLen)}  ${verdicts.padEnd(19)}  ${rules}`);
  }

  lines.push('');
  return lines.join('\n');
}

function formatGapTerminal(gap: ComplianceGap): string {
  const color = severityColor(gap.severity);
  const icon = severityIcon(gap.severity);
  const lines: string[] = [];
  lines.push(`  ${color}[${icon}] ${gap.severity.toUpperCase()}: ${gap.type}${RESET}`);
  lines.push(`      ${gap.description}`);
  lines.push(`      Affected: ${gap.affectedItems.join(', ')}`);
  lines.push('');
  return lines.join('\n');
}

function formatGapCountTerminal(gapCount: { info: number; warning: number; critical: number }): string {
  const parts: string[] = [];
  if (gapCount.critical > 0) parts.push(`${RED}${gapCount.critical} critical${RESET}`);
  if (gapCount.warning > 0) parts.push(`${YELLOW}${gapCount.warning} warning${RESET}`);
  if (gapCount.info > 0) parts.push(`${GREEN}${gapCount.info} info${RESET}`);
  return parts.length > 0 ? parts.join(', ') : `${GREEN}none${RESET}`;
}

function formatPayload(payload: unknown): string {
  if (payload === null || payload === undefined) return 'null';
  if (typeof payload === 'string') return `"${payload}"`;
  if (typeof payload === 'object') return JSON.stringify(payload);
  return String(payload);
}

// ── Markdown formatter ─────────────────────────────────────────────────

/** Format an audit report as a markdown document. */
export function formatMarkdown(report: AuditReport): string {
  const lines: string[] = [];

  // Title
  lines.push('# Compliance Audit Report');
  lines.push('');

  // Metadata table
  lines.push('## Contract Metadata');
  lines.push('');
  lines.push('| Field | Value |');
  lines.push('|-------|-------|');
  lines.push(`| Contract | \`${report.contractId}\` |`);
  lines.push(`| Timestamp | ${report.timestamp} |`);
  lines.push(`| Facts (declared) | ${report.summary.totalFacts} |`);
  lines.push(`| Facts (provided) | ${report.factInputs.filter((f) => f.type !== 'not_provided').length} |`);
  lines.push(`| Verdicts | ${report.summary.totalVerdicts} |`);
  lines.push('');

  // Fact Inputs
  lines.push('## Fact Inputs');
  lines.push('');
  lines.push('| Fact | Value | Type |');
  lines.push('|------|-------|------|');
  for (const f of report.factInputs) {
    const val = f.type === 'not_provided' ? '_(not provided)_' : `\`${JSON.stringify(f.value)}\``;
    lines.push(`| ${f.id} | ${val} | ${f.type} |`);
  }
  lines.push('');

  // Verdict Audit Trail
  lines.push('## Verdict Audit Trail');
  lines.push('');

  for (const trace of report.verdictTraces) {
    lines.push(formatVerdictTraceMarkdown(trace));
  }

  // Fact Coverage Matrix
  lines.push('## Fact Coverage Matrix');
  lines.push('');
  lines.push('| Fact | Verdicts | Rules |');
  lines.push('|------|----------|-------|');

  for (const entry of report.factCoverage) {
    const verdicts = entry.usedByVerdicts.length > 0 ? entry.usedByVerdicts.map((v) => `\`${v}\``).join(', ') : '_(none)_';
    const rules = entry.usedByRules.length > 0 ? entry.usedByRules.map((r) => `\`${r}\``).join(', ') : '-';
    lines.push(`| ${entry.factId} | ${verdicts} | ${rules} |`);
  }
  lines.push('');

  // Compliance Gaps
  lines.push('## Compliance Gaps');
  lines.push('');

  if (report.complianceGaps.length === 0) {
    lines.push('No compliance gaps detected.');
    lines.push('');
  } else {
    for (const gap of report.complianceGaps) {
      lines.push(formatGapMarkdown(gap));
    }
  }

  // Summary
  lines.push('## Summary');
  lines.push('');
  lines.push('| Metric | Value |');
  lines.push('|--------|-------|');
  lines.push(`| Total facts | ${report.summary.totalFacts} |`);
  lines.push(`| Facts with coverage | ${report.summary.factsWithCoverage} |`);
  lines.push(`| Facts without coverage | ${report.summary.factsWithoutCoverage} |`);
  lines.push(`| Total verdicts | ${report.summary.totalVerdicts} |`);
  lines.push(`| Info gaps | ${report.summary.gapCount.info} |`);
  lines.push(`| Warning gaps | ${report.summary.gapCount.warning} |`);
  lines.push(`| Critical gaps | ${report.summary.gapCount.critical} |`);
  lines.push('');

  // Overall assessment
  const { critical, warning } = report.summary.gapCount;
  if (critical > 0) {
    lines.push(`**Assessment:** ${critical} critical gap(s) found. Review required before compliance sign-off.`);
  } else if (warning > 0) {
    lines.push(`**Assessment:** ${warning} warning(s) found. No critical issues, but review recommended.`);
  } else {
    lines.push('**Assessment:** No critical or warning gaps. Evaluation provenance is well-covered.');
  }
  lines.push('');

  return lines.join('\n');
}

function formatVerdictTraceMarkdown(trace: VerdictTrace): string {
  const lines: string[] = [];
  lines.push(`### \`${trace.verdictType}\` = ${formatPayload(trace.payload)}`);
  lines.push('');
  lines.push('| Field | Value |');
  lines.push('|-------|-------|');
  lines.push(`| Rule | \`${trace.rule}\` |`);
  lines.push(`| Stratum | ${trace.stratum} |`);
  lines.push(`| Facts used | ${trace.factsUsed.length > 0 ? trace.factsUsed.map((f) => `\`${f}\``).join(', ') : '_(none)_'} |`);
  if (trace.verdictsUsed.length > 0) {
    lines.push(`| Depends on | ${trace.verdictsUsed.map((v) => `\`${v}\``).join(', ')} |`);
  }
  lines.push('');

  if (trace.dependencyChain.length > 0) {
    lines.push('**Dependency chain:**');
    lines.push('');
    for (const step of trace.dependencyChain) {
      lines.push(`- ${step}`);
    }
    lines.push('');
  }

  return lines.join('\n');
}

function formatGapMarkdown(gap: ComplianceGap): string {
  const badge = gap.severity === 'critical' ? '**CRITICAL**' : gap.severity === 'warning' ? '**WARNING**' : '_INFO_';
  const lines: string[] = [];
  lines.push(`### ${badge}: ${gap.type}`);
  lines.push('');
  lines.push(gap.description);
  lines.push('');
  lines.push(`**Affected:** ${gap.affectedItems.map((i) => `\`${i}\``).join(', ')}`);
  lines.push('');
  return lines.join('\n');
}
