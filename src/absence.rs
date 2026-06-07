//! Absence pattern: "P must never occur".
//!
//! Variants:
//! - Unconditional absence: P never appears in scope.
//! - Absence after Q: P must not appear after the first occurrence of Q.

use serde::{Deserialize, Serialize};

use crate::{Scope, Trace};

/// "P must never occur" within the scope.
///
/// If `after` is `Some(Q)`, then P must not appear at any position
/// following the first occurrence of Q (within scope).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbsencePattern {
    /// Proposition that must not occur.
    pub proposition: String,
    /// If set, absence is only enforced after this proposition first appears.
    pub after: Option<String>,
    /// Scope restricting the checked region.
    pub scope: Scope,
}

impl AbsencePattern {
    /// Unconditional absence: P must never occur.
    pub fn new(proposition: impl Into<String>) -> Self {
        Self {
            proposition: proposition.into(),
            after: None,
            scope: Scope::Global,
        }
    }

    /// Absence after Q: P must not occur after Q first appears.
    pub fn after(proposition: impl Into<String>, trigger: impl Into<String>) -> Self {
        Self {
            proposition: proposition.into(),
            after: Some(trigger.into()),
            scope: Scope::Global,
        }
    }

    /// Attach a custom scope.
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Check whether `trace` satisfies this absence pattern.
    pub fn check(&self, trace: &Trace) -> bool {
        let Some((start, end)) = self.scope.scope_range(trace) else {
            return true; // no scope → vacuously satisfied
        };
        let region = &trace[start..end];

        match &self.after {
            None => {
                // Unconditional: proposition must never appear.
                !region.iter().any(|step| step.contains(&self.proposition))
            }
            Some(trigger) => {
                // Find first occurrence of trigger.
                let trigger_idx = region.iter().position(|step| step.contains(trigger));
                match trigger_idx {
                    None => true, // trigger never appeared → vacuously true
                    Some(idx) => {
                        // Check that proposition never appears after trigger.
                        !region[idx + 1..]
                            .iter()
                            .any(|step| step.contains(&self.proposition))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_prop_satisfies() {
        let p = AbsencePattern::new("error");
        let trace: Trace = vec![vec!["ok".into()], vec!["ok".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn present_prop_violates() {
        let p = AbsencePattern::new("error");
        let trace: Trace = vec![vec!["ok".into()], vec!["error".into()]];
        assert!(!p.check(&trace));
    }

    #[test]
    fn absent_after_trigger_ok() {
        let p = AbsencePattern::after("error", "shutdown");
        let trace: Trace = vec![
            vec!["error".into()], // before trigger — allowed
            vec!["shutdown".into()],
            vec!["ok".into()],
        ];
        assert!(p.check(&trace));
    }

    #[test]
    fn absent_after_trigger_violated() {
        let p = AbsencePattern::after("error", "shutdown");
        let trace: Trace = vec![vec!["shutdown".into()], vec!["error".into()]];
        assert!(!p.check(&trace));
    }

    #[test]
    fn trigger_never_appears_vacuously_true() {
        let p = AbsencePattern::after("error", "shutdown");
        let trace: Trace = vec![vec!["error".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn empty_trace_satisfies() {
        let p = AbsencePattern::new("x");
        assert!(p.check(&Trace::new()));
    }

    #[test]
    fn scoped_absence() {
        let p =
            AbsencePattern::new("error").with_scope(Scope::Between("open".into(), "close".into()));
        let trace: Trace = vec![
            vec!["open".into()],
            vec!["ok".into()],
            vec!["close".into()],
            vec!["error".into()], // outside scope — allowed
        ];
        assert!(p.check(&trace));
    }

    #[test]
    fn absence_serde() {
        let p = AbsencePattern::after("x", "y");
        let json = serde_json::to_string(&p).unwrap();
        let back: AbsencePattern = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
