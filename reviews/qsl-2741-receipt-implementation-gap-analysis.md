---
id: SR-202611210022442313144108547
title: "QSL-2741 receipt binding gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; whole repository static matrix, public-item inventory and stub/extraction scan; scoped QSL-2741 PR3365 and FR-1046"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-1046", type: references}
---

## Summary

Planless repository gap analysis at frozen source `f6d7f7fe289b8819a7cdf071e82245f9ccd7ceca`, with a detailed receipt-binding acceptance check. The computed static tags do not establish execution acceptance: the Package counterpart is actually failing before binding, and the separate Code/Rust review identifies Source lasso/depth defects.

## Verdict

**FAIL globally; scoped receipt acceptance findings resolved after final disposition round 2.** The historical unrelated baseline FND-001/002/003 remains accepted-no-change for this bounded PR only. Package FND-004 remains fixed and passes again in the final focused command cases; the separate Code/Rust FND-004 resource projection is now fixed too. No new global matrix or reverse-audit PASS, and no whole-ticket/CLI completion, is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The fresh full-repository matrix reports 1133 untagged criteria, including live FR-001-AC-7, so this audit cannot establish repository-wide traceability. | spec/functional/FR-001-read-exact-source.md:AC-7 |
| FND-002 | high | The fresh full-repository matrix reports 79 criteria bound only by ignored tests, including FR-056-AC-60; ordinary execution does not verify those rows. | fsl-eval/tests/it/model_simulation.rs:2625 |
| FND-003 | high | The full-repository reverse audit remains incomplete: 15594 public-item candidate lines were inventoried without exhaustive requirement mapping, and global extraction diagnostics and unmatched tags remain unclassified. | source-tree:repository |
| FND-004 | high | FR-1046-AC-2 is tagged but unqualified: the actual Package baseline without receipts refuses unsupported_construct/not-yet-implemented before recommendation binding, so Source/Package parity has no passing evidence. | fsl-command/tests/it/effect_decision.rs:566-619 |

## Matrix verification

quoin 0.36.7 matrix --repo /Users/peter/dev/qsl-2741-receipt-spec --json exited 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. This refusal was recorded once, without retry or old binary. The lead/architecture-approved static fallback `quire matrix --scope /Users/peter/dev/qsl-2741-receipt-spec --format json` exited 0, and `quire coverage --scope ... --json` exited 0.

The fresh computed matrix contains 6221 criteria: 4890 tagged, 1133 untagged, 79 tagged-by-ignored-test, 106 method-without-symbol and 13 retired. These counts are from this candidate, not copied from the newer/different-base QSL-2691 snapshot. FR-1046 has eleven Test criteria tagged and one Inspection criterion classified method-without-symbol. AC-2 is tagged to checked_package_receipts_match_the_same_actual_recommendation; the actual c5 evidence is 12 passed / 1 failed / 0 ignored, and this exact Package test is the failure. Static tagged status is not a successful qualification record.

Coverage extraction reports Rust census candidates 13689, tagged 13119 and bound 12598, with 22792 diagnostics and 5832 untracked-symbol entries. Symbol collisions, marker/extraction warnings and unmatched references have not all been classified; none is silently promoted to good coverage. No evidence-backed matrix was available after the single CORE_REFUSED result.

## Reverse gaps, stubs and scoped mapping

A full-source public-item regex inventory produced 15594 candidate lines; it includes functions, methods, structs/enums and test helpers and is not an exhaustive production behavior-to-requirement mapping. The audit therefore retains FND-003 rather than claiming a complete reverse PASS. Scoped source/test scans found no todo!/unimplemented! placeholder in the new receipt decoder, matcher, projection or receipt integration tests. The implementation is genuine behavior, not a mock confirmation. Global candidate classification and extraction repair remain incomplete.

