---
id: TC-2063
title: "Probabilistic event operators are admitted by the profile"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-186 FR-195
    type: verifies
---
# TC-2063: Probabilistic event operators are admitted by the profile

## Description

Verify that a probabilistic event's operators go through the profile's operator admission before the claim check, that `unbounded-event` is reserved for the infinite-trace profile, and that timed bounded-window events check.

Scope: FR-186-AC-3, FR-186-AC-7, FR-195-AC-1.

## Test Procedure

1. Under a bounded profile, check `[always holds(n.healthy)]`, `[eventually holds(p.done)]` and `[eventually[2, *] holds(n.healthy)]`, and the same formulas as `temporal` clauses.
2. Under the infinite-trace profile, check `[eventually[2, *] holds(n.healthy)]`.
3. Under the timed profile over a subject with a time binding, check `[eventually[0 ms, 4 ms] holds(n.healthy)]` and `[eventually holds(n.healthy)]`.

Tag the tests `#[trace("TC-2063", "FR-186-AC-n")]` (and `FR-195-AC-1`).

## Expected Results

- Step 1: each refused `unsupported_construct`/`expression-form` at the operator or interval, equal to the clause refusal; never `unbounded-event`.
- Step 2: `unbounded-event`.
- Step 3: the windowed event checks with time horizon 4 ms; the interval-less one is refused `unsupported_construct`/`expression-form`.
