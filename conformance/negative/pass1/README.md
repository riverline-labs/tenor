# Pass 1 Negative Tests

Some Pass 1 tests involve multiple files that must be elaborated together as a bundle.

## Multi-file test convention

Files named `<test>_a.tenor` and `<test>_b.tenor` belong to the same test.
The expected error is in `<test>_a.expected-error.json`.
The elaborator should be invoked with `<test>_a.tenor` as the root input; imports
are resolved relative to this directory.

## Tests

| Test | Files | Error |
|------|-------|-------|
| missing_import | missing_import.tenor | Import of nonexistent file |
| import_cycle | import_cycle_a.tenor, import_cycle_b.tenor | Circular import |
| dup_across_files | dup_across_files_a.tenor, dup_across_files_b.tenor | Same-kind duplicate id across files |
| type_library_import | type_library_import_a.tenor, type_library_import_b.tenor | Type library file contains import declaration (§4.6) |
