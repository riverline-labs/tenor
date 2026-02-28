# Conformance Suite — Known Limitations

Limitations of the current elaborator and conformance test infrastructure
that are intentionally deferred. Each entry references the audit finding
that identified it.

## TaggedUnion type not implemented

The JSON interchange schema defines `TaggedUnionType`, but the parser and
elaborator do not yet support the `TaggedUnion` base type. A conformance
test cannot be written until the parser is extended. (Round 2 audit,
Tier 5 — CONF-02)

## Decimal default rounding uses literal precision, not declared type precision

When a Decimal Fact has a default literal, the serializer preserves the
literal's precision and scale rather than rounding to the declared type's
precision and scale. For example, `Decimal(10,2)` with default `3.145`
serializes as `{"value":"3.145","precision":4,"scale":3}` instead of
`{"value":"3.15","precision":10,"scale":2}`. This affects round-half-to-even
conformance testing. (Round 2 audit, Tier 5 — CONF-03)

## Flow step outcome map exhaustiveness not checked

Pass 5 does not validate that an OperationStep's `outcomes` map covers all
outcomes declared by the referenced Operation. A step could silently omit
an outcome, which would be unreachable at runtime. (Round 2 audit, Tier 5 —
CONF-04)

## Undeclared persona in flow step not fully validated

Pass 5 checks that a flow step's persona appears in the construct index,
but Operations may reference personas via `allowed_personas` that are not
top-level Persona declarations. Validating this fully would require checking
the union of explicit Persona declarations and Operation `allowed_personas`
lists. (Round 2 audit, Tier 5 — CONF-05)

## Cross-contract System constraints require multi-file elaboration

The following System-level constraints (C-SYS-*) cannot be tested in the
conformance runner because they require elaborating multiple contract files
and cross-referencing their constructs:

- **C-SYS-02**: Shared persona must exist in all listed member contracts
- **C-SYS-06**: Trigger source flow must exist in source contract
- **C-SYS-09**: Trigger target flow must exist in target contract
- **C-SYS-10**: Trigger persona must be valid for target flow
- **C-SYS-12**: Shared entity must exist in all listed member contracts
- **C-SYS-13**: Shared entity state sets must be identical across contracts
- **C-SYS-14**: Shared entity transitions must be compatible across contracts

The single-file conformance runner elaborates each `.tenor` file
independently. These constraints will be testable once the multi-contract
elaboration pipeline is available. (Round 2 audit, Tier 5 — CONF-06/07)
