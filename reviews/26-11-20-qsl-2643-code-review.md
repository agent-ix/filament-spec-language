---
id: SR-2026112100331600000000001
title: "Code review — QSL-2643 PR #3231 event-position hyper clauses over closed finite traces"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3231 diff at 768d5a6012 (merge base f7b1d41bce): fsl-eval/src/hyper/{closed,mod}.rs, fsl-eval/src/temporal/{mod,tuple}.rs, fsl-analyze/src/analyze{,/document,/hyper_route,/items,/request}.rs, fsl-analyze/src/model_check/hyper{,/closed,/projected,/single}.rs, fsl-replay/src/model_check/{existential_outcome,hyper_counterexample,hyper_outcome,hyper_replay,hyper_settle}.rs, fsl-replay/src/{lib,outcome,proof_result}.rs, fsl-replay/src/outcome/counterexample.rs, fsl-semantics/src/check/hyper_clause.rs, tests fsl-analyze/tests/it/{hyper_closed,hyper_world,analyze_family_cli,main}.rs, fsl-semantics/tests/it/hyper_clauses.rs"
review_set: subset
evaluated_revision: "768d5a6012"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-173
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
---

## Summary

Ticket: QSL-2643. I reviewed PR #3231 at head `768d5a6012` against its merge base with `origin/main` (`f7b1d41bce`). I checked it against FSL FR-173 ("Event-position form", AC-3, AC-6), FR-640-AC-12, ADR-023 HS-7/HM-2/HV-6, ADR-014 TR-4, ADR-018 CX-1, and the arch ruling on the ticket: the event-position form reads closed finite traces, with one clock configuration per trace and no progressing lasso; it reaches a verdict and replays.

These parts are correct:

- `closed_executions` (fsl-eval/src/hyper/closed.rs:120-181) stops a path either at a terminal state or at exactly `h + 1` positions. It never stutter-extends and never builds a lasso. Successors are read in transition-key and state-key order. Passing the execution limit returns `ClosedStop::Executions`, which the engine settles as `MaxStates` incomplete, not as a verdict.
- `evaluate_closed_tuple` (fsl-eval/src/temporal/tuple.rs:428-531) reads every atom that touches a component past its last position as false (FR-043). It refuses `align skip`, an empty trace and an arity mismatch.
- HP-2 and HP-3 closed decisions (fsl-analyze/src/model_check/hyper/closed.rs) are refuted only by a tuple whose counterexample is built from the same executions. Replay (`hyper_replay.rs::closed`) re-executes each trace, refuses `TerminalStutter`, refuses a trace that neither covers the horizon nor ends at a terminal state, recomputes the verdict with the same `fsl-eval` code, and reproduces only when `trace_position` agrees. A refutation without a reproduced counterexample is refused by `TerminalRecord::of_hyper`. `max_transitions` (tuples) and `max_witness_set` (existential product) stop the run as incomplete.
- AC-3: event-position `NonInterference` records `Universal`, and fixed-sample records no form (hyper_clauses.rs test, now tagged `FR-173-AC-3`).
- The new tests use real fixtures and can fail. No `#[allow]` attributes, stubs or tautological assertions were added in production code.

Four things are wrong:

- The horizon of an unbounded operator is 0, so an unbounded body is decided on one-position traces and can settle a wrong `proved`.
- HP-6 settles `HoldsClosed` from the infinite-trace projected product, which stutter-extends terminal states.
- HP-5 treats an incomplete or undetermined evaluation as "no witness" and so reports `Trapped`.
- No trace is read under its own clock configuration.

## Verdict

**NOT MERGEABLE — fix round needed.** There are three highs (FND-001 to FND-003), four mediums (FND-004 to FND-007) and two lows (FND-008, FND-009). These are outside the declared gaps: HP-5 route unbuilt, infinite-trace `Holds` settlement unbuilt, settlement errors mapping to `Failed(BROKEN)`, and the 7 pre-existing hyper_single/projected/swap reds.

## Rust review

