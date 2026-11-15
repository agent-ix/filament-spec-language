---
id: SR-2026111414542500000006
title: "Gap analysis of QSL-1961 S2 handoff compile-back and protocol subject (PR #2717)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-1961; PR #2717 head 3fda08450d402709fd8ec9ae462e3f74b8845861; FR-517 (Compiling a handoff back in process, AC-24); FR-571 (The protocol subject, AC-7); TC-1312 row 24; TC-1411 step 7; tests/it/checked_handoff.rs; tests/it/checked_handoff_realizability.rs"
review_set: subset
---
# Gap analysis: QSL-1961 S2

## Summary

Ticket: QSL-1961. Planless, scoped to the PR diff. Plan completion: not
assessed. The new criteria were checked against their tagged tests by reading
the `#[trace]` tags in the diff; the full `quoin matrix` was not run.

- FR-517-AC-24: backed by
  `checked_handoff::a_written_directory_compiles_back_and_a_changed_one_refuses`
  (all three R kinds, the switch unit with execution points and the sidecar
  members, `ReadBackDiffers` for dropped points and a replaced package, `Label`
  for `sources/../x`, `Read` for a removed sidecar) and by the realizability
  test.
- FR-571-AC-7: backed by
  `checked_handoff_realizability::a_package_that_arrives_only_as_a_handoff_reaches_realizability`
  (`proved`, exhaustive, uncertified through `Engines::fsl()`).
- Code without an owning requirement: none; `protocol_subject` is owned by
  FR-571 "The protocol subject", `compile` by FR-517.
- No stubs or coverage inflation found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `an_undeclared_protocol_builds_no_subject` is tagged FR-571-AC-7, but AC-7 states only the handoff happy path; the `UnknownProtocol` and `Memory` refusals of "The protocol subject" have no AC, and `Memory` has no test | tests/it/checked_handoff_realizability.rs:360-397; spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md:54-67 |

## Verdict

Both new ACs are backed. One low trace gap: the refusal behaviour the FR-571
section states is tested under an AC that does not state it.
