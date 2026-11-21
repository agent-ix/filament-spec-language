---
id: SR-1542
title: "Code and Rust review of QSL-2749 guard undefined propagation"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2749; fsl-eval/src/temporal/finite_window/run.rs; fsl-replay/src/spine/clause/tests/temporal.rs; 214cdc7acbb57fa22be31ec03601e2c2a0fa9ccb..52bc8d9b58734bd9d7fdf72263eb7f2629d0796c"
review_set: subset
---

## Summary

Reviewed only QSL-2749's guard-specific finite-window Stop/Tail propagation and its timestamped regression/pending tests. Code-review includes the Rust lane; inherited PR3327 behavior and coordinated peer atom plumbing are outside this delta.

## Verdict

**PASS** — no functional findings in the assigned delta. A defined undefined guard retains its typed cause and actual position through settlement; genuinely incomplete scope remains pending without truth.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined guard-specific enum variants, conversion from Stop to Tail, activation evaluation, and settlement; exhaustive typed matching preserves the cause without guessed formula identity. The timestamped regression checks open and closed scope, Undefined/DivisionByZero at position 0, exit 10, activation locus, and one-snapshot prefix. The separate incomplete-scope case checks Pending and absent truth. No production unsafe, external decoding, dependency, CI, or gate changes occur in this delta.

Accepted the coder lane's four distinct passing focused cases and preserved initial fixture profile error as reported. No tests, builds, lint, matrix, profile audit, batch re-review, or full gate were rerun, as explicitly directed. No activation-axis contract was inferred. General demanded-atom plumbing belongs to the coordinated peer change.

## Examined requirements

- FR-044-AC-7 (`spec/functional/FR-044-activate-temporal-obligations.md`), examined: “A guard evaluating false creates no instance and is not a refusal; a closed-complete scope whose every admitted trigger is guard-false reports `inactive`; a guard that cannot be established reports incomplete or refused activation naming the guard rather than treating it as false.”
- FR-267-AC-14 (`spec/functional/FR-267-write-run-results-as-native-run-result-2.md`), examined guard unit: “and the `GuardUndefined` clause (`on each (q: Counters::Counter) when (3 mod (2 - q.value) >= 0)`) over `Finite` 2, 0, 1, whose `undefined` at `position` 0 has the envelope `locus` `{"guard": "on-each"}`, `cause` `division-by-zero` and a one-snapshot `prefix`. None writes a `refusal` member.”
- FR-331-AC-9 (`spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md`), context_only: “The checked activation guard is undefined at position 2 with cause `division-by-zero`, so replay settles `inconclusive`, `ReplayParity`, retaining the typed `Undefined` at position 2; the atom locus does not identify the activation guard.”
