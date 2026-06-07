//! Precedence pattern: "Q must be preceded by P".
//!
//! Every occurrence of `follower` in the trace (within scope) must have a
//! `predecessor` that occurred at some earlier position. This is the
//! temporal dual of the response pattern.

use serde::{Deserialize, Serialize};

use crate::{Scope, Trace};

/// "Every occurrence of `follower` must be preceded by `predecessor`."
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrecedencePattern {
    /// Proposition that must appear before the follower.
    pub predecessor: String,
    /// Proposition whose occurrences require a prior predecessor.
    pub follower: String,
    /// Scope restricting the checked region.
    pub scope: Scope,
}

impl PrecedencePattern {
    /// Create a globally-scoped precedence pattern.
    pub fn new(predecessor: impl Into<String>, follower: impl Into<String>) -> Self {
        Self {
            predecessor: predecessor.into(),
            follower: follower.into(),
            scope: Scope::Global,
        }
    }

    /// Attach a custom scope.
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Check whether `trace` satisfies this precedence pattern.
    ///
    /// For every position in scope where `follower` holds, there must exist
    /// an earlier position (still in scope) where `predecessor` holds.
    pub fn check(&self, trace: &Trace) -> bool {
        let Some((start, end)) = self.scope.scope_range(trace) else {
            return true;
        };
        let region = &trace[start..end];
        let mut seen_pred = false;

        for step in region {
            if step.contains(&self.predecessor) {
                seen_pred = true;
            }
            if step.contains(&self.follower) && !seen_pred {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satisfied_when_preceded() {
        let p = PrecedencePattern::new("grant", "use");
        let trace: Trace = vec![vec!["grant".into()], vec!["use".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn violated_when_not_preceded() {
        let p = PrecedencePattern::new("grant", "use");
        let trace: Trace = vec![vec!["use".into()], vec!["grant".into()]];
        assert!(!p.check(&trace));
    }

    #[test]
    fn no_follower_vacuously_true() {
        let p = PrecedencePattern::new("grant", "use");
        let trace: Trace = vec![vec!["idle".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn multiple_followers_all_need_predecessor() {
        let p = PrecedencePattern::new("grant", "use");
        let trace: Trace = vec![vec!["grant".into()], vec!["use".into()], vec!["use".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn scoped_precedence() {
        let p = PrecedencePattern::new("grant", "use").with_scope(Scope::After("open".into()));
        let trace: Trace = vec![
            vec!["use".into()], // before scope — ignored
            vec!["open".into()],
            vec!["grant".into()],
            vec!["use".into()],
        ];
        assert!(p.check(&trace));
    }

    #[test]
    fn empty_trace() {
        let p = PrecedencePattern::new("a", "b");
        assert!(p.check(&Trace::new()));
    }

    #[test]
    fn precedence_serde() {
        let p = PrecedencePattern::new("x", "y");
        let json = serde_json::to_string(&p).unwrap();
        let back: PrecedencePattern = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
