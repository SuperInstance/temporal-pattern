//! Existence pattern: "P must occur at least once".
//!
//! Optionally bounded: P must occur within `k` steps.

use serde::{Deserialize, Serialize};

use crate::{Scope, Trace};

/// "P must occur at least once" (optionally within `bound` steps).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExistencePattern {
    /// Proposition that must appear.
    pub proposition: String,
    /// If `Some(k)`, the proposition must appear within `k` steps of the
    /// scope start.
    pub bound: Option<usize>,
    /// Scope restricting the checked region.
    pub scope: Scope,
}

impl ExistencePattern {
    /// Unbounded existence: P must occur at least once.
    pub fn new(proposition: impl Into<String>) -> Self {
        Self {
            proposition: proposition.into(),
            bound: None,
            scope: Scope::Global,
        }
    }

    /// Bounded existence: P must occur within `k` steps.
    pub fn within(proposition: impl Into<String>, k: usize) -> Self {
        Self {
            proposition: proposition.into(),
            bound: Some(k),
            scope: Scope::Global,
        }
    }

    /// Attach a custom scope.
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Check whether `trace` satisfies this existence pattern.
    pub fn check(&self, trace: &Trace) -> bool {
        let Some((start, end)) = self.scope.scope_range(trace) else {
            return false; // no scope → cannot find proposition
        };
        let region = &trace[start..end];

        match self.bound {
            None => region.iter().any(|step| step.contains(&self.proposition)),
            Some(k) => {
                let limit = std::cmp::min(k, region.len());
                region[..limit]
                    .iter()
                    .any(|step| step.contains(&self.proposition))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbounded_exists() {
        let p = ExistencePattern::new("ready");
        let trace: Trace = vec![vec!["idle".into()], vec!["ready".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn unbounded_missing() {
        let p = ExistencePattern::new("ready");
        let trace: Trace = vec![vec!["idle".into()], vec!["idle".into()]];
        assert!(!p.check(&trace));
    }

    #[test]
    fn bounded_within_limit() {
        let p = ExistencePattern::within("ready", 3);
        let trace: Trace = vec![
            vec!["idle".into()],
            vec!["idle".into()],
            vec!["ready".into()],
        ];
        assert!(p.check(&trace));
    }

    #[test]
    fn bounded_outside_limit() {
        let p = ExistencePattern::within("ready", 2);
        let trace: Trace = vec![
            vec!["idle".into()],
            vec!["idle".into()],
            vec!["ready".into()],
        ];
        assert!(!p.check(&trace));
    }

    #[test]
    fn bounded_at_exact_boundary() {
        let p = ExistencePattern::within("ready", 1);
        let trace: Trace = vec![vec!["ready".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn empty_trace_fails() {
        let p = ExistencePattern::new("x");
        assert!(!p.check(&Trace::new()));
    }

    #[test]
    fn scoped_existence() {
        let p =
            ExistencePattern::new("go").with_scope(Scope::Between("open".into(), "close".into()));
        let trace: Trace = vec![vec!["open".into()], vec!["go".into()], vec!["close".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn existence_serde() {
        let p = ExistencePattern::within("x", 5);
        let json = serde_json::to_string(&p).unwrap();
        let back: ExistencePattern = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
