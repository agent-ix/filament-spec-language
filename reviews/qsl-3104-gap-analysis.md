---
id: SR-3639006
title: "QSL-3104 PR 3875 gap analysis (scoped, manual): strict workspace clippy follow-up"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@1f5e76ccd40585ca16528e4c9c344733472aff2a; fsl-command/src/command/monitor.rs, fsl-replay/src/outcome/counterexample.rs, fsl-eval/src/simulation/protocol/memory.rs, fsl-command/tests/it/cli_contract/monitor.rs, fsl-analyze/tests/it/package_probabilistic_refutation.rs, spec/functional/FR-644-stream-observations-through-the-monitor-command.md, spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-644", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-940", type: references }
---

## Summary

Ticket: QSL-3104. PR #3875 head 1f5e76ccd40585ca16528e4c9c344733472aff2a. This is a scoped,
manual acceptance-to-test check. By brief, `quoin matrix` and the whole-repo reverse-gap audit
were not run (they hang on this repo), so this is not a full gap audit.

The PR is a lint refactor with no spec change. The check is limited to the public surfaces it
reshapes and the criteria whose tagged tests exercise them:

- `monitor_with_target_preparer` is reached by three TC-721 tests tagged FR-644-AC-16 and
  FR-644-AC-17. Their call sites change only in how arguments are passed, and their
  assertions are unchanged.
- `probabilistic_wire_with_enum_members` is reached by
  `source_enum_scheduler_evidence_encodes_with_the_package_member_index`, tagged FR-940-AC-6,
  FR-741-AC-3 and FR-203-AC-8. Its assertions are unchanged.

## Verdict

CONDITIONAL. The criteria that were examined keep their tagged tests and assertions. One reverse
gap: the new `Unspelled("enum member index")` refusal has no owning requirement and no test
(FND-001, the same defect as the code-review SR-3639005 FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Reverse gap: the refusal branch added to `probabilistic_wire_with_enum_members` (schema without enum members gives `CounterexampleWireError::Unspelled("enum member index")`) has no owning FR/AC and no test. FR-413's ProbabilisticCounterexample spelling and FR-940-AC-6 do not describe it, and the only tagged caller always supplies the index. | fsl-replay/src/outcome/counterexample.rs:2084-2101 |

## Coverage

- Criteria examined:
  - FR-644-AC-16: tagged by `accepted_target_and_activation_bound_reach_driver_preparer` and `timed_target_reaches_driver_preparer_before_timed_trace_refusal`.
  - FR-644-AC-17: tagged by `malformed_target_objects_refuse_before_driver_handoff` and the two tests above.
  - FR-940-AC-6: tagged by `source_enum_scheduler_evidence_encodes_with_the_package_member_index`.
- Stubs: none in the diff (`unreachable!` in `letters.rs::unguarded!` is the pre-existing test-fixture guard, moved).
- Tests were not executed (no cargo by brief).
- Unavailable: `quoin matrix` and the whole-repo reverse-gap audit. Not run, by brief.
- Plan completion: not assessed
