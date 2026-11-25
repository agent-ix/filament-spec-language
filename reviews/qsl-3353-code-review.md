---
id: SR-202611251525111
title: "code-review of PR 3936 state-graph certificate shape checks and no-initial-state order"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3936 head 8c4787bfd8f41e634840d81ac3f5473283bdaf1a; git diff origin/main...HEAD (fsl-replay certificate/state_graph.rs, certificate/closure.rs, probability_certificate/document/key.rs; fsl-analyze model_check/state_graph.rs, analyze/state_graph_item.rs; two it test files)"
review_set: subset
---

## Summary

Ticket: QSL-3353. This is a code review with the rust-review lane folded in. It covers soundness of the new `Malformed` shape checks in `check_state_graph`: the state-key domain and the other form's list. It also covers the order no-initial-state, then no-instance, in the analyze engine, both timed routes and the two certificate checkers, and the two test changes the PR discloses. The specs read are FR-169 Check and AC-14/15, FR-125 Instancing and AC-13, FR-339, and FSpec FR-361-AC-16 at 57c662b16e.

Tests were run at the head under lock C, by name only, in two runs:

- fsl-replay `--lib certificate::state_graph::tests::`: 3 passed.
- fsl-analyze `--test it`: 11 named tests passed, which are the 8 ticket reds, the 2 new order tests and `a_subject_with_no_initial_state_is_never_certified`. A separate run of the 3 NotLiveness and LivenessNeedsComponents tests also passed.

The merged tree with #3927 and #3908 was not built. Those PRs touch none of the changed functions, and `git merge-tree` is clean.

## Verdict

PASS for the code. Nothing that should be refused can now be accepted, and no valid certificate is refused. Each point below was traced:

- **State-key domain.** `is_model_state_key` requires an RFC 8785 parse, the owner envelope (`model_form`) and byte equality with the canonical re-encoding. Engine keys come from `ModelSystem::state_key`, so they pass. Any other bytes are `Malformed`, which is a rejection and never an acceptance. An allocation failure stops the check `ResourceExhausted`, and that is not an acceptance either.
- **The other form's list.** `Carried::of` maps possible and always possible to ranks, and unique path to counts. Any entry of the other list rejects at `entry(index)`, after the entries before it in stored order. This matches the locus rule and order in FR-169 Check. Engine certificates carry an empty list of the other form, so they are still accepted (AC-15, and the run above).
- **Order in `check_state_graph`.** Admission bounds and the instancing errors of `bound_over` (`NotSimulated`, `Fault`) still come first. Only `Ok` or `NoInstance` with an empty `initial()` becomes `NoInitialState`. This is FR-125 Instancing items 1, 2 and 4. `Malformed` still precedes all refusals, as it did on base (FR-169: before any recomputation).
- **Order in `check_system` (closure).** `NoInitialState` now comes after the unbounded-root refusal and before `NoInstance`, `NotLiveness` and `LivenessNeedsComponents`. On an empty instance list `NotLiveness` (`!all`) and `LivenessNeedsComponents` (`any`) cannot fire, so that case was only ever NoInstance against NoInitialState. Moving `NoInitialState` ahead of the two proof-form refusals affects only a clause with no `over` (one instance) over a subject with no initial state. Both outcomes are refusals. FR-339 does not order them, and FR-125 says each instancing outcome comes before exploration. The existing NotLiveness and LivenessNeedsComponents tests still pass.
- **Analyze engine.** `instances()`, the requires-bound refusal, is still computed before the `no_initial` branch. Every item now gets one item-level `NoInitialState` marker with `over: None`. Before, an over-less item got the same marker, and an `over` item had zero views and was marked `NoInstance`. `graph_settle` maps both to their inconclusive causes. `run_digital` and `run_timed` were swapped the same way.
- **Test changes.** Neither weakens a test. `certificates_reject_the_other_forms_list...` had passed `None` as the `over` binding for a claim with an `over` parameter, which `bound_over` must refuse (`NoInstance`). It now passes the real binding, and every expected value is unchanged. `malformed_entries_reject_before_recomputation` had ended by asserting that a certificate with both lists is well-formed, which contradicts FR-169-AC-15. That assertion became two, one per form with its own list. The both-lists case is now asserted as rejected by the new `the_other_forms_list_is_malformed_in_stored_order`.
- **Rust.** No new unwrap or panic in production paths. `u64::try_from(..).unwrap_or(u64::MAX)` is used only as a read limit. Cancel is polled per closure entry. There is one linear pass plus one re-encode per entry.

**Rulings on the spec points the author raised:**

1. FR-169 Check has a duplicated line, recorded as FND-001. It is low, sits on main and is outside this diff, so it does not block.
2. The spec names the locus `ModelState(StateKey)` and the code names it `CertificateLocus::ModelKey`. This is the locus wire-form question that QSL-3333 owns, so it gets no finding here and does not block.

The PR is mergeable at 8c4787bfd8f41e634840d81ac3f5473283bdaf1a.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-169 Check, Malformed bullet: the stray line "`Entry { index }` when the index names none;" repeats the end of the sentence before it. This is pre-existing on main and outside this diff, so it does not block. Fix it as a one-line spec edit, which can ride with QSL-3333 since that ticket edits the same paragraph. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:226 |

## Scope and evidence

- fsl-replay certificate/state_graph.rs `check_state_graph`, `bound_over`, `Carried`, `malformed`, unit tests (examined).
- fsl-replay certificate/closure.rs `check_system` refusal order (examined); check.rs `finish`, `poll`, `admit` (context_only).
- fsl-replay probability_certificate/document/key.rs `is_model_state_key`, `model_form` (examined).
- fsl-analyze model_check/state_graph.rs `check_state_graph`, `instances`, `no_initial_state` (examined); analyze/state_graph_item.rs `run_digital`, `run_timed` (examined); fsl-replay model_check/graph_settle.rs (context_only).
- Tests: `the_checker_refuses_no_initial_state_before_no_instance`, `a_clause_over_no_initial_state_is_refused_before_no_instance`, `certificates_reject_the_other_forms_list_and_accept_an_empty_one` (examined).
- FR-169 Check and AC-14, AC-15; FR-125 Instancing and AC-13 (examined); FR-339 refusals (context_only); FSpec FR-361-AC-16 (context_only).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | The closure checker now refuses `NoInitialState` before `NotLiveness`. For the one case where both apply, a component certificate offered for the deadlock-freedom item (one instance whatever the initial list holds) over a subject with no initial state, the settlement changes from `failed` (`NotLiveness`, via settle.rs `Row::of_check` `Err(_)`) to `inconclusive`/`no-initial-state`. Neither outcome proves anything. FR-125 Instancing orders only the instancing outcomes, and FR-339 does not order its form refusals against `NoInitialState`, so the spec does not decide this. No test covers it. It does not block. A one-sentence ruling in FR-339 would settle it. | fsl-replay/src/certificate/closure.rs:664-685, fsl-replay/src/model_check/item.rs:263, fsl-replay/src/model_check/settle.rs:973-977 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Moved to QSL-3333 (comment on QSL-3333 confirms), which edits the same FR-169 paragraph; pre-existing on main, outside this diff. |
| FND-002 | deferred | Not blocking: refusal against refusal, never `proved`; the spec leaves the order open, so the ruling belongs in FR-339 under its own ticket or QSL-3333's spec pass. The code lead decides whether to file it. |
