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
use serde_json::Value;
use std::path::Path;

/// Elaborate the given root `.tenor` file and return the interchange bundle,
/// or the first elaboration error encountered.
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
