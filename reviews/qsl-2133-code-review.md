---
id: SR-2026111608414701
title: "Code review of filament-spec-language PR 2741 (QSL-2133 run-request trace kinds and progress ledger)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; git diff origin/main...617dafe9585eacb4289256ca6fc0f2f9de7d76e1 (35 files: fsl-replay spine request/clause/temporal/model_trace/trace_refusal/ledger_wire/native_result, fsl-replay model_check/replay.rs, fsl-analyze monitor.rs and tests/it/model_trace_run.rs, fsl-eval temporal, fsl-command, tests/it/run_wire.rs)"
review_set: subset
evaluated_revision: "617dafe9585eacb4289256ca6fc0f2f9de7d76e1"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-645
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: reviews
---

## Summary

Ticket: QSL-2133. This is a code review with the rust-review lane, over
`git diff origin/main...HEAD` at 617dafe9. It was checked against FR-641,
FR-642, FR-645, FR-329, FR-330 (AC-16 and "The decisive atom"), FR-283-AC-13,
FR-255 (the `expansion.max_candidates` row), FR-516 and FR-502. Lead rulings
applied: timed traces decode and refuse until QSL-2254; FR-645 wins over
FR-267-AC-14 for empty traces (QSL-2290); half_open 0 refuses
`invalid-request` (QSL-2291); window aggregates over non-finite sample traces
refuse naming QSL-2138.

Focused runs, all green: `cargo test -p fsl-replay --lib -- spine` (658
passed, 8 ignored), `cargo test -p filament-spec-language --test it --
run_wire` (8 passed, 13 ignored, all ignores name QSL-2254/2136/1977/1989),
`cargo test -p fsl-analyze --test it -- model_trace_run monitor` (128 passed,
1 ignored) with `FSPEC_DIR` set. No Python and no logic-bearing shell in the
diff.

Checked and clean:

- The request reader decodes every FR-641 trace kind under closed envelopes,
  enforces exactly one trace member, refuses a key/sample mix, a sample trace
  with `over` and a key trace without `over` at stage `request`.
- The ledger file is read once under the intake budget, refuses duplicate or
  out-of-order bindings, and writes canonically; the round trip test passes.
- The run_wire fixture split (`trace_unit`, `trace_clocks`) removed only the
  timed clause and clock. Every assertion of the old kind-mismatch test is
  kept: the timed case moved verbatim into the ignored
  `a_timed_clause_over_untimed_samples_refuses_with_profile_and_kinds` through
  the shared `assert_kind_mismatch` helper.
- The restated FR-330 temporal tests for an empty lasso loop, an empty finite
  trace, bounded-over-lasso and ledger-without-progress keep stage and
  category and add the FR-645 reason; the wire test checks code, cause,
  reason, message and locus.
- The fresh-ledger admission in `temporal.rs` produces FR-641's returned
  ledger for a request with no `ledger`, and a refused run that supplied none
  writes none. Gating it on `run.sources` only affects the package route,
  which takes no ledger and returns none; the package route's capacity change
  to `DEFAULT_COVERED_POSITIONS` is therefore inert.
- `request_pointer` is written only by the `command::run` writer
  (`write_clause_run_result_for` with the selection), so the in-process record
  has none (FR-645-AC-5).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A model run whose expansion stops (candidate bound or meter) reports `pending`, exit 0, instead of `incomplete` naming the limit and its FR-255 setting | fsl-replay/src/spine/clause/model_trace.rs:206-211 |
