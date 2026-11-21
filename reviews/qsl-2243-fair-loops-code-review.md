---
id: SR-20261121181402001
title: "code-review of PR3357 exact timed fair-loop search for hidden-field refinement liveness"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3357 head e0487871c326d18465a53e299981acc0525a39f8 against base 49f9e7a18cce9e02769ce0a44e4ed7e69f272bdd; fsl-analyze/src/model_check/fair_summary.rs, fsl-analyze/src/model_check/mod.rs, fsl-analyze/src/model_check/refine.rs, fsl-analyze/src/zone_check/refine/live.rs, fsl-analyze/src/zone_check/refine/live/fair_loops.rs, fsl-analyze/tests/it/timed_refinement_liveness.rs"
review_set: subset
---

## Summary

Ticket: QSL-2243. This is a soundness and completeness review of the diff, with the rust-review lane folded in. It covers FR-899's every-concrete-fair-loop obligation (AC-7, AC-9 and "Timed subjects") for the fourth remainder only. The new private walk-summary search over the zone product is sound for refutation: the antichain direction, the acceptance marks, concrete weak and strong fairness, and the consecutive-powers idempotent search are all correct. It is also exact for Holds, as far as source reading can establish. Three gaps remain. Unlike the untimed search, nothing checks the Holds direction against the independent candidate-graph check. The budget integration test can only stop in the liveness product build. A non-concretising unfair loop ends the search, so a concretising one can go unreported. No Cargo, clippy, fmt or test run was allowed by the brief, and none was made. Runtime qualification has not happened.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The timed search checks the walk summary against candidate_loop_has_fair_run only when the summary finds no fair candidate. The untimed search also faults when the summary finds a fair candidate that the lasso check refutes. A summary, composition or power defect therefore becomes a silent wrong Holds, and the liveness Holds is uncertified | fsl-analyze/src/zone_check/refine/live/fair_loops.rs:179-204, fsl-analyze/src/model_check/refine.rs:2773-2787 |
| FND-002 | medium | The budget test sets max_symbolic_transitions to the liveness arc count + 1, but safety edges and liveness arcs share one Run budget. The liveness half therefore stops while add_arc builds the product, before the walk-summary search is reached. The test passes whether or not the search charges its work | fsl-analyze/tests/it/timed_refinement_liveness.rs:1111-1131, fsl-analyze/src/zone_check/refine/live.rs:1285-1290, fsl-analyze/src/zone_check/search.rs:1632-1640 |
| FND-003 | medium | unfair_candidate_loop returns the first unfair closed walk it finds. If that walk's lasso does not concretise, decide_live marks not_concretised and moves to the next component, so another unfair loop of the same component that would concretise is never tried. The result is LassoNotConcretized where a replayable HiddenUnfair exists | fsl-analyze/src/zone_check/refine/live.rs:308-340, fsl-analyze/src/zone_check/refine/live/fair_loops.rs:183-204 |
| FND-004 | low | With a hidden field, fair_model is the same for every abstract class (no class edge filter, within = all vertices, no strong set). The full-scope search still runs once per class, so the same component is searched and charged up to once per class. A decidable Holds can reach MaxSymbolicTransitions sooner | fsl-analyze/src/zone_check/refine/live.rs:268-320, fsl-analyze/src/zone_check/refine/live.rs:984-1069 |
| FND-005 | low | candidates_at is a third copy of the expand_state stop, fault and undetermined mapping, after abstract_enabled and candidate_loop_has_fair_run. arc_relation's CandidateSteps and Stutter arms repeat the untimed candidate_edge_relation's labelling rather than sharing a pure helper | fsl-analyze/src/zone_check/refine/live/fair_loops.rs:223-280, fsl-analyze/src/zone_check/refine/live/fair_loops.rs:303-372, fsl-analyze/src/zone_check/refine/live.rs:667-718 |

## Scope and evidence

I read the whole diff and these call sites: decide_live, fair_model, abstract_enabled, candidate_loop_has_fair_run, first_unfair_constraint, lasso, add_arc and the idle-tail rule in live.rs. I also read check_timed_refinement's shared Run and HiddenFieldTimedAbstractLiveness admission in zone_check/refine.rs, charge_edge and escape_edge in search.rs, and the untimed walk-summary search, edge labels and idempotent_power in model_check/refine.rs. Governing text: FR-899-AC-7 and FR-899-AC-9.

