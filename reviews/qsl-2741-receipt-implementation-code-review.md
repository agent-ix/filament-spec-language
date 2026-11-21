---
id: SR-202611210022442313144108546
title: "QSL-2741 receipt binding code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3365; QSL-2741; fsl-analyze/src/analyze.rs, analyze/clause_item.rs, analyze/document.rs, analyze/document/effect_decision.rs, analyze/state_graph_item.rs, plan.rs, provider.rs; fsl-command/src/command/analyze.rs; fsl-command/tests/it/effect_decision.rs, main.rs; fsl-eval/src/simulation/protocol/identity.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-1046", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-277", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-259", type: references}
---

## Summary

One independent code-review/Rust pass over frozen source `f6d7f7fe289b8819a7cdf071e82245f9ccd7ceca` for QSL-2741 receipt binding. The Source implementation preserves many required boundaries, but lasso multiplicity and opaque-document depth handling contain concrete defects. The new ordinary Package positive also remains a known failing test. No source or test was edited, and no Cargo invocation was run by this reviewer.

## Verdict

**CONDITIONAL after disposition round 1** — original FND-001/002/003 are fixed; the new typed resource/classifier failure mapping defect FND-004 remains open and blocks merge. The original review snapshot and finding texts are preserved below. The same coder has one remaining semantic fix round; no second initial review or broad requalification is requested.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Occurrence counting ignores the native lasso cycle: one matching recommendation in a non-terminal repeated cycle is counted once and accepted instead of refusing ambiguous-decision. | fsl-analyze/src/analyze/document/effect_decision.rs:366-386 |
| FND-002 | high | The new recursive Serde receipt decoder imposes the pinned default depth cap of 128 on otherwise valid opaque receipt objects, returning invalid-value instead of admitting a below-budget canonical document; command intake also reparses receipts into recursive Value. | fsl-analyze/src/analyze/document/effect_decision.rs:212-229,505-579 |
| FND-003 | medium | The newly registered, unignored Package positive test is known to fail before receipt binding and would add a default-suite failure if the bounded Source slice is merged unchanged. | fsl-command/tests/it/effect_decision.rs:564-619 |

## Code and Rust evidence

The actual recommendation matcher uses the typed ProtocolTransition projection shared with the native trace writer. Complete canonical object bytes retain the type, node, role, instance, optional actor, qualified operation, typed receiver and parameter order. It counts real prefix transitions rather than persistent state copies, indexes by item, and does not accept another item's record as a candidate. Equal independently emitted records remain eligible. Canonical receipt admission rejects unknown outer fields while leaving FR-329 nested objects opaque, uses closed typed refusal variants and existing catalog causes, and preserves null details/locus. Selector shape checking and dependent-file byte charging occur in the existing command intake. Replay premise documents exclude recognized receipts, and successful binding leaves the normal outcome unchanged. Full canonical grants and effect requests participate in retry reconciliation. The binding opens no adapter, subprocess or network path and does not treat transport or effect-observation claims as model truth.

FND-001 is specific to the native lasso representation, not the already passing bounded-repeat test. TemporalCounterexample.prefix stores the stem followed by one traversal of a lasso cycle; Lasso.entry identifies where that cycle starts. The new loop at effect_decision.rs:368 visits that representation once and never reads lasso. Unbounded protocol repeats are admitted with Loop.max=None, and settle.rs:318 re-enters their body without a counter. Existing protocol_bounded_lasso.rs and protocol_roles.rs demonstrate native protocol cycle traces. A matching recommendation inside a non-terminal repeated cycle therefore has infinitely many occurrences but count one when that cycle contains one copy of the record. A stem-only recommendation remains unique and must not become ambiguous merely because a later unrelated cycle exists. The fix needs those real max=None native-cycle and stem-only controls, alongside the current two-unrolled-occurrence and independent-item controls.

FND-002 follows directly from the pinned decoder. Cargo.lock selects serde_json 1.0.151. Its Deserializer constructor initializes remaining_depth=128, and deserialize_any checks/decrements it for each object or array. UniqueValue recursively calls next_value_seed/next_element_seed using a default Deserializer at effect_decision.rs:212, then maps the depth error to InvalidValue. A canonical receipt whose otherwise opaque authority_grant contains 130 nested small objects fits the dependent-byte allowance but is refused only because of nesting. The following generic Value parse in command/analyze.rs:1171 would impose the same boundary even after fixing the first reader. FR-277 and ADR-030 D-1 forbid a depth cap or proportional native recursion. The already pinned quire-canonical revision 2204c5c exports its iterative arena Document/NodeRef and read/read_bounded reader: reuse that existing boundary, keep the file charge before parsing, and do not disable Serde's guard, invent a new depth setting, duplicate a parser or leave a later recursive receipt reparse in place. This source falsifier was not executed by the reviewer.

