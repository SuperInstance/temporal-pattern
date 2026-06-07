//! Response pattern: "if P occurs then eventually Q".
//!
//! The response pattern is the most frequently used property in
//! specification practice (Dwyer et al. 1999). It captures causal
//! relationships: every occurrence of a trigger `P` must be followed
//! (not necessarily immediately) by a response `Q`.

use serde::{Deserialize, Serialize};

use crate::{Scope, Trace};

/// "If `trigger` occurs then `response` must eventually occur."
///
/// The check is scoped — only the portion of the trace returned by
/// [`Scope::scope_range`] is considered.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResponsePattern {
    /// Proposition that triggers the obligation.
    pub trigger: String,
    /// Proposition that must eventually hold after each trigger.
    pub response: String,
    /// Scope restricting the checked region.
    pub scope: Scope,
}

impl ResponsePattern {
    /// Create a globally-scoped response pattern.
    pub fn new(trigger: impl Into<String>, response: impl Into<String>) -> Self {
        Self {
            trigger: trigger.into(),
            response: response.into(),
            scope: Scope::Global,
        }
    }

    /// Attach a custom scope.
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Check whether `trace` satisfies this response pattern.
    ///
    /// For every position `i` in scope where `trigger` holds, there must
    /// exist a later position `j > i` (still in scope) where `response`
    /// holds.
    pub fn check(&self, trace: &Trace) -> bool {
        let Some((start, end)) = self.scope.scope_range(trace) else {
            // If scope cannot be established, vacuously true.
            return true;
        };
        let region = &trace[start..end];

        for (i, step) in region.iter().enumerate() {
            if !step.contains(&self.trigger) {
                continue;
            }
            // Look for response at any later position in the region.
            let later = &region[i + 1..];
            if !later.iter().any(|s| s.contains(&self.response)) {
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
    fn global_response_satisfied() {
        let p = ResponsePattern::new("req", "ack");
        let trace: Trace = vec![vec!["req".into()], vec!["idle".into()], vec!["ack".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn global_response_violated() {
        let p = ResponsePattern::new("req", "ack");
        let trace: Trace = vec![vec!["req".into()], vec!["idle".into()], vec!["done".into()]];
        assert!(!p.check(&trace));
    }

    #[test]
    fn no_trigger_vacuously_true() {
        let p = ResponsePattern::new("req", "ack");
        let trace: Trace = vec![vec!["idle".into()], vec!["idle".into()]];
        assert!(p.check(&trace));
    }

    #[test]
    fn multiple_triggers_all_need_response() {
        let p = ResponsePattern::new("req", "ack");
        let trace: Trace = vec![
            vec!["req".into()],
            vec!["ack".into()],
            vec!["req".into()],
            vec!["ack".into()],
        ];
        assert!(p.check(&trace));
    }

    #[test]
    fn second_trigger_no_response() {
        let p = ResponsePattern::new("req", "ack");
        let trace: Trace = vec![
            vec!["req".into()],
            vec!["ack".into()],
            vec!["req".into()],
            vec!["done".into()],
        ];
        assert!(!p.check(&trace));
    }

    #[test]
    fn scoped_after_response() {
        let p = ResponsePattern::new("req", "ack").with_scope(Scope::After("start".into()));
        let trace: Trace = vec![
            vec!["req".into()], // before scope — ignored
            vec!["start".into()],
            vec!["req".into()],
            vec!["ack".into()],
        ];
        assert!(p.check(&trace));
    }

    #[test]
    fn scoped_before_response() {
        let p = ResponsePattern::new("req", "ack").with_scope(Scope::Before("end".into()));
        let trace: Trace = vec![
            vec!["req".into()],
            vec!["ack".into()],
            vec!["end".into()],
            vec!["req".into()], // after scope — ignored
        ];
        assert!(p.check(&trace));
    }

    #[test]
    fn scoped_between_response() {
        let p = ResponsePattern::new("req", "ack")
            .with_scope(Scope::Between("open".into(), "close".into()));
        let trace: Trace = vec![
            vec!["open".into()],
            vec!["req".into()],
            vec!["ack".into()],
            vec!["close".into()],
        ];
        assert!(p.check(&trace));
    }

    #[test]
    fn empty_trace_satisfies() {
        let p = ResponsePattern::new("req", "ack");
        assert!(p.check(&Trace::new()));
    }

    #[test]
    fn response_serde_roundtrip() {
        let p = ResponsePattern::new("a", "b");
        let json = serde_json::to_string(&p).unwrap();
        let back: ResponsePattern = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
