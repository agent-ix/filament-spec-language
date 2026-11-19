---
id: SR-2026111911313600000003
title: "Code review — FSL hybrid undefined read-back and finite end replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/hybrid_requests.rs, fsl-replay/src/hybrid.rs, fsl-replay/src/hybrid/locus.rs, fsl-replay/src/hybrid/provider.rs, fsl-replay/src/hybrid_replay.rs, fsl-replay/src/hybrid_replay/exact.rs, fsl-replay/src/hybrid_replay/static_replay.rs, fsl-replay/src/hybrid_replay/tests.rs, fsl-replay/src/hybrid_replay/trace.rs, fsl-replay/src/lib.rs, fsl-replay/src/proof_result.rs, fsl-replay/src/proof_result/plugin_read.rs, fsl-replay/src/proof_result/plugin_read/trace.rs, fsl-replay/src/result/wire.rs; FR-750-AC-9, FR-750-AC-12, FR-750-AC-13, FR-750-AC-14, FR-193-AC-23, FR-450-AC-10, TC-3037"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/spec", type: references}
---

## Summary

Reviewed PR #3020's Rust wire reader/writer, undefined-cause and locus handling, finite trace endpoint admission/static/exact replay, and activated analyze integration coverage against the repository rules and the linked requirements. The reader reconstructs parameter-range causes from the wrong index for valid call loci, so AC-14 read-back is incorrect for root calls and other call positions.

## Verdict

**CONDITIONAL** — one medium implementation defect remains in undefined parameter cause read-back.

## Assurance Context

No applicable AssuranceProfile is present under `spec/`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Parameter-range read-back treats a term path segment as the function parameter index | fsl-replay/src/proof_result/plugin_read/trace.rs:208 |

## Review notes

`read_undefined_kind` resolves an emitted locus to the checked `HybridTerm::Call`, then uses `locus.term.term.last()` as the `parameter` field. That path is the call's position in its enclosing expression, not an argument index. A call at the expression root has an empty path and is rejected at `/kind/where/locus`; a call nested as another child can decode a different parameter. A valid `outside-parameter-range` provider result therefore cannot reliably round-trip or agree with exact replay. The AC-14 test uses a single parameter and a call at path `[0]`, which happens to make the structural segment equal parameter zero and misses both cases.

The `HybridTrace.end` member is serialized, read back, admitted against `loop`, and used by exact dwell replay. Static check 6 and exact bad-prefix behavior are covered by the direct replay test; the activated analyze tests exercise end read-back/reproduction and irrational undefined read-back. The W68/W72 tracker issues and the pinned FSpec requirements were inspected. `quoin 0.36.7` has no `gap-analysis` command; the repository matrix completed successfully. No tests or aggregate gates were run.

## Rationale

| Finding | Requirement and consequence |
| --- | --- |
| FND-001 | FR-750-AC-14 requires parameter payloads to round-trip from a provider's undefined-evaluation wire result. The checked term locus names the call node, and its structural path does not encode the parameter index; root calls cannot be decoded and nested calls may be assigned a sibling index. Reconstruct the parameter index from an unambiguous wire field or the actual range-fault locus, and cover a root call plus a nonzero parameter. |
