//! Scope definitions for temporal patterns.
//!
//! A scope restricts the portion of a trace over which a pattern is
//! evaluated. This mirrors the scope hierarchy from Dwyer et al. (1999):
//!
//! | Scope   | Meaning                                           |
//! |---------|---------------------------------------------------|
//! | Global  | Entire trace                                      |
//! | After R | From the first occurrence of R to end of trace    |
//! | Before S| From start of trace up to (excluding) first S     |
//! | Between | From first R up to (excluding) first S after R    |

use serde::{Deserialize, Serialize};

use crate::Trace;

/// Restricts the region of a trace a pattern covers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    /// Evaluate over the entire trace.
    Global,
    /// Evaluate from the first occurrence of `R` to the end of the trace.
    After(String),
    /// Evaluate from the start up to (but not including) the first `S`.
    Before(String),
    /// Evaluate from the first `R` up to (but not including) the first `S`
    /// that appears after `R`. If `S` never appears after `R`, the scope
    /// extends to the end of the trace.
    Between(String, String),
}

impl Scope {
    /// Returns the `(start, end)` slice indices of `trace` covered by this
    /// scope, or `None` if the scope cannot be established (e.g. `After(R)`
    /// but `R` never occurs).
    pub fn scope_range(&self, trace: &Trace) -> Option<(usize, usize)> {
        match self {
            Scope::Global => Some((0, trace.len())),
            Scope::After(r) => {
                let idx = find_first(trace, r)?;
                Some((idx, trace.len()))
            }
            Scope::Before(s) => {
                let idx = find_first(trace, s)?;
                Some((0, idx))
            }
            Scope::Between(r, s) => {
                let r_idx = find_first(trace, r)?;
                let s_idx = find_after(trace, s, r_idx)?;
                Some((r_idx, s_idx))
            }
        }
    }
}

/// Find the first step containing proposition `p`.
fn find_first(trace: &Trace, p: &str) -> Option<usize> {
    trace
        .iter()
        .position(|step| step.iter().any(|prop| prop == p))
}

/// Find the first step containing proposition `p` at or after `start`.
fn find_after(trace: &Trace, p: &str, start: usize) -> Option<usize> {
    trace[start..]
        .iter()
        .position(|step| step.iter().any(|prop| prop == p))
        .map(|i| start + i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trace() -> Trace {
        vec![
            vec!["init".into()],
            vec!["ready".into()],
            vec!["go".into()],
            vec!["done".into()],
            vec!["reset".into()],
        ]
    }

    #[test]
    fn global_covers_all() {
        let scope = Scope::Global;
        assert_eq!(scope.scope_range(&make_trace()), Some((0, 5)));
    }

    #[test]
    fn after_finds_boundary() {
        let scope = Scope::After("go".into());
        assert_eq!(scope.scope_range(&make_trace()), Some((2, 5)));
    }

    #[test]
    fn before_finds_boundary() {
        let scope = Scope::Before("done".into());
        assert_eq!(scope.scope_range(&make_trace()), Some((0, 3)));
    }

    #[test]
    fn between_finds_region() {
        let scope = Scope::Between("ready".into(), "done".into());
        assert_eq!(scope.scope_range(&make_trace()), Some((1, 3)));
    }

    #[test]
    fn after_missing_returns_none() {
        let scope = Scope::After("missing".into());
        assert_eq!(scope.scope_range(&make_trace()), None);
    }

    #[test]
    fn before_missing_returns_none() {
        let scope = Scope::Before("missing".into());
        assert_eq!(scope.scope_range(&make_trace()), None);
    }

    #[test]
    fn between_missing_r_returns_none() {
        let scope = Scope::Between("missing".into(), "done".into());
        assert_eq!(scope.scope_range(&make_trace()), None);
    }

    #[test]
    fn between_missing_s_returns_none() {
        let scope = Scope::Between("ready".into(), "missing".into());
        assert_eq!(scope.scope_range(&make_trace()), None);
    }

    #[test]
    fn scope_serde_roundtrip() {
        let scope = Scope::Between("a".into(), "b".into());
        let json = serde_json::to_string(&scope).unwrap();
        let back: Scope = serde_json::from_str(&json).unwrap();
        assert_eq!(scope, back);
    }
}
