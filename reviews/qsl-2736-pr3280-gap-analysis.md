---
id: SR-202611210022442313144108505
title: "QSL-2736 inverted delivery interval gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2736; fsl-command/tests/it/protocol_refusal_text.rs; FR-509-AC-2; TC-1304 row 2"
review_set: subset
---

## Summary

Ticket: QSL-2736. Reviewed only exact base 195707cf790ee26e50285ccff81e8723ae1bf320 to frozen head 59ee030fa44aadab338399eb01a407d3d9459f68, PR filament-spec-language#3280: 138 added lines in one existing Rust integration-test file. Parent boxing and formatting changes were excluded.

## Verdict

**CONDITIONAL — no scoped traceability or ownership defects; the computed matrix and repository-wide audit were excluded by the dispatch, so this is not a full repository gap-audit PASS.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Examined FR-509-AC-2: Source `delivery [4,3]` refuses `ill_typed`/`type-mismatch` at the interval naming 4 and 3; `delivery [-1,3]`, `delivery [0.5,3]` and `delivery [0,18446744073709551616]` each refuse `invalid_syntax` at the offending token; `delivery [2,2]`, `delivery [0,0]` and `delivery [0,18446744073709551615]` admit.

Examined TC-1304 row 2: `delivery [4,3]` → `ill_typed`/`type-mismatch` at the interval naming 4 and 3. Only that row and the scoped compile-command regression are claimed; other AC-2 branches retain their existing checker/parser evidence and were not newly qualified.

Both tests bind exact FSL trace IDs `TC-1304` and `FR-509-AC-2`. They call the real `command::compile_outcome` with the existing fixture model and request builder. Assertions require exit 20, Check refusal, first cause `ill_typed`/`type-mismatch`, no UnsupportedFeature, and exact interval byte span. The Boolean-channel case also requires the typed ChannelDeliveryRange bounds 4 and 3 and compiles a `[3, 3]` control. The record-channel fixture exercises a distinct profile/payload context.

Source ownership: fsl-semantics/src/check/protocol_controls.rs:243-253 already rejects lower > upper; its existing delivery_interval_checks_order_and_retains_valid_bounds test checks typed bounds and the authored interval. fsl-replay/src/spine.rs:1699 owns diagnostic formatting, with its existing channel_delivery_range_message_names_the_channel_and_both_counts test. This PR adds command-boundary regression protection and changes no production semantics. No existing assertion or gate is removed or weakened. Searched delivery helpers and ChannelDeliveryRange tests in fsl-command/tests and fsl-semantics/src; the existing checker helper is a different internal seam, and the existing command request/model fixture is reused. No copied upstream contract, schema, or semantic implementation is added.

The adapted filed-example fixture is not evidence of the original upstream driver's build or cause. The ticket's reported exit 21 is unreproduced; the external example's current negative-bound spelling does not establish what driver build produced the report. No upstream fix or cause is claimed.

Read AGENTS.md, CLAUDE.md, README.md, CONTRIBUTING.md and applicable reviewer, code-review, rust-review, rust-style/testing/implementation and gap-analysis instructions. No AssuranceProfile exists under spec/. No spec or CI workflow changes occur in the exact diff, so spec-review analyses do not apply.

No cargo, runtime tests, fmt, clippy, cargo-deny, matrix, or full workspace gate was run, per dispatch. Two targeted test filters are pending from the lead's Draco lane at this exact head. Computed-matrix verification and repository-wide reverse-gap/stub inventory are intentionally incomplete; only the changed test bodies, existing owning checker and diagnostic path were inspected. Optional repository semantic review was not run. These limitations are not code findings.

PR reports MERGEABLE against peter/qsl-2678-clippy-green. Merge requires parent PR #3278 to land, both actual targeted tests to PASS, and the lead's remaining gates. A later rebase onto the actual parent squash needs a source-identical check; this review does not authorize merging a changed patch. Ticket remains In Progress.
