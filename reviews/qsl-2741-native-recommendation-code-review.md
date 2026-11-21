---
id: SR-9609930108528406226052840833875755297
title: "QSL-2741 native recommendation code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2741 native recommendation diff only"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: references
---

## Summary

Ticket: QSL-2741. PR: filament-spec-language#3401. Frozen head `3754f30dfc3430f20902d6f8e75e6879460d757a`, actual base `21edb2ead1c8b69768beedaab003e077ce4ed042`. Reviewed only the two native adapter commits (11 files, 291 insertions, 30 deletions). No receipt binding implementation or qualification claim is included.

## Verdict

**CONDITIONAL** — one medium traceability defect; native behavior has no additional concrete defect found by static inspection. Focused runtime evidence and final aggregate checks remain with the lead; none were run or assumed by this reviewer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Receipt-free native recommendation test falsely binds FR-1046-AC-2, which requires canonical receipt validation. | fsl-analyze/tests/it/package_recommendation_bridge.rs:40 |

## Finding detail

The positive test provisions only workflow, snapshot and model files. It neither provisions a canonical FR-329 receipt nor selects `items[i].effect_decisions`, and asserts only native recommendation, state and replay behavior. Removing receipt validation entirely would leave it green. The static matrix nevertheless records FR-1046-AC-2 as tagged with this test as its only binder. Remove this test's FR-1046-AC-2 tag; preserve its native parity/recommendation tags and leave combined receipt qualification open for the Source-owner integration base.

## Examined scope

