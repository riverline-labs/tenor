//! Six-pass elaborator: Tenor -> TenorInterchange JSON bundle.
//!
//! This is a thin orchestrator that calls each pass module in order.
//! See CLAUDE.md for the pass overview.

use crate::error::ElabError;
use crate::pass1_bundle;
use crate::pass2_index;
use crate::pass3_types;
use crate::pass4_typecheck;
use crate::pass5_validate;
use crate::pass6_serialize;
use crate::source::SourceProvider;
use serde_json::Value;
use std::path::Path;

/// Elaborate the given root `.tenor` file and return the interchange bundle,
/// or the first elaboration error encountered.
///
/// Uses the default [`FileSystemProvider`](crate::source::FileSystemProvider)
/// for file I/O. For filesystem-independent elaboration (e.g., WASM),
/// use [`elaborate_with_provider`] instead.
pub fn elaborate(root_path: &Path) -> Result<Value, ElabError> {
    // Passes 0+1: parse all files in the import graph
    let (constructs, bundle_id) = pass1_bundle::load_bundle(root_path)?;

    // Pass 2: construct indexing
    let index = pass2_index::build_index(&constructs)?;

    // Pass 3: type environment
    let type_env = pass3_types::build_type_env(&constructs, &index)?;

    // Pass 4: resolve types in all constructs (TypeRef -> concrete BaseType)
    let constructs = pass4_typecheck::resolve_types(constructs, &type_env)?;

    // Pass 4 (continued): expression type-checking
    pass4_typecheck::type_check_rules(&constructs)?;

    // Pass 5: validation
    pass5_validate::validate(&constructs, &index)?;
    pass5_validate::validate_operation_transitions(&constructs, &index)?;

    // Pass 6: serialization
    let bundle = pass6_serialize::serialize(&constructs, &bundle_id);
    Ok(bundle)
}

/// Elaborate the given root `.tenor` file using the provided [`SourceProvider`]
/// for all file I/O, and return the interchange bundle.
///
/// This enables elaboration without filesystem access (e.g., in WASM
/// environments) by using an [`InMemoryProvider`](crate::source::InMemoryProvider).
pub fn elaborate_with_provider(
    root_path: &Path,
    provider: &dyn SourceProvider,
) -> Result<Value, ElabError> {
    // Passes 0+1: parse all files in the import graph
    let (constructs, bundle_id) = pass1_bundle::load_bundle_with_provider(root_path, provider)?;

    // Pass 2: construct indexing
    let index = pass2_index::build_index(&constructs)?;

    // Pass 3: type environment
    let type_env = pass3_types::build_type_env(&constructs, &index)?;

    // Pass 4: resolve types in all constructs (TypeRef -> concrete BaseType)
    let constructs = pass4_typecheck::resolve_types(constructs, &type_env)?;

    // Pass 4 (continued): expression type-checking
    pass4_typecheck::type_check_rules(&constructs)?;

    // Pass 5: validation
    pass5_validate::validate(&constructs, &index)?;
    pass5_validate::validate_operation_transitions(&constructs, &index)?;

    // Pass 6: serialization
    let bundle = pass6_serialize::serialize(&constructs, &bundle_id);
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::InMemoryProvider;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn elaborate_in_memory_simple_fact() {
        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("/contract/main.tenor"),
            "fact is_active {\n  type: Bool\n  source: \"service.active\"\n}".to_string(),
        );
        let provider = InMemoryProvider::new(files);
        let result =
            elaborate_with_provider(std::path::Path::new("/contract/main.tenor"), &provider);
        assert!(result.is_ok(), "elaboration failed: {:?}", result.err());
        let bundle = result.unwrap();
        assert_eq!(bundle["id"], "main");
        let constructs = bundle["constructs"].as_array().unwrap();
        assert_eq!(constructs.len(), 1);
        assert_eq!(constructs[0]["kind"], "Fact");
        assert_eq!(constructs[0]["id"], "is_active");
    }

    #[test]
    fn elaborate_in_memory_missing_file() {
        let provider = InMemoryProvider::new(HashMap::new());
        let result = elaborate_with_provider(std::path::Path::new("/missing.tenor"), &provider);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.pass, 1);
        assert!(err.message.contains("cannot open file"));
    }

    #[test]
    fn elaborate_in_memory_with_import() {
        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("/contract/main.tenor"),
            "import \"types.tenor\"\n\nfact score {\n  type: Int\n  source: \"svc.score\"\n}"
                .to_string(),
        );
        files.insert(
            PathBuf::from("/contract/types.tenor"),
            "fact rate {\n  type: Decimal(precision: 10, scale: 4)\n  source: \"svc.rate\"\n}"
                .to_string(),
        );
        let provider = InMemoryProvider::new(files);
        let result =
            elaborate_with_provider(std::path::Path::new("/contract/main.tenor"), &provider);
        assert!(result.is_ok(), "elaboration failed: {:?}", result.err());
        let bundle = result.unwrap();
        let constructs = bundle["constructs"].as_array().unwrap();
        // Should have both facts: rate (from import) and score (from main)
        assert_eq!(constructs.len(), 2);
    }
}
