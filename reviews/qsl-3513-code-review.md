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