- `FR-1046-AC-2`; role: examined; excerpt: Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise.
- `FR-940-AC-6`; role: examined; excerpt: For each item family whose evaluator FR-531's inventory lists as taking a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`), a `program` request and a `package` request over the handoff directory of the same compiled unit give equal item records, counterexamples and replays, and no package-route record is `unsupported`; a handoff directory whose `checked-package.json` the strict reader refuses, or that holds no `identities.json`, settles no item and compiles no source (FR-850-AC-
- `FSpec-FR-822-AC-3`; role: examined; excerpt: Exploring the AC-1 clause from an order with `total = 5` gives `recommend` steps whose transition identity has `step` `recommend` and a `recommendation` record with `type` `protocol-recommendation`, the control's `node`, `role` `Auditor`, `operation` `Orders::Order::refund`, the receiver and the arguments, one step per application whose selected precondition holds; every successor of a `recommend` step has the predecessor's `semantic` key member and compensation registrations; a counterexample through it carries the record in its step, and a replay of that counterexample with the record's `arg
- `fsl-analyze/src/analyze/clause_item.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-analyze/src/analyze/document/package_run.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-analyze/src/protocol_analysis/profile.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-analyze/tests/it/main.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-analyze/tests/it/package_recommendation_bridge.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-eval/src/simulation/protocol/package_program.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-eval/src/simulation/protocol/program.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-eval/src/simulation/protocol/settle.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-package/src/bare_slot.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-package/src/protocol_view.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later
- `fsl-replay/src/spine/protocol_structure.rs`; role: examined; excerpt: // SPDX-License-Identifier: AGPL-3.0-or-later

## Coverage

Plan completion: not assessed

The code-review method includes the Rust review lane, with repository AGENTS.md, CLAUDE.md, CONTRIBUTING.md, root Cargo lints and the API, implementation, safety, wire and testing references read. No applicable AssuranceProfile was found. No CI, manifest, lock, receipt implementation, foreign fixture, threshold or suppression changes occur in the diff. No new vendored semantics or duplicate recommendation/replay algorithm was found.

Eight production paths were examined: package control decode/slot lookup; native call role/operation/contracts; result-free recommendation and slot supply; existing argument enumeration/precondition selection; Check normalization; named declaration capability routing; bounded profile containment; replay control projection. All map to FR-940 package parity and FSpec FR-822 native recommendation semantics. No unowned behavior, source stub, test stub or coverage suppression was found in these changed paths. Existing role, replication, correlation and other unported-control guards remain. The three new tests use the real compile/analyze/model/replay seams; assertions check native step order, unchanged advice model state, later model write, blocked check, false constraint and typed Fence refusal.

Reconciliation: quire matrix 0.52.0 (engine 0.62.2), explicit repository scope, exit 0; no run evidence read. quire coverage with that same scope exited 0. For the two examined local criteria, the engine reports tagged 2, untagged 0, ignored-only 0; FR-1046-AC-2's tag is semantically wrong as FND-001 records. Foreign FSpec-FR-822-AC-3 is unmatched in the single-repository corpus as expected; its pinned f6b0480 specification was inspected directly. The two foreign tags are not local missing-criterion defects. Repository-wide unrelated warnings and gaps are retained in matrix.json/matrix.stderr and coverage.json/coverage.stderr, and are outside this explicitly bounded diff review. No whole-repository gap PASS is claimed.

Optional standalone semantic gap review: skipped; ordinary code/test alignment was examined as required by code-review. No plan was supplied. No Cargo, test runtime, full gate, Git mutation or relay action was performed by this reviewer. Lead-provided compile/fmt evidence is context only; focused runtime is pending, and receipt AC-2 remains unqualified.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | D1 inspected cd02e5ac6d69dc183b04a8e983d35a4037ac910d: removed only the receipt-validation AC tag from the receipt-free positive test, preserving the native package parity and FSpec recommendation tags. |
| FND-002 | still-open | D1 actual runtime exposed whole-request Fault for an admitted unported Fence; no fix yet at that runtime head. |
| FND-002 | fixed | D2 inspected add2a73e486d09d0524217b26ec7a5696cff36ca: Fence now decodes required scalar text; the shared closed ordering word match preserves Attempt optional-group semantics. Runtime rerun pending. |

## Disposition pass 1 verdict

**CONDITIONAL** — all findings are fixed; no new finding arose from the one-line fix. The review of native implementation and tests is clean. Runtime qualification remains pending the lead's three focused test results and final required gate. FR-1046-AC-2 remains unqualified; neither this adapter nor these receipt-free tests satisfy the combined receipt-validation criterion.

After excerpt (fsl-analyze/tests/it/package_recommendation_bridge.rs:40):

```rust
#[trace("FR-940-AC-6", "FSpec-FR-822-AC-3")]
```

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | An admitted Fence control faults the whole Package analysis because its required scalar ordering is decoded as an optional group. | fsl-package/src/protocol_view.rs:1822; fsl-analyze/src/analyze/document/package_run.rs:807 |

Actual lead-provided runtime `3401-3754-c1` at frozen 3754f30dfc3430f20902d6f8e75e6879460d757a returned exit 101: 2 passed, 1 failed, 0 ignored. The positive recommendation/Check Source-versus-Package replay test and false-constraint test passed. The unported-Fence test failed at line 115: item category was null rather than unsupported, with request category internal-failure, runtime_invariant, message "the analyze run faulted", null cause/details/locus and no items. Handoff compilation succeeded; failure occurred in Package analyze. This is an actual product capability-classification defect rather than missing receipt integration.

Grounding: the existing emitted Fence ordering is required scalar text (fsl-package/src/emit/tests/protocol_controls.rs:1243 and the scalar assertion at line 653). The old Fence control decoder called optional_ordering, whose optional_text path expects an aggregate optional group. Valid scalar `seq_cst` therefore produced MalformedMember(ordering), and resolve_names mapped that to request Fault before the existing PackageProgramRefusal::Unsupported(Fence) item route could run. Attempt ordering remains an optional group and must retain that shape. FND-002 was still open when the failing runtime evidence arrived; its later fix is recorded separately below.

## Disposition pass 2 review

Inspected the entire cd02e5ac6d69dc183b04a8e983d35a4037ac910d..add2a73e486d09d0524217b26ec7a5696cff36ca diff. It changes only fsl-package/src/protocol_view.rs: Fence reads required scalar text; the old closed AccessOrdering word match is extracted to ordering_word and reused by optional_ordering. The five existing words and malformed-word refusal are unchanged. Attempt still traverses optional_text and keeps optional-group shape/refusals. No Fence execution, profile expansion, malformed-request policy, oracle, receipt, Source, manifest or dependency change occurs. This makes the existing Unsupported(Fence) item route reachable for the already-admitted control.

**CONDITIONAL** — FND-002 is statically fixed; all three tests at add2a73e486d09d0524217b26ec7a5696cff36ca remain pending. The two earlier positive passes are retained as actual historical evidence, not a green negative rerun. Final exact-head aggregate verification remains the lead's responsibility. FR-1046-AC-2 remains unqualified.

After excerpt (fsl-package/src/protocol_view.rs:1822):

```rust
ControlOperation::Fence => ControlBody::Fence {
    ordering: ordering_word(
        node,
        "ordering",
        text(required(node, "ordering")?)
            .ok_or_else(|| malformed(&node.node_id, "ordering"))?,
    )?,
},
```

## Final bounded verdict

**PASS** — both recorded defects are fixed: FND-001's false receipt AC tag was removed in cd02e5ac6d69dc183b04a8e983d35a4037ac910d; FND-002's Fence required-scalar decoding was corrected in add2a73e486d09d0524217b26ec7a5696cff36ca. No additional defect was found in the targeted disposition review.

TitanCode runtime simple3401-add2-c1 at add2a73e486d09d0524217b26ec7a5696cff36ca returned rc 0: 3 passed, 0 failed, 0 ignored.
The lead supplied this actual result for the complete `package_recommendation_bridge` prefix: `package_recommendations_preserve_native_steps_and_replay`, `a_false_package_recommendation_constraint_disables_the_call`, and `an_unported_package_control_names_its_typed_item_refusal` all passed. Earlier 3754 runtime evidence remains 2 passed / 1 failed / 0 ignored and is superseded for the negative Fence path by this run, not erased. The positive evidence establishes native Source/Package recommendation and Check behavior, model-state/replay parity, false-constraint blocking, and the existing typed unsupported Fence item route.

This PASS applies only to the reviewed native recommendation adapter and targeted fixes. FR-1046-AC-2's combined canonical receipt validation remains unqualified with the Source owner; this review does not complete QSL-2741 or claim receipt binding acceptance. Final current-main, exact-final-head, five-crate all-targets verification after review-document integration remains pending with the lead and is not claimed here. No Cargo or runtime was run by this reviewer.

Both final review files were explicitly validated with installed quire 0.52.0 (engine 0.62.2), repository scope `/home/peter/dev/worktrees/protocol-batch-2-qsl-2741-recommendation-selector`, and their actual cache paths; exit 0. Initial private review comments were posted once by the lead (code-review prefix b5f642ea, gap-analysis prefix f5e742a0); D1 comment prefixes 3faab599 and 06d63f1e. No tracker query or post was performed by this reviewer; tracker read-back remains with the posting lead.
