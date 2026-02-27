---
phase: 06-advanced-policies
plan: 04
subsystem: documentation
tags: [agent-policies, llm, human-in-the-loop, composite, anthropic, approval]

# Dependency graph
requires:
  - phase: 06-advanced-policies/06-01
    provides: HumanInTheLoopPolicy, StdinApprovalChannel, CallbackApprovalChannel
  - phase: 06-advanced-policies/06-02
    provides: LlmPolicy, LlmClient, AnthropicClient
  - phase: 06-advanced-policies/06-03
    provides: CompositePolicy, ApprovalPredicate, FlowIdPredicate, EntityStatePredicate
provides:
  - Comprehensive docs/policies.md for all three advanced agent policies
  - Usage examples for HumanInTheLoopPolicy (stdin + callback)
  - LLM configuration guide (ANTHROPIC_API_KEY, feature gate, model selection, retry)
  - CompositePolicy usage with FlowIdPredicate and EntityStatePredicate examples
  - Three composition patterns: autonomous, supervised, risk-based threshold
affects: [all future agent integration work, users implementing policy pipelines]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Risk-based routing: LlmPolicy proposes, FlowIdPredicate gates, HumanInTheLoopPolicy approves"
    - "AlwaysApprove predicate enables fully-supervised CompositePolicy wrapping"

key-files:
  created:
    - docs/policies.md
  modified: []

key-decisions:
  - "Document that HumanInTheLoopPolicy delegates timeout enforcement to the ApprovalChannel implementation — the struct stores the Duration for channel use, but choose() does not wrap with tokio::time::timeout"
  - "Use AnthropicClient::from_env() as the primary example; AnthropicClient::new() as secondary"

patterns-established:
  - "Timeout note pattern: document the gap between struct fields and runtime enforcement when they diverge"

requirements-completed:
  - Documentation covers all three advanced policies
  - Usage examples for each policy individually
  - Composition examples showing policies chained together
  - Configuration guide for LLM policy (API key, model)
  - Example scenarios mapping to real use cases

# Metrics
duration: 3min
completed: 2026-02-27
---

# Phase 6 Plan 4: Agent Policies Documentation Summary

**595-line docs/policies.md covering HumanInTheLoopPolicy, LlmPolicy, and CompositePolicy with complete Rust code examples, AnthropicClient configuration guide, and three production-ready composition patterns**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-27T16:27:06Z
- **Completed:** 2026-02-27T16:29:57Z
- **Tasks:** 6 (all in one commit — single-file documentation plan)
- **Files modified:** 1

## Accomplishments
- `docs/policies.md` created with thorough coverage of all three advanced agent policies
- Each policy has purpose, how-it-works, configuration table, and multiple Rust code examples
- LLM section documents `ANTHROPIC_API_KEY`, `anthropic` feature gate, `from_env()` / `new()`, model strings, retry loop behavior, and custom client interface
- Composition patterns section shows three complete real-world scenarios with full `main`-style Rust code
- All types, method signatures, and field names verified against `crates/eval/src/policy.rs` and `crates/eval/Cargo.toml`

## Task Commits

All six tasks committed atomically (single-file documentation plan):

1. **Tasks 1-6: Create docs/policies.md** - `9c9bf98` (docs)

## Files Created/Modified
- `docs/policies.md` — 595 lines; overview, AgentPolicy/AgentSnapshot trait signatures, HumanInTheLoopPolicy, LlmPolicy, CompositePolicy sections, three composition examples, reference quick-table

## Decisions Made
- `HumanInTheLoopPolicy.timeout` field stores the intended duration but does not enforce it via `tokio::time::timeout` in `choose()`. Documented this accurately: the field is available to channel implementations; the policy responds to `ApprovalResult::Timeout` via `timeout_behavior`. Added an explicit note so users don't assume the timeout is automatically enforced.
- Used `AnthropicClient::from_env()` as the primary example since it's the expected production pattern.

## Deviations from Plan

One accuracy correction made during Task 6 verification:

**[Rule 1 - Bug] Corrected timeout enforcement documentation**
- **Found during:** Task 6 (documentation accuracy review)
- **Issue:** Initial documentation implied `HumanInTheLoopPolicy.timeout` was automatically enforced. Code review showed it is not — `choose()` does not call `tokio::time::timeout`. The `timeout` field is available for channel implementations to use.
- **Fix:** Added a clarifying note under the Configuration section explaining the delegation model and advising users to wrap the channel for production timeout enforcement.
- **Files modified:** docs/policies.md
- **Committed in:** 9c9bf98

---

**Total deviations:** 1 accuracy correction
**Impact on plan:** Essential for documentation correctness. No scope creep.

## Issues Encountered
None.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
Phase 6 (Advanced Policies) is complete. All four plans executed:
- 06-01: HumanInTheLoopPolicy implementation
- 06-02: LlmPolicy + AnthropicClient implementation
- 06-03: CompositePolicy + ApprovalPredicate implementations
- 06-04: Documentation (this plan)

Phase 7 can begin whenever scheduled.

---
*Phase: 06-advanced-policies*
*Completed: 2026-02-27*