| Check | Result |
| --- | --- |
| Behaviour vs spec | Closed enumeration and the HP-2/HP-3 closed judging match FR-173. Unbounded bodies (FND-001), HP-6 stutter (FND-002), HP-5 incomplete (FND-003), per-trace clock (FND-004) and per-conjunct `match` (FND-005) do not. |
| Error/limit mapping | Cancellation inside the HP-3 partner search becomes an internal fault (FND-006). Replay's existential limit differs from the engine's (FND-008). |
| Tests | Real fixtures, tagged `#[trace("TC-2243", "FR-173-AC-6")]`. The FR-640-AC-12 CLI test accepts either verdict, and the edge cases are untested (FND-007). |
| Safety/panics | No `unsafe`, no `#[allow]` and no production `unwrap`/`expect`. Integer conversions use `try_from` with saturating fallbacks. |
| Comments | One stale comment (FND-009). |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `horizon` adds 0 for an operator with no finite upper bound, and event-position hyper bodies are checked `bounded: false`, so `always p` over a non-terminal subject is decided on one-position traces and settles a wrong `proved` (ClosedScope) | fsl-eval/src/hyper/closed.rs:72-80; fsl-semantics/src/check/hyper_clause.rs:1364-1372, 1549-1562 |
| FND-002 | high | Event-position HP-6 settles `HoldsClosed` (`proved`, ClosedScope) from the projected product, which stutter-extends terminal states (`joint.rs:287-288`), against FR-173's closed reading | fsl-analyze/src/model_check/hyper/projected.rs:45-54; fsl-eval/src/hyper/joint.rs:287-288 |
| FND-003 | high | HP-5 closed search drops `Incomplete`, `MatchUndetermined` and cancellation into `_ => {}`, so a run that ran out of budget reports `Trapped` instead of `NoDecision` | fsl-analyze/src/model_check/hyper/single.rs:1336-1356 |
| FND-004 | medium | No trace is read under its own `temporal_bindings` clock configuration: the route builds one `ModelSystem` for every alias, and neither the engine nor the replay reads the per-trace rows | fsl-analyze/src/analyze/hyper_route.rs:66-70; fsl-replay/src/model_check/hyper_replay.rs:1392-1548 |
| FND-005 | medium | `step_reading` skips the whole `μ` part when any component lacks the joint step, instead of skipping only the conjuncts that read that component | fsl-eval/src/hyper/closed.rs:272-301, 326-339 |
| FND-006 | medium | Cancellation inside `search_partner` returns `Stopped(Fault)`, which `stopped_reading` maps to `ModelCheckRefusal::Fault` (BROKEN), not `cancelled` | fsl-eval/src/hyper/closed.rs:451-453; fsl-analyze/src/model_check/hyper/closed.rs:213 |
| FND-007 | medium | The FR-640-AC-12 CLI test accepts `proved` for a clause AC-6 says is refuted. No test covers a terminal state inside the horizon, an unbounded operator, limits settling incomplete, cancellation or HP-5 incomplete. The not-closed replay test asserts only `is_err()` | fsl-analyze/tests/it/analyze_family_cli.rs:907; fsl-analyze/tests/it/hyper_closed.rs:139 |
| FND-008 | low | HP-3 replay bounds the existential execution enumeration by `max_witness_set`, but the engine bounds it by `max_states`, so a valid refutation can fail to reproduce. Replay also accepts a terminal trace longer than the horizon | fsl-replay/src/model_check/hyper_replay.rs:1445, 1503; fsl-analyze/src/model_check/hyper/closed.rs:63 |
| FND-009 | low | The comment says a clause under another profile "records its profile and no form, so no bounded-profile rule applies", which is now false for event-position | fsl-semantics/src/check/hyper_clause.rs:1364-1366 |

## Findings detail

- **FND-001.** ADR-014 TR-4 defines the horizon as "the greatest reach of a bounded-profile formula's intervals", and FR-173 says a closed trace "covers the body's ADR-014 TR-4 horizon from every activation the body reads". `horizon` maps `TemporalUpper::Open` and `None` to 0. The hyper checker reads intervals with `IntervalReading::Steps { bounded: false }` under every untimed profile, so FR-326's bounded-profile refusal of a bare operator never runs for a hyper body. The engine's `Window::of(None)` is `[0, *]` over the finite trace. QSL-2277's own fixture uses such a body: `always holds(d.l @ s != e.l @ n)` (fsl-replay/tests/it/hyper_per_trace_clock.rs).
  - **Failure.** Take `forall trace a of V forall trace b of N { always holds(v.l @ a = w.l @ b) }` over AC-6's Secure vaults. `h = 0`, so each trace is the initial state alone, where `l = 0 = 0`. The run returns `HoldsClosed`, which settles `proved`/Exhaustive/ClosedScope, but `step(1)` breaks it.
  - **Nested case.** `always [0,3] eventually p` gets `h = 3` and can be refuted on the truncation. That refutation "replays" only because the replay uses the same horizon.
  - **Fix.** Under event-position, refuse a hyper body operator with no finite upper bound (FR-326 "Bounded profiles"), or make `horizon` return `None` for one so that `decide_closed` refuses. Add tests for both cases.
- **FND-002.** FR-173: "A terminal state is not stutter-extended ... every atom that reads that component there evaluates false". The HP-6 path reuses the infinite-trace projected product, where a terminal state gets `ComponentStep::Stutter`. The PR then labels its exhaustion `HoldsClosed`, claiming "an event-position body is bounded, so its monitor product is complete and closed". Neither part holds: the body may be unbounded (FND-001), and the product is not the closed reading.
  - **Failure.** Use an aligned subject that reaches a terminal state at visible position 1, with body `always [0,2] holds(v.l @ a = v.l @ b)`. Under stutter the body holds, giving `proved`/ClosedScope. Under the closed reading position 2 is past both ends, the atom is false, and the clause is refuted.
  - **Counterexamples.** A projected counterexample under event-position may also carry `TerminalStutter` steps. The projected replay does not reject them for this profile.
  - **Fix.** Decide event-position HP-6 over closed executions, as HP-2/HP-3 do, or keep it undecided until that exists. Do not emit `HoldsClosed` from the stuttering product.
