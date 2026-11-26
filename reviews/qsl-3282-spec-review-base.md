---
id: SR-20261126055136003
title: "spec review of PR 4039 NFR-015 rebinding criteria"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR #4039 head 5bb1c4475ad785773df85a9fe765e6075fd955b4; spec/test-cases/TC-1111-fsl-package-gates-the-checked-package-wire-behind-std.md, spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md (AC-9, AC-10), spec/functional/FR-306-carry-the-abstraction-relation-in-the-checked-package.md"
review_set: subset
---

## Summary

Ticket: QSL-3282. The PR's spec edit renames `emitted_package_id` to `emitted_package_name` in TC-1111 steps 7-8 and their expected results; the rename matches the code. Reviewed: NFR-015-AC-9 and AC-10 against the code they govern, and FR-306 "Rebinding is a new package value" against the rebind behaviour the review found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | NFR-015-AC-9 says "emitting again includes the bound abstraction" but names no emitter. An unsited `bind_abstraction` relation has only a generated location, and `emit_checked` refuses it with `UnlocatedOccurrence`. Only `emit_package` with caller-placed regions includes it. Two implementers would read the AC differently | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:219; spec/test-cases/TC-1111-fsl-package-gates-the-checked-package-wire-behind-std.md:46-47 |
| FND-002 | medium | No FSL criterion says that a rebind gives the check-time results a fresh check of the rebound relation gives: guarantee item refusals, the guarantee relation reference, and step-relation implementations. FR-306 "Rebinding is a new package value" covers only the relation body and node id. Arch ruled that one new criterion is added in this PR | spec/functional/FR-306-carry-the-abstraction-relation-in-the-checked-package.md:55-64 |

## Verdict

Request changes on head 5bb1c4475ad785773df85a9fe765e6075fd955b4. FND-001: name the emitter and the placement in AC-9 and TC-1111 step 7. FND-002: add the arch-ordered criterion and give it tests on both routes (bind, and a fresh check), with hand-written values.

FR-306 also says a rebinding's relation node has an equal `node_id`. The key is derived from the relation's targets, so that holds only while the bound object types and populations stay the same. The new criterion should state which rebinding it covers.

The TC-1111 rename is correct, and step 8 now matches NFR-015-AC-10.
