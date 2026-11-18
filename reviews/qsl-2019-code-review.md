---
id: SR-2026111612172400000031
title: "Code review - QSL-2019 FSL command extraction route"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2891 @adb7ba8aa452dc7134554638ce582a490a39b177; fsl-command/Cargo.toml, fsl-command/src/command.rs, fsl-command/src/command/output.rs, fsl-replay/src/spine/native_result.rs, tests/it/request_surface_s1.rs; FR-662-AC-3, FR-031-AC-2/5/6/7/8/10, FR-030"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-662", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-031", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-030", type: references }
---

## Summary

Re-reviewed PR #2891 at adb7ba8 after the second fix round. The call-mode refusal, formatting, TC-1744 step-2 test, stage mapping, success result envelope, and completed Quire outcome details are fixed. One output gap remains for original source regions on body-span refusals.

## Verdict

**CONDITIONAL** - one medium finding remains. The branch is based on current origin/main. Focused gates are queued; no tests or builds were run by this reviewer.

## Review Scope

- Changed FSL files: fsl-command/Cargo.toml; fsl-command/src/command.rs; fsl-command/src/command/clause.rs; fsl-command/src/command/output.rs; fsl-command/src/command/output/types.rs; fsl-replay/src/spine.rs; fsl-replay/src/spine/native_result.rs; tests/it/request_surface_s1.rs.
- Examined FR-030, FR-031-AC-2/5/6/7/8/10, FR-662-AC-3, TC-109, TC-1744, and the downstream clause-result writer.
- The computed Quoin matrix lists FR-662-AC-3 and FR-031-AC-5/7/10 as tagged with binders. It has no run evidence in the matrix snapshot. No test or build was run by this reviewer; focused gates are queued. No aggregate or package-wide gate was run by this reviewer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | When an extraction request carries call instead of clause, the new command route previously returned the generic missing-clause refusal before classifying the extraction mode. FR-031-AC-6 requires extraction-requires-run before reading dependent files. | fsl-command/src/command.rs:1582-1583,1858-1864; spec/functional/FR-031-run-extracted-native-source.md:132-134 |
| FND-002 | low | The changed extraction closure was not formatted by Rust 1.98 rustfmt 1.9. | fsl-command/src/command.rs:1584-1588 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The extraction call-mode guard now precedes the missing-clause lookup at fsl-command/src/command.rs:1925-1928 and returns extraction-requires-run. |
| FND-002 | fixed | The changed extraction route is formatted; the check output only reported unchanged regions. |
| FND-003 | fixed | Preflight now maps to Selection; a Join whose completed outcome is unavailable maps to Quire; other Join errors map to Selection at fsl-command/src/command.rs:2002-2011. Focused tests cover selection and unavailable outcomes. |
| FND-004 | fixed | The command creates an extraction record and passes it to the extraction-aware result writer; the success test checks the /2 member and strict read-back equality. |
| FND-005 | fixed | The command error adapter now inserts the completed Quire outcome when present; focused selection and unavailable tests inspect that details member. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The stage mapping reverses the FR-031 contract: every Cause::Preflight becomes quire, though preflight must be extraction-selection; every Cause::Join becomes extraction-selection, including Quire-unavailable outcomes that must be quire. Use the actual typed cause and completed extraction availability to distinguish these stages, then test both sides. | fsl-command/src/command.rs:1998-2000; fsl-source/src/lib.rs:86-91; spec/functional/FR-031-run-extracted-native-source.md:91-99 |
| FND-004 | medium | The new successful command route passes the report to the generic clause_report writer, which serializes no extraction member. Thus extracted /2 results omit the native/formal body identities and the Quire outcome/map required by FR-031-AC-5/10. Preserve the ExtractedSource record through serialization and assert the complete member in a command-level round-trip test. | fsl-command/src/command.rs:1973-1986; fsl-command/src/command/output.rs:412-425; fsl-replay/src/spine/native_result.rs:3175-3200; spec/functional/FR-031-run-extracted-native-source.md:190-194 |
| FND-005 | medium | fsl_source::Error retains extraction: Option<ClausesOutcome>, but the new command-error details include only original/path/body/binding and discard error.extraction(). A missing-clause/correspondence refusal happens after Quire has completed extraction, so the command loses that actual producer outcome, contrary to FR-031-AC-2/7. Serialize the unchanged outcome when present and test the selection-refusal envelope. | fsl-command/src/command.rs:2002-2036; fsl-source/src/lib.rs:98-121; spec/functional/FR-031-run-extracted-native-source.md:84-99,190-194 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The extraction record is assembled before run_clause and contains original/body/binding/outcome/map only. The result writer serializes that value unchanged, so an extracted compile refusal with a body span has no original_regions despite FR-031-AC-8 requiring the mapped original regions in the record. Add the body-span mapping after the report exists and a command-level compile-refusal regression asserting exact regions. | fsl-command/src/command.rs:2075-2169; fsl-replay/src/spine/native_result.rs:3245-3260,3305-3332; spec/functional/FR-031-run-extracted-native-source.md:143-154,194 |

## Gate Notes

- Focused test run: not run by reviewer; focused gates are queued.
- Exact-head approval: withheld pending the body-span original_regions fix and focused checks.
- No aggregate or package-wide test gate was run.
