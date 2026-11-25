---
id: SR-967175026302197768900474181941935084865
title: "Gap analysis of FSL PR 3807 at head cf9b9048e2"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3807 head cf9b9048e2e27bf9cbcd0733fede5d3c23380582: changed production modules fsl-package/src/emit/value_validity_requirements/frames.rs, fsl-package/src/emit.rs, fsl-bench/src/deep_input.rs and their tests"
review_set: subset
---
## Summary

Ticket: QSL-3120. Planless; plan completion was not assessed. This is a manual acceptance-criteria-to-tests check of the behaviour changed since d87eb75fbd, scoped to the diff against merge base e928938db1. The reviewer has no checkout of the commit and runs no cargo, so `quire matrix` was not run at this head. The matrix step is therefore incomplete and is replaced by reading the trace attributes in the diff. Reverse-gap and stub inspection covered the changed modules only.

- FR-094-AC-13 (QSL-3120): the bindings recorded at d87eb75fbd (flow_millisecond_dimension_equivalence_emits_and_reads_canonically, ..._reader_rejects_invalid_scales) are unchanged, and both pass at this head (F-pkg-modules 37/0).
- FSpec FR-825 literal-guard frames (QSL-3283): owned by FSpec FR-825 and FSL FR-062 claim sites. Three new tests with literal expected chains are admitted by the pinned strict reader; one carries `FR-825-AC-12` (see FND-001).
- FR-356-AC-7 deep input: untagged, which is correct (the test stops at 10,000 levels); the lead decision 58a9ed9f on QSL-3283 records it.
- FR-337-AC-30 / TC-1326: timestamped_role_readback_identity_and_selection_refusals_are_exact carries `#[trace("TC-1326", "FR-337-AC-30")]` and now reads back the package with the clause over the imported event role, as the AC requires; 1/1 at this head.
- QSL-3159: state_graph_identity_readback_uses_the_explicit_temporal_selection is back on temporal_config_world and passes (H-analyze 2/0). The deleted helper has no other caller (the workspace check exit 0 at this head).
- No stubs and no unowned behaviour were found in the changed modules.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `#[trace("FR-825-AC-12")]` binds a test that proves only the ordinal-advance clause of FR-825-AC-12; the AC's other clauses (one row with no holds-false row, unused extra row refused, parent refusal, absent member when every chain is empty) are not asserted by it. | fsl-package/src/emit/value_validity_requirements/tests.rs:400 |

## Verdict

No gap blocks merge. FND-001 is low: narrow the tag or extend the test. The code-review artifact SR-967175026302197768900474181941935084864 records the merge blocker (the codegen pin).