FND-003 is distinct from a claimed regression in Package execution: the Package selector limitation predates receipt consumption, and the PR is honestly draft. Nevertheless, the newly added unignored positive at effect_decision.rs:566 fails its independent Package baseline before reaching the receipt assertions. Merging that test unchanged adds a known ordinary-suite failure. The lead must either receive a real Package fix and qualify the positive, or retain a clearly named unmet positive outside normal qualification while preserving its full assertions and open QSL-2741 acceptance debt.

## API, ownership and resource review

The cross-crate projection belongs below Analyze in Eval and reuses the existing trace encoder. The Analyze-owned receipt module and Command intake introduce no dependency cycle, new wire version, output property or catalog cause. ReceiptCause and ReceiptRefusal retain typed selection before formatting. Input bytes are paid once through intake.file before canonical parsing; selectors remain request bytes, and no alternate file read is introduced. Actual recommendation indexing is one traversal per selected item's typed trace. Matching, grant/request reconciliation and successful no-output behavior were inspected; this is not a proof of repository-wide work/storage qualification. The recursive opaque JSON path is explicitly rejected by FND-002 rather than silently treated as bounded by the byte ceiling.

## Validation and limitations

- Frozen HEAD and clean worktree verified. Direct rustfmt --check --edition 2021 --config skip_children=true on all eleven changed Rust files: exit 0. git diff --check origin/main...HEAD: exit 0.
- Independently acquired the complete existing Titan log into /private/tmp/qsl2741-receipt-c5-verified.log. Actual result: 12 passed, 1 failed, 0 ignored, 189 filtered, 1.56 s. The failure names the supported temporal-satisfaction/deadlock-freedom item, actual emitted recommendation:Review protocol node and public source map, then refuses the receipt-free Package baseline with exit 21, unsupported_construct/not-yet-implemented, items [], last_stage null.
- The 12 passing tests cover Source normal-output/model-truth preservation, canonical component changes including argument order and actor, item-local matching, bounded-repeat ambiguity, retry/request contradictions, malformed/canonical causes and pointers, selector/schema behavior, exact aggregate byte allowance, precedence and incomplete/no-trace preservation. Prior seven recommendation controls were reported passing by the coder at ec585; no claim is made that they were rerun on the final head.
- Both coder Clippy attempts exit 101 in unchanged fsl-eval/src/simulation/model/restore.rs:111,246,407. The recorded baseline/head blob is 78661e07452a6743c3f183b8ebca5dc0abd8bc74. Analyze/Command/Eval lint is not qualified; no suppression or foreign fix is requested.
- No additional runtime, compile, Clippy, whole-crate, workspace or aggregate gate ran in this review. No applicable AssuranceProfile declaration was found.
- The pending public CLI cohort containing the new recommendation behavior is not qualified by the older f597 driver cohort. Package parity and that CLI proof remain unmet; this review does not close QSL-2741.

## Coverage

All eleven changed files, their real command/replay/native-transition seams and all twelve FR-1046 criteria were inspected. No specification file changed, so spec-review and its optional sub-analyses are not applicable to this frozen diff. Code-review includes Rust API, implementation, safety/resource, testing and wire/error checks. The separate gap-analysis artifact retains the global baseline and actual Package acceptance gap. Optional whole-repository semantic gap analysis was not requested. Plan completion: not assessed.

## Review data

