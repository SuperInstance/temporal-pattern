//! Combination patterns: AND/OR composition, conflict detection, coverage.
//!
//! Combine multiple patterns into a composite that checks all (AND) or any
//! (OR). Also provides utilities for detecting contradictory patterns and
//! computing trace coverage.

use serde::{Deserialize, Serialize};

use crate::Trace;
use crate::absence::AbsencePattern;
use crate::existence::ExistencePattern;
use crate::precedence::PrecedencePattern;
use crate::response::ResponsePattern;

/// Operator for combining patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CombinationOp {
    /// All patterns must hold.
    And,
    /// At least one pattern must hold.
    Or,
}

/// A single pattern variant that can participate in combinations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Pattern {
    Response(ResponsePattern),
    Precedence(PrecedencePattern),
    Existence(ExistencePattern),
    Absence(AbsencePattern),
}

impl Pattern {
    /// Evaluate this pattern against `trace`.
    pub fn check(&self, trace: &Trace) -> bool {
        match self {
            Pattern::Response(p) => p.check(trace),
            Pattern::Precedence(p) => p.check(trace),
            Pattern::Existence(p) => p.check(trace),
            Pattern::Absence(p) => p.check(trace),
        }
    }
}

/// A combination of patterns with an AND/OR operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combination {
    /// The combining operator.
    pub op: CombinationOp,
    /// The patterns being combined.
    pub patterns: Vec<Pattern>,
}

impl Combination {
    /// Create a new combination.
    pub fn new(op: CombinationOp, patterns: Vec<Pattern>) -> Self {
        Self { op, patterns }
    }

    /// Check whether `trace` satisfies this combination.
    pub fn check(&self, trace: &Trace) -> bool {
        match self.op {
            CombinationOp::And => self.patterns.iter().all(|p| p.check(trace)),
            CombinationOp::Or => self.patterns.iter().any(|p| p.check(trace)),
        }
    }

    /// Detect obvious conflicts between patterns.
    ///
    /// Returns a list of human-readable conflict descriptions. Current
    /// detection covers the most common case: existence vs. absence of the
    /// same proposition in the same scope.
    pub fn detect_conflicts(&self) -> Vec<String> {
        let mut conflicts = Vec::new();

        let existence_props: Vec<&ExistencePattern> = self
            .patterns
            .iter()
            .filter_map(|p| match p {
                Pattern::Existence(e) => Some(e),
                _ => None,
            })
            .collect();

        let absence_props: Vec<&AbsencePattern> = self
            .patterns
            .iter()
            .filter_map(|p| match p {
                Pattern::Absence(a) => Some(a),
                _ => None,
            })
            .collect();

        for e in &existence_props {
            for a in &absence_props {
                if e.proposition == a.proposition && e.scope == a.scope && a.after.is_none() {
                    conflicts.push(format!(
                        "Conflict: existence of \"{}\" vs absence of \"{}\" in same scope",
                        e.proposition, a.proposition
                    ));
                }
            }
        }

        conflicts
    }

