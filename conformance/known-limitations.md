# Conformance Suite — Known Limitations

Limitations of the current elaborator and conformance test infrastructure
that are intentionally deferred. Each entry references the audit finding
that identified it.

## Cross-contract System constraints — v1.0 design scope

System constraints requiring cross-contract awareness are enforced at the
executor level at runtime. The elaborator processes single contracts by
design. Multi-contract elaboration is planned for a future version.

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
independently. These constraints are validated at the executor level at
runtime and will be testable in the conformance suite once the multi-contract
elaboration pipeline is available.
