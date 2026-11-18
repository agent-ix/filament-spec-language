---
id: SR-2026111612172400000032
title: "Gap analysis - QSL-2019 FSL command extraction slice"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2891 @b1f676f1a6b75164e2694daa845ea43b180bda94 (base 80bd45a); FR-662-AC-3, FR-031-AC-2/5/7/8/10, TC-109, TC-1744"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-662", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-031", type: references }
---

## Summary

This focused planless review covers the command extraction slice. The previous TC-1744 step-2 gap, command-level success/error envelope gaps, and normal body-span original_regions gap are fixed. The failure branch for an unmappable body locus remains uncovered and silently omits original_regions.

## Verdict

**CONDITIONAL** - one medium FR-031 uncovered-map behavior and regression-test gap remains; plan completion was not assessed.

## Coverage

- Reconciliation: quoin matrix --repo . --json completed with Quoin 0.36.6 and Quire 0.52.0; FR-662-AC-3 and FR-031-AC-5/7/10 are statically tagged with the expected command test binders. Matrix run evidence is absent. No test was run by this reviewer; focused five-test gates are active.
- Focused criteria: TC-1744 steps 1 and 2 have separate command-level tests; step 2 asserts exit 20, extraction-selection, original identity and binding. The new selection test also checks completed outcome details; the unavailable test checks quire stage and producer outcome. The successful extraction test checks the /2 record and strict reader round trip.
- FR-031-AC-8: the command-level compile-refusal test now checks exact mapped original_regions and preserved body span. However, there is no failure-path test for a body-identity locus outside the verified map. That branch currently treats map failure as no regions instead of an invalid_source_map refusal.
- Plan completion: not assessed
- Semantic review: skipped; not opted in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The first test tagged to FR-662-AC-3 checked TC-1744 step 1 only. There was no command-level test for step 2, a single binding naming a clause the Markdown document does not hold and the FR-030 extraction-selection refusal retaining original identity and binding. | tests/it/request_surface_s1.rs:326-347; spec/test-cases/TC-1744-run-command-reaches-markdown-extraction-in-its-default-build.md:17-25; spec/functional/FR-662-check-every-native-fence-of-a-markdown-specification.md:117 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | the_run_command_preserves_a_missing_markdown_clause_selection now exercises TC-1744 step 2 through command::run and asserts the stage, original identity, and binding. |
| FND-002 | fixed | the_run_command_writes_a_strictly_readable_markdown_extraction_record asserts the /2 extraction member and strict-reader equality. |
| FND-003 | fixed | The selection and unavailable extraction tests now assert completed outcome details; the unavailable test asserts stage quire. |
| FND-004 | fixed | `the_run_command_maps_body_refusal_regions_to_the_original_markdown` asserts exact original regions for a body-span compile refusal while preserving the body span in the `/2` extraction record. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | No command-level test verifies FR-031-AC-5/10: the successful extraction path must write and read back the /2 extraction member with native/formal body identity and the unchanged outcome/map. The command path uses a writer that omits the member. Add a successful command-level round-trip test. | tests/it/request_surface_s1.rs; fsl-command/src/command.rs:1973-1986; fsl-command/src/command/output.rs:412-425; spec/functional/FR-031-run-extracted-native-source.md:190-194 |
| FND-003 | medium | No command-level test verifies that FR-031-AC-2/7 error envelopes retain Quire's completed outcome for selection/correspondence failures or the producer outcome for unavailable extraction. The current adapter drops error.extraction() and the stage classifier also mislabels preflight/unavailable cases. Add focused tests for those envelopes. | tests/it/request_surface_s1.rs:373-410; fsl-command/src/command.rs:1996-2036; spec/functional/FR-031-run-extracted-native-source.md:84-99,190-194 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The command-level success test does not cover a run result whose compile refusal has a body span. FR-031-AC-8 requires its extraction record to carry every original region produced by the verified source map, but the current record serializer never adds original_regions. Add a focused command test asserting exact body span preservation and mapped original regions. | tests/it/request_surface_s1.rs:459-499; fsl-command/src/command.rs:2075-2169; fsl-replay/src/spine/native_result.rs:3245-3260; spec/functional/FR-031-run-extracted-native-source.md:143-154,194 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | No targeted command-level regression covers a body-identity refusal locus that the source map does not cover. The adapter returns no `original_regions` and still writes the result, but FR-031 requires an `invalid_source_map` refusal that preserves the body span. Test the explicit failure branch; retain no-region behavior for dependency loci and refusals without body spans. | fsl-command/src/command/output.rs:482-511; tests/it/request_surface_s1.rs:500-577; spec/functional/FR-031-run-extracted-native-source.md:172-176; fsl-foundation/src/diagnostic.rs:201,625-630 |

## Plan Completion

Plan completion: not assessed.
