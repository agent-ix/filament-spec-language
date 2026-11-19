---
id: SR-202611030003
title: "Code review of filament-spec-language PR #2778: protocol finish and recovery"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/protocol_clause/tests/controls.rs; fsl-eval/tests/it/protocol_retry_bound.rs; PR #2778 head b89a5fc7bd3452a7752a1538dd66ab1003ac4f22"
review_set: subset
---
# Code review of PR #2778

## Summary

Ticket: QSL-2206. Rechecked the frozen head `b89a5fc7bd3452a7752a1538dd66ab1003ac4f22`; no fix commit changed the reviewed tests. Code-review with Rust lane. FND-001 remains: the negative unbound-name test is still tagged to the positive recovery-record criterion. This is trace metadata and does not change runtime behavior.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FSpec-FR-426-AC-1 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |
| FSpec-FR-426-AC-9 | fsl-eval/tests/it/protocol_retry_bound.rs | context_only |
| FSpec-FR-427-AC-6 | fsl-semantics/src/check/protocol_clause/tests/controls.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The unbound-name S3 test is tagged to the positive per-record recovery-evaluation criterion. | fsl-semantics/src/check/protocol_clause/tests/controls.rs:45-50 |

## Dispositions

Disposition round 1 at `b89a5fc7bd3452a7752a1538dd66ab1003ac4f22`:

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The test remains tagged `FSpec-FR-427-AC-6`, although it exercises an unbound-name refusal rather than per-record recovery-binder evaluation; correcting this trace finding requires a code/test metadata change. |

## Verdict

Not mergeable under the review findings: FND-001 remains open. It is a traceability issue rather than a runtime defect.

Disposition round 2 at `6d89a77dfa1a1fd9da8c5487e40340c7bfff8d11`:

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The erroneous trace tag was removed from the negative unbound-name test; the positive recovery evaluation test retains the criterion tags. |

After excerpt:

```rust
#[test]
fn a_recovery_predicate_over_an_unbound_name_refuses() {
```