The new receipt selectors, canonical admission, actual native recommendation matching, retry/key rules, refusal projection and replay-premise exclusion map to FR-1046; the shared canonical formatter preserves FR-822. Test tags for Source normal behavior, canonical component rejection, same-item isolation, retries/contradictions, canonical malformed causes, selector/schema identity, exact dependent bytes and no-trace outcomes name the intended criteria. The bounded max-2 ambiguity test is genuine but does not exercise a native repeated lasso cycle, and there is no deep opaque-object test: those concrete defects are recorded once in the separate Code/Rust artifact. No duplicate parser or alternate receipt format is specified or justified.

## Acceptance limits

The existing public Package baseline has the same Source-supported selector, public compile package_name recommendation, emitted protocol node recommendation:[Review]:0 and source map example:recommendation bytes 355..361. It still refuses the declaration family before consuming receipts. The full positive oracle must remain intact and AC-2 remains unmet until genuine Package execution succeeds; an Unsupported expectation cannot be counted as parity. No new public CLI run at the post-9943 cohort was supplied. Normal Source command tests do not qualify that separate CLI boundary.

## Coverage

Reconciliation: quire matrix static fallback after one quoin CORE_REFUSED; no run evidence store was read. Optional whole-repository semantic gap analysis was not requested; scoped implementation/test faithfulness is part of the separately required code review. No Cargo was run. No plan was supplied. Plan completion: not assessed.

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
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-001-AC-7",
      "related_ids": [],
      "path": "spec/functional/FR-001-read-exact-source.md",
      "lines": "AC-7",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key.",
      "finding": "The fresh full-repository matrix reports 1133 untagged criteria, including live FR-001-AC-7, so this audit cannot establish repository-wide traceability."
    },
    {
      "fnd": "FND-002",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-056-AC-60",
      "related_ids": [],
      "path": "fsl-eval/tests/it/model_simulation.rs",
      "lines": "2625",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`.",
      "finding": "The fresh full-repository matrix reports 79 criteria bound only by ignored tests, including FR-056-AC-60; ordinary execution does not verify those rows."
    },
    {
      "fnd": "FND-003",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "repository-public-surface",
      "related_ids": [],
      "path": "source-tree",
      "lines": "repository",
      "excerpt": "Full repository public-item candidates and their owning requirements.",
      "finding": "The full-repository reverse audit remains incomplete: 15594 public-item candidate lines were inventoried without exhaustive requirement mapping, and global extraction diagnostics and unmatched tags remain unclassified."
    },
    {
      "fnd": "FND-004",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-1046-AC-2",
      "related_ids": [],
      "path": "fsl-command/tests/it/effect_decision.rs",
      "lines": "566-619",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise.",
      "finding": "FR-1046-AC-2 is tagged but unqualified: the actual Package baseline without receipts refuses unsupported_construct/not-yet-implemented before recommendation binding, so Source/Package parity has no passing evidence."
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

Round 1 at `a119dbf556daf4df3c24c8cf8be80b1f650d05d8`. The original repository findings and matrix counts above are the historical initial-review snapshot; no fresh global matrix/reverse sweep was requested or performed for this disposition.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | Root explicitly accepts the existing unrelated repository-wide traceability baseline for this bounded PR only. The global gap verdict remains FAIL; no current matrix sweep was requested or run. |
| FND-002 | accepted-no-change | Root explicitly accepts the existing unrelated ignored-only baseline for this bounded PR only. This does not qualify ignored tests or claim global coverage. |
| FND-003 | accepted-no-change | Root explicitly accepts the existing incomplete global reverse/extraction audit for this bounded PR only; no global assurance PASS is claimed. |
| FND-004 | fixed | Actual full Source-derived receipt/Package parity now passes: the public handoff Package baseline executes, and the receipt-bound Package outcome equals that baseline. The same unchanged full oracle also passed normally after ignore removal. |

## Round 1 acceptance evidence

The unchanged complete Package receipt positive now executes successfully after the native Package bridge base landed. The actual 44702 explicit run passed one test, and the normal unignored a119 run passed that same complete test. This is the actual Source-derived receipt, public compile/handoff, receipt-free Package baseline and receipt-bound Package equality oracle; receipt-free controls alone were not substituted for AC-2. Source c4 supplied 22 actual passes including the genuine native cycle, stem-only, depth-160 selected/unselected intake and all seven prior recommendation controls. The Package ignore was removed, not weakened into an Unsupported expectation.

AC-2's previously missing Package evidence is therefore fixed. The broader QSL-2741 public CLI proof remains unprovided and the ticket is not closed here. The arena's newly introduced resource/fault misclassification is a separate Code/Rust FND-004 in the same original review, still open. Global FND-001/002/003 are accepted-no-change only under the root's explicit bounded-PR disposition; the global gap verdict stays FAIL and no ignored row or reverse-audit gap is qualified by that exception.

No new initial review, Cargo, matrix sweep or tracker operation occurred. Root will publish and round-trip the pending disposition comments centrally. Plan completion: not assessed.

## Round 1 disposition data

```json
{
  "round": 1,
  "head": "a119dbf556daf4df3c24c8cf8be80b1f650d05d8",
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
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-001-AC-7",
      "related_ids": [],
      "path": "spec/functional/FR-001-read-exact-source.md",
      "lines": "AC-7",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key.",
      "finding": "The fresh full-repository matrix reports 1133 untagged criteria, including live FR-001-AC-7, so this audit cannot establish repository-wide traceability."
    },
    {
      "fnd": "FND-002",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-056-AC-60",
      "related_ids": [],
      "path": "fsl-eval/tests/it/model_simulation.rs",
      "lines": "2625",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`.",
      "finding": "The fresh full-repository matrix reports 79 criteria bound only by ignored tests, including FR-056-AC-60; ordinary execution does not verify those rows."
    },
    {
      "fnd": "FND-003",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "repository-public-surface",
      "related_ids": [],
      "path": "source-tree",
      "lines": "repository",
      "excerpt": "Full repository public-item candidates and their owning requirements.",
      "finding": "The full-repository reverse audit remains incomplete: 15594 public-item candidate lines were inventoried without exhaustive requirement mapping, and global extraction diagnostics and unmatched tags remain unclassified."
    },
    {
      "fnd": "FND-004",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-1046-AC-2",
      "related_ids": [],
      "path": "fsl-command/tests/it/effect_decision.rs",
      "lines": "566-619",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise.",
      "finding": "FR-1046-AC-2 is tagged but unqualified: the actual Package baseline without receipts refuses unsupported_construct/not-yet-implemented before recommendation binding, so Source/Package parity has no passing evidence."
    }
  ],
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "accepted-no-change",
      "reason": "Root explicitly accepts the existing unrelated repository-wide traceability baseline for this bounded PR only. The global gap verdict remains FAIL; no current matrix sweep was requested or run.",
      "artifact_id": "FR-001-AC-7",
      "related_ids": [],
      "path": "spec/functional/FR-001-read-exact-source.md",
      "lines": "AC-7",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key."
    },
    {
      "fnd": "FND-002",
      "outcome": "accepted-no-change",
      "reason": "Root explicitly accepts the existing unrelated ignored-only baseline for this bounded PR only. This does not qualify ignored tests or claim global coverage.",
      "artifact_id": "FR-056-AC-60",
      "related_ids": [],
      "path": "fsl-eval/tests/it/model_simulation.rs",
      "lines": "2625",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`."
    },
    {
      "fnd": "FND-003",
      "outcome": "accepted-no-change",
      "reason": "Root explicitly accepts the existing incomplete global reverse/extraction audit for this bounded PR only; no global assurance PASS is claimed.",
      "artifact_id": "repository-public-surface",
      "related_ids": [],
      "path": "source-tree",
      "lines": "repository",
      "excerpt": "Full repository public-item candidates and their owning requirements."
    },
    {
      "fnd": "FND-004",
      "outcome": "fixed",
      "reason": "Actual full Source-derived receipt/Package parity now passes: the public handoff Package baseline executes, and the receipt-bound Package outcome equals that baseline. The same unchanged full oracle also passed normally after ignore removal.",
      "after_excerpt": "fixture.select(\"package\", &fixture.receipt.clone());\n    let bound = fixture.run();\n    assert_eq!(bound.document, baseline.document, \"{}\", bound.document);",
      "artifact_id": "FR-1046-AC-2",
      "related_ids": [],
      "path": "fsl-command/tests/it/effect_decision.rs",
      "lines": "566-619",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise."
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

## Final disposition round 2 evidence

Final frozen head `19d75436a8067688a5e5c3c4eed83a889cade09f`, base `2f01e55faee9c79ee5b59b99ce85e22011f8e3b2`. The actual final focused c5 run passes all 34 cases (3 AnalyzeLib, 6 AnalyzeIT, 1 CommandLib, 24 CommandIT), zero failed/ignored. The full Package receipt positive is among those 24 ordinary command cases, so prior scoped Gap FND-004 / FR-1046-AC-2 remains fixed. The Source deep-object, lasso-cycle, stem-only, full identity/canonical matching, retry, exact-byte and no-trace controls also remain selected. The separate Code/Rust FND-004 now preserves existing allocation/resource evidence through all affected projections.

C4's real fixture failure is retained rather than counted as a passing run: the model test selected a fixture with no Bounded clause, after AnalyzeLib3P and AnalyzeIT5P/1F; Command did not run. Git verifies the only final change is using the existing real bounded counter fixture. No assertion, normative criterion or production refusal behavior was weakened. Earlier checkpoints were not used as final qualification.

No fresh matrix or reverse audit was performed, so the initial 1133-untagged/79-ignored-only counts above remain historical snapshot findings, not current-tree measurements. Prior global FND-001/002/003 scoped exceptions remain accepted-no-change and the global gap verdict stays FAIL. Existing public CLI proof is still outside the supplied evidence and whole QSL-2741 is not closed here. This is the final disposition of the same review, not another initial review. Plan completion: not assessed.

## Round 2 disposition data

```json
{
  "round": 2,
  "final": true,
  "head": "19d75436a8067688a5e5c3c4eed83a889cade09f",
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
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-001-AC-7",
      "related_ids": [],
      "path": "spec/functional/FR-001-read-exact-source.md",
      "lines": "AC-7",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key.",
      "finding": "The fresh full-repository matrix reports 1133 untagged criteria, including live FR-001-AC-7, so this audit cannot establish repository-wide traceability."
    },
    {
      "fnd": "FND-002",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-056-AC-60",
      "related_ids": [],
      "path": "fsl-eval/tests/it/model_simulation.rs",
      "lines": "2625",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`.",
      "finding": "The fresh full-repository matrix reports 79 criteria bound only by ignored tests, including FR-056-AC-60; ordinary execution does not verify those rows."
    },
    {
      "fnd": "FND-003",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "repository-public-surface",
      "related_ids": [],
      "path": "source-tree",
      "lines": "repository",
      "excerpt": "Full repository public-item candidates and their owning requirements.",
      "finding": "The full-repository reverse audit remains incomplete: 15594 public-item candidate lines were inventoried without exhaustive requirement mapping, and global extraction diagnostics and unmatched tags remain unclassified."
    },
    {
      "fnd": "FND-004",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-1046-AC-2",
      "related_ids": [],
      "path": "fsl-command/tests/it/effect_decision.rs",
      "lines": "566-619",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise.",
      "finding": "FR-1046-AC-2 is tagged but unqualified: the actual Package baseline without receipts refuses unsupported_construct/not-yet-implemented before recommendation binding, so Source/Package parity has no passing evidence."
    }
  ],
  "new_findings": [],
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "accepted-no-change",
      "reason": "Root explicitly accepts the existing unrelated repository-wide traceability baseline for this bounded PR only. The global gap verdict remains FAIL; no current matrix sweep was requested or run.",
      "artifact_id": "FR-001-AC-7",
      "related_ids": [],
      "path": "spec/functional/FR-001-read-exact-source.md",
      "lines": "AC-7",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key."
    },
    {
      "fnd": "FND-002",
      "outcome": "accepted-no-change",
      "reason": "Root explicitly accepts the existing unrelated ignored-only baseline for this bounded PR only. This does not qualify ignored tests or claim global coverage.",
      "artifact_id": "FR-056-AC-60",
      "related_ids": [],
      "path": "fsl-eval/tests/it/model_simulation.rs",
      "lines": "2625",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`."
    },
    {
      "fnd": "FND-003",
      "outcome": "accepted-no-change",
      "reason": "Root explicitly accepts the existing incomplete global reverse/extraction audit for this bounded PR only; no global assurance PASS is claimed.",
      "artifact_id": "repository-public-surface",
      "related_ids": [],
      "path": "source-tree",
      "lines": "repository",
      "excerpt": "Full repository public-item candidates and their owning requirements."
    },
    {
      "fnd": "FND-004",
      "outcome": "fixed",
      "reason": "Actual full Source-derived receipt/Package parity now passes: the public handoff Package baseline executes, and the receipt-bound Package outcome equals that baseline. The same unchanged full oracle also passed normally after ignore removal.",
      "after_excerpt": "fixture.select(\"package\", &fixture.receipt.clone());\n    let bound = fixture.run();\n    assert_eq!(bound.document, baseline.document, \"{}\", bound.document);",
      "artifact_id": "FR-1046-AC-2",
      "related_ids": [],
      "path": "fsl-command/tests/it/effect_decision.rs",
      "lines": "566-619",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise."
    }
  ],
  "evidence": {
    "head": "19d75436a8067688a5e5c3c4eed83a889cade09f",
    "production_assessed_head": "390416401105f8c279a665684cf0848007ec3333",
    "base": "2f01e55faee9c79ee5b59b99ce85e22011f8e3b2",
    "tag": "qsl2741-r2-full-focused-c5",
    "rc": 0,
    "raw": "/private/tmp/qsl2741-r2-full-focused-c5.log",
    "binaries": [
      {
        "binary": "fsl-analyze lib",
        "passed": 3,
        "failed": 0,
        "ignored": 0,
        "filtered": 196,
        "seconds": 0
      },
      {
        "binary": "fsl-analyze it",
        "passed": 6,
        "failed": 0,
        "ignored": 0,
        "filtered": 2787,
        "seconds": 0.23
      },
      {
        "binary": "fsl-command lib",
        "passed": 1,
        "failed": 0,
        "ignored": 0,
        "filtered": 98,
        "seconds": 0.12
      },
      {
        "binary": "fsl-command it",
        "passed": 24,
        "failed": 0,
        "ignored": 0,
        "filtered": 186,
        "seconds": 3.12
      }
    ],
    "total_passed": 34,
    "total_failed": 0,
    "total_ignored": 0,
    "retained_c4": {
      "head": "390416401105f8c279a665684cf0848007ec3333",
      "raw": "/private/tmp/qsl2741-r2-full-focused-c4.log",
      "analyze_lib": "3P/0F/0I",
      "analyze_it": "5P/1F/0I",
      "command": "NOT RUN",
      "cause": "model fixture lacked the actual Bounded clause; derived_request.rs184 index0 with zero clauses"
    },
    "final_delta": "Only one model-settle fixture line changed from CONTRACT to derived_request::counter_body(\"\"); production code unchanged. Git diff verified.",
    "earlier_checkpoint": "16c0 boundary/public passes were not used to qualify later final production changes.",
    "runtime_provenance": "Root confirmed rc0/frozenhead; reviewer independently read full local c5 and retained c4 raw logs. No reviewer Cargo or remote runtime command.",
    "publication": "Root central pending comment publication/readback only."
  },
  "limits": "Global gap FAIL remains; prior global baselines accepted-no-change for this bounded PR only. Package AC2 remains fixed and is included in actual final CommandIT24. No fresh global counts or CLI proof claimed."
}
```
