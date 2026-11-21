---
id: SR-20261121032700
title: "code review of PR3250 window aggregates over a timestamped sample trace through fsl run"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-eval/src/temporal/observed.rs, fsl-eval/src/temporal/observed_aggregate.rs, fsl-eval/src/temporal/finite_window/trace.rs, fsl-eval/src/temporal/mod.rs, fsl-eval/tests/it/window_aggregates.rs, fsl-replay/src/spine/clause/temporal.rs, tests/it/clock_reach.rs; PR3250 head 950d2e1b5031cf7f25e2ea15782c4b58f7a775a8"
review_set: subset
---

## Summary

Ticket: QSL-2138. Code-review with the rust-review lane folded in. Diff only, read at the PR head with `git show`, no build. The batch timestamped verdict path is correct: window membership, anchor cut, units, every kind's arithmetic, `min_count`, undefined handling and visit charging all match FR-194 "Evaluation" and reuse the event-position evaluator's fold. Two medium defects. First, the widened spine guard also lets in the FR-820 carried clause-state route, where the new reader faults internally. Second, the checker refuses a `rational(n, d)` fraction threshold as `ambiguous-literal`, against FR-194, and the PR's test edit hides that refusal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new guard exemption also admits a carried clause-state run (FR-820, `Reading::Observed`, `tape` Some) over a closed timestamped sample trace. There `read_sample_letter` keeps an aggregate clause's samples `Held`, the positions are `Tape` and `CarriedWindow` builds `ObservedLetters` with `windows: None`. So `read_aggregate` stops with InternalFault `aggregate-reads-samples-with-a-stamped-trace`, an EvaluateFault where a typed `unsupported-window-aggregate` refusal stood | fsl-replay/src/spine/clause/temporal.rs:1940-1941; fsl-eval/src/temporal/observed.rs:689, 1139-1144 |
| FND-002 | medium | FR-194 binds a `fraction` threshold to family `rational` and checks `n/d` "as the rational literal `rational(n, d)`". But the checker types a non-`n/d` fraction threshold with `Goal::Threshold(None)` (no expected type), so `>= rational(9, 10)` refuses `ill_typed`/`ambiguous-literal`. The PR rewrote the test's thresholds to `9/10` and `19/20` rather than fix the checker | fsl-semantics/src/check/temporal_clause/condition.rs:314-327; tests/it/clock_reach.rs:233-238 |
| FND-003 | low | AC-1's threshold `0.005 s` (seconds against an `ms` value) is never run on the timestamped route. That conversion is the one unit step the new `magnitude()` comparison depends on | tests/it/clock_reach.rs:196-258 |

## Verdict

Not mergeable until FND-001 and FND-002 are fixed (FND-003 is cheap; add it in the same round).

- FND-001: add `&& tape.is_none()` to the new `TraceInput::Timestamped` exemption so a carried state keeps the typed refusal. The other option is to make `read_aggregate` read `Held` tape samples and give `CarriedWindow::evaluate` the held trace's `timed_window_trace()`, then add a clause-state test.
- FND-002: in `exact_fraction` (or just before it), also accept a root `ExprNode::Rational(n, d)` and type it at its point domain as the `n/d` path does. Then restore one row in `every_window_aggregate_kind_reaches_a_verdict_from_source` that uses `rational(9, 10)`, keeping the `9/10` row.
- FND-003: add `quantile 0.95 ... <= 0.005 s` (violated, 10) and `quantile 0.9 ... <= 0.005 s` (holds, 0) rows to that test.

Checked clean:

- **Membership, closed ends.** `WindowTrace::window` Duration: `lo = partition_point(< now - b)`, `hi = partition_point(<= now - a)` over `stamps[..=i]`: `t_i - b <= t_j <= t_i - a`, closed both ends, nothing after `i`.
- **Same stamp.** FR-194: "the events whose time stamp `t_j` satisfies `t_i − b <= t_j <= t_i − a`. No window SHALL read a position after `i`." The slice `..=index` keeps only same-stamp events at or before the anchor, which is what FR-194 says. FR-501 calls same-stamp observations "readings of one instant", which is in tension with this, but FR-194's explicit rule governs.
- **Units.** The checker converts both bounds exactly into `units.clock_unit(timestamp_unit)` of the selected `TimestampedEvent` clock (temporal_clause.rs:1564-1612). The trace stamps are integer ticks of that same clock's `timestamp_unit`, and the spine checks the trace binding against the clause clock (FR-522). So stamps and bounds share one unit at the comparison, including `past[0 ms, 300 s]` (300 s becomes 300000 ms). The no-clock fallback (lower bound's unit) is refused by the emitter, so no run reaches it.
- **Value vs threshold.** `infer_in_unit_of` wraps a threshold of another unit of the value's dimension in an exact `convert` to the value's unit (typing.rs:978-999). Any other mismatch is refused `unit-mismatch` at check, and the value expression has one static unit. So comparing `magnitude()` values compares like units.
- **Kinds.** count, sum, min, max, fraction and quantile all go through the shared `AggregateAtom`/`Gathered` fold the event-position route uses. Quantile is `nearest_rank = ceil(q * n)` on exact `Rational`, with no floats. The only per-route difference is the `WindowSource`, which reads the same filter, value and predicate bodies with the same bindings (subject, trigger, captures) as `SubjectWindowSource`.
- **min_count, empty, undefined.** `count < min_count` gives `InsufficientData{count, min_count}`, and it is not noted as an undefined evaluation. Undefined filter, value or predicate returns the first cause oldest-first and is noted. An undefined threshold is noted.
- **Limit and work.** The overflow check comes after the read, as on the event route. `DEFAULT_MAX_WINDOW_VALUES` matches the event-position `fsl run` route (temporal.rs:1974), which also passes the default. Each window position charges `temporal.visit` exactly once: through the `visit` closure, with `charge_meter: false`.

## Scope and evidence

Read the PR diff and, at the PR head, window.rs (`WindowTrace::window`, `evaluate_observed_with_visits`, `nearest_rank`, `of_checked`), observed.rs (`atom`, `taped`, `CarriedWindow::evaluate`), tape.rs `read_sample_letter`/`read_letter`, clause_state.rs reading selection and tape filling, temporal.rs guard, Timestamped route and `run_window`, trace_subject.rs `SubjectWindowSource`, temporal_clause.rs `DurationWindow` and clock-unit conversion, condition.rs threshold typing and `exact_fraction`, and typing.rs `infer_in_unit_of`. Specs read: FR-194, FR-093 "Decimal constructors", FR-172-AC-6, FR-501 "Verdicts", FR-330 "Trace-subject route behavior" and FR-820. No cargo. Gap-analysis not run (one proportionate pass per the brief; the AC check is folded in here).

- FR-194 Evaluation (spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md): examined
- FR-194 threshold rule (spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md): examined
- FR-194-AC-1: examined
- FR-194-AC-3: examined
- FR-194-AC-13: examined
- FR-093 Decimal constructors: context_only
- FR-501 Verdicts: context_only
- FR-820 Summarized positions: context_only
