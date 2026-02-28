# Pass 3 Negative Tests

Pass 3 is responsible for resolving TypeDecl definitions and detecting cycles
in the TypeDecl reference graph via DFS. §12.2 Pass 3; §4.5.

SC-4 (named type declaration syntax) is now resolved — §4.5 TypeDecl has been
added to tenor-language-specification.md. The tests below exercise Pass 3 cycle detection.

## Tests

| File | Construct | Violation |
|------|-----------|-----------|
| `typedecl_self_ref.tenor` | TypeDecl | Record TypeDecl with a field of its own type (direct cycle of length 1) |
| `typedecl_mutual_cycle.tenor` | TypeDecl | Two TypeDecls referencing each other (cycle of length 2) |

## Notes

- Pass 3 also builds the complete type environment. Valid type-environment
  construction is exercised by the positive tests (fact_basic, rule_basic,
  typedecl_basic, integration_escrow).
- Only Record and TaggedUnion may be named (§4.5). Scalar types cannot form
  cycles by construction and have no TypeDecl syntax.