- **FND-003.** In `decide_closed` for HP-5, only `Holds`, `Undefined` and `Fault` are handled. `Incomplete(_)`, which includes a meter cancelled through `with_cancel`, and `MatchUndetermined` fall through `_ => {}`, as if the candidate were not a witness. When every candidate from an initial state runs out of budget, `witness` stays `None` and the outcome is `ExistentialOutcome::Trapped`, a refutation (SE-5), where HV-5 requires incomplete. The HP-5 route is unbuilt (declared), but this is the engine's own verdict, so wiring the route later inherits it.
  - **Fix.** Map `Incomplete` and cancellation to `NoDecision` with the meter or cancel cause. Only `Violated` and `ExistentialUnmatched` mean "not a witness".
- **FND-004.** FR-173: "Each trace is read under its own clock configuration: the `event_position` configuration of the `temporal_bindings` row ... Two traces over two models are read under two configurations". AC-6: "`fsl_replay::replay` reads each as a closed trace under its own `temporal_bindings` row". In this PR, `hyper_route` builds one `ModelSystem` from the request's combined subject and passes `vec![&system; n]`, and `HyperReplay` gets the same vector. Nothing in the engine, the route or the replay reads the rows that QSL-2277 emits.
  - **Consequence.** Each alias also ranges over the executions of the combined subject (both models' operations), not of its own model. A trace "of V" can take N's steps, which act as stutters for V.
  - **Fix.** Build one system per alias from its own model subject and that trace's row, and replay with the same per-trace systems. Add an assertion that the replay used both rows.
  - **Confidence.** Medium: the combined subject may be intentional, but the spec text requires per-trace configuration.
- **FND-005.** FR-173: "a `match` conjunct that reads that component's step label is not evaluated there, so `μ` constrains only the joint steps every component it reads takes". `step_reading` returns `Ok(None)` for the whole part as soon as any component in the part has no step at `joint`.
  - **Failure.** Take HP-2 with `forall a, b, c` and a `μ_U` conjunct over `a` and `b` only. If `c` terminates after one step while `a` and `b` continue with different inputs, the conjunct is never evaluated at step 2. The tuple is admitted and can produce a refutation that μ should have excluded, and it replays, because the replay shares the code.
  - **Fix.** Evaluate per conjunct, using the components that conjunct reads.
- **FND-006.** `search_partner` returns `PartnerSearch::Stopped(closed_fault("a-cancelled-search-stops"))` on cancel. `partner_reading` passes it to `stopped_reading`, whose `ClosedVerdict::Fault` arm returns `Err(ModelCheckRefusal::Fault)` before the `_` arm that checks `self.cancel`. A cancelled HP-3 run is therefore reported as an internal failure (BROKEN), not as HV-5 `cancelled`.
  - **Fix.** Add a `PartnerSearch::Cancelled` variant and map it to `self.cancelled(totals)`.
- **FND-007.** `a_hyper_clause_over_two_models_analyzes_and_replays_per_trace` asserts `matches!(verdict, "proved" | "refuted")`. The clause is AC-6's HP-2 clause, which settles `refuted`, so a wrong `proved` passes. The test also does not check that each trace was replayed under its own row.
  - **Missing cases.** No test covers:
    - a subject with a terminal state inside the horizon (the false-extension path)
    - an unbounded operator (FND-001)
    - `max_states`, `max_transitions` or `max_witness_set` settling incomplete
    - cancellation
    - HP-5 with an incomplete evaluation
  - **Weak assertion.** `replay_refuses_a_trace_that_is_not_closed` asserts `is_err()`, which any refusal satisfies.
  - **Fix.** Assert `refuted` and `reproduced`, add the missing cases, and match the refusal's message.
- **FND-008.** The engine enumerates existential executions with `max_states` as the limit (`closed.rs:63`). Replay uses `max_witness_set` as the limit for the same enumeration (`hyper_replay.rs:1503`), then again as the product limit. When the number of executions is above `max_witness_set` but below `max_states`, an engine refutation replays as `Stopped` and is not accepted. That direction is safe, but it is a mismatch. The `covers` check is `taken == horizon`, so a terminal trace with more than `h` steps is accepted, a shape the engine never emits.
- **FND-009.** The comment at hyper_clause.rs:1364-1366 predates the event-position form and now misdescribes the code. It is also where the bounded-profile rule should apply (FND-001).