What was checked and found sound:

- The antichain direction keeps a summary that is concretely at least as fair and has fewer or harder candidate paths, and is monotone under composition.
- Every walk vertex is the source of one edge, so its acceptance marks (divergence, position) are each set by the source vertex.
- Concrete weak fairness is taken or not enabled everywhere. Concrete strong fairness is taken or not enabled anywhere.
- With a hidden field, abstract strong fairness is decided by candidate labels and not by a fair_model acceptance set.
- The candidate sets keyed by q match positions[q].abstract_states, which is valid under the current untimed-abstract admission guard.
- The consecutive-powers search P := P.R returns the first idempotent R^k. R^k is eventually periodic, so it terminates. The 3-cycle unit test shows the squaring defect.
- Every composition is charged through charge_edge and escape_edge. Exhaustion is Stopped and never Holds.
- Composition allocates before its work is charged. The overshoot is bounded by one composition, and the untimed search does the same, so this is not a defect.
- Concretisation is FR-241 timed_lasso, which enforces positive total delay. Replay is asserted in the hiding test.
- The extracted fair_summary helpers are byte-identical moves. The untimed idempotent_power is unchanged.
- The live.rs change is limited to the one hidden branch, and no untimed engine logic was rewritten.

Rust lane: the nested Result<Result<_, Decided>, Halt> and inline btree_map::Entry match the module's idiom. The doc comments agree with the code. Every new test has a trace tag. Gates (fmt, clippy, tests, deny) were not run because the brief forbids Cargo. The coder reports a compile-only exit 0.

Out of diff and pre-existing, so not a finding here: the untimed model_check/refine.rs idempotent_power (3045-3056) still squares. On a candidate 3-cycle it alternates between R and R^2 until the budget runs out, so it can never decide Holds.

## Verdict

Source: CONDITIONAL. There are no high findings, and the refutation path is sound. FND-001 to FND-003 should be fixed or disposed before merge. FND-001 restores an independent gate on the false-Holds direction. FND-002 makes the budget oracle reach the search. Test qualification is pending, since no runtime evidence exists yet.

Integration conditions with PR3362 (do not rebase until it merges):

- Replace the q-keyed candidates_at prefill with PR3362's per-vertex, per-candidate clock-valuation and guard-split enabledness provider. The search reads candidates only through candidates_of.
- The search's FairScope and naming must stay on first_unfair_constraint.
- Re-run all seven focused tests on the merged parent.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The docs overclaim completeness. Refuted::NotConcretised says "Such walks exist and none concretises", and refute_hidden_component says it ends "with every walk examined". The antichain drops a walk prefix whose summary a retained prefix subsumes, and concretisability depends on path history the summary does not keep, so an unfair loop under a dominated prefix is never tried. The result is a sound LassoNotConcretized (Undecided), not a proof that no unfair loop concretises | fsl-analyze/src/zone_check/refine/live/fair_loops.rs:54, fsl-analyze/src/zone_check/refine/live/fair_loops.rs:66-71, fsl-analyze/src/zone_check/refine/live/fair_loops.rs:218 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | still-open | Doc-only correction, which the root can check mechanically. In fair_loops.rs, reword line 54 to say that every examined walk fails to concretise and that walks under a subsumed prefix are not tried. Reword lines 66-71 to say the search ends with every retained walk examined. No code change is needed |
| FND-006 | fixed | - |
| FND-007 | fixed | - |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | At 98c8a13f, two_loop_world admitted a quiescent initial state with no time invariant, so a concrete behaviour could idle for ever taking neither tock nor send. That idle tail genuinely violates weak idle at h=true and weak send at h=false, so the strict Holds and budget oracles failed on a fixture defect, not a product defect | fsl-analyze/tests/it/timed_refinement_liveness.rs:1225-1284, fsl-eval/src/simulation/model/timed.rs:1796, fsl-analyze/tests/it/timed_refinement_liveness.rs:526 |
