---
id: NFR-130
title: "Bound evaluator memory through the work budget and the live-occurrence limit"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-262, type: constrains }
  - { target: ix://agent-ix/filament-spec-language/FR-100, type: constrains }
  - { target: ix://agent-ix/filament-spec-language/FR-255, type: constrains }
  - { target: ix://agent-ix/filament-spec-language/NFR-009, type: constrains }
---
# NFR-130: Bound evaluator memory through the work budget and the live-occurrence limit

## Statement

While a checked-term evaluation runs, the evaluator SHALL keep the memory it
holds for live call frames, pending operands and partial results at no more
than 1 KiB per `work_units` unit charged so far and at no more than 1 KiB per
live occurrence counted by the `evaluation.live_occurrences` limit, plus the
size of the admitted inputs. The live-occurrence limit bounds the evaluation's
memory independently of its work budget, so a caller who raises
`work_units` keeps a memory bound. The evaluator SHALL have no frame-count,
depth or byte limit.

**Live occurrences.** Before it binds the evaluated arguments of a checked
pure-function call (the `function.call` charge point, after that charge is
granted), the evaluator SHALL count the call's live occurrences: over every
call frame in progress, the new callee's frame included, one for the frame
plus `occ(v)` (`filament.value.accounting/v1`'s occurrence count) of each
argument value bound in it. When that count exceeds the
`evaluation.live_occurrences` setting (FR-255, default 1048576), the
evaluation SHALL stop `incomplete` with `stage_limit_exceeded` naming
`evaluation.live_occurrences`, its configured value and the count reached, at
that call, before the frame is pushed. A frame's occurrences leave the count
when its call returns. Both the std evaluator and the no-std evaluator
(FR-402) SHALL count and stop identically.

## Rationale

ADR-030 decides that nesting depth is never a limit, and FSpec FR-460 admits
occurrence counts among the resource limits that bound a stage. The work
budget alone bounds memory only through the work charged so far, which is
cumulative: a caller who raises `work_units` to let a long computation finish
also lifts every memory bound, and a 5,000,000-frame recursion then holds
gigabytes. Live occurrences are a resource measure, not a depth: they count
the frames and argument values the evaluation actually holds at one moment,
so they fall when calls return, and two calls holding the same live values
count the same whatever their nesting. A frame-count or depth limit would
make a result depend on depth alone, and a byte limit would make it depend on
the platform's allocator and data layout, so two machines could disagree on
whether one evaluation completes; the occurrence count is defined on values
and is the same everywhere. Reaching either limit is incomplete, never a
refusal or a false result, and each names the setting that raises it.

## Scope

One checked-term call, family evaluation or protocol-slot operation on the
`filament.value.accounting/v1` meter (NFR-009). Input documents, the checked
package and immutable environments are inputs, not evaluation memory.
Intake and model normalization keep their own limits (FR-056, FR-106).

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Peak evaluation memory per charged work unit over a recursion of depth 1,000,000 | at most 1 KiB | at most 1 KiB | test: growth of the peak resident set across the evaluation divided by the `work_units` consumed |
| Peak evaluation memory per live occurrence over the same recursion | at most 1 KiB | at most 1 KiB | test: the same growth divided by the largest live-occurrence count reached |
| Deep recursion outcome | completes, or `incomplete` at the work budget or the live-occurrence limit | no depth refusal, no native stack overflow | negative-abuse-testing |

## Verification

A Rust public-API test calls the recursive function of AC-1 through the
checked-package call entry on an unlimited meter and on a short one and
reads the outcome and the consumed `work_units`. A second test, run alone on
Linux, runs the AC-2 call and divides the growth of the process's peak
resident set (`VmHWM`) by the consumed `work_units` and by the live-occurrence
high-water count. A run-level test drives the same function through the
`run` operation with raised and default limits.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-130-AC-1 | `function d using v(n: Int[0, 1000000]): Integer pure decreases(n) { if n > 0 then d(n - 1) else 0 }` called with `n = 200000` on a meter with unlimited `work_units` and the default `evaluation.live_occurrences` completes with `0` on the default thread stack; called on a meter whose `work_units` is below the work the call needs, it returns `Incomplete` naming the `work_units` limit, with no completed value; no outcome of either call names depth or frames. | Test (TC-1930) |
| NFR-130-AC-2 | For the call of AC-1 with `n = 1000000` on unlimited `work_units` and unlimited `evaluation.live_occurrences`, the growth of the process's peak resident set across the evaluation divided by the `work_units` it consumed is at most 1 KiB, and divided by the largest live-occurrence count reached is at most 1 KiB. | Test (TC-1931) |
| NFR-130-AC-3 | Through the `run` operation, `d` called with `n = 1000000` and `accounting.work_units` raised to `100000000000` under the default `evaluation.live_occurrences` (1048576; each frame holds two occurrences) stops with exit 22 and the outcome `{"kind": "incomplete", "limit": "evaluation.live_occurrences"}`; with `call.evaluation.live_occurrences` 4000000 the same call completes with `0`. | Test (TC-2164) |
| NFR-130-AC-4 | Through the `run` operation under default limits, `d` called with `n = 1000` completes with `0`; with `call.evaluation.live_occurrences` 0 it stops incomplete at its first call, because zero is a real limit; a `call.evaluation` with `live_occurrences` `-1`, `1.5` or `"x"`, or with an unknown member, refuses `invalid-request` at stage `request`, exit 20, naming the member. | Test (TC-2165) |

## Dependencies

- ADR-030 D-1 (depth is never a limit), FSpec FR-460 (occurrence counts are a
  resource limit kind), FR-262 (evaluate values and calls at any depth),
  FR-255 (the accounting settings and `evaluation.live_occurrences`) and
  NFR-009 (bound checked-term evaluation work).
