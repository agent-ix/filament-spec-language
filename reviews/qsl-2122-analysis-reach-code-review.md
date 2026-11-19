---
id: SR-2026111610120001
title: "code review of PR 2743 FSL analysis reach"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 2743 head 39b65c91fc938a547ce59cf47de935203db19e10; fsl-analyze/src/analyze/{document,request,items,clause_item,derived_item,reduction_item,state_graph_item,timed_item}.rs, fsl-analyze/src/analyze.rs, fsl-analyze/src/model_check/manifest.rs, fsl-replay/src/{outcome.rs,outcome/counterexample.rs,proof_result.rs,lib.rs}, fsl-route/src/lib.rs, fsl-analyze/tests/it/*"
review_set: subset
evaluated_revision: "39b65c91fc938a547ce59cf47de935203db19e10"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-623
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-626
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-627
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-628
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-632
    type: references
---

## Summary

Tickets: QSL-2122, QSL-2123, QSL-2129, QSL-2124, QSL-2125 (one commit each, plus the QSL-2122/2123 finish commit 39b65c91f). Code review with the rust-review lane (rust-style loaded) of `git diff origin/main...HEAD`. The branch is based on W5 (#2744); origin/main also has #2745, which this diff does not touch. Out of scope by the brief: the EN-1 invariant item (QSL-2301), `record.backend` (W6), the QSL-2296 spec questions and QSL-1900 (QSL-2299). The five unspecified state-graph choices recorded in QSL-2296 are not re-ruled here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The unrouted check now fires only for a `supported` disposition, so `requires-bound` (and negotiation's own `unsupported`) also override it, not only the family refusal the comment names; an item with an unread member can be told to add a bound that would still leave it unsupported | fsl-analyze/src/analyze/document.rs:1465-1486 |
| FND-002 | medium | A derived item resolved by model alias echoes the named `backend` but keeps candidates computed with no backend, so an unknown backend never settles `unknown-backend` and `en-6` on an untimed subject still runs EN-1 | fsl-analyze/src/analyze/document.rs:918-968, 1208-1212 |
| FND-003 | medium | `TimedSystem::with_max_clocks` refusals map to `runtime_invariant`/`established-invariant-broken`, including `NotSimulated::RequiresBound` and `Unbounded`, which depend on the request's binding; zone search maps the same refusal to `RequiresBound` | fsl-analyze/src/analyze/timed_item.rs:134-136, 272-274 |
| FND-004 | medium | A state-graph item skips FR-243's timing restriction and runs on EN-1's untimed system even over a timed subject; EN-6 lists no `state_graph_forms`, so the timed case should settle `unsupported`, not run untimed | fsl-analyze/src/analyze/document.rs:838-846, fsl-analyze/src/analyze/request.rs:456-469, 676 |
| FND-005 | medium | Ignore reasons of split stubs name the ticket that closes with this PR, not the ticket that unblocks them (QSL-2301 for the EN-1 invariant item, QSL-2296 for the spec questions) | fsl-analyze/tests/it/analyze_cli.rs:108, 259, 315; fsl-analyze/tests/it/analyze_coverage_cli.rs:237, 275; fsl-analyze/tests/it/analyze_family_cli.rs:320 |
| FND-006 | low | `temporal.max_depth` on a deadlock-freedom item is echoed in the item's request but the EN-1 derived run hard-codes `max_depth: None`, so the member is silently ignored | fsl-analyze/src/analyze/derived_item.rs:129 |
| FND-007 | low | `replay_over` writes `backend: en-1` into the replay request and envelope for every caller, including EN-6 timed replays | fsl-analyze/src/analyze/clause_item.rs:367, 392 |
| FND-008 | low | `max_clocks` left `sets_other_engine_limits`, so an untimed EN-1 item accepts it silently (it was `unsupported`), and no un-ignored test shows EN-6 reads it | fsl-analyze/src/analyze/document.rs:393-400, 1042 |
| FND-009 | low | A proved state-graph item whose evidence fails to spell loses its evidence silently, while the same failure on a refuted item is a fault; FR-628 requires a `possible` proof to carry its witness | fsl-analyze/src/analyze/state_graph_item.rs:184-194 |
| FND-010 | low | QSL-2129 tests under-assert: timed deadlock accepts `proved` or `refuted`, the exact-delay check accepts any object or string, and the ticket's nested `until` is tested as nested `eventually` | fsl-analyze/tests/it/analyze_timed_cli.rs:68, 103; fsl-analyze/tests/it/zone_mitl.rs:208 |
| FND-011 | low | Doc comment of `a_derived_item_over_a_name_that_is_no_model_alias_is_an_unknown_claim` starts mid-sentence | fsl-analyze/tests/it/analyze_cli.rs:402 |
| FND-012 | low | PR body is stale: commit shas do not match the branch and it lists QSL-2295 stubs as ignored that 39b65c91f un-ignores | PR 2743 body |

## Verdict

REQUEST CHANGES for FND-001 to FND-005. FND-006 to FND-012 are small fixes in the same round.

### FND-001 detail

`analyze_document` (document.rs:1465) now negotiates first and applies the unrouted settlement only when `disposition` is `Supported`. The comment says the intent is that "a kind its claim's family does not admit is refused first (FR-507, FR-627-AC-4)". That is the `InvalidRequest` disposition only. With the guard written as `Supported`, a `RequiresBound` disposition goes through to settlement: an item carrying `premises`, `hybrid`, `abstract_subject`, `smt_encoding` or another member no route reads, whose one candidate is bounded-only, now settles `requires-bound` instead of `unsupported` `unsupported-requested-capability`. Negotiation's own `Unsupported` also replaces the unrouted record (same exit code, different diagnostic payload). Fix: `if unrouted && !matches!(disposition, Disposition::InvalidRequest)`, plus a test with an unread member over an unbounded extent.

### FND-002 detail

`resolve_derived` (document.rs:918) takes the claim from `derived_items(package, subject, registry, &[], 0)`, whose candidates come from `registry.candidates(item.kind(), None)` (line 1211). It then sets `claim.item.backend` from the selection but never recomputes `claim.candidates`. The test double and the driver negotiate from `candidates`, so `{"claim": "M", "backend": "no-such", "temporal": {"item_kind": "deadlock-freedom"}}` settles as if no backend were named, and its echo names a backend its `candidates` do not contain. Fix: when `selection.backend` is set, recompute with `registry.candidates(kind, named)` and the same timing restriction, and add a test for the unknown-backend case.

### FND-003 detail

`TimedSubjectRefusal::NotSimulated(RequiresBound)` is what `TimedSystem` returns for an unbounded root (`zone_mitl.rs::an_unbounded_root_requires_a_bound`), and `Unbounded` is a clock owner whose population has no universe in the binding. Both come from the request's subject. `request.rs:676` forces mode `Bounded` for every zone item, so the extent check does not stop such an item before the run. Fix: map `NotSimulated` to the refusal `zone_refusal` gives for `ZoneCheckRefusal::RequiresBound` (items.rs:898), map `Unbounded` to a typed refusal, and keep `BROKEN` only for the variants the checker guarantees absent.

### FND-004 detail

In `resolve_one` the state-graph branch replaces `restrict_to_timing` instead of adding to it, and in `work` the state-graph arm comes before the timed arms. A `possible` claim over a timed subject therefore runs `ModelSystem` without the timed reading of `TimedSystem`. Applying both restrictions gives `unsupported` for a timed subject, because EN-6 advertises no `state_graph_forms`. That is the honest answer until a timed state-graph route exists. Confidence is low: this review did not confirm what the untimed system does with clock guards.

### FND-005 detail

- `invariant_items_prove_and_refute` names QSL-2122. The work is split to QSL-2301, and `record.backend` to the W6 ruling (QSL-2296).
- `possible_claims_prove_and_refute` names QSL-2125, and `digital_clocks_agree_and_the_clock_budget_stops` names QSL-2129. Both wait on QSL-2296 (items 5 and 6).
- The three QSL-2132 stubs wait on QSL-2296 (items 8 and 9).

When QSL-2122, QSL-2125 and QSL-2129 close, nothing points at these stubs. Fix: lead each reason with the unblocking ticket, for example `QSL-2301 (from QSL-2122): ...` and `QSL-2296 (from QSL-2125): ...`.

### What was checked and is sound

- **document.rs ordering (brief item 1).** `InvalidRequest` now wins over unrouted, as FR-627-AC-4 needs, and `a_finite_replay_item_over_a_temporal_clause_is_inapplicable` asserts exit 20, `inapplicable-kind` and family `temporal-trace`. `reductions` on a non-temporal kind stays unrouted. Settlements change only in the FND-001 cases.
- **QSL-2125 pipeline (brief item 2).** `restrict_to_state_graph_form` keeps only descriptors listing the form, and EN-1 lists all three forms. The unsupported test names `en-6` and gets `unsupported-requested-capability`, exit 21. The FR-393 spelling writes `trap`, `path-pair`, `witness` and `undefined` arms from one function. Each refuted instance is replayed through `replay_model_graph`, and a refuted record with no spelled evidence is a fault, never a verdict. `witness_samples` and `seed` are read only for state-graph claims. On other items they stay unrouted. The five QSL-2296 choices are applied consistently with one another. FND-004 and FND-009 are the only inconsistencies found.
- **QSL-2124 fixture fix (brief item 3).** `symmetric_document()` sets `symmetric: true` on the domain package's populations, which is FR-150's annotation. The old `symmetric M::counters;` unit text is not FSL. `malformed_selections_are_refused_typed` rewrites `document(false)` so that `"declared"` has no class. The test is unchanged in what it asserts, apart from the `cause.kind` spelling the other analyze tests already use.
- **QSL-2129 (brief item 4).** Clause, MITL, deadlock and time-lock items over a timed subject run on EN-6 with zone certificates and timed replay. `max_states` and `max_transitions` bound the symbolic states and edges. The wrong-dimension bound is refused at check, exit 20. Gaps: FND-003, FND-007, FND-008 and FND-010.
- **Internal faults reachable from a request (brief item 5).** FND-003. The other `BROKEN` returns found are guarded by checker invariants (clause letters, clause name, claim occurrence).
- **No Python and no logic-bearing shell** is in the diff.

Evidence: `nice -n 19 qsl-slot cargo test -p fsl-analyze -p fsl-route -p fsl-replay`: fsl-analyze `it` 1648 passed, 0 failed, 64 ignored (the 9 FSPEC_DIR-gated tests rerun with FSPEC_DIR set, 54 passed); fsl-route + fsl-replay 1754 passed, 0 failed, 20 ignored.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | high | The FND-006 fix sends `temporal.max_depth` into the derived deadlock request, but FSpec FR-368-AC-20 says a derived item carries no `max_depth` and a derived item with one is refused; the new test asserts the forbidden behaviour | fsl-analyze/src/analyze/derived_item.rs:130, fsl-analyze/tests/it/analyze_liveness_cli.rs:114 |
| FND-014 | low | `subject_refusal` keeps `NotSimulated::Extent` as `established-invariant-broken`, and its doc says the checker rules it out, but `Extent(ClassifyFailure::Limit)` (the 2^20 `BOUND_POSITIONS` walk) is not bounded by any checker invariant found | fsl-analyze/src/analyze/timed_item.rs:206-221 |
| FND-015 | low | The new refusal tests are tagged FR-632-AC-1 (the MITL verdict AC), which they do not verify; the unit test also omits the `RequiresBound` arm | fsl-analyze/src/analyze/timed_item.rs:383-406, fsl-analyze/tests/it/analyze_timed_cli.rs:171, 244 |

### FND-013 detail

FSpec FR-368-AC-20 reads: "A derived item carries no `max_depth` (a derived item with one is refused) ... no item inherits `limits`, `max_depth` or `smt_encoding` from another". The pre-fix code comment cited this AC. The round-1 FND-006 text offered "pass form.max_depth, or settle the member unsupported". The first option was the wrong suggestion, and the coder took it. Fix:

- restore `max_depth: None` in `deadlock_item`;
- refuse a derived item whose request sets `temporal.max_depth` with `invalid-request` naming `temporal.max_depth`, for both the EN-1 path and the EN-6 path (`zone_derived_item` already passes no depth);
- replace `max_depth_bounds_a_deadlock_freedom_item` with a test of that refusal.

### FND-014 detail

`check_model_bounds` returns only `RequiresBound` or `Extent` (fsl-eval/src/simulation/not_simulated.rs:168-173), so the other `NotSimulated` variants (`GeneratorMismatch`, `EmptyInitial`, `KeyEncoding`) cannot reach `subject_refusal`. `Extent(Fault)` is a broken check invariant by its own definition (fsl-semantics/src/family/requirements.rs:304-311). `Extent(Limit)` is a stage limit over the model's root types, and S3 classifies claim roots, not model state roots, under its own limit. The zone engine's pre-existing mapping (items.rs:908) treats it the same way, so this is not a regression. Fix: map `Extent(Limit)` to a resource-limit refusal, or narrow the doc comment to say the limit stays a fault.

`NoTimedModel` cannot be reached from a request:

- `timed_model` takes the alias from `time_sources()`;
- the checker pushes a `ModelTimeSource` only for a resolved model with exactly one object-type model node, and otherwise refuses with `MissingTimedModel` (fsl-semantics/src/check/mod.rs:1971-2000, 2148);
- `prepare_models` builds a `TimedModel` for each source whose model it times (fsl-semantics/src/check/timed_model.rs:4409);
- so `graph.timed_model(alias)` and `is_timed_by` both resolve.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | still-open | The fix applies `max_depth` to the derived item, which FSpec FR-368-AC-20 forbids; see FND-013 for the required refusal |
| FND-007 | fixed | - |
| FND-008 | still-open | The untimed half is fixed. The timed half proves at `max_clocks` 1 over a one-call snapshot (one live clock), so the limit is never exercised; W6 FR-632-AC-3 provisions two calls and expects `incomplete` `LimitReached{max_clocks}` value 1 |
| FND-009 | fixed | - |
| FND-010 | fixed | - |
| FND-011 | still-open | The first line was added twice: analyze_cli.rs:402-403 now repeat "FR-626-AC-1: a derived item named by its model alias is the one derived" |
| FND-012 | fixed | - |

### Round 2 notes

- **FND-001: correct and complete.** `run` (analyze.rs:1539) refuses `inapplicable-kind` from the claim's family before it reads the disposition. The test double never returns `InvalidRequest` for that case. So the extra `inapplicable` clause (document.rs:1504-1512) is needed: without it, `unsupported` would win over FR-627-AC-4's exit 20. The clause computes the family as `work` does (request.rs:433-438). It omits `work`'s `wrong_occurrence` arm, which a document request cannot reach because `declarations()` keys each name by its claim occurrence. `an_unread_member_settles_unsupported_not_requires_bound` covers the `requires-bound` case.
- **FND-010: delays from the spec and the fixture.**
  - `Rpc` with `T = 4`: `send` resets `x`; `reply` needs `x >= 1 ms`; `ServerDeadline` keeps `x <= 3 ms` while in flight.
  - Under `eventually[0 ms, 3 ms)` the only violating `reply` is at exactly `x = 3`. FR-241 picks the least admissible delay, which gives `send` delay 0 and then `reply` delay 3. `[0, 3]` is confirmed independently.
  - The tail `[0, 0, 1]` (`reset` and `send` have no clock guard, so 0; `reply` after a fresh reset needs 1) matches the least-delay rule.
  - Whether the counterexample continues past the violating `reply` comes from the lasso concretisation (FR-241 "Lassos": loop delays solved from the loop's linear constraints, stem extended by loop copies). The spec does not fix that, so the last three values pin the engine's output, not the spec.
  - Not a defect. Asserting `[0, 3]` as a prefix, plus the operation names, would make the test spec-derived.
- **FND-008: an observable effect exists.** EN-6 stops with `ZoneStop::LimitReached(MaxClocks)` (zone_check/search.rs:948), and settle.rs:295 maps it to `LimitReached{max_clocks}`. A two-call `Rpc` snapshot with `max_clocks` 1 should settle `incomplete` with that limit and value 1, as W6 FR-632-AC-3 (origin/fsl-spec-w6) states.

Round 2 evidence: fsl-analyze + fsl-route + fsl-replay with FSPEC_DIR set, 3563 passed, 0 failed, 85 ignored.

Round 2 verdict: REQUEST CHANGES for FND-013 (high) and the still-open FND-006, FND-008 and FND-011. FND-014 and FND-015 are low.

## Dispositions (round 2, head 9b01e5f69)

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | - |
| FND-008 | fixed | - |
| FND-011 | fixed | - |
| FND-013 | fixed | - |
| FND-014 | fixed | - |
| FND-015 | still-open | `an_unread_member_settles_unsupported_not_requires_bound` is retagged to FR-507-AC-7, which is about relation claims and refinement occurrences and says nothing of unread members or `requires-bound`; the other two retags (TC-694) and the unit-test arms are correct |

### Round 3 notes

- **FND-013.** `resolve_derived` (document.rs:956) refuses a derived item that sets `temporal.max_depth` with `invalid-request` and `invalid_runtime_input`/`invalid-value`, naming `temporal.max_depth`. It does so before the EN-1/EN-6 split, so both paths are covered by construction. `deadlock_item` is back to `max_depth: None`, and `a_derived_item_that_sets_max_depth_is_refused_naming_the_member` asserts the refusal and exit 20 (FSpec FR-368-AC-20).
- **FND-008.** `two_call_snapshot` adds a second `Call`. `max_clocks` 1 then settles `incomplete`, `limit-reached`, limit `max_clocks`, value 1, which matches W6 FR-632-AC-3 on origin/fsl-spec-w6 (244962bbe). W6 FR-632 also names the setting `model_check.max_clocks`. The test does not assert that, but it is not required for this AC.
- **FND-014.** `Extent(ClassifyFailure::Limit)` now settles `incomplete` `ResourceExhausted` without naming a limit. That is acceptable:
  - FR-625 "Budgets" and FR-621-AC-3 require `LimitReached{limit}` for a run that reaches one of the item's named limits (FSpec FR-127 `ModelCheckLimit`, raised by a request member).
  - The type-position walk is bounded by the constant `BOUND_POSITIONS`. It is not one of those limits, `ModelCheckLimit` has no variant for it, and no request member raises it.
  - FSpec FR-331 `ResourceExhausted` ("exhausted a configured resource bound") is the matching cause. The doc comment now states this.
- **FND-015.** The unit test now covers the `RequiresBound` and `Extent(Limit)` arms. `a_timed_subject_refusal_is_typed_by_what_causes_it` and `a_timed_subject_with_an_unbounded_root_requires_a_bound` are retagged to TC-694, the FR-239 test case. FR-239 states "An unbounded population root SHALL refuse `RequiresBound` before any state is explored", so both retags are correct. `an_unread_member_settles_unsupported_not_requires_bound` (analyze_timed_cli.rs:273) is wrong as tagged:
  - FR-507-AC-7 covers relation-claim and refinement occurrences.
  - No FR AC states the unread-member precedence. FR-281 (lines 75-83) states the dispositions it chooses between.
  - Fix: tag it at test-case level to the refusal behaviour (for example `#[trace("TC-762")]` for FR-281, or TC-1584 for FR-624) with no AC.
- **#10.** The test asserts only `send` at 0 and `reply` at 3, with their operation names. These are the values derived from FR-241 and the fixture in round 2, and the lasso tail is left to the engine.

Round 3 evidence: fsl-analyze + fsl-route + fsl-replay with FSPEC_DIR set, 3563 passed, 0 failed, 85 ignored.

Round 3 verdict: REQUEST CHANGES, for the single still-open low FND-015, which needs a one-line retag. Every medium and high finding is fixed.
