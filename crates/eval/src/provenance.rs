//! Provenance chain construction types for verdict tracing.
//!
//! Each verdict carries provenance recording which rule produced it,
//! at what stratum, and which facts and verdicts were accessed during
//! evaluation.

/// Provenance record for a single verdict instance.
#[derive(Debug, Clone)]
pub struct VerdictProvenance {
    /// The rule id that produced this verdict.
    pub rule_id: String,
    /// The stratum at which the rule was evaluated.
    pub stratum: u32,
    /// Fact ids that were accessed during predicate evaluation.
    pub facts_used: Vec<String>,
    /// Verdict types that were accessed during predicate evaluation.
    pub verdicts_used: Vec<String>,
}

/// Collector that tracks fact and verdict references during
/// predicate evaluation, for building provenance chains.
#[derive(Debug, Clone)]
pub struct ProvenanceCollector {
    pub facts_used: Vec<String>,
    pub verdicts_used: Vec<String>,
}

impl Default for ProvenanceCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl ProvenanceCollector {
    pub fn new() -> Self {
        ProvenanceCollector {
            facts_used: Vec::new(),
            verdicts_used: Vec::new(),
        }
    }

    /// Record a fact reference access.
    pub fn record_fact(&mut self, fact_id: &str) {
        if !self.facts_used.contains(&fact_id.to_string()) {
            self.facts_used.push(fact_id.to_string());
        }
    }

    /// Record a verdict reference access.
    pub fn record_verdict(&mut self, verdict_type: &str) {
        if !self.verdicts_used.contains(&verdict_type.to_string()) {
            self.verdicts_used.push(verdict_type.to_string());
        }
    }

    /// Finalize into a VerdictProvenance.
    pub fn into_provenance(self, rule_id: String, stratum: u32) -> VerdictProvenance {
        VerdictProvenance {
            rule_id,
            stratum,
            facts_used: self.facts_used,
            verdicts_used: self.verdicts_used,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collector_records_facts() {
        let mut c = ProvenanceCollector::new();
        c.record_fact("balance");
        c.record_fact("threshold");
        assert_eq!(c.facts_used, vec!["balance", "threshold"]);
    }

    #[test]
    fn collector_deduplicates_facts() {
        let mut c = ProvenanceCollector::new();
        c.record_fact("balance");
        c.record_fact("balance");
        assert_eq!(c.facts_used, vec!["balance"]);
    }

    #[test]
    fn collector_records_verdicts() {
        let mut c = ProvenanceCollector::new();
        c.record_verdict("delivery_confirmed");
        assert_eq!(c.verdicts_used, vec!["delivery_confirmed"]);
    }

    #[test]
    fn collector_deduplicates_verdicts() {
        let mut c = ProvenanceCollector::new();
        c.record_verdict("active");
        c.record_verdict("active");
        assert_eq!(c.verdicts_used, vec!["active"]);
    }

    #[test]
    fn into_provenance() {
        let mut c = ProvenanceCollector::new();
        c.record_fact("f1");
        c.record_verdict("v1");
        let p = c.into_provenance("rule1".to_string(), 0);
        assert_eq!(p.rule_id, "rule1");
        assert_eq!(p.stratum, 0);
        assert_eq!(p.facts_used, vec!["f1"]);
        assert_eq!(p.verdicts_used, vec!["v1"]);
    }
}