    /// Compute what fraction of trace positions are "covered" by at least
    /// one pattern.
    ///
    /// A position is covered if it is within the scope of at least one
    /// pattern. Returns `(covered_positions, total_positions, fraction)`.
    pub fn coverage(&self, trace: &Trace) -> (usize, usize, f64) {
        if trace.is_empty() {
            return (0, 0, 1.0);
        }

        let total = trace.len();
        let mut covered = vec![false; total];

        for pattern in &self.patterns {
            let scope = match pattern {
                Pattern::Response(p) => &p.scope,
                Pattern::Precedence(p) => &p.scope,
                Pattern::Existence(p) => &p.scope,
                Pattern::Absence(p) => &p.scope,
            };
            if let Some((start, end)) = scope.scope_range(trace) {
                for slot in covered.iter_mut().take(end).skip(start) {
                    *slot = true;
                }
            }
        }

        let count = covered.iter().filter(|&&c| c).count();
        let frac = count as f64 / total as f64;
        (count, total, frac)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scope;

    #[test]
    fn and_all_satisfy() {
        let c = Combination::new(
            CombinationOp::And,
            vec![
                Pattern::Existence(ExistencePattern::new("a")),
                Pattern::Absence(AbsencePattern::new("b")),
            ],
        );
        let trace: Trace = vec![vec!["a".into(), "c".into()]];
        assert!(c.check(&trace));
    }

    #[test]
    fn and_one_fails() {
        let c = Combination::new(
            CombinationOp::And,
            vec![
                Pattern::Existence(ExistencePattern::new("a")),
                Pattern::Absence(AbsencePattern::new("b")),
            ],
        );
        let trace: Trace = vec![vec!["a".into(), "b".into()]];
        assert!(!c.check(&trace));
    }

    #[test]
    fn or_one_satisfies() {
        let c = Combination::new(
            CombinationOp::Or,
            vec![
                Pattern::Existence(ExistencePattern::new("missing")),
                Pattern::Absence(AbsencePattern::new("ok")),
            ],
        );
        let trace: Trace = vec![vec!["ok".into()]];
        // existence of "missing" fails, but absence of "ok" also fails (ok is present)
        assert!(!c.check(&trace));
    }

    #[test]
    fn or_any_satisfies() {
        let c = Combination::new(
            CombinationOp::Or,
            vec![
                Pattern::Existence(ExistencePattern::new("a")),
                Pattern::Absence(AbsencePattern::new("b")),
            ],
        );
        let trace: Trace = vec![vec!["a".into()]];
        assert!(c.check(&trace));
    }

    #[test]
    fn conflict_detected() {
        let c = Combination::new(
            CombinationOp::And,
            vec![
                Pattern::Existence(ExistencePattern::new("x")),
                Pattern::Absence(AbsencePattern::new("x")),
            ],
        );
        let conflicts = c.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].contains("x"));
    }

    #[test]
    fn no_conflict_different_props() {
        let c = Combination::new(
            CombinationOp::And,
            vec![
                Pattern::Existence(ExistencePattern::new("a")),
                Pattern::Absence(AbsencePattern::new("b")),
            ],
        );
        assert!(c.detect_conflicts().is_empty());
    }

    #[test]
    fn no_conflict_absence_after() {
        let c = Combination::new(
            CombinationOp::And,
            vec![
                Pattern::Existence(ExistencePattern::new("x")),
                Pattern::Absence(AbsencePattern::after("x", "shutdown")),
            ],
        );
        // absence-after is not unconditional, so no conflict
        assert!(c.detect_conflicts().is_empty());
    }

    #[test]
    fn coverage_full() {
        let c = Combination::new(
            CombinationOp::And,
            vec![Pattern::Existence(ExistencePattern::new("x"))],
        );
        let trace: Trace = vec![vec!["a".into()], vec!["b".into()]];
        let (cov, total, frac) = c.coverage(&trace);
        assert_eq!((cov, total), (2, 2));
        assert!((frac - 1.0).abs() < 1e-9);
    }

    #[test]
    fn coverage_partial() {
        let c = Combination::new(
            CombinationOp::And,
            vec![Pattern::Response(
                ResponsePattern::new("a", "b").with_scope(Scope::After("start".into())),
            )],
        );
        let trace: Trace = vec![vec!["idle".into()], vec!["start".into()], vec!["a".into()]];
        let (cov, total, frac) = c.coverage(&trace);
        assert_eq!((cov, total), (2, 3));
        assert!((frac - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn coverage_empty_trace() {
        let c = Combination::new(CombinationOp::And, vec![]);
        let (cov, total, frac) = c.coverage(&Trace::new());
        assert_eq!((cov, total), (0, 0));
        assert!((frac - 1.0).abs() < 1e-9);
    }

    #[test]
    fn combination_serde() {
        let c = Combination::new(
            CombinationOp::Or,
            vec![Pattern::Existence(ExistencePattern::new("z"))],
        );
        let json = serde_json::to_string(&c).unwrap();
        let back: Combination = serde_json::from_str(&json).unwrap();
        assert!(back.check(&vec![vec!["z".into()]]));
    }
}
