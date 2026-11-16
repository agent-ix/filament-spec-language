---
id: SR-2026111610160900000002
title: "Gap analysis of QSL-1892 native enum values in the protocol transition identity (PR #2746)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-1892; PR #2746 head d98ad9452f8926a36c4324e629742aa5f0098e3c; FR-215-AC-5; TC-660; fsl-analyze/tests/it/protocol_system.rs; fsl-eval/src/simulation/model/encode/projection.rs"
review_set: subset
---
# Gap analysis: QSL-1892

## Summary

Ticket: QSL-1892. Planless, scoped to the PR diff. Plan completion: not
assessed. `quire trace --id FR-215-AC-5` (quire 0.38.3) resolves one claim:
`a_finite_field_record_binder_steps_in_domain_order`
(`fsl-analyze/tests/it/protocol_system.rs:1648`, rust-trace-attribute).

- FR-215-AC-5: backed. The test asserts the exact eight-step sequence, with
  `n`, `m` and `i.up` read from the transition identity, and asserts the `Wide`
  `RequiresBound` refusal.
- Code without an owning requirement: none. Enum encoding in the transition
  identity is owned by FR-215 and FR-120's typed canonical form.
- No stubs or coverage inflation found. The old "Partial" note was removed
  along with the gap it described.
- Outside this diff (pre-existing, not a PR finding): the public wire helpers
  `typed_value_json` (`fsl-eval/src/simulation/model/encode.rs:1041`),
  `assignments_wire` and `probabilistic_wire`
  (`fsl-replay/src/outcome/counterexample.rs:416,451`), and the contract
  counterexample value spelling (`fsl-replay/src/contract_check.rs:722-729`)
  still read variant types only. The value check enumerates any
  `ValueType::Enum` parameter, source enums included
  (`fsl-replay/src/value_check.rs:659-666`). So a counterexample over a
  source-enum parameter encodes `Unencodable` and fails with
  `Unspelled("assignment value")`. This is a real gap. It belongs in a
  follow-up ticket, not in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean for this PR's scope. One follow-up ticket is recommended for the wire
helpers that read variant types only.
