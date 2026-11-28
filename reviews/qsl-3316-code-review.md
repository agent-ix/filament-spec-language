---
id: SR-5590832
title: "Code review — QSL-3316 keep certificate identity node ids structural"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/probability_certificate/source.rs, fsl-semantics/src/check/mod.rs, fsl-semantics/src/check/wire_id.rs, fsl-analyze/src/analyze/document.rs, fsl-analyze/src/analyze/document/composition_item.rs, fsl-analyze/src/analyze/document/held_identity.rs, fsl-analyze/tests/it/analyze_cli_support.rs, fsl-analyze/tests/it/analyze_request_wire.rs, fsl-analyze/tests/it/probability_from_source.rs, tools/fsl-probability-check/tests/it/roundtrip.rs; PR #3926 at cb815d2f5e0040d8f3be068fc4cb0b2fcc27e6f4"
review_set: subset
---

## Summary

Ticket: QSL-3316. Code review with the rust-review lane folded in, over `git diff origin/main...HEAD` at cb815d2f5e0040d8f3be068fc4cb0b2fcc27e6f4. The change follows pinned FSpec 691ac5a6: backend-provider `CertificateIdentity.node_id` is `CertificateNodeId` (FR-201 structural id, path segments string or nested id), and FR-412 says it is not a checked-package-v2 wire id. Analyze now leaves a certificate identity's `node_id` structural. The independent checker reads it structurally, refuses a held one, and requires the identity model to be the claim's owning model.

## Verdict

**CHANGES REQUESTED.** The checker does not accept anything main refused. The owner check closes the one acceptance a structural model-bearing identity would otherwise open, and is exercised by the existing `foreign-model` case in `a_fresh_checker_refuses_altered_values_objectives_identities_ranks_and_provisions`. `owning_model` moved to `CheckedGraph` unchanged. `detach_node_ids` recursion is bounded (`parts` 2, at most 3 frames), and serde_json's RawValue skip does not recurse. One regression: a hostile-depth identity now overflows the stack in `HeldCertificate::resolve`. Main refused that input with a typed `Malformed`. The test expectation changes are required by FSpec FR-412 and are not weaker. No tests were run: cargo admission refused (lock C: MemAvailable 20482520 kB below the 31457280 floor; locks A and B held).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `HeldCertificate::resolve` turns the untrusted identity `node_id` (unbounded depth, read past serde's limit via RawValue) into a `serde_json::Value` with `node_key_to_json`, then encodes it through `fsl_foundation::canonical_json::encode` (`serde_json::to_value`, recursive Serialize) and drops it with serde_json's recursive Drop. filament-exact `node_wire` and quire-canonical `value.rs` both state that this overflows the stack on deep input and that `discard_node_json` / `drop_value` must be used. A certificate tens of thousands of levels deep aborts the checker process. Main refused the same bytes with `Malformed` at depth 128 (`serde_json::from_slice`). Reachable before any certificate byte ceiling: on the package path `resolve` runs first. | fsl-replay/src/probability_certificate/source.rs:497-524 |
| FND-002 | medium | `resolve` nulls only the top-level identity `model`, while `detach_node_ids` and analyze's `hold` keep a non-null model in each conditional part's identity (FR-621: every identity inside is non-null). The measure checker requires `numerator.identity == document.identity` and `denominator.identity == document.identity` (measure.rs:2298-2299), so a held workload `ratio` certificate as analyze writes it is always refused `MalformedMember("conditional")`. Not a soundness gap: a part with a wrong model can never be accepted. It is a completeness defect, already present in main's `resolve`. The new test `a_deeply_nested_certificate_identity_resolves` asserts exactly this unacceptable resolved shape (top `model:null`, numerator `model:"m"`). | fsl-replay/src/probability_certificate/source.rs:517-520, 582-588; fsl-replay/src/probability_certificate/document/measure.rs:2298-2299 |
| FND-003 | low | The doc comment moved below `owning_model` lost its space: `/// FR-081/FR-400:the completed effective declaration view`. | fsl-semantics/src/check/mod.rs:9827 |
| FND-004 | low | The new negative case in `certificate_checks_independently` asserts only `code != 0`. Any refusal passes, including one unrelated to the held identity. The unit test pins `Malformed`, but the fresh-process check should assert the refusal cause or status as well. | tools/fsl-probability-check/tests/it/roundtrip.rs:665-672 |

## Examined, no finding

- `check_recompiled` owner check (source.rs:345-351): it runs on both the source and package paths after recompilation and before `bind_claim`. It is skipped only when the request names no model, and then `resolve` requires the top model to be null too.
- `detach_node_ids` / `HeldCertificate::read`: every non-identity member still parses under serde's 128 limit, so only an identity `node_id` escapes it. Held detection is unchanged apart from excluding the detached ids.
- `value_bounds::unwrap` parses with `serde_json::from_slice` (limit 128). A deep structural identity in a lower/upper document is refused `Malformed`, which refuses rather than accepts. Main refused it as well (main's `resolve` produced the same structural bytes). This is not a regression; see the spec-review SR on FR-621-AC-16's scope.
- `wire_id::certificate_identity` skip in the held writer and `needs_hold`: only FSpec `CertificateIdentity` has the `{node_id, model, ordinal}` shape in the pinned backend-provider schema.
- Test changes: `analyze_request_wire` and `probability_from_source` now compare the identity with the held claim resolved through the package graph (`assert_identity_names_claim`), which is equivalent strength under FR-412. `probability_from_source` builds the same request `run()` built. `assert_no_nested_node_ids` exempts only certificate identities and asserts that their `node_id` is structural. `certificate_checks_independently` swaps the index-segment assertion for structural-identity and held-locus assertions and adds a held-identity refusal.

## Methods and limits

Read-only review. Cargo was not run: admission refused on lock C (memory) and locks A/B were held. FND-001's mechanism comes from the dependency documentation and code (filament-exact 08a5a17 `node_wire.rs`, quire-canonical v0.3.2 `value.rs`, fsl-foundation `canonical_json::encode`) and was not reproduced. FND-002 was traced by reading the measure reader and was not run. The author-reported remaining reds (timed_state_graph, and certificate_kinds under QSL-2903) were not re-run.
