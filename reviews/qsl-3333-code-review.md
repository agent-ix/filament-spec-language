---
id: SR-967175026302197768900474181941935084864
title: "Code review — filament-spec-language QSL-3333 State-object rejection loci (PR #3949)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/certificate.rs, fsl-replay/src/certificate/product/relation.rs, fsl-analyze/tests/it/{component_certificate,relation_certificate,state_graph_settle,zone_certificate,exact_probabilistic_item}.rs; FSpec FR-331 + backend-provider-v1 schema (pinned 691ac5a6 = origin/main 53fe180b for these defs); FSL FR-163, FR-169-AC-14, FR-245, FR-338-AC-12"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-163", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-169", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-245", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-338", type: references }
---

## Summary

Ticket: QSL-3333. Reviewed PR #3949 at head `9edac2a8a33f638a059f3a5a0aaa30a898557726` (`git diff origin/main...HEAD`, one commit on base `d9888e39fc`), rust-review lane folded in. The 16 commits on origin/main since the base touch none of the changed files or the FSpec pin; `git merge-tree` merges cleanly.

The production change is small and correct. `ModelState` and `ModelKey` now serialize through the existing `TimedStateWire` (`{"key": <unpadded base64url>}`), which is FSpec FR-331's `State`/`StateLocus`. `HyperTransitionTuple` gets a hand-written `Serialize` that writes each edge's `pre_state`/`post_state` as `State`, as the schema's `HyperTransitionTupleLocus` requires. The HP-1 certificate edge (`HyperTransitionEdge`, used only by `relation_bytes`) keeps bare strings through a shared `EdgeWire<K>`. `HyperTupleStop.first_unpaid` is not serialized anywhere, so the tuple change reaches only the rejection `at`.

Dependents checked:
- Producers: state_graph.rs (ModelKey), zone_certificate.rs and task_check.rs (ModelState), and product/relation.rs (tuple), all through the one `Serialize for CertificateLocus`.
- Readers: nothing deserializes `CertificateLocus`. plugin_read.rs only names the cause kind.
- Unaffected: tools/fsl-probability-check uses only `ProbabilityProductState`. fsl-command plan_request.rs has its own `entry` cause. fsl-inspect and fsl-conformance have no reference to these loci. No docs or examples spell a `certificate-rejected` `at`.
- Test dependent: exact_probabilistic_item.rs, updated.

Spec split (question a): no conflict between the spec and the code. FSpec defines no schema for the HP-1 certificate closure. FSL FR-163 owns those bytes and requires "Each key SHALL serialize in the existing unpadded base64url StateKey spelling". FR-331's schema `HyperTransitionTupleLocus` requires `$ref State` only for the rejection locus. The wording of FR-163 is still loose: "carry the existing `pre_state` … of FSpec FR-331's `HyperTransitionTuple`" could also be read as State objects. That is a spec-clarity finding in the spec-review SR, not a code defect.

## Verdict

**PASS for this PR's change.** The findings below are pre-existing causes outside the change: FND-001 and FND-002 are separate tickets (drafted in the Linear comment), and FND-003 is a low test-independence nit. The tests that were run and their results are recorded in the Linear comment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The written `HyperTransitionTupleLocus` still fails the FR-331 schema on `transition`. `TimedTransitionWire` writes `operation` as the canonical `{node, package}` object (schema `Nonempty` string) and a reference receiver in the engine spelling. The new test `assert_tuple_conforms` replaces every edge's `transition` before validating, so only the State members are proven. Separate cause, in the writer shared with counterexample and not-markov transitions (`TaskSourceTransitionWire` already maps to a string). Pre-existing; separate ticket. | fsl-replay/src/outcome.rs:600-606,724-737; fsl-analyze/tests/it/relation_certificate.rs:603-629 |
| FND-002 | high | `ZoneNode`/`ZoneEdge`/`ZoneComponent` loci serialize `node`/`edge`/`component` as decimal strings. FSL FR-245 ("FSpec FR-331 writes them as its zone node, zone edge, zone component … forms") and the schema (`Count`, integer) require integers. `zone_loci_are_written_in_their_fr331_forms` asserts the non-conforming strings. Pre-existing on main; separate cause; separate ticket. | fsl-replay/src/certificate.rs:296-310; fsl-analyze/tests/it/zone_certificate.rs:409-431 |
| FND-003 | low | The comment says "Encode the complete canonical octets independently of the locus serializer", but the expected value is still produced by `CertificateLocus::ModelKey`'s serializer, now indexed `["key"]`. Use the independent `state_locus`/base64 encoding instead. | fsl-analyze/tests/it/exact_probabilistic_item.rs:368-376 |
