//! Rule validation functions.

use crate::ast::*;
use crate::error::ElabError;
use crate::pass2_index::Index;
use std::collections::HashSet;

// ── Rule validation ───────────────────────────────────────────────────────────

pub(super) fn validate_rule(
    id: &str,
    stratum: i64,
    stratum_line: u32,
    when: &RawExpr,
    prov: &Provenance,
    index: &Index,
    produced_verdicts: &HashSet<String>,
) -> Result<(), ElabError> {
    if stratum < 0 {
        return Err(ElabError::new(
            5,
            Some("Rule"),
            Some(id),
            Some("stratum"),
            &prov.file,
            stratum_line,
            format!("stratum must be a non-negative integer; got {}", stratum),
        ));
    }

    validate_verdict_refs_in_expr(when, id, stratum, prov, index, produced_verdicts)?;

    Ok(())
}

fn validate_verdict_refs_in_expr(
    expr: &RawExpr,
    rule_id: &str,
    rule_stratum: i64,
    prov: &Provenance,
    index: &Index,
    produced_verdicts: &HashSet<String>,
) -> Result<(), ElabError> {
    match expr {
        RawExpr::VerdictPresent { id: vid, line } => {
            if !produced_verdicts.contains(vid.as_str()) {
                return Err(ElabError::new(
                    5, Some("Rule"), Some(rule_id),
                    Some("body.when"),
                    &prov.file, *line,
                    format!("unresolved VerdictType reference: '{}' is not produced by any rule in this contract", vid),
                ));
            }
            if let Some((producing_rule_id, producing_stratum)) = find_producing_rule(vid, index) {
                if producing_stratum >= rule_stratum {
                    return Err(ElabError::new(
                        5, Some("Rule"), Some(rule_id),
                        Some("body.when"),
                        &prov.file, *line,
                        format!(
                            "stratum violation: rule '{}' at stratum {} references verdict '{}' produced by rule '{}' at stratum {}; verdict_refs must reference strata strictly less than the referencing rule's stratum",
                            rule_id, rule_stratum, vid, producing_rule_id, producing_stratum
                        ),
                    ));
                }
            }
        }
        RawExpr::And(a, b) | RawExpr::Or(a, b) => {
            validate_verdict_refs_in_expr(
                a,
                rule_id,
                rule_stratum,
                prov,
                index,
                produced_verdicts,
            )?;
            validate_verdict_refs_in_expr(
                b,
                rule_id,
                rule_stratum,
                prov,
                index,
                produced_verdicts,
            )?;
        }
        RawExpr::Not(e) => {
            validate_verdict_refs_in_expr(
                e,
                rule_id,
                rule_stratum,
                prov,
                index,
                produced_verdicts,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn find_producing_rule(verdict_type: &str, index: &Index) -> Option<(String, i64)> {
    index.verdict_strata.get(verdict_type).cloned()
}
