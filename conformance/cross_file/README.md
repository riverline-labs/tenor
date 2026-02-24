# Cross-file reference tests

Convention: the file containing the `import` declaration is the test root.
The elaborator is invoked with the root file; it resolves imports from that directory.
Expected interchange reflects the fully assembled bundle.

## Tests

| Root file | Imports | Expected output |
|-----------|---------|-----------------|
| `rules.tenor` | `facts.tenor` | `bundle.expected.json` |

## Notes

- Leaf files (no `import` declarations) are secondary inputs identified by the import graph.
- No manifest file is required — the import graph is self-describing.
- The assembled bundle contains constructs from all files in declaration order,
  with provenance blocks recording the originating file for each construct.
- Same convention as multi-file negative tests (see `negative/pass1/README.md`)
  but using descriptive filenames instead of `_a`/`_b` suffixes.
