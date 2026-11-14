---
id: SR-202611141401005
title: "Dependency review of the QSL-1942 JIT requirement fixes"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language; PR #2704 at 8ef45a09e17ae92270ebd7a973209a86e40f05d3, diff origin/main...HEAD: dependencies introduced by FR-295's corpus definition and by moving `quire-aot` to QSL (ADR-029 EB-6, FR-563)"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-563, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-295, type: references }
  - { target: ix://agent-ix/quire-spec-language/ADR-029, type: references }
---

## Summary

Ticket: QSL-1942. No frontmatter `relationships` changed in the diff. Two new prose
dependencies appear: FR-295 now defines its conformance corpus by FR-563, and QSL's
`xtask parity` now gates `quire-aot`, whose input is an artifact produced by the driver's
`lower` and `generate` (CG).

## Verdict

FAIL on FND-001 (medium). FND-002 is low.

Examined and clean: FR-295 to FR-350 (through FR-563, which already depends on FR-350);
FR-562-AC-3 no longer depends on FR-495-AC-3's integer `d1` refusal (FR-562 keeps its
`depends_on` FR-495 for `CheckMode` and the division law, which is still needed);
FR-560-AC-2 and FR-562-AC-5 add dependencies only on FR-505/FR-056 bounded type forms that
already exist.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-563: "`make parity` runs the same operation over `quire-aot`, a QSL crate" and "`xtask parity` runs `run_parity` over `quire-aot` ... and gates the AOT backend on it". `quire-aot` runs an artifact that only the driver's `lower` and `generate` with the AOT target (CG) emit (ADR-029 EB-6), and a QSL crate that calls CG closes a package cycle (ADR-011 FB-11, FR-280). The enablement is unstated: which repository produces the AOT artifacts for the 10,000 generated programs in the QSL gate, and how `xtask parity` reaches them without a CG dependency. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:55, spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:175-177 |
| FND-002 | low | FR-295 now defines "conformance corpus" as FR-563's run vectors (description and FR-295-AC-1) but its frontmatter has no edge to FR-563; FR-563 already `depends_on` FR-295, so the new edge should be `references`, not `depends_on`. | spec/functional/FR-295-return-interpreter-outcomes-from-jit-and-aot.md:5-17 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
