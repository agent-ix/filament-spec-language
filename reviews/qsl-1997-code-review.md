---
id: SR-2026111612172400000020
title: "Code review — protocol batch 1 partial implementation"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-eval/src/value/expression/causes.rs, fsl-eval/src/value/expression/evaluate.rs, fsl-package/src/emit/tests/protocol_graph.rs, fsl-replay/src/spine.rs, fsl-semantics/src/check/mod.rs, fsl-semantics/src/check/protocol_clause/tests/graph.rs, fsl-semantics/src/check/protocol_clause/tests/rules.rs, fsl-semantics/src/check/protocol_controls.rs, fsl-semantics/src/check/protocol_controls/model_state_tests.rs, fsl-semantics/src/check/protocol_slots.rs, fsl-semantics/src/check/refusal.rs; QSL-1137, QSL-1142, QSL-1997, QSL-2215, QSL-2216, QSL-2219, QSL-2220, QSL-2221 partial slices"
review_set: subset
---

## Summary

Reviewed PR #2815 at `c38f1ad828195344ae068b2c1e21e9834214b3a4`, including Rust implementation, targeted tests, and repository conventions. The shipped partial slices are scoped clearly, but the FR-881 refusal does not retain all diagnostic data required by its acceptance criterion.

## Verdict

**CONDITIONAL** — one medium diagnostic-contract finding remains; the documented reader and end-to-end blockers are not treated as defects in this partial PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FIFO-key refusal omits the required operator and key text from its retained cause and rendered message | fsl-semantics/src/check/refusal.rs:414-421; fsl-replay/src/spine.rs:1383-1389 |

## Review notes

The cause stores `channel`, `key_type`, and `span`; the rendered message prints the type and channel. It does not retain the operator or key expression text as refusal fields or render either value. FR-881-AC-1 explicitly requires retention of `=`, the key text, channel, and type. The span lets a separate consumer consult source text if available, but it does not make the operator or key text part of the refusal record.

Targeted checks passed: the choice/duplicate-role, duplicate-protocol, IEEE FIFO-key, package refusal, protocol-context `lookup`/`allInstances`, and replay-message tests; `rustfmt --check` on changed Rust files; `git diff --check`. No aggregate gate was run. Clippy and integration-test tooling gaps stated in the PR body were not rerun as part of this targeted review.

The bounded-carries/quantity/model-owned reader mismatch, end-to-end capture paths, remaining reference/run-control work, and stepper inventory are documented as blockers or exclusions in the PR body and remain outside this review's completion claim.
