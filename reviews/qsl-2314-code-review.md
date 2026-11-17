---
id: SR-2026111612172400000005
title: "Code review — QSL-2314 source time declaration key assertion"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2843; fsl-semantics/tests/it/source_objects.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-995
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-996
    type: references
---

## Summary

Reviewed PR #2843 at frozen head `49c8dd055c6cf3144f969d7715626fbe3106b757`. The test-only change now derives the checked source object's declaration key from its SourceOwner and asserts that the checked time source retains that key.

## Verdict

**PASS** — the assertion fails if the checked time source is keyed to the selected model's `model/object_type` node instead of the source-owner declaration key required by amended FR-230-AC-65.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review notes

The change leaves production code and emitted output untouched. `a_source_object_type_name_times_the_source_model` obtains the source object's domain declaration, derives the SourceOwner identity and declared type name, constructs `{package: I, node: "ix://I/O"}`, then compares its NodeKey to `graph.time_sources()[0].model_node`. A selected-model node, wrong owner, or wrong type name makes the equality assertion fail. The test retains `FR-230-AC-65` and `TC-2361` trace tags.

The exact affected test passed (1/1). The changed file passed rustfmt, and `git diff --check origin/main...HEAD` passed. Manual requirement-to-assertion alignment was done against FR-230-AC-65; Quoin gap-analysis was not applicable because the only changed file is a test and no production behavior changed. No aggregate gate was run during this review.

## Examined acceptance scope

- FR-230-AC-65 (examined): the source time source retains the source declaration key `{package: I, node: "ix://I/O"}`, where `I` is the `SourceOwner` identity, and remains a timed model with clock places `w.c` and `d`.
- TC-2361 (examined): the source-route test for dense time identity, clock places, missing names, duplicate time sources, tick refusal and missing dense time.
- FR-995-AC-1 (context only): source declaration keys use the `SourceOwner` identity and `ix://I/<Type>` spelling.
- FR-996-AC-1 (context only): emitted clock-place rows join through source declaration keys.
