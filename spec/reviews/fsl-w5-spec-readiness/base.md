---
id: SR-2026111608544801
title: "Spec review of the FSL wave 5 spec readiness batch"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR #2744 branch fsl-spec-w5 at f38affa78, git diff origin/main...HEAD (FR-132, FR-255, FR-267, FR-501, FR-624, FR-625, FR-627, FR-640, FR-641, NFR-017, TC-1160, TC-1584, TC-1585, TC-1587, TC-2241..TC-2246; stubs in fsl-analyze, fsl-command and tests/it)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-625
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-627
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: reviews
  - target: ix://agent-ix/filament-spec-language/NFR-017
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: references
---

## Summary

Ticket: QSL-2295 (batch also covers QSL-2277, QSL-2047, QSL-2288, QSL-2290,
QSL-2291, QSL-2294 and the FR-641 `max_candidates` addition). Combined review
with the standard side, agent-ix/filament-specification#662.

QSL-2295 rulings, checked against the whole spec tree at HEAD:

- (a) FR-127 wins. FR-625 "Search horizon", FR-625-AC-3, TC-1585 step 3 and
  the model_check_oracle stub now say `inconclusive` `bound-reached`. No
  other text in the tree still states `LimitReached{max_depth}`. Fully amended.
- (b) FR-507 wins. FR-625 `kind`, the Decision, FR-625-AC-4, TC-1585 and the
  stub now keep the invariant as `operation-contract`. No other text pairs an
  invariant with `temporal-satisfaction`. But the amended text says the item
  "runs on EN-1", and FR-489-AC-2 gives `operation-contract` exactly the
  `contract-checker` candidate (FND-002).
- (c) FR-125 wins with the staying Counter. FR-627-AC-1, TC-1587 and both
  stubs use the staying Counter. `cycle` is renamed `loop` in FR-627-AC-3 and
  in the coverage stub. No "stutters below 3" text is left. Fully amended.
- (d) FR-507 wins. FR-627 Behavior, the Decision, FR-627-AC-4, FR-132 and
  TC-1587 are amended. FR-621 line 108 and FR-621-AC-7 (QSL-2272) still state
  a `finite-replay` item over a temporal clause that reports a temporal
  disposition, and so does their stub (FND-001). Not fully amended.

No feature is dropped or narrowed. (d) moves the supplied-trace fairness
refusal onto the clause-run and monitor paths. It does not remove it.
QSL-2277 is specified in full: package rows, model check, replay and
identity. QSL-2047 broadens the ticket's stages with `parse`.

Ids: TC-2241 to TC-2246 are in the TC-2240..2269 range and absent from
origin/main. TC-2240 is already taken on main (FR-766) and was skipped
correctly. No new FR was minted; amendments are in place. `quire validate`
exits 0. `cargo check --tests -p fsl-analyze -p fsl-command -p
filament-spec-language` passes.

Stubs: each edited stub keeps its ignore and is keyed to its code ticket
(QSL-2122, QSL-2123, QSL-2277, QSL-2047, QSL-2133). The
request_overrides.rs retag (TC-2244/TC-2245, FR-640-AC-13/14) is on live
QSL-2294 tests and matches the criteria.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Ruling (d) is not applied to FR-621. FR-621-AC-7 (QSL-2272) still requires a `finite-replay` item over a temporal clause that is pending at the trace end to report `disposition.temporal` `pending` with exit 0. FR-627-AC-4 and FR-507 now settle that item `invalid-request` `inapplicable-kind`, exit 20. FR-621 line 108 still lists `finite-replay` as a temporal-disposition evaluation, and TC-1581 step 7 and the stub (finite-replay of `Reaches`, expecting exit 0 pending) assert the losing behaviour. Fix: restate AC-7's pending case over a `monitor` item, or over the FR-641 clause run of a finite trace. Keep the pending-disposition feature. Amend line 108 to name `monitor` items and the clause run, and update TC-1581 step 7 and the stub's expectation, keeping its QSL-2272 ignore. | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:108,167; spec/test-cases/TC-1581-analyze-outcome-document-carries-verdict-certificate-counterexample-and-accounting.md:23; fsl-analyze/tests/it/analyze_request_wire.rs:553-585 |
| FND-002 | high | The ruling (b) amendment says an `operation-contract` invariant item "runs on EN-1" and that `analyze` "decides it on EN-1 because the request names a model subject". No routing rule supports this. FR-489-AC-2 says the `OperationContract` capability gives exactly `contract-checker` (bounded), and EN-1 advertises only `temporal-satisfaction` (analyze.rs:2217). As written, negotiation sends the item to the contract checker, and AC-4's EN-1 exhaustive `proved` and its counterexample at `value = 3` cannot both hold. Fix: state the route. Either EN-1 advertises `operation-contract` for invariant claims over a model subject (amend FR-489-AC-2 and the EN-1 advertisement, plus a rule that picks EN-1 over `contract-checker` when a model subject is named), or AC-4 states the `contract-checker` outcome. Keep the exhaustive reachable-state decision either way. | spec/functional/FR-625-request-and-report-an-explicit-state-model-check-item.md:45-58,118; spec/functional/FR-489-build-the-production-linked-provider-registry.md:106; fsl-analyze/src/analyze.rs:2217 |
| FND-003 | medium | Some new criteria have no failing ignored stub, which the brief requires. FR-641-AC-7 / TC-2246 (`max_candidates`) has none. NFR-017-AC-6 and AC-7 (reject and pending fences) have none. The NFR-017-AC-5 stubs check only the stage tags and the bare-fence scan; neither runs a fence at its stage or asserts the count equality AC-5 states. FR-640-AC-12's analyze and replay half has no stub, only the compile rows. Fix: add ignored stubs keyed to the code tickets (QSL-2047 for NFR-017; the FR-641-AC-7 code ticket; QSL-2277 for the analyze and replay half) that assert each criterion. | spec/functional/FR-641-carry-every-temporal-trace-kind-and-the-progress-ledger-on-the-run-request.md:188; spec/non-functional/NFR-017-build-a-clean-clone-with-locked-dependencies-and-run-the-documented-examples.md:121-123; tests/it/doc_snippets.rs:83-147; tests/it/clock_derivation.rs:325-380 |
| FND-004 | low | `every_fsl_fence_declares_its_stage` also carries `#[trace("TC-1398", "NFR-017-AC-2")]`. NFR-017-AC-2 is the gate's step-5 config_version fixtures and walkthrough, which this test does not verify. Fix: drop the tag, or bind TC-1398 to the NFR-017 criterion the fence check verifies. | tests/it/doc_snippets.rs:86 |
| FND-005 | low | Stub prose still describes the losing behaviour. model_check_oracle.rs says "a horizon stop[s] the search incomplete". The FR-627-AC-4 stub's doc says "fairness over a supplied trace is a missing premise", and its ignore reason says "finite-replay items with fairness are not reachable", though it now asserts `inapplicable-kind`. Fix: reword them to the amended expectations. | fsl-analyze/tests/it/model_check_oracle.rs:50; fsl-analyze/tests/it/analyze_cli.rs:204,207 |

## Verdict

REQUEST CHANGES. Rulings (a) and (c) and items 2 to 7 are sound and fully
amended. FND-001 is the gap the author left open. FND-002 is a new
contradiction introduced by the ruling (b) text. Both are high.