| FND-002 | medium | The model-run candidate bound is hard-coded at 1,000,000; FR-255 says `expansion.max_candidates` has no default and the caller sets it, and no FR gives the run request a way to set it | fsl-replay/src/spine/clause/model_trace.rs:33-37 |
| FND-003 | high | A `model` trace with an empty loop is evaluated as an FR-328 finite prefix (reported `pending`) instead of refused `empty-lasso-loop` | fsl-replay/src/spine/clause/model_trace.rs:182-195 |
| FND-004 | medium | The model run's `decisive` record never evaluates the atom: quantifier, index and value path are not read and the observation identity is invented | fsl-replay/src/spine/clause/model_trace.rs:239-268 |
| FND-005 | medium | Universe snapshots are not admitted; a universe reference missing from the provisions, or one that does not parse, is silently dropped and shrinks the simulated population | fsl-replay/src/spine/clause/model_trace.rs:76-107 |
| FND-006 | medium | The monitor's `ClauseRefused` for an admit-stage trace refusal keeps only `reason`, dropping FR-645's `profile`, `expected`, `supplied` and message | fsl-analyze/src/monitor.rs:696-707 |
| FND-007 | medium | An empty `timestamped` trace moved from stage `admit` to `evaluate` with reason `empty-finite-trace`; FR-645's table does not list timestamped traces and says each refusal keeps its stage | fsl-replay/src/spine/clause/temporal.rs:764-768 |
| FND-008 | low | The result reader accepts any evaluate-stage `invalid_runtime_input` refusal carrying a `reason` field without checking cause, message or the reason token, and the tamper checks for this record were dropped | fsl-replay/src/spine/native_result.rs:1440-1454 |
| FND-009 | low | The kind-mismatch test still claims FSpec-TC-1161/FSpec-FR-541-AC-3 after the timed case moved to the ignored test | tests/it/run_wire.rs:455 |
| FND-010 | low | The window-aggregate refusal test is tagged FR-645-AC-3, which is the event-position-over-lasso kind mismatch, not this refusal | tests/it/run_wire.rs:1012 |
| FND-011 | low | A model run reports `ClauseRunUsage::default()` on every path, so its evaluation work is never reported | fsl-replay/src/spine/clause/model_trace.rs:226-232 |
| FND-012 | low | The half-open refusal names `trace.progress.watermark.half_open`, a path that omits the trace kind | fsl-replay/src/spine/request.rs:1526-1530 |

### FND-001

`evaluate_model_trace` returns `Ok(None)` when `replay_prefix` or
`loop_states_of` gets an `ExpansionStop` from `expand`
(`model_check/replay.rs:1467`, `:1957`). `run_model` maps that to
`ClauseDisposition::Temporal(TemporalVerdict::Pending)`. FR-330 maps
`Pending` to "completed without violation", exit 0. A resource stop is
`Incomplete` (exit 22), and FR-203/FR-126 name it
`LimitReached{max_candidates, value, expansion.max_candidates}` or the
exhausted `accounting.<counter>`. As written, a run that could not finish
reads as a clean pass. Fix: return the stop from `evaluate_model_trace` and
map it to `TemporalVerdict::Incomplete` with the FR-255 limit, value and
setting. Add a test with a small bound.

### FND-002

