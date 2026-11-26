---
id: SR-20261126055136002
title: "gap analysis of PR 4039 bind abstraction criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #4039 head 5bb1c4475ad785773df85a9fe765e6075fd955b4; NFR-015-AC-9, NFR-015-AC-10, TC-1111 steps 7-8, fsl-package/src/emit/tests/abstraction.rs, fsl-package/src/checked.rs"
review_set: subset
---

## Summary

Ticket: QSL-3282. Planless gap check of the criteria the PR claims: NFR-015-AC-9 and AC-10. The `#[implements]` tags and the `#[trace]` bindings of the two new tests were checked against the code paths the tests drive. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | NFR-015-AC-9's "emitting again includes the bound abstraction" is shown only through `emit_package` with a test-supplied fallback region. The fallback places every occurrence that has no region at bytes 0..1 of the source: the relation's occurrence and the population node's generated one. `emit_checked`, the emitter the AC names for the first emission, refuses the unsited binding with `UnlocatedOccurrence` (`3282-red.log`). The trace proves the reset half; the inclusion half holds only for the caller-placed route, and no test pins the `emit_checked` refusal | fsl-package/src/emit/tests/abstraction.rs:388-416 (a_binding_after_an_emission_emits_the_relation); fsl-package/src/emit.rs:5232-5243; fsl-semantics/src/check/abstraction.rs:1286-1290 |

## Verdict

Request changes on head 5bb1c4475ad785773df85a9fe765e6075fd955b4: settle FND-001 together with the NFR-015-AC-9 wording (spec-review FND-001). Either reword AC-9 and TC-1111 step 7 to name `emit_package` with caller-placed regions and add a hand-valued assertion that `emit_checked` refuses `UnlocatedOccurrence`, or keep the AC and make `emit_checked` place an unsited relation.

The `#[implements("NFR-015-AC-9")]` tag on `bind_abstraction` is honest: what the method implements is the reset, which is tested and holds. `#[implements("NFR-015-AC-10")]` on `bind_abstraction_at` and the AC-10 test are complete. The test binds at a site that the unit's regions resolve, emits through `emit_checked`, asserts the relation occurrence, checks the `MissingDeclaration` refusal, and checks the kept emission.
