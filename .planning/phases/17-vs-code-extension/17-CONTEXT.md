# Phase 17: VS Code Extension - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

VS Code extension for Tenor contract authoring: syntax highlighting with semantic tokens, LSP-powered diagnostics with check-on-save, and a rich Agent Capabilities panel that shows what an AI would see when reading the contract plus analysis insights. Does not include code actions/quick fixes (future phase).

</domain>

<decisions>
## Implementation Decisions

### Syntax highlighting
- TextMate grammar for base highlighting (keywords, strings, comments, construct blocks)
- LSP semantic tokens on top for construct-aware coloring (fact refs vs entity refs vs type refs colored distinctly)
- Map to standard TextMate scopes — blend into user's existing theme, no custom palette override
- Construct + field-level semantic token granularity: highlight field names within entities, parameters within operations, transition targets within flows
- Bracket matching for { } and code folding regions for each construct block

### LSP diagnostics
- Default to check-on-save, with user-configurable setting to enable check-on-type (debounced)
- Stop at first failing pass — don't show cascading errors from downstream passes
- Full project awareness: resolve imports, show cross-file errors, workspace scanning
- No code actions or quick fixes in this phase — diagnostics only

### Agent Capabilities panel
- Webview panel with rich HTML/CSS rendering, not a tree view
- Shows agent's operational view: available operations grouped by persona, required parameters with types, entity states and transitions, preconditions/postconditions
- Includes analysis insights from tenor-analyze: coverage gaps (operations no persona can call), unreachable states, dead-end flows
- Entity state machines rendered as visual SVG diagrams, not text listings
- Updates on save (matches diagnostics trigger) plus manual refresh button
- Refresh button lets authors read the panel without surprise redraws

### Extension UX
- Full command palette set: Open Agent Capabilities Panel, Elaborate File, Validate Project, New Tenor File (from template), Show Elaboration Output (JSON), Run Conformance Tests, Open Docs
- Snippets for scaffolding new constructs (entity, rule, operation, flow skeletons)
- LSP completions for context-aware fills: field names, construct references, type references
- Status bar item showing elaboration status: checkmark when valid, X with error count on errors, spinner during elaboration
- Full navigation support: go-to-definition on construct references, hover info with type and construct summary, find all references

### Claude's Discretion
- Specific SVG diagram layout algorithm for state machines
- Webview panel visual design and CSS styling
- Snippet template content details
- Debounce timing for on-type diagnostics
- Exact semantic token type/modifier mapping to TextMate scopes

</decisions>

<specifics>
## Specific Ideas

- "Show Elaboration Output (JSON)" is particularly valuable for debugging and understanding what the agent SDK actually consumes
- "New Tenor File from template" lowers the barrier for first-time authors
- Status bar should let authors know immediately on save if something broke, without switching focus to the diagnostics panel
- Panel should make authors feel safe before deployment — see not just what the contract says but what might be wrong with it
- Entity state machine diagrams should let authors glance and immediately spot missing transitions or unreachable states

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 17-vs-code-extension*
*Context gathered: 2026-02-22*
