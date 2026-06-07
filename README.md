# temporal-pattern

**Temporal pattern library for agent specification.**

A Rust library for defining and checking temporal properties over proposition traces. Built on the pattern taxonomy established by Dwyer, Avrunin & Corbett (1999), it provides a clean, composable API for specifying how agents should behave over time.

```
[Step 0] → [Step 1] → [Step 2] → ... → [Step N]
  {a,b}     {c}        {a,d}              {c}
```

A **trace** is `Vec<Vec<String>>` — each inner vec is the set of propositions that hold at that discrete time step. Patterns evaluate over these traces and return `true` or `false`.

## Table of Contents

- [Why This Exists](#why-this-exists)
- [Pattern Taxonomy](#pattern-taxonomy)
- [Scopes](#scopes)
- [Modules](#modules)
- [Quick Start](#quick-start)
- [Examples](#examples)
  - [Example 1: HTTP Request/Response](#example-1-http-requestresponse)
  - [Example 2: Resource Lifecycle](#example-2-resource-lifecycle)
  - [Example 3: Agent Safety Constraints](#example-3-agent-safety-constraints)
- [API Reference](#api-reference)
- [Design Decisions](#design-decisions)
- [Theory](#theory)
- [References](#references)
- [License](#license)

---

## Why This Exists

When specifying agent behavior, you often need to express temporal constraints:

- "After a request is sent, an acknowledgment must eventually arrive"
- "A resource can only be used after it's been acquired"
- "The agent must visit the charging station at least once per mission"
- "The shutdown command must never be issued twice"

These are **temporal patterns** — recurring specification idioms that appear across domains. Rather than writing custom verification logic each time, this library provides a small, well-tested set of composable pattern building blocks.

The pattern system is grounded in the seminal work of Dwyer, Avrunin & Corbett (1999), who analyzed 555 real-world specifications and found that most temporal properties fall into a small number of patterns with scoped variants.

---

## Pattern Taxonomy

The core patterns, adapted from Dwyer et al.:

```
                        ┌──────────────┐
                        │   Pattern    │
                        │  Hierarchy   │
                        └──────┬───────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
        ┌─────┴─────┐   ┌─────┴─────┐   ┌──────┴──────┐
        │ Existence │   │  Order    │   │  Absence    │
        └─────┬─────┘   └─────┬─────┘   └──────┬──────┘
              │               │                 │
     ┌───────┴──────┐  ┌─────┴─────┐    ┌──────┴──────┐
     │              │  │           │    │             │
  At least     Bounded  Response  Precedence  Never   After
  once         (k steps)                       occur   Q
```

| Pattern     | English reading                       | LTL approximation       |
|-------------|---------------------------------------|--------------------------|
| Response    | If P then eventually Q                | □(P → ◇Q)              |
| Precedence  | Q must be preceded by P               | Q → ○P (past)           |
| Existence   | P must occur at least once            | ◇P                      |
| Bounded     | P must occur within k steps           | ◇≤k P                   |
| Absence     | P must never occur                    | □¬P                     |
| AbsenceAfter| P must not occur after Q              | □(Q → □¬P)             |

Each pattern can be evaluated under a **scope** that restricts the checked
region of the trace.

---

## Scopes

Scopes are the key innovation from the Dwyer et al. taxonomy. The same
pattern means different things depending on where in the trace it applies:

```
Scope         Trace Region
──────────────────────────────────────────────────────
Global        |═════════════════════════════════════|
              [0]  [1]  [2]  [3]  [4]  [5]  [6]  [7]

After(R)                 R
              |════════|─────── R ──────────────────|
              [0]  [1]  [2]  [3]  [4]  [5]  [6]  [7]

Before(S)                         S
              |─────────────────── S ──────|════════|
              [0]  [1]  [2]  [3]  [4]  [5]  [6]  [7]

Between(R,S)          R                    S
              |══════|── R ─────────────── S ──|════|
              [0]  [1]  [2]  [3]  [4]  [5]  [6]  [7]
```

| Scope          | `scope_range()` semantics                                    |
|----------------|--------------------------------------------------------------|
| `Global`       | `(0, trace.len())` — the whole trace                        |
| `After(R)`     | From first `R` to end; `None` if `R` never appears         |
| `Before(S)`    | From start up to (excluding) first `S`; `None` if no `S`   |
| `Between(R,S)` | From first `R` up to (excluding) first `S` after `R`        |

---

## Modules

| Module          | Key Type              | Purpose                                   |
|-----------------|-----------------------|-------------------------------------------|
| `scope`         | `Scope`               | Define checked region of trace            |
| `response`      | `ResponsePattern`     | "If P then eventually Q"                  |
| `precedence`    | `PrecedencePattern`   | "Q must be preceded by P"                 |
| `existence`     | `ExistencePattern`    | "P must occur" (optionally within k steps)|
| `absence`       | `AbsencePattern`      | "P must never occur" / "not after Q"      |
| `combination`   | `Combination`         | AND/OR composition, conflict detection    |

---

## Quick Start

```toml
# Cargo.toml
[dependencies]
temporal-pattern = "0.1"
```

```rust
use temporal_pattern::{ResponsePattern, Trace};

fn main() {
    let pattern = ResponsePattern::new("request", "response");

    let trace: Trace = vec![
        vec!["request".into()],
        vec!["processing".into()],
        vec!["response".into()],
    ];

    assert!(pattern.check(&trace));
}
```

---

## Examples

### Example 1: HTTP Request/Response

Verify that every HTTP request receives a response:

```rust
use temporal_pattern::*;

fn main() {
    // Build patterns
    let req_resp = ResponsePattern::new("http_request", "http_response");

    // Good trace: request followed by response
    let good: Trace = vec![
        vec!["http_request".into(), "connected".into()],
        vec!["processing".into()],
        vec!["http_response".into()],
    ];
    assert!(req_resp.check(&good));

    // Bad trace: request with no response
    let bad: Trace = vec![
        vec!["http_request".into()],
        vec!["timeout".into()],
    ];
    assert!(!req_resp.check(&bad));

    // Multiple requests, all need responses
    let multi: Trace = vec![
        vec!["http_request".into()],
        vec!["http_response".into()],
        vec!["http_request".into()],
        vec!["http_response".into()],
    ];
    assert!(req_resp.check(&multi));
}
```

### Example 2: Resource Lifecycle

Ensure resources are acquired before use and never used after release:

```rust
use temporal_pattern::*;
use temporal_pattern::combination::*;

fn main() {
    // Precedence: use must be preceded by acquire
    let acq_use = PrecedencePattern::new("acquire", "use");

    // Absence: use must not appear after release
    let no_use_after = AbsencePattern::after("use", "release");

    // Existence: must acquire at least once
    let must_acquire = ExistencePattern::new("acquire");

    // Combine with AND
    let lifecycle = Combination::new(
        CombinationOp::And,
        vec![
            Pattern::Precedence(acq_use),
            Pattern::Absence(no_use_after),
            Pattern::Existence(must_acquire),
        ],
    );

    let good: Trace = vec![
        vec!["acquire".into()],
        vec!["use".into()],
        vec!["use".into()],
        vec!["release".into()],
    ];
    assert!(lifecycle.check(&good));

    // Use before acquire — fails precedence
    let bad: Trace = vec![
        vec!["use".into()],
        vec!["acquire".into()],
    ];
    assert!(!lifecycle.check(&bad));

    // Check for conflicts (should be none)
    assert!(lifecycle.detect_conflicts().is_empty());

    // Check coverage
    let (cov, total, frac) = lifecycle.coverage(&good);
    println!("Coverage: {}/{} = {:.1}%", cov, total, frac * 100.0);
}
```

### Example 3: Agent Safety Constraints

A robot agent with safety requirements scoped to different mission phases:

```rust
use temporal_pattern::*;

fn main() {
    // During navigation: must reach goal after starting
    let reach_goal = ResponsePattern::new("nav_start", "goal_reached")
        .with_scope(Scope::Between("mission_start".into(), "mission_end".into()));

    // Collision must never occur (globally)
    let no_collision = AbsencePattern::new("collision");

    // Must visit charging station at least once within 10 steps
    let must_charge = ExistencePattern::within("charging", 10)
        .with_scope(Scope::After("mission_start".into()));

    let trace: Trace = vec![
        vec!["mission_start".into()],
        vec!["nav_start".into()],
        vec!["moving".into()],
        vec!["goal_reached".into()],
        vec!["charging".into()],
        vec!["mission_end".into()],
    ];

    assert!(reach_goal.check(&trace));
    assert!(no_collision.check(&trace));
    assert!(must_charge.check(&trace));
}
```

---

## API Reference

### `Scope`

```rust
pub enum Scope {
    Global,
    After(String),
    Before(String),
    Between(String, String),
}
```

Methods:
- `scope_range(&self, trace: &Trace) -> Option<(usize, usize)>` — returns the `(start, end)` slice of the trace this scope covers.

### `ResponsePattern`

```rust
pub struct ResponsePattern {
    pub trigger: String,
    pub response: String,
    pub scope: Scope,
}
```

Methods:
- `new(trigger, response) -> Self` — global scope
- `with_scope(scope) -> Self` — builder pattern
- `check(&self, trace: &Trace) -> bool`

### `PrecedencePattern`

```rust
pub struct PrecedencePattern {
    pub predecessor: String,
    pub follower: String,
    pub scope: Scope,
}
```

Methods:
- `new(predecessor, follower) -> Self`
- `with_scope(scope) -> Self`
- `check(&self, trace: &Trace) -> bool`

### `ExistencePattern`

```rust
pub struct ExistencePattern {
    pub proposition: String,
    pub bound: Option<usize>,
    pub scope: Scope,
}
```

Methods:
- `new(proposition) -> Self` — unbounded
- `within(proposition, k: usize) -> Self` — bounded
- `with_scope(scope) -> Self`
- `check(&self, trace: &Trace) -> bool`

### `AbsencePattern`

```rust
pub struct AbsencePattern {
    pub proposition: String,
    pub after: Option<String>,
    pub scope: Scope,
}
```

Methods:
- `new(proposition) -> Self` — unconditional
- `after(proposition, trigger) -> Self` — absence after trigger
- `with_scope(scope) -> Self`
- `check(&self, trace: &Trace) -> bool`

### `Combination`

```rust
pub struct Combination {
    pub op: CombinationOp,
    pub patterns: Vec<Pattern>,
}
```

Methods:
- `new(op, patterns) -> Self`
- `check(&self, trace: &Trace) -> bool`
- `detect_conflicts(&self) -> Vec<String>`
- `coverage(&self, trace: &Trace) -> (usize, usize, f64)`

---

## Design Decisions

### Trace representation: `Vec<Vec<String>>`

We use `String` propositions rather than generics or enums for maximum
flexibility. Traces come from external systems (logs, simulations, agent
runtimes) where propositions are naturally string-valued. This avoids the
need for the caller to define a prop enum and implement conversion traits.

### Zero external dependencies (except `serde`)

The only external dependency is `serde` with `derive` for serialization.
All pattern logic is implemented with standard library iterators. This
keeps compile times minimal and the dependency tree trivial.

### Simple struct-per-pattern

Each pattern is a plain struct with a `check()` method. No trait objects,
no generics on the pattern types, no macros. This makes the API easy to
discover and straightforward to extend.

### Scope as a field, not a wrapper

Scopes are embedded directly in each pattern struct. This keeps patterns
self-contained and serializable as a single unit. Alternative designs
(scope as a combinator/wrapper) are possible but add indirection for
little practical benefit.

### Conflict detection is best-effort

The `detect_conflicts()` method catches common contradictions (existence
vs. unconditional absence of the same proposition) but does not attempt
full LTL satisfiability checking. This keeps it fast and deterministic.

### Vacuous truth conventions

- **Response/Precedence**: If the scope cannot be established (e.g.
  `After(R)` but `R` never occurs), the pattern is vacuously satisfied.
  This follows standard LTL semantics.
- **Existence**: If the scope cannot be established, the pattern fails
  (we cannot find the proposition in a nonexistent region).
- **Absence**: If the scope cannot be established, the pattern is
  vacuously satisfied (nothing bad happened in a region that doesn't
  exist).

---

## Theory

### Propositional Traces

We model system executions as finite traces over propositional variables.
At each discrete time step `i ∈ {0, ..., n-1}`, a set of propositions
holds:

```
trace[i] ⊆ 2^AP    where AP is the set of atomic propositions
```

This is equivalent to a finite word over the alphabet `2^AP`, a standard
model in linear-time verification (Clarke, Grumberg & Peled, 1999).

### Pattern System

The pattern system is organized along two axes:

1. **Pattern type** — what temporal relationship is being specified
2. **Scope** — where in the trace the relationship must hold

This two-dimensional taxonomy was established by Dwyer, Avrunin & Corbett
(1999) through analysis of 555 real-world specifications from academia
and industry. They found that the vast majority of specifications fall
into a small number of pattern/scope combinations:

```
                        Scope
Pattern     Global    After    Before    Between
─────────   ──────    ─────    ──────    ────────
Existence     ■        ■        ■          ■
Absence       ■        ■        ■          ■
Response      ■        ■        ■          ■
Precedence    ■        ■        ■          ■
```

The five most common combinations (shaded ■) account for the majority
of real-world specifications.

### Scope Formalization

Given a trace `σ = σ₀σ₁...σₙ₋₁`, the scope determines a subtrace:

- **Global**: `σ[0..n]`
- **After(R)**: `σ[i..n]` where `i = min{j | R ∈ σⱼ}`
- **Before(S)**: `σ[0..k]` where `k = min{j | S ∈ σⱼ}`
- **Between(R,S)**: `σ[i..k]` where `i = min{j | R ∈ σⱼ}` and
  `k = min{j > i | S ∈ σⱼ}`

If the boundary propositions don't occur, the scope is undefined and
patterns default to vacuous truth or failure depending on type.

### Response Pattern Semantics

The response pattern checks:

```
∀i ∈ scope: trigger ∈ σᵢ → ∃j > i: response ∈ σⱼ
```

This is a finite-trace approximation of the LTL formula `□(P → ◇Q)`.
In the finite-trace setting, we interpret "eventually" as "at some future
position within the trace" — there is no infinite future to defer to.

### Precedence Pattern Semantics

The precedence pattern checks:

```
∀i ∈ scope: follower ∈ σᵢ → ∃j < i: predecessor ∈ σⱼ
```

Every occurrence of the follower must have a prior occurrence of the
predecessor within scope. This is the temporal mirror of response.

### Existence and Bounded Existence

Unbounded: `∃i ∈ scope: proposition ∈ σᵢ`

Bounded (within k steps):
```
∗i ∈ scope ∧ i < scope_start + k: proposition ∈ σᵢ
```

### Absence Semantics

Unconditional: `∀i ∈ scope: proposition ∉ σᵢ`

After trigger Q: `∀i > trigger_pos(Q): proposition ∉ σᵢ`

### Combination and Composition

Patterns compose via boolean connectives:

```
AND: ∀p ∈ patterns: p.check(trace) = true
OR:  ∃p ∈ patterns: p.check(trace) = true
```

Coverage measures what fraction of trace positions fall within the scope
of at least one pattern:

```
coverage = |{i | ∃p: i ∈ scope(p)}| / |trace|
```

---

## References

1. **Dwyer, M. B., Avrunin, G. S., & Corbett, J. C.** (1999). Patterns
   in property specifications for finite-state verification. In
   *Proceedings of the 21st International Conference on Software
   Engineering (ICSE '99)*, pp. 411–420. ACM.
   DOI: [10.1145/302405.302672](https://doi.org/10.1145/302405.302672)

2. **Clarke, E. M., Grumberg, O., & Peled, D. A.** (1999). *Model
   Checking*. MIT Press. ISBN: 978-0-262-03270-4.

3. **Pnueli, A.** (1977). The temporal logic of programs. In *Proceedings
   of the 18th Annual Symposium on Foundations of Computer Science
   (FOCS '77)*, pp. 46–57. IEEE.
   DOI: [10.1109/SFCS.1977.32](https://doi.org/10.1109/SFCS.1977.32)

4. **Manna, Z. & Pnueli, A.** (1995). *Temporal Verification of Reactive
   Systems: Safety*. Springer. ISBN: 978-0-387-94459-3.

5. **Bienmüller, T., Damm, W., & Wittke, H.** (2000). The STATEMATE
   verification system — making it happen. In *Proceedings of the 12th
   International Conference on Computer Aided Verification (CAV 2000)*,
   LNCS 1855, pp. 501–514. Springer.

6. **Mondadori, M., Cimatti, A., & Tonetta, S.** (2022). A temporal
   pattern language for requirements specification. In *Proceedings of
   the 26th International Conference on Formal Methods (FM 2022)*,
   LNCS 13550, pp. 453–470. Springer.

7. **Autili, M., Inverardi, P., & Pelliccione, P.** (2006). A pattern
   system for temporal properties. Technical Report, University of
   L'Aquila. Available at:
   <https://www.di.univaq.it/marco.autili/papers/TechnicalReportTR_002_2006.pdf>

---

## License

MIT
