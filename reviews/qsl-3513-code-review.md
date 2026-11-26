---
id: SR-967185
title: "QSL-3513 code review of the cross-binding order, conformance objects and policy instances (filament-spec-language PR 4040)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@adf503520a905d72041960c5832c201e87296279; spec/decisions/ADR-018, spec/functional/FR-126, FR-621, FR-896, FR-990, FR-1012, spec/test-cases/TC-1085, TC-2532, TC-2800"
review_set: subset
date: 2026-11-26
---

## Summary

Ticket: QSL-3513. PR filament-spec-language#4040, head adf503520a905d72041960c5832c201e87296279.

Spec-only diff; the Rust and React lanes do not apply. The expected values of
FR-126-AC-27 to AC-29, FR-896-AC-7 and AC-8 and FR-990-AC-11 were worked by hand
and are right under the new text. AC-28 relies on the joint safety product (three
product states use `max_states` 3). The conformance rule was compared with
`protocol_analysis/conformance.rs`, `analyze/request.rs`, `analyze/document.rs`,
`finite_replay.rs` and `fsl-eval` `simulation/protocol.rs`. It is implementable:
the code checks the first admitted instance only, faults on an empty set and drops
`subject.protocol`.

Not run: gap-analysis (the diff has no production code).

## Verdict

One low finding. The new criteria are sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | With `subject.protocol`, FR-896 decides the item over the given `over` object, and FR-205 accepts any object of the type's population universe. The subject builder in `fsl-eval/src/simulation/protocol.rs` (`over_value`) refuses any object that is not an admitted key (FR-125) with `invalid_value`. Failure: a request binds `over` to a universe object that no initial state holds; by the spec it is checked, by the code it is refused, and no criterion decides which. State the admitted-key requirement or the intended check. | spec/functional/FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md:92-94 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | AC-60 does not state `inc`'s frame. The `Counter` of FR-315-AC-1 is built with the type-wide frame `modifies [value]` (`fsl-eval/tests/it/smt_query.rs` `operation`; `fsl-analyze/tests/it/model_check.rs` says that frame lets an operation on one receiver change every other counter), which is why AC-38 and AC-58 state `modifies self.value`. Failure: with two objects, one `inc` on `d` may set `c` to 2 or 3, so `c` is violated in one step, the `First(1)` query at depth 1 is satisfiable, and the item reports `c`, not `d`. State `modifies self.value` (FR-154) in AC-60. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1492 |
| FND-003 | low | With an empty `initial` list no key is admitted, so every binding is now refused `invalid-value`, while FR-125 orders `NoInitialState` first and FR-896 settles a subject with no instance that way. Failure: a `global-conformance` request with `subject.protocol.over` set over a subject with no initial state settles `declined invalid-value` by FR-205 and `inconclusive no-initial-state` by FR-896's last bullet. Say which, as FR-315-AC-59 does for hyper parameters. | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:95-99 |
| FND-004 | low | Step 19 is placed after the `Tag the tests` line with no blank line, so it renders as part of that paragraph and not as a procedure step. Move it above the line, after step 18. | spec/test-cases/TC-650-protocol-subject-and-state-key.md:102-104 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Checked at head 8bc5cfda3663de78b37a32d5be17391ce9345c54. |
| FND-002 | fixed | Checked at head 1de8eecdc4f9de17bd8e690943721fc493c3f671. |
| FND-003 | fixed | Checked at head 1de8eecdc4f9de17bd8e690943721fc493c3f671. |
| FND-004 | fixed | Checked at head 1de8eecdc4f9de17bd8e690943721fc493c3f671. |
