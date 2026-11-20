---
id: SR-1475
title: "Code and Rust review of protocol source contracts"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2080; PR3154 at 0176c72bdaee28097b79b3558aee54a00e6900fd; fsl-semantics/src/check/protocol_clause/tests/source_contracts.rs and tests.rs registration"
review_set: subset
---

## Summary

Reviewed the new real-source S3 protocol cases and their single module registration. The tests use the existing parser/forms/assembly/checker path and meaningful accepted/refused inputs; two confined assertions or trace bindings need correction.

## Verdict

**PASS** at `847b6cfc28b793261798626e7139e1c426d8e901` — both findings are fixed in the confined correction diff. Original findings remain below as the review record.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The replicated-role admission test tags FR-208-AC-1 but never executes spawn, retirement, enabled-step or retained-retired-state behavior. Its asserted behavior is FR-218-AC-28; remove the unrelated FR-208-AC-1 tag rather than expanding this S3 ticket into runtime testing. | fsl-semantics/src/check/protocol_clause/tests/source_contracts.rs:248 | correct-requirement-no-evidence |
| FND-002 | medium | Sorting duplicate-role refusal positions before comparison hides a reversal of the required source-order diagnostics. Compare emitted positions directly with the source-ordered expected positions. | fsl-semantics/src/check/protocol_clause/tests/source_contracts.rs:239 | correct-requirement-no-evidence |

## Examined scope

- `FR-218-AC-2`, `FR-218-AC-5`, `FR-218-AC-10`, `FR-218-AC-12`, `FR-218-AC-13`, `FR-218-AC-17`, `FR-218-AC-19`, `FR-218-AC-23`, `FR-218-AC-28`, `FR-218-AC-33`, `FR-218-AC-35`: examined for the specific new assertion bindings, not a claim that each complete criterion is exhausted by this file.
- `FR-884-AC-2`: examined — duplicate role names must each be refused in source order.
- `FR-884-AC-5`: examined — real accepted counterparts and rejected subspans within the protocol.
- `FR-208-AC-1`: examined — the claimed runtime spawn/retire binding is unsupported by this test.
- `FR-113-AC-3`: examined for missing scoped-name resolution in the compensation counterpart.
- Existing `rules::protocol`, `rules::located_refusals`, `checks`, `check_refusals` and `recovery_flow` helpers: context only, to trace the new cases through real S3 behavior.
- Rust testing and traceability rules: examined for deterministic actual-path assertions, named tests, SPDX, trace correctness and absence of copied external fixtures.

## Evidence and limits

The author's receipt reports nine passing new test functions, nine accepted programs and eleven refusal counterparts. Those in-lane results were accepted without repetition. Existing QSL-2215/QSL-1997 delivery was accepted; no earlier cases were reprobed.

Scoped lint is blocked by the accepted peer receipt of 595 unchanged `fsl-semantics` findings. No tests, builds, lint, product probes, matrix or baseline measurements ran in this review. The original coder owns the single final applicable component gate after review fixes. No repository-wide assurance/profile inventory or source audit was performed; this verdict covers the declared diff only.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The replicated-role test now carries only FR-218-AC-28 and FR-884-AC-5; the unexercised runtime FR-208-AC-1 tag is removed. |
| FND-002 | fixed | The test compares emitted duplicate-refusal positions directly with the source-ordered expected vector; the mutability and sorting call are removed. |

Round 1 inspected only the correction diff from `0176c72bdaee28097b79b3558aee54a00e6900fd` to `847b6cfc28b793261798626e7139e1c426d8e901`. The author reports the changed duplicate-role case and new missing-await-anchor case passed; those results were accepted without repetition. No new findings, test/build/lint/probe, broader review or old owner checks occurred. The one final protocol-clause component gate remains pending with the original coder.