```json
{
  "clean": false,
  "scope": [
    {
      "id": "FR-1046-AC-1",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A real protocol analysis emits one typed recommendation before another role's model write. An FR-329 canonical document whose `decision` is that emitted object, supplied through `{file, identity}` and selected by `items[0].effect_decisions`, binds and returns the unchanged normal item: its native trace, replay, formal result and outcome member set equal the same run without the receipt."
    },
    {
      "id": "FR-1046-AC-2",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise."
    },
    {
      "id": "FR-1046-AC-3",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Changing only the emitted recommendation's type tag, node, role, instance, replicated actor, operation identity, typed receiver, one typed argument or argument order in `decision` refuses `invalid_runtime_input` / `invalid-value`, exit 20. The existing message begins `effect-decision wrong-decision` and names the selector, document and `/decision`, while `details` and `locus` are null; equal display names do not make distinct declaration identities match."
    },
    {
      "id": "FR-1046-AC-4",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A receipt whose decision matches a recommendation of item 0, but is selected only by item 1 and has no equal actual recommendation in item 1, refuses for item 1. If item 1 independently emits exactly one equal actual recommendation, the receipt binds there even when item 0 also emits an equal record. A native trace containing two indistinguishable occurrences within the selected item refuses `invalid_runtime_input` / `invalid-value` with message prefix `effect-decision ambiguous-decision`; no occurrence is chosen by discovery order and no item-origin field or decision id is generated."
    },
    {
      "id": "FR-1046-AC-5",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Selecting the same canonical receipt twice, including under two provision identities, returns the same normal outcome as one selection and performs no additional protocol step or external effect. Two different transport receipts with the same request key and canonical effect request validate against that same recommendation without an additional result entry or FSL-owned history."
    },
    {
      "id": "FR-1046-AC-6",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Two selected receipts with the same FR-329 request key and different canonical effect requests refuse `invalid_runtime_input` / `invalid-value` at the later document's `/effect_request`. The existing message begins `effect-decision request-key-contradiction` and names both provision identities; no successful receipt-binding output or replacement request is published."
    },
    {
      "id": "FR-1046-AC-7",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Invalid JSON, a missing required property, duplicate member, unknown outer member and mistyped property refuse `invalid_runtime_input` with `malformed-json`, `missing-member`, `duplicate-member`, `unknown-member` and `wrong-value-kind`, respectively. A wrong contract version, out-of-domain value and non-canonical encoding use `invalid-value`. Each message begins `effect-decision malformed-receipt`; no data is repaired or converted into another canonical document version. The catalog selection comes from typed variants, not message parsing."
    },
    {
      "id": "FR-1046-AC-8",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A non-array item member, malformed or unresolved document selector, and a top-level `effect_decisions` member each refuse `invalid-request`, exit 20, at their request pointer. Absent and empty item selectors retain the old request identity and outcome. No selected receipt is forwarded into the admitted FR-331 request item, and all outcomes validate against the unchanged published schema."
    },
    {
      "id": "FR-1046-AC-9",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "With the request below its independent request-byte ceiling, the real selected program, model and canonical receipt files are admitted at their exact aggregate dependent-byte allowance. One byte less gives the existing `resource_exhausted` incomplete outcome and original dependent-byte ceiling before receipt consumption; the receipt bytes are neither omitted nor read through an uncharged path."
    },
    {
      "id": "FR-1046-AC-10",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A malformed selected receipt paired with a wrong decision fails canonical admission first. After canonical admission, a wrong decision on the first selector precedes a later key contradiction; a valid first selector followed by a conflicting request reports that later contradiction. Other request items keep their original settled records."
    },
    {
      "id": "FR-1046-AC-11",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A bounded incomplete or unsupported protocol item with no binding-eligible trace retains its own cause and stage and reports no accepted receipts; it does not report a successful binding or a nonexistent-decision fact about an unexplored continuation."
    },
    {
      "id": "FR-1046-AC-12",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Adding a transport receipt, grant, effect request, lineage or optional effect observation changes neither model fields nor compensation registrations, queues or formal truth. The binding path calls no adapter, starts no process and opens no network connection; receipt admission makes no claim that the supplied grant authorizes the action or the business effect occurred."
    },
    {
      "id": "FR-277",
      "path": "spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md",
      "role": "examined",
      "excerpt": "Nesting depth is not a limit. Node, byte and work limits bound every stage,\nand no stage recurses on the native stack (ADR-030 D-1)."
    },
    {
      "id": "Lasso",
      "path": "fsl-replay/src/model_check/counterexample.rs",
      "role": "examined",
      "excerpt": "The steps of the lasso are [`TemporalCounterexample::prefix`]: the stem,\n/// then the loop. The loop re-enters at the position `entry`, so its steps\n/// are the prefix's steps from `entry` on and the last post-state of the\n/// prefix is the state at position `entry`."
    },
    {
      "id": "FR-329",
      "path": "filament-specification:spec/objects/interfaces/FR-329-effect-decision-receipt.md",
      "role": "context_only",
      "excerpt": "A document whose `contract_version` is not that identifier, missing required\nproperties, duplicate keys, out-of-domain values and non-canonical encodings\nrefuse before consumption."
    },
    {
      "id": "ProtocolTransition::recommendation_document",
      "path": "fsl-eval/src/simulation/protocol/identity.rs",
      "role": "examined",
      "excerpt": "Uses the same projection as the trace writer."
    },
    {
      "id": "SubjectBinding::replay_documents",
      "path": "fsl-analyze/src/analyze.rs",
      "role": "examined",
      "excerpt": "Formal replay premises exclude descriptive effect-decision receipts."
    },
    {
      "id": "FR-001-AC-7",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key.",
      "excerpt_part": "1/1"
    },
    {
      "id": "FR-056-AC-60",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`. From initial `a` `3ff0000000000000` (1.0) it reaches exactly the states 1.0 and `bff0000000000000` (-1.0), with one candidate per application. From initial `a` `0000000000000000` (+0) it reaches exactly +0 and `8000000000000000` (-0), two states with distinct keys, and each application has two candidates, because both zeros are `numericEqual` to the solved value (FR-120-AC-34). A",
      "excerpt_part": "1/2"
    },
    {
      "id": "FR-056-AC-60",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "simulation run applying `flip()` twice from 1.0 ends at 1.0 with no flag.",
      "excerpt_part": "2/2"
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "FR-1046-AC-4",
      "related_ids": [
        "FR-1046"
      ],
      "path": "fsl-analyze/src/analyze/document/effect_decision.rs",
      "lines": "366-386",
      "excerpt": "A receipt whose decision matches a recommendation of item 0, but is selected only by item 1 and has no equal actual recommendation in item 1, refuses for item 1. If item 1 independently emits exactly one equal actual recommendation, the receipt binds there even when item 0 also emits an equal record. A native trace containing two indistinguishable occurrences within the selected item refuses `invalid_runtime_input` / `invalid-value` with message prefix `effect-decision ambiguous-decision`; no occurrence is chosen by discovery order and no item-origin field or decision id is generated.",
      "finding": "Occurrence counting ignores the native lasso cycle: one matching recommendation in a non-terminal repeated cycle is counted once and accepted instead of refusing ambiguous-decision."
    },
    {
      "fnd": "FND-002",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "FR-277",
      "related_ids": [
        "ADR-030",
        "FR-1046-AC-7",
        "FR-1046-AC-9"
      ],
      "path": "fsl-analyze/src/analyze/document/effect_decision.rs",
      "lines": "212-229,505-579",
      "excerpt": "Nesting depth is not a limit. Node, byte and work limits bound every stage,\nand no stage recurses on the native stack (ADR-030 D-1).",
      "finding": "The new recursive Serde receipt decoder imposes the pinned default depth cap of 128 on otherwise valid opaque receipt objects, returning invalid-value instead of admitting a below-budget canonical document; command intake also reparses receipts into recursive Value."
    },
    {
      "fnd": "FND-003",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "test-intent",
      "artifact_id": "FR-1046-AC-2",
      "related_ids": [
        "FR-620"
      ],
      "path": "fsl-command/tests/it/effect_decision.rs",
      "lines": "564-619",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise.",
      "finding": "The newly registered, unignored Package positive test is known to fail before receipt binding and would add a default-suite failure if the bounded Source slice is merged unchanged."
    }
  ],
  "bindings": [
    {
      "test_id": "receipt_binding_preserves_actual_source_outcome_and_model_truth",
      "ac_id": "FR-1046-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "checked_package_receipts_match_the_same_actual_recommendation",
      "ac_id": "FR-1046-AC-2",
      "trace": "correct",
      "qualification": "actual Package baseline failed"
    },
    {
      "test_id": "repeated_indistinguishable_actual_occurrences_refuse_ambiguity",
      "ac_id": "FR-1046-AC-4",
      "trace": "correct",
      "limitation": "bounded unrolling only; native lasso cycle not covered"
    },
    {
      "test_id": "receipt_bytes_use_the_exact_aggregate_dependent_allowance",
      "ac_id": "FR-1046-AC-9",
      "trace": "correct"
    }
  ]
}
```

## Dispositions

Round 1 at `a119dbf556daf4df3c24c8cf8be80b1f650d05d8`, production source qualified at direct parent `44702d380422543a6cc7d13b3cfdceb11dad0499`. This is the same original review and its first disposition; no new initial review, matrix sweep, Cargo or tracker access occurred.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Native non-terminal cycle recommendations contribute ambiguity while stem-only records remain unique; authentic max=None cycle and stem-only controls passed on 44702d380422543a6cc7d13b3cfdceb11dad0499. |
| FND-002 | fixed | The depth-capped UniqueValue walk was removed; opaque receipts use the existing iterative quire-canonical arena through canonical comparison, integer checking, key normalization and selected/unselected intake. The separate newly introduced resource mapping defect is FND-004. |
| FND-003 | fixed | The unchanged full Package positive passed once explicitly at the qualified production head and then passed in its normal unignored selection at the ignore-only child a119dbf556daf4df3c24c8cf8be80b1f650d05d8. |
| FND-004 | still-open | Known arena allocation failures are currently mapped to invalid receipt data; classifier errors can re-enter the recursive ordinary-document decoder. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The arena integration collapses known reader and encoder allocation failures into malformed receipt data and reduces classifier failures to false, so valid receipts can be blamed for resource faults or re-enter recursive parsing. | fsl-analyze/src/analyze/document/effect_decision.rs:19-31,208-220,261-296 |

The arena reader now removes the original depth cap, but its known `ReadError::Allocation { requested }` error falls through the wildcard malformed/invalid-value arm. Its boolean classifier uses `is_ok_and`, so an arena allocation failure there becomes false and can re-enter ordinary recursive Value parsing. All three new encoder sites (whole canonical comparison, component encoding and request-key normalization) also collapse errors into invalid-value. FR-259 Behavior 6 and Foundation ALLOCATION_FAILED already require reader or encoder allocation failure to retain resource_exhausted/allocation-failed and requested bytes, with no configured bound or setting. Classifier propagation must reach Command and all three replay-document helper callers. A whole-document encoder limit at input length before canonical comparison can instead establish noncanonical input (for example numeric spelling 1e5 expanding to 100000); that is not a caller resource ceiling. Component/key limits after canonical admission are internal derived bounds, not new caller settings. Preserve those phase distinctions. The same distinction applies to known typed resource failures: a valid receipt is not invalid merely because a machine/resource operation failed. Preserve an existing typed resource/internal-failure path, including classifier propagation; do not add a receipt catalog cause or schema field. The already existing clause_item.rs read helper distinguishes Allocation from Malformed, so this is an established boundary distinction, not a new capability request. The lead confirmed FND-004 must be fixed in the second and final permitted semantic fix round.

## Round 1 verification

- Inspected the cycle-aware native-prefix loop and authentic max=None fixtures. A matching event at or after a non-terminal Lasso.entry is ambiguous; a unique stem recommendation before an unrelated cycle still binds. The current fixture proves real native Source cycle topology and does not mutate a fabricated lasso or add item-origin identity.
- Inspected the pinned shared arena reader, complete canonical comparison, iterative integer walk and five-component request-key encoder. No opaque subtree is converted to recursive Value. Command intake skips that ordinary decoder for selected and unselected recognized receipts; bytes are charged by intake.file before any parse. The depth-160 selected and unselected public prepare_analyze boundary runs on 256 KiB; normal Source binding/replay and ordinary observation mutation controls also pass.
- The 1e30 grant scalar remains valid under the preexisting integer-versus-finite-float classification; unsafe integer controls pass. Full node/role/instance/actor/operation/receiver/ordered-arguments matching, item locality, retry/key rules, null details/locus, premise exclusion and no-output behavior are preserved. No new receipt format, nested schema, authorization or effect path is introduced.
- Existing captured Source command: qsl2741-r1-bridge-c4 at 44702, rc0, 22 passed, 0 failed, 1 ignored, 186 filtered, 2.13 s. Its raw log was read locally by this reviewer. All seven recommendation controls are among those actual passes; the sole ignored test was the Package positive being qualified separately.
- Root's actual full Package positive: qsl2741-r1-package-c2 at 44702, rc0, 1 passed/0 failed/0 ignored, 208 filtered, 0.42 s. First c1 ENOSPC compile ran no test. The full oracle preserves Source counterpart/replay, public compile/handoff namespace, Package baseline without receipts, and receipt-bound Package equality to that baseline.
- Normal unignored Package positive: root confirmed qsl2741-r1-package-normal-c3 at a119, rc0, 1 passed/0 failed/0 ignored, 208 filtered, 0.27 s. Git verifies its sole change from 44702 is deleting the ignore attribute; all assertions remain unchanged. Shared binding plus the same Source-emitted full canonical recommendation now qualifies AC-2's receipt validation path, without asserting byte equality of unrelated Source/Package envelope metadata.
- Direct changed-fix rustfmt and diff checks exit 0. No new Cargo, full repository review, matrix run, tracker query/write or relay operation was performed. Public CLI proof for the broader QSL-2741 remains absent; this disposition does not close that ticket.

## Round 1 disposition data

```json
{
  "round": 1,
  "head": "a119dbf556daf4df3c24c8cf8be80b1f650d05d8",
  "base": "2f01e55faee9c79ee5b59b99ce85e22011f8e3b2",
  "scope": [
    {
      "id": "FR-1046-AC-1",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A real protocol analysis emits one typed recommendation before another role's model write. An FR-329 canonical document whose `decision` is that emitted object, supplied through `{file, identity}` and selected by `items[0].effect_decisions`, binds and returns the unchanged normal item: its native trace, replay, formal result and outcome member set equal the same run without the receipt."
    },
    {
      "id": "FR-1046-AC-2",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise."
    },
    {
      "id": "FR-1046-AC-3",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Changing only the emitted recommendation's type tag, node, role, instance, replicated actor, operation identity, typed receiver, one typed argument or argument order in `decision` refuses `invalid_runtime_input` / `invalid-value`, exit 20. The existing message begins `effect-decision wrong-decision` and names the selector, document and `/decision`, while `details` and `locus` are null; equal display names do not make distinct declaration identities match."
    },
    {
      "id": "FR-1046-AC-4",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A receipt whose decision matches a recommendation of item 0, but is selected only by item 1 and has no equal actual recommendation in item 1, refuses for item 1. If item 1 independently emits exactly one equal actual recommendation, the receipt binds there even when item 0 also emits an equal record. A native trace containing two indistinguishable occurrences within the selected item refuses `invalid_runtime_input` / `invalid-value` with message prefix `effect-decision ambiguous-decision`; no occurrence is chosen by discovery order and no item-origin field or decision id is generated."
    },
    {
      "id": "FR-1046-AC-5",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Selecting the same canonical receipt twice, including under two provision identities, returns the same normal outcome as one selection and performs no additional protocol step or external effect. Two different transport receipts with the same request key and canonical effect request validate against that same recommendation without an additional result entry or FSL-owned history."
    },
    {
      "id": "FR-1046-AC-6",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Two selected receipts with the same FR-329 request key and different canonical effect requests refuse `invalid_runtime_input` / `invalid-value` at the later document's `/effect_request`. The existing message begins `effect-decision request-key-contradiction` and names both provision identities; no successful receipt-binding output or replacement request is published."
    },
    {
      "id": "FR-1046-AC-7",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Invalid JSON, a missing required property, duplicate member, unknown outer member and mistyped property refuse `invalid_runtime_input` with `malformed-json`, `missing-member`, `duplicate-member`, `unknown-member` and `wrong-value-kind`, respectively. A wrong contract version, out-of-domain value and non-canonical encoding use `invalid-value`. Each message begins `effect-decision malformed-receipt`; no data is repaired or converted into another canonical document version. The catalog selection comes from typed variants, not message parsing."
    },
    {
      "id": "FR-1046-AC-8",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A non-array item member, malformed or unresolved document selector, and a top-level `effect_decisions` member each refuse `invalid-request`, exit 20, at their request pointer. Absent and empty item selectors retain the old request identity and outcome. No selected receipt is forwarded into the admitted FR-331 request item, and all outcomes validate against the unchanged published schema."
    },
    {
      "id": "FR-1046-AC-9",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "With the request below its independent request-byte ceiling, the real selected program, model and canonical receipt files are admitted at their exact aggregate dependent-byte allowance. One byte less gives the existing `resource_exhausted` incomplete outcome and original dependent-byte ceiling before receipt consumption; the receipt bytes are neither omitted nor read through an uncharged path."
    },
    {
      "id": "FR-1046-AC-10",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A malformed selected receipt paired with a wrong decision fails canonical admission first. After canonical admission, a wrong decision on the first selector precedes a later key contradiction; a valid first selector followed by a conflicting request reports that later contradiction. Other request items keep their original settled records."
    },
    {
      "id": "FR-1046-AC-11",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A bounded incomplete or unsupported protocol item with no binding-eligible trace retains its own cause and stage and reports no accepted receipts; it does not report a successful binding or a nonexistent-decision fact about an unexplored continuation."
    },
    {
      "id": "FR-1046-AC-12",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Adding a transport receipt, grant, effect request, lineage or optional effect observation changes neither model fields nor compensation registrations, queues or formal truth. The binding path calls no adapter, starts no process and opens no network connection; receipt admission makes no claim that the supplied grant authorizes the action or the business effect occurred."
    },
    {
      "id": "FR-277",
      "path": "spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md",
      "role": "examined",
      "excerpt": "Nesting depth is not a limit. Node, byte and work limits bound every stage,\nand no stage recurses on the native stack (ADR-030 D-1)."
    },
    {
      "id": "Lasso",
      "path": "fsl-replay/src/model_check/counterexample.rs",
      "role": "examined",
      "excerpt": "The steps of the lasso are [`TemporalCounterexample::prefix`]: the stem,\n/// then the loop. The loop re-enters at the position `entry`, so its steps\n/// are the prefix's steps from `entry` on and the last post-state of the\n/// prefix is the state at position `entry`."
    },
    {
      "id": "FR-329",
      "path": "filament-specification:spec/objects/interfaces/FR-329-effect-decision-receipt.md",
      "role": "context_only",
      "excerpt": "A document whose `contract_version` is not that identifier, missing required\nproperties, duplicate keys, out-of-domain values and non-canonical encodings\nrefuse before consumption."
    },
    {
      "id": "ProtocolTransition::recommendation_document",
      "path": "fsl-eval/src/simulation/protocol/identity.rs",
      "role": "examined",
      "excerpt": "Uses the same projection as the trace writer."
    },
    {
      "id": "SubjectBinding::replay_documents",
      "path": "fsl-analyze/src/analyze.rs",
      "role": "examined",
      "excerpt": "Formal replay premises exclude descriptive effect-decision receipts."
    },
    {
      "id": "FR-001-AC-7",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key.",
      "excerpt_part": "1/1"
    },
    {
      "id": "FR-056-AC-60",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`. From initial `a` `3ff0000000000000` (1.0) it reaches exactly the states 1.0 and `bff0000000000000` (-1.0), with one candidate per application. From initial `a` `0000000000000000` (+0) it reaches exactly +0 and `8000000000000000` (-0), two states with distinct keys, and each application has two candidates, because both zeros are `numericEqual` to the solved value (FR-120-AC-34). A",
      "excerpt_part": "1/2"
    },
    {
      "id": "FR-056-AC-60",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "simulation run applying `flip()` twice from 1.0 ends at 1.0 with no flag.",
      "excerpt_part": "2/2"
    },
    {
      "id": "FR-259",
      "path": "spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md",
      "role": "examined",
      "excerpt": "When `quire-canonical` cannot reserve memory for\n   a read or an encoding, FSL SHALL refuse with\n   `resource_exhausted`/`allocation-failed`, carrying `requested`, the size\n   in bytes of the reservation that failed. It is not a limit: it names no\n   bound and no setting. FSL SHALL NOT report it as a limit outcome, a\n   malformed value or malformed input."
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "FR-1046-AC-4",
      "related_ids": [
        "FR-1046"
      ],
      "path": "fsl-analyze/src/analyze/document/effect_decision.rs",
      "lines": "366-386",
      "excerpt": "A receipt whose decision matches a recommendation of item 0, but is selected only by item 1 and has no equal actual recommendation in item 1, refuses for item 1. If item 1 independently emits exactly one equal actual recommendation, the receipt binds there even when item 0 also emits an equal record. A native trace containing two indistinguishable occurrences within the selected item refuses `invalid_runtime_input` / `invalid-value` with message prefix `effect-decision ambiguous-decision`; no occurrence is chosen by discovery order and no item-origin field or decision id is generated.",
      "finding": "Occurrence counting ignores the native lasso cycle: one matching recommendation in a non-terminal repeated cycle is counted once and accepted instead of refusing ambiguous-decision."
    },
    {
      "fnd": "FND-002",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "FR-277",
      "related_ids": [
        "ADR-030",
        "FR-1046-AC-7",
        "FR-1046-AC-9"
      ],
      "path": "fsl-analyze/src/analyze/document/effect_decision.rs",
      "lines": "212-229,505-579",
      "excerpt": "Nesting depth is not a limit. Node, byte and work limits bound every stage,\nand no stage recurses on the native stack (ADR-030 D-1).",
      "finding": "The new recursive Serde receipt decoder imposes the pinned default depth cap of 128 on otherwise valid opaque receipt objects, returning invalid-value instead of admitting a below-budget canonical document; command intake also reparses receipts into recursive Value."
    },
    {
      "fnd": "FND-003",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "test-intent",
      "artifact_id": "FR-1046-AC-2",
      "related_ids": [
        "FR-620"
      ],
      "path": "fsl-command/tests/it/effect_decision.rs",
      "lines": "564-619",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise.",
      "finding": "The newly registered, unignored Package positive test is known to fail before receipt binding and would add a default-suite failure if the bounded Source slice is merged unchanged."
    }
  ],
  "new_findings": [
    {
      "fnd": "FND-004",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "FR-259",
      "related_ids": [
        "FR-1046-AC-7",
        "FR-624",
        "FR-277"
      ],
      "path": "fsl-analyze/src/analyze/document/effect_decision.rs",
      "lines": "19-31,208-220,261-296",
      "excerpt": "When `quire-canonical` cannot reserve memory for\n   a read or an encoding, FSL SHALL refuse with\n   `resource_exhausted`/`allocation-failed`, carrying `requested`, the size\n   in bytes of the reservation that failed. It is not a limit: it names no\n   bound and no setting. FSL SHALL NOT report it as a limit outcome, a\n   malformed value or malformed input.",
      "finding": "The arena integration collapses known reader and encoder allocation failures into malformed receipt data and reduces classifier failures to false, so valid receipts can be blamed for resource faults or re-enter recursive parsing."
    }
  ],
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "reason": "Native non-terminal cycle recommendations contribute ambiguity while stem-only records remain unique; authentic max=None cycle and stem-only controls passed on 44702d380422543a6cc7d13b3cfdceb11dad0499.",
      "after_excerpt": "if counterexample\n                        .lasso\n                        .as_ref()\n                        .is_some_and(|lasso| !lasso.terminal_stutter && position >= lasso.entry)\n                    {\n                        // The prefix holds one traversal of this native cycle.\n                        // Its recommendation event occurs on every traversal.\n                        *count = 2;",
      "artifact_id": "FR-1046-AC-4",
      "related_ids": [
        "FR-1046"
      ],
      "path": "fsl-analyze/src/analyze/document/effect_decision.rs",
      "lines": "366-386",
      "excerpt": "A receipt whose decision matches a recommendation of item 0, but is selected only by item 1 and has no equal actual recommendation in item 1, refuses for item 1. If item 1 independently emits exactly one equal actual recommendation, the receipt binds there even when item 0 also emits an equal record. A native trace containing two indistinguishable occurrences within the selected item refuses `invalid_runtime_input` / `invalid-value` with message prefix `effect-decision ambiguous-decision`; no occurrence is chosen by discovery order and no item-origin field or decision id is generated."
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "reason": "The depth-capped UniqueValue walk was removed; opaque receipts use the existing iterative quire-canonical arena through canonical comparison, integer checking, key normalization and selected/unselected intake. The separate newly introduced resource mapping defect is FND-004.",
      "after_excerpt": "let document = quire_canonical::read(bytes, bound).map_err(|error| match error {",
      "artifact_id": "FR-277",
      "related_ids": [
        "ADR-030",
        "FR-1046-AC-7",
        "FR-1046-AC-9"
      ],
      "path": "fsl-analyze/src/analyze/document/effect_decision.rs",
      "lines": "212-229,505-579",
      "excerpt": "Nesting depth is not a limit. Node, byte and work limits bound every stage,\nand no stage recurses on the native stack (ADR-030 D-1)."
    },
    {
      "fnd": "FND-003",
      "outcome": "fixed",
      "reason": "The unchanged full Package positive passed once explicitly at the qualified production head and then passed in its normal unignored selection at the ignore-only child a119dbf556daf4df3c24c8cf8be80b1f650d05d8.",
      "after_excerpt": "#[test]\n#[trace(\"FR-1046-AC-2\")]\nfn checked_package_receipts_match_the_same_actual_recommendation() {",
      "artifact_id": "FR-1046-AC-2",
      "related_ids": [
        "FR-620"
      ],
      "path": "fsl-command/tests/it/effect_decision.rs",
      "lines": "564-619",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise."
    },
    {
      "fnd": "FND-004",
      "outcome": "still-open",
      "reason": "Known reader and all three encoder allocation faults are currently misclassified as invalid receipt data; classifier errors can re-enter recursive ordinary-document parsing. Preserve existing typed allocation/resource or internal-failure paths without inventing a caller setting from receipt-local bounds.",
      "artifact_id": "FR-259",
      "related_ids": [
        "FR-1046-AC-7",
        "FR-624",
        "FR-277"
      ],
      "path": "fsl-analyze/src/analyze/document/effect_decision.rs",
      "lines": "19-31,208-220,261-296",
      "excerpt": "When `quire-canonical` cannot reserve memory for\n   a read or an encoding, FSL SHALL refuse with\n   `resource_exhausted`/`allocation-failed`, carrying `requested`, the size\n   in bytes of the reservation that failed. It is not a limit: it names no\n   bound and no setting. FSL SHALL NOT report it as a limit outcome, a\n   malformed value or malformed input."
    }
  ],
  "evidence": {
    "source": {
      "head": "44702d380422543a6cc7d13b3cfdceb11dad0499",
      "tag": "qsl2741-r1-bridge-c4",
      "rc": 0,
      "passed": 22,
      "failed": 0,
      "ignored": 1,
      "filtered": 186,
      "seconds": 2.13,
      "raw": "/private/tmp/qsl-2741-receipt-implementation-review/round1-source-c4.log"
    },
    "package_explicit": {
      "head": "44702d380422543a6cc7d13b3cfdceb11dad0499",
      "tag": "qsl2741-r1-package-c2",
      "rc": 0,
      "passed": 1,
      "failed": 0,
      "ignored": 0,
      "filtered": 208,
      "seconds": 0.42,
      "provenance": "root actual full-oracle execution receipt"
    },
    "package_normal": {
      "head": "a119dbf556daf4df3c24c8cf8be80b1f650d05d8",
      "tag": "qsl2741-r1-package-normal-c3",
      "rc": 0,
      "passed": 1,
      "failed": 0,
      "ignored": 0,
      "filtered": 208,
      "seconds": 0.27,
      "provenance": "root actual full-oracle execution receipt"
    },
    "resource_attempt": "First Package c1 stopped ENOSPC during compilation before runtime; it contributes zero passing or failing runtime cases.",
    "head_delta": "44702 -> a119 is only deletion of the ignore attribute; full positive assertions unchanged, verified with git.",
    "publication": "ROOT CENTRAL ONLY; these private pending comments are not yet posted/read back."
  }
}
```
