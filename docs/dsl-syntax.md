# Tenor DSL Quick Reference

Canonical surface syntax accepted by the Tenor parser. This is the single source of truth for DSL syntax -- for humans, for LLMs, and for the conversational authoring tool.

For formal semantics, see `docs/tenor-language-specification.md`.

---

## Top-level Constructs

```
import "path.tenor"
persona <id>
source <id> { protocol: <word>  [description: "..."]  [<key>: <value>]... }
type <id> { <field>: <Type>, ... }
fact <id> { type: <Type>  source: <SourceDecl>  [default: <Literal>] }
entity <id> { states: [s, ...]  initial: <s>  transitions: [(from, to), ...] [parent: <id>] }
rule <id> { stratum: <int>  when: <Expr>  [produce: verdict <name> { payload: <Type> = <Term> }] }
operation <id> { allowed_personas: [p, ...]  precondition: <Expr>  [effects: [(E, from, to), ...]]  [outcomes: [o, ...]]  [error_contract: [e, ...]] }
flow <id> { [snapshot: <id>]  [entry: <id>]  steps: { <step_id>: <StepKind> { ... }, ... } }
system <id> { [members: [...]]  [shared_personas: [...]]  [triggers: [...]]  [shared_entities: [...]] }
```

## Expressions

```
<Expr> ::= <Expr> "|" <Expr>                           // OR  (also ∨)
         | <Expr> "&" <Expr>                            // AND (also ∧, also "and")
         | "!" <Expr> | "not" <Expr> | "¬" <Expr>      // NOT
         | verdict_present(<rule_id>)
         | ∀ <var> ∈ <domain> . <Expr>                  // forall
         | ∃ <var> ∈ <domain> . <Expr>                  // exists
         | ( <Expr> )
         | <Term> <cmp> <Term>                          // comparison (REQUIRED)

<cmp>  ::= "=" | "!=" | "<" | "<=" | ">" | ">="
```

**No bare boolean literals as expressions.** `true` alone is not a valid precondition.
Use `some_fact = true` or `verdict_present(some_rule)` instead.

## Types

```
Bool  Date  DateTime
Int  |  Int(min, max)
Decimal(precision, scale)
Text  |  Text(max_length)
Money(currency: "USD")  |  Money("USD")
Duration(unit: "days", min: 0, max: 365)
Enum(["val1", "val2"])
List(element_type: <Type>, max: N)
Record({ field: <Type>, ... })
TaggedUnion { variant: <Type>, ... }
<name>                                    // named type reference
```

## Effects

```
effects: [(Entity, from_state, to_state), ...]                  // single-outcome
effects: [(Entity, from_state, to_state, outcome_label), ...]   // multi-outcome
```

Effects use **tuple syntax** with parentheses and commas. The colon-arrow form (`Entity: from -> to`) is not accepted.

## Flow Steps

```
OperationStep { op: <id>  persona: <id>  outcomes: { <label>: <target>, ... }  on_failure: <handler> }
BranchStep { condition: <Expr>  persona: <id>  if_true: <target>  if_false: <target> }
HandoffStep { from_persona: <id>  to_persona: <id>  next: <id> }
SubFlowStep { flow: <id>  persona: <id>  on_success: <target>  on_failure: <handler> }
ParallelStep { branches: [Branch { id: <id>  entry: <id>  steps: { ... } }, ...]  join: JoinPolicy { ... } }
```

## Step Targets

```
Terminal(<outcome_id>)
<step_id>
```

## Failure Handlers

```
Terminate(outcome: <id>)        // or Terminate(<id>) — the "outcome:" prefix is optional
Compensate(steps: [{ op: <id>  persona: <id>  on_failure: Terminal(<id>) }, ...]  then: Terminal(<id>))
Escalate(to_persona: <id>  next: <step_id>)
```

## Source Declarations (on facts)

```
source: "freetext.string"                    // freetext
source: <source_id> { path: "some/path" }    // structured
```

## Literals

```
true | false | 42 | 3.14 | "string"
Money { amount: "100.00", currency: "USD" }
```

## Error Contract

A list of bare identifiers (not quoted strings, not a map):

```
error_contract: [precondition_failed, persona_rejected]
```

## Outcomes

Optional on operations. Defaults to `["success"]` when omitted.

```
outcomes: [approved, rejected]
```

## Keywords

All Tenor keywords are **lowercase** in `.tenor` source files:

```
fact  entity  rule  operation  flow  type  persona  source  system  import
```