FR-255's engine table has `expansion.max_candidates | ... | none: the caller
sets it | FR-120`, and `EngineLimits::default()` keeps `expansion: None`. The
comment "it is the one a model check defaults to" is wrong. No FR governs a
default for a model-arm run, and neither FR-641 nor FR-312 gives the request a
member for it. Fix: this needs a spec decision. Either FR-641/FR-312 carries
the bound (for example through the request's limits, named
`expansion.max_candidates`), or FR-330 states the default for a model run.
Until then the constant is unspecified behaviour. Together with FND-001, a
stop at this bound is also invisible.

### FND-003

FR-645's table gives `empty-lasso-loop` for "a `lasso`, `model` or timed
lasso with an empty loop". FR-330 evaluates `Model` by FR-329, whose lasso
needs `L >= 1` (FR-329-AC-3). `run_model` builds `lasso: None` when `repeat`
is empty and there is no terminal stutter, and then evaluates a finite trace.
`a_model_trace_holds_or_stays_pending` asserts this `Pending`. Fix: refuse it
with `site.empty_loop()` at stage `evaluate`, and give `request_pointer` the
`selection.temporal.trace.model.loop` path. `request_pointer` currently
hard-codes `lasso.loop` for every kind (`temporal.rs:117`). Then restate the
test.

### FND-004

FR-330 "Outputs" says `decisive` holds the quantifier, element, index and
value path "all read by evaluating the atom over the observation at
`decisive.position`". For a `Model` trace, "the atom is evaluated over that
state as it is for a `Lasso`". `decisive_record` evaluates nothing. It always
builds `TemporalDecisive::undecided` with an invented
`ObservationIdentity{authority: "model-state", identity: "position-N"}`, and
it ignores the `_system` parameter. For a clause whose atom is decided by a
collection quantifier, the record says no quantifier decided it, which is
wrong. FR-330-AC-16's own clause has no quantifier, so it hides this.
Fix: evaluate the decisive atom over the replayed state with the clause
evaluator, as `with_decisive` does for snapshot traces. If a model state truly
has no observation identity, that is a spec question for FR-330, not a value
to invent.

### FND-005

`universes()` takes `filter_map(|r| run.provisions.snapshots.get(r))` and
`continue` on a serde failure. A universe reference that the provisions do not
hold, or a snapshot that does not parse, is ignored. The simulated population
is then smaller than the request named, and the verdict changes with no
refusal. The universe snapshots also never pass FR-106 admission: the `Model`
branch returns before the admission loop, and `complete` defaults to `false`
when absent. FR-330: "The entry SHALL admit every observation of the trace by
FR-106 ... and report the first admission failure at stage `admit`". Fix:
admit each universe snapshot, and refuse a missing or unreadable one at
`admit`.

### FND-006

FR-645 Behavior: "Each refusal above SHALL carry its `reason`, `profile`,
`message` and `locus`". FR-645's inputs include the monitor routes
(FR-643). The new arm builds `refused(InvalidRuntimeInput, "invalid-value",
{reason})`, so a monitor consumer cannot see which profile refused or what it
expected. FR-283-AC-13's code and cause are met. Fix: copy `profile`,
`expected` (as the writer does), `supplied` and `position` into the record's
fields, and carry the message where the monitor record has a slot for it.

### FND-007

`run_temporal` now refuses an empty `Timestamped` trace as
`TraceRefused(site.empty_trace("timestamped"))`, which is stage `evaluate`.
Before this change it was stage `admit`, and
`timestamped_completeness_may_run_ahead_and_an_empty_trace_is_refused` was
restated to match. FR-645's `empty-finite-trace` row covers "a `finite`,
`progressed` or finite `timed` trace", not `timestamped`. Its Behavior says a
refusal "SHALL keep its existing stage", and FR-330 never put an empty
timestamped trace at evaluate. This is not covered by the FR-267-AC-14
ruling, which is about the finite and lasso records. Fix: keep the empty
timestamped trace at stage `admit` (reason `empty-finite-trace` is fine), or
add timestamped to FR-645's table in QSL-2290 and record the stage there.

### FND-008

`validate_disposition` returns `Ok(())` for any evaluate-stage refusal whose
code is `invalid_runtime_input` and whose fields contain `reason`. It does not
check the cause `invalid-value`, that `reason` is one of FR-645's tokens, or
that `message` is present. The restated wire test dropped the two `refuses(...)`
tamper checks this record had. Fix: validate those three, and add one tamper
case back.

### FND-009

`a_trace_kind_the_profile_does_not_read_refuses_with_profile_and_kinds` still
carries `FSpec-TC-1161`/`FSpec-FR-541-AC-3`. That AC is "a timed claim over a
trace without stamps each refuse naming the defect", and the timed case now
lives in the ignored test. The matrix counts FR-541-AC-3 as covered by a test
that no longer asserts it. Fix: move both tags to
`a_timed_clause_over_untimed_samples_refuses_with_profile_and_kinds`.

### FND-010

`a_window_aggregate_over_a_progressed_sample_trace_refuses_and_never_faults`
is tagged `TC-1670`/`FR-645-AC-3`. FR-645-AC-3 is `Bounded` over a `lasso` and
a timestamped clause over `finite`; this test asserts neither. Fix: drop the
FR-645-AC-3 tag. The QSL-2138 refusal has no AC yet, so leave it untagged or
tag it once QSL-2138 specifies it.

### FND-011

Every `ModelRun` returns `ClauseRunUsage::default()`, including after
`evaluate_with_enabledness_decisive` ran on a `Meter`. FR-330's report carries
the usage, as FR-109 states it. Fix: return the meter's temporal visits and
the admission usage, as the snapshot route does.

### FND-012

FR-641 names refusals by the member's path from `clause`, for example
`selection.temporal.trace.timestamped.progress.watermark`. `progress_assertion`
does not know its trace kind, so it reports `trace.progress.watermark.half_open`
for both kinds. Fix: pass the kind's prefix in.

## Verdict

REQUEST CHANGES. The request wire, ledger file, FR-645 records on the
`command::run` path and the fixture split are sound. The new model-trace path
is not sound yet. FND-001 and FND-003 report `pending` (exit 0) for runs that
should report `incomplete` or refuse. FND-004 and FND-005 make the decisive
record and the universe silently wrong. FND-002 is a spec gap: FR-255 says the
caller sets the candidate bound, and FR-641 gives the caller no way to set it.
It needs a spec decision before the constant can stay.
