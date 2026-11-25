---
id: SR-967175026302197768900474181941935084851
title: "Code review of filament-spec-language PR 3902 (QSL-3281): FR-196 deadline monitor, dual nested monitor, replay clock bound"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@6920e185ee1aefd037d9ff4a67931c9314110843; PR #3902 diff against origin/main 93ac255c48; fsl-analyze/src/exact_probabilistic{,/digital.rs,/build.rs,...}, fsl-eval/src/probabilistic.rs, fsl-eval/src/temporal/{mod,timed}.rs, fsl-replay/src/probability_certificate/document/{key,timed}.rs, fsl-analyze/tests/it/{digital_clocks,exact_mdp_avoidance}.rs, fsl-analyze/tests/it/timed_product_certificate/bounded_monitor_profile.rs, fsl-semantics/tests/it/probabilistic_declarations.rs"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-196
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-204
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-498
---
# Code review of filament-spec-language PR #3902

## Summary

Ticket: QSL-3281. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (17 files, +354/-819) at head
6920e185ee1aefd037d9ff4a67931c9314110843. No cargo or tests were run by the
reviewer (brief); the author's run evidence in the PR body was read.

**(a) Removing `Form::TimedTrace`.** The history monitor was admitted only for
a root `eventually[0, D] holds(P)` with a closed interval and lower end 0, in
the producer (`Form::of`) and the checker (`bounded_event`). `Form::Deadline`
admits every closed bounded `eventually`/`always`/`until` with direct `holds`
operands, so that form is still admitted, and the checker's `bounded_event`
(certificate check) and `deadline_event` (witness replay) still accept it. No
admitted form is lost. The verdict is unchanged: for a lower-0 Eventually the
deadline monitor accepts at the first letter where P holds with t <= D (the
initial position included) and rejects on the first delay past D, before any
later letter. The history monitor got the same answer: it filled delay ticks
with the previous values (false whenever still pending), OR-merged letters at
the same timestamp and evaluated over observations <= D. Race jumps apply the
crossing before reading the endpoint in both. `scale_of` (horizon = upper,
lower 0) and `deadline_lower` (`Some(0)` vs `bound_units(0)`) agree. In the
checker, `Watch::Bounded` reads the letter and then applies the timeout inside
the pending block, so a letter past D cannot accept. The key admission's
4-element `timed-formula-trace` arm is removed in step with the producer, which
no longer emits it. No source, test or spec still references
`timed_history`/`TimedObservation` (analyze)/`evaluate_timed_formula_at`.

**(b) Dual nested monitor.** `advance_nested_timed_deadline` runs
`always[a,b] eventually[d,d] P` as the complement of
`eventually[a,b] always[d,d] (not P)`: it complements both stored phases
(`Pending` is fixed, `Accept`/`Reject` swap, an involution), advances on
`!holds` and complements back. That is the exact formula duality, so the dual
is as exact as the existing eventually-then-always monitor. The d = 0 case
(last same-time hop decides) and the d > 0 case (any true observation at the
target time) are the correct duals. Producer, checker and key admission accept
exactly the two pairs; same-operator nesting still settles `TimedFormShape` at
the same locus as before. Both new FR-204-AC-21 tests have hand-derivable
values (1 and 0 over `rise_and_fall`, 0 for the zero-offset last hop) and fail
if the complement is dropped or the polarity is wrong. Two problems remain:
the monitor is shared by producer and checker (FND-002), and the producer's
race edges skip timestamps the checker observes (FND-001). Both are
pre-existing for the eventually-then-always order on main, and this PR extends
them to the dual.

**(c) Replay clock bound.** `replay_timed_witness` now passes
`watch.horizon()` (the claim's `timed_horizon`) for `Bounded`/`Nested`
watches, which is what the certificate check passes (`timed.rs:160,174`) and
what the producer uses (`deadline_bound`: outer upper + inner upper for
nested). For single-operator `Bounded` watches the horizon equals the root
upper bound `time_bound` used before, so only nested watches change. `Deadline`
and `Elapsed` keep `time_bound`. Correct.

**(d) Tests.** `timed_eventually_zero_delay_cycle_has_a_finite_repeated_key`
keeps every structural assertion: at least two distinct keys, a zero-elapsed
edge between them closing a cycle, and no accepting state (now "every monitor
is Eventually Pending|Reject"). Each can still fail. The replaced
FR-498-AC-8 test asserts an exact value 9/10, a `Refuted` terminal at 0.99 and
the locus member set (see FND-003). The retag to AC-3 fits: that test asserts
the cause carries the selected claim's identity, which is AC-3's "a
`TimedFormShape` cause carries the same locus". No assertion was weakened.

**(e) Trace tags.** All tags name real ids. `quire matrix` binds FR-498-AC-3,
AC-8, AC-9 and FR-204-AC-21 to the new and retagged tests (status `tagged`).

Not a finding, but an evidence gap: the PR body reports
`always[0 ms, 4 ms] holds(c.late) >= 0.99` over `Retx` settling
`inconclusive`/`ReplayParity` "on this branch only; not compared with main".
The code paths for a single root Always are unchanged by this diff (same
`Form::Deadline` arm, same key, and replay `clock_bound == time_bound` for a
single operator), so it is very likely pre-existing. That still needs a main
run to confirm.

## Verdict

PASS with two medium and one low finding, none introduced as a regression of
an existing verdict. (a), (c), (d) and (e) hold. The dual monitor (b) is
logically exact relative to the existing nested monitor. FND-001 and FND-002
are shared with main's eventually-then-always path. The checker steps every
tick, so FND-001 shows up as an `inconclusive` item, never as a wrong `proved`.
The lead decides whether they are fixed here or ticketed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The producer's race edge advances the nested (and now dual) monitor once, at the endpoint of a k-unit drawn delay (probe at `digital.rs:2219-2227`, `control_after` at `digital.rs:1570-1590`). The non-decided branch also omits the source letter at the endpoint. The checker observes every tick and the pre-step state (`digital_state_after_delay`, `timed.rs:4245-4269`). When a drawn delay of 2 or more units spans a target timestamp in [a+d, b+d], the producer and checker monitors diverge. Example: `always[0,0] eventually[2,2] P` from a not-P state with a 3-unit draw. The producer advances once at elapsed 3, finds no observation in the window, so the inner Eventually run rejects and the complement accepts. The checker observes not-P at t=2 and rejects. The certificate then fails and the item is not decided exactly, contrary to FR-204-AC-21. This is pre-existing for eventually-then-always. | fsl-analyze/src/exact_probabilistic/digital.rs:2219-2227, fsl-analyze/src/exact_probabilistic/digital.rs:1570-1590, fsl-replay/src/probability_certificate/document/timed.rs:4245-4269 |
| FND-002 | medium | The checker is not independent of the producer for nested windows. Both call `fsl_eval::probabilistic::advance_nested_timed_deadline` (`digital.rs:1517,1583`, `timed.rs:1693`), and the new complement dispatch at `probabilistic.rs:147-162` is a common-mode point: a polarity or ordering defect there is reproduced by the checker and certified. Only the hand-derived values in the three nested FR-204-AC-21 tests guard it. | fsl-eval/src/probabilistic.rs:147-162, fsl-replay/src/probability_certificate/document/timed.rs:1693 |
| FND-003 | low | The FR-498-AC-8 test computes the wire locus from the package claim identity (`claim_locus_wire(&identity)`), not from the settled record. A settlement that reported a different locus would not fail it. AC-8 asks for "the selected claim's FSpec FR-331 wire `locus`" of the settled claim. | fsl-analyze/tests/it/digital_clocks.rs:1711-1729 |
