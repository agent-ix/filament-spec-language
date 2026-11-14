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
| FND-001 | fixed | 396551695: FR-563 Overlap and ADR-029 EB-6 state the enablement: `xtask parity` constructs `quire-aot` with a fixture generator from `qsl-conformance` (test-only) and the run's toolchain, and never calls the driver's or CG's `generate`, so no QSL-to-CG edge exists (consistent with FR-280 and ADR-011 FB-11). The consequence is recorded as FND-003. |
| FND-002 | fixed | edd5dee65: FR-295 frontmatter adds `FR-563` with type `references`. |
| FND-003 | fixed | 9ef39ed49: FR-563 Behavior and FR-563-AC-10 specify the real-generator AOT parity run (driver composes `quire-aot` with CG `lower` then `generate`, `run_parity` over the corpus and default generated units; a rounding generator must fail with a `difference`); FR-567 Overlap names both runs; driver side tracked in IR-1540. The placement of that obligation is recorded as FND-005. |
| FND-004 | fixed | 9ef39ed49: EB-6 no longer cites driver FR-008; it reads "QSL requires the driver to supply the real generator ... and the toolchain for driver runs"; the FR-008 amendment is tracked in IR-1540. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | With Option A, FR-563's `xtask parity` gates `quire-aot` only behind the `qsl-conformance` fixture generator, and the driver-supplied generator (CG's `lower` then `generate` with the AOT target), which produces the code users actually run, is compared with the interpreter nowhere: no requirement here or in driver FR-008 runs the FR-295-AC-1 corpus and 10,000 programs over `quire-aot` built with the real generator. FR-295 and EB-5 require each non-interpreter backend's outcomes to equal the interpreter's, and FR-567 now says QSL qualifies AOT parity; as specified, the gate tests the build-and-run harness, not AOT code generation. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:175-178, spec/functional/FR-567-qualify-qsl-on-the-final-path-the-driver-calls.md:119 |
| FND-004 | low | ADR-029 EB-6 says "The driver supplies the generator and the toolchain for driver runs (driver FR-008)", but driver FR-008 on quire-driver main says only that the driver calls QSL's AOT backend with the checked package, entries, limits, cancellation and the caller-named toolchain; it names no generator. The citation needs a matching quire-driver FR-008 amendment (a driver-repo ticket). | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:890-891 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-563-AC-10 (and the new FR-563 Behavior bullet "The driver repository shall run the AOT backend ...") put an obligation on another repository inside a QSL requirement. AC-10 can fail, but only in quire-driver: no QSL test can carry its trace tag, so `quire matrix` lists it as method "Test", status untagged, permanently (it parses and `quire validate` passes; "Test (driver repository)" is the only verification cell of that form in spec/functional, against 3,780 `Test (TC-n)` and 529 bare `Test`). The repo's pattern for cross-repo duties is the Overlap section plus the other repo's own requirement and TC (driver FR-008 / TC-015, now IR-1540). Move the AC and the Behavior bullet's obligation to Overlap (naming IR-1540), keeping FR-567's and FR-563's Overlap sentences; or, if the AC stays, it needs a QSL TC that can be bound here. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:126, spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:162 |
