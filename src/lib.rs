//! # temporal-pattern
//!
//! Temporal pattern library for agent specification.
//!
//! Provides a structured way to define and check temporal properties over
//! proposition traces, inspired by the pattern taxonomy of Dwyer, Avrunin &
//! Corbett (1999). A **trace** is `Vec<Vec<String>>` — each inner vec is the
//! set of propositions true at that discrete time step.
//!
//! ## Modules
//!
//! - [`scope`] — Scoping constructs (Global, After, Before, Between)
//! - [`response`] — Response pattern: "if P then eventually Q"
//! - [`precedence`] — Precedence pattern: "Q must be preceded by P"
//! - [`existence`] — Existence pattern: "P must occur at least once"
//! - [`absence`] — Absence pattern: "P must never occur"
//! - [`combination`] — Combine patterns with AND/OR; detect conflicts

pub mod absence;
pub mod combination;
pub mod existence;
pub mod precedence;
pub mod response;
pub mod scope;

pub use absence::AbsencePattern;
pub use combination::{Combination, CombinationOp};
pub use existence::ExistencePattern;
pub use precedence::PrecedencePattern;
pub use response::ResponsePattern;
pub use scope::Scope;

/// Alias for a single time-step's active propositions.
pub type Step = Vec<String>;

/// Alias for a full execution trace.
pub type Trace = Vec<Step>;
