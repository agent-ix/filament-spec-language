---
id: SR-2026111414542500000003
title: "Code and Rust review of QSL-1932 shared evaluator drivers (PR #2710)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1932; PR #2710 head 97f32eea06926042177612ec7d81fb1b9b838570 vs origin/main bb909513989a; qsl-eval/src/value/{collections.rs,composite.rs,mod.rs}; qsl-eval/src/value/expression/{mod.rs,evaluate.rs,evaluate/invariants.rs,evaluate/union_case.rs}"
review_set: subset
---
# Code and Rust review: QSL-1932 shared evaluator drivers

## Summary

Ticket: QSL-1932. PR #2710, one commit on origin/main. Methods: code-review
with the rust-review lane (rust-style baseline). Scope is
`git diff origin/main...HEAD` only. The new modules were compared line by line
with the qsl-eval hunks of #2599 (`composite.rs`, identical) and #2601
(`collections.rs`, identical except `Sequence` fields narrowed from `pub` to
`pub(crate)`), and every removed evaluator block was compared with the shared
function that replaces it.

Behaviour preservation, checked per site: tuple and record construction
(same pop order, same `record-declaration-unresolved`,
`record-present-value-missing` and the six construction fault identifiers);
field read (same `field-composite-value-expected`, `field-slot-unresolved`,
FR-359 slot lookup through `world.record_slot`, `witnessed = false`); slot
projection (same five `field-*` identifiers); `size` (text stays on
`size_of`, a collection goes to `collections::size`; the moved
`Value::Collection` arm in `size_of` is unreachable from the one caller);
`contains`, `flatten` (result type checked before the pop, as before),
`ConvertCollection` (kept elements only when traced); exact `sum` steps
(`sum_admits` equals `ValueType::admits` for the exact families, so the
seed check, the located `SumOutOfDomain`, the unlocated running-total
`SumOutOfDomain`, the decimal loss record and `sum-accumulator-value-invalid`
are unchanged); the empty sum (exact zero inside the driver, float and
quantity zero pre-filled by the host); query and fold absorb, early exit
(`forall` false, `exists` true), visit charging including the FR-139
`only`-over-`where` exemption, `only` population check and family result,
reduce seeding with `EmptyReduction`, finish for every visit, and `seq`.
Charge order is unchanged on every path. The invalid-node fallback in
`iterate` reproduces the old pop-or-charge before `iteration-node-invalid`.
Main's `NodeKind::PlaceReference` (QSL-1818) arm is untouched by the diff.

Iteration state: `Iteration` and `Sequence` fields are `pub(crate)`; the
JIT-facing surface is the methods. `Spec` is a `Copy` description of the
node and `Finished` a result, so the ticket's "no `pub` mutable iteration
state leaves `qsl-eval`" holds. No compatibility shim, re-export of a
removed path, or Python is added.

Evidence: `cargo test -p qsl-eval` with `QSPEC_DIR` in the ticket worktree
with RUST_TEST_THREADS=2: 219 + 1294 + 56 passed, 0 failed. The aggregate gate was not run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `only_population` is a new public function with no caller on main; only the later JIT uses it, and the interpreter keeps calling `population_of` directly | qsl-eval/src/value/expression/evaluate.rs:517-523; qsl-eval/src/value/mod.rs:25; qsl-eval/src/value/expression/mod.rs:55 |
| FND-002 | low | `Iteration::accumulator()` has no caller; the interpreter's fold body reads the `pub(crate)` field directly instead of the getter | qsl-eval/src/value/collections.rs:379-383; qsl-eval/src/value/expression/evaluate.rs:4259-4263 |
| FND-003 | low | `Spec` has public fields, so a caller outside qsl-eval can build `only_over_filter: true` for a non-`only` shape and skip the visit charges `Spec::new` would keep | qsl-eval/src/value/collections.rs:296-321 |
| FND-004 | low | `Sequence` and its `new`/`step`/`finish` are public in the module described as shared with the JIT, but no JIT code uses the `seq` driver; only the interpreter does | qsl-eval/src/value/collections.rs:588-650 |

## Verdict

Mergeable as a partial slice. The extraction is behaviour-preserving and
the iteration-state AC holds. All four findings are low; FND-001 and
FND-002 are JIT-facing items landed ahead of their only consumer, which fits
the ticket's purpose (land the code the JIT shares) and can be accepted
without change if the JIT landing ticket consumes them; FND-002 also has a
one-line fix (use the getter in the fold body). The omitted
`execution.rs`/`execution/scan.rs` hunks are correctly left out: neither
`BackendCoverage` nor `execution/scan.rs` exists on main. The resulting
scope gap is recorded in the gap-analysis artifact for this ticket.

Tests: the new modules carry no unit tests of their own. For a pure
extraction that is adequate: the existing evaluator suites drive every
shared function, the invariant probes (`iteration-node-invalid`,
`query-body-value-invalid`, `sum-accumulator-value-invalid`,
`iteration-source-position-invalid`, fold and seq owner tests) were updated
only to reach the state through `.state`, and the record source-proof test
now asserts the count and zip in `composite.rs`. No acceptance criterion
changed, so no trace tags are needed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, commit 56da6b472: `only_population` and both re-exports removed; the JIT landing adds it with its caller. |
| FND-002 | fixed | Round 1, commit 56da6b472: the fold body reads `iteration.state.accumulator().cloned()`, so the getter has a caller. |
| FND-003 | fixed | Round 1, commit 56da6b472: `Spec` fields are private; `shape()` and `value_type()` accessors are used by the evaluator's `finish`; `only_over_filter` is set only by `Spec::new`. |
| FND-004 | fixed | Round 1, commit 56da6b472: `Sequence` and `new`/`step`/`finish` are `pub(crate)`. |
