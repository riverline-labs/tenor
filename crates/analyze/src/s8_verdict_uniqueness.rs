//! S8 — Verdict Uniqueness confirmation.
//!
//! Verdict uniqueness is already enforced during elaboration (Pass 5).
//! The analyzer confirms this pre-verification rather than re-implementing
//! the check.
//!
//! Spec reference: Section 15, S8.

use serde::Serialize;

/// S8 result: confirmation that verdict uniqueness was pre-verified.
#[derive(Debug, Clone, Serialize)]
pub struct S8Result {
    pub pre_verified: bool,
    pub note: String,
}

/// S8 — Confirm verdict uniqueness is pre-verified by Pass 5.
pub fn confirm_verdict_uniqueness() -> S8Result {
    S8Result {
        pre_verified: true,
        note: "Verdict uniqueness enforced during elaboration (Pass 5). No duplicate verdict types can exist in a valid interchange bundle.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_always_pre_verified() {
        let result = confirm_verdict_uniqueness();
        assert!(result.pre_verified);
        assert!(result.note.contains("Pass 5"));
    }
}
