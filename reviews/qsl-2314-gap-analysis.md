---
id: SR-2026111612172400000006
title: "Gap analysis — QSL-2314 scoped acceptance evidence"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-088-AC-8/13/14, FR-230-AC-65, FR-569-AC-12/13/15; fsl-semantics/src/check/assemble/clock_forms/source_places.rs, fsl-semantics/src/model/index.rs, fsl-semantics/tests/it/source_objects.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-088
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: references
---

## Summary

Compared the QSL-2314 acceptance and latest rulings with the merged FR-088, FR-230, and FR-569 criteria and their computed test bindings. Targeted `quire matrix` output binds the in-scope criteria to tests, but the current PlaceRow assertion does not prove the exact FR-322 identity text required by FR-088-AC-13.

## Verdict

**CONDITIONAL** — one medium evidence gap remains: the AC-13 identity assertion can pass without proving the expected source type identity. QC-30-dependent intake and unchanged-byte checks are outside this PR by ruling.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The PlaceRow test checks equality with the next row's declaring type and excludes `resolved:` and `Composite(`, but never asserts the expected FR-322 identity string or equality to the corresponding record-field row. An empty or otherwise incorrect matching string can satisfy these assertions, leaving the AC-13 identity requirement unverified. | fsl-semantics/src/check/assemble/clock_forms/source_places.rs:597 | correct-requirement-no-evidence |

## Coverage

- `quire matrix --scope . --format tsv` reports FR-088-AC-13/14, FR-230-AC-65, and FR-569-AC-12/13/15 as tagged. FR-088-AC-8 is tagged as `method-without-symbol`, consistent with its inspection verification.
- Source-object filter 57/57 and TC-260 2/2 passed. Targeted library checks passed 10/10 and 5/5 after the frozen head; workspace check, default-feature workspace Clippy, formatting, and qualified-core architecture lint also passed per owner report.
- Manual acceptance-to-test check used because the installed Quoin CLI has no `gap-analysis` command (`quoin gap-analysis --help` reports `command gap-analysis not found`). The available computed matrix was inspected directly. Full repository reverse-gap and stub audit were not run. The skill's planless default applies: `Plan completion: not assessed`.
- FR-088-AC-13 tests cover source names/aliases, resolved identity formatting, conformance refusal spelling, and the in-process PlaceRow path. The PlaceRow assertion is the specific gap recorded above.
- FR-088-AC-14 has direct bounded/unbounded/nested Population spelling assertions.
- FR-230-AC-65 tests cover source object time resolution, missing and duplicate names, tick refusal and location, missing dense time, and selected-model time isolation. Source layout tests cover direct, nested and collection clock places.
- FR-569-AC-12/13/15 have tagged integration coverage for accepted member forms, matching refusals, member representation, conformance cases, and clock paths.
- QC-30-dependent intake joining, emitted root binding, and unchanged-byte checks remain outside the PR per the latest ruling.
- Plan completion: not assessed.
