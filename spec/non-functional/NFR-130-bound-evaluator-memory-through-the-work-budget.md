---
id: NFR-130
title: "Bound evaluator memory through the work budget"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-262, type: constrains }
  - { target: ix://agent-ix/filament-spec-language/FR-100, type: constrains }
  - { target: ix://agent-ix/filament-spec-language/FR-255, type: constrains }
  - { target: ix://agent-ix/filament-spec-language/NFR-009, type: constrains }
---
# NFR-130: Bound evaluator memory through the work budget

## Statement

While a checked-term evaluation runs, the evaluator SHALL keep the memory it
holds for live call frames, pending operands and partial results at no more
than 1 KiB per `work_units` unit charged so far, plus the size of the
admitted inputs, so that the `work_units` limit is also the evaluation's
memory limit; it SHALL have no separate frame-count, depth or byte limit.

## Rationale

ADR-030 decides that nesting depth is never a limit: a deep recursion and a
shallow loop of equal work must give the same result. A frame-count or depth
limit would make a result depend on depth, and a byte limit would make it
depend on the platform's allocator and data layout, so two machines could
disagree on whether one evaluation completes. `work_units` is deterministic
and already caller-settable without a ceiling (FR-255). Every live frame and
every pending operand is created by a charged operation (a `function.call`,
a collection visit, an arithmetic step), so live memory grows with charged
work. Stating the constant turns that into a guarantee: a caller who sets
`work_units` to `W` knows the evaluation holds at most about `W` KiB beyond
its inputs, and the evaluation stops `incomplete` with
`stage_limit_exceeded` naming `accounting.work_units` before it can hold
more. Exhausting the budget is incomplete, never a refusal or a false result.

## Scope

One checked-term call, family evaluation or protocol-slot operation on the
`filament.value.accounting/v1` meter (NFR-009). Input documents, the checked
package and immutable environments are inputs, not evaluation memory.
Intake and model normalization keep their own limits (FR-056, FR-106).

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Peak evaluation memory per charged work unit over a recursion of depth 1,000,000 | at most 1 KiB | at most 1 KiB | test: growth of the peak resident set across the evaluation divided by the `work_units` consumed |
| Deep recursion outcome | completes, or `incomplete` at the work budget | no depth refusal, no native stack overflow | negative-abuse-testing |

## Verification

A Rust public-API test calls the recursive function of AC-1 through the
checked-package call entry on an unlimited meter and on a short one and
reads the outcome and the consumed `work_units`. A second test, run alone on
Linux, runs the AC-2 call and divides the growth of the process's peak
resident set (`VmHWM`) by the consumed `work_units`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-130-AC-1 | `function d using v(n: Int[0, 1000000]): Integer pure decreases(n) { if n > 0 then d(n - 1) else 0 }` called with `n = 200000` on a meter with unlimited `work_units` completes with `0` on the default thread stack; called on a meter whose `work_units` is below the work the call needs, it returns `Incomplete` naming the `work_units` limit, with no completed value; no outcome of either call names depth, frames or memory. | Test (TC-1930) |
| NFR-130-AC-2 | For the call of AC-1 with `n = 1000000`, the growth of the process's peak resident set across the evaluation divided by the `work_units` it consumed is at most 1 KiB. | Test (TC-1931) |

## Dependencies

- ADR-030 D-1 (depth is never a limit), FR-262 (evaluate values and calls at
  any depth), FR-255 (the accounting settings) and NFR-009 (bound checked-term
  evaluation work).
