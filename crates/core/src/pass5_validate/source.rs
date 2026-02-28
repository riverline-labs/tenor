//! Source validation functions.

use crate::ast::*;
use crate::error::ElabError;
use crate::pass2_index::Index;
use std::collections::BTreeMap;

// ── Source validation ─────────────────────────────────────────────────────────

pub(super) fn validate_source(
    id: &str,
    protocol: &str,
    fields: &BTreeMap<String, String>,
    prov: &Provenance,
    _index: &Index,
) -> Result<(), ElabError> {
    // C-SRC-01: Unique source identifiers (already checked in Pass 2 index)

    // C-SRC-03: Core protocol required fields
    let required: &[&str] = match protocol {
        "http" => &["base_url"],
        "database" => &["dialect"],
        "graphql" => &["endpoint"],
        "grpc" => &["endpoint"],
        "static" | "manual" => &[],
        tag if tag.starts_with("x_") => {
            // C-SRC-04: Extension protocol tag format
            let re_valid = tag.len() > 2
                && tag[2..].split('.').all(|seg| {
                    !seg.is_empty()
                        && seg.starts_with(|c: char| c.is_ascii_lowercase())
                        && seg
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                })
                && !tag.contains("..")
                && !tag.ends_with('.');
            if !re_valid {
                return Err(ElabError::new(
                    5,
                    Some("Source"),
                    Some(id),
                    Some("protocol"),
                    &prov.file,
                    prov.line,
                    format!("invalid extension protocol tag '{}'", tag),
                ));
            }
            &[] // Extension tags have no required fields
        }
        _ => {
            return Err(ElabError::new(
                5,
                Some("Source"),
                Some(id),
                Some("protocol"),
                &prov.file,
                prov.line,
                format!("unknown protocol tag '{}'", protocol),
            ));
        }
    };

    for &req in required {
        if !fields.contains_key(req) {
            return Err(ElabError::new(
                5,
                Some("Source"),
                Some(id),
                Some("protocol"),
                &prov.file,
                prov.line,
                format!(
                    "source '{}' with protocol '{}' is missing required field '{}'",
                    id, protocol, req
                ),
            ));
        }
    }

    Ok(())
}
