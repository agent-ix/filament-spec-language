---
id: SR-2026111612172400000005
title: "Code review — QSL-2314 source member conformance and clock handling"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/assemble/clock_forms.rs, fsl-semantics/src/check/assemble/clock_forms/source_places.rs, fsl-semantics/src/check/assemble/source_spellings.rs, fsl-semantics/src/check/lowering.rs, fsl-semantics/src/check/lowering/model.rs, fsl-semantics/src/check/mod.rs, fsl-semantics/src/check/temporal_clause/timed.rs, fsl-semantics/src/check/timed_model.rs, fsl-semantics/src/model/conformance.rs, fsl-semantics/src/model/domain_package.rs, fsl-semantics/src/model/index.rs, fsl-semantics/src/model/mod.rs, fsl-semantics/src/model/normalize.rs, fsl-semantics/src/model/spelling.rs, fsl-semantics/tests/it/source_objects.rs, xtask/src/typestate_scan.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-088
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: references
---

## Summary

Reviewed PR #2775 at frozen head `d193bd91ea6a60f54812ce95461b3b1435b73ee8` against its scoped acceptance, the merged spec changes, repository conventions, and the Rust implementation. No code correctness, safety, API, or test defects were found in the reviewed diff; QC-30-dependent intake and byte-invariance obligations remain explicitly outside this PR.

## Verdict

**PASS** — the implementation and targeted evidence align with the accepted PR scope. The separate gap-analysis review records one missing exact identity assertion in the PlaceRow test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review notes

The review checked the new resolved conformance relation, source spelling registry and formatter, source clock layout propagation, source time member resolution, and affected tests. Reported evidence was source_objects filter 57/57, TC-260 2/2, focused library checks 10/10 and 5/5, workspace check, default-feature workspace Clippy, formatting, and qualified-core architecture lint. No Cargo command was run during this review. The repository's aggregate gate was not run per the direct owner instruction.

## Examined acceptance scope

- FR-088-AC-8: `ValueTypeRef` is exactly the five-member union; resolved display is the `resolved:` identity form.
- FR-088-AC-13: formatter output, alias preservation, and resolved member `PlaceRow` identity behavior; QC-30 intake and byte invariance excluded by ruling.
- FR-088-AC-14: `Population` fallback spelling and refusal for source object members.
- FR-230-AC-65: source object time name resolution, dense/tick/missing-time cases, refusal spans; emitted root reference excluded by QC-30.
- FR-569-AC-12/13: source object member type parity and unsupported-type refusals, as related behavior.
- FR-569-AC-15: outer collection representation, closed conformance cases, and source clock paths.
