---
id: SR-202611251433001
title: "code-review of PR 3930 no-run-member refusal ordering"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3930 head 1a2e899a9d5982f804bab095c670c1184153185a; fsl-command/src/command.rs; fsl-command/src/command/request_shape.rs; fsl-command/tests/it/request_diagnostics.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1047
    type: references
---

# Code review of PR 3930 (QSL-3274 item 1)

## Summary

Ticket: QSL-3274 item 1. The diff (origin/main...1a2e899a9d5982f804bab095c670c1184153185a) changes three files in `fsl-command`. The three newer main commits (#3894, #3649, #3925) touch no file in this diff. A rust-review lane is folded into this file. The review covers the FR-1047 "A selected source with no run member" ordering at `run_with_providers`, `prepare_run` and `read_clause_run_request`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. No tests were run: the C lock's 30 GiB floor refused the run (16.5 GB available), and locks A and B were both held. This verdict therefore comes from reading the code. CI on this head runs only the CLA checks, so no one except the author has run these tests at this head.

Ordering per entry, checked against the FR-1047 table:

- `request_shape::admit`: when the run group is the only missing entry and the request selects exactly one source, `admit` now returns `Ok`. The typed reader then gives its own refusals first. When `models` is also missing, the full list is reported, as the last bullet of the section states. The `missing == [run_group()]` equality holds because `collect` builds the group with the same `/request/<token>` format.
- `prepare_run` with a package: the no-run-member check stays where the old FR-100-AC-25 check was, after `refinement_internals`, the transfer conflict and the relationships check. `package_run::validate_request` (integer_division, libraries, refinement_internals, clocks) still runs first. The only change is that the refusal now carries `details.missing`.
- `prepare_run` with a program: the refusal comes after decode (sources, requirements, unknown, repeated and wrong-kind members), `refinement_internals`, request and library `integer_division`, `validate_extension_entries` (blank labels and conflicting-definition), the file ceiling (`intake_with_bytes` opens no file) and `program.source.identity.bind`. It comes before the source read, the declared edition, `NativeModel`, the extension catalog and the extraction clause count. This matches AC-14 and AC-16.
- `read_clause_run_request`, and a run entry with `program.extraction` through `run_extracted_clause_bytes`: only decode, the package's `validate_request` and `check_extraction_clause_count` come before the refusal. `refinement_internals`, `integer_division`, extension checks, the file ceiling and `formal_revision` come after it. This matches AC-15, AC-16 and the "Every other check" bullet. `ExtractionPackageConflict` needs both sources and `ExtractionRequiresRun` needs `call`, so neither can meet the new branch. No refusal is reordered except for requests that satisfy `selects_no_run_member`.
- `run_with_providers`: FR-961 provider registration still comes first, which the spec leaves unordered. The special readers (export, plan, version, check, synthesis, items) key on members this request does not carry. Requests carrying `export` or `export_evidence` take the export or evidence reading before any typed run path. At the run-reader entries `export_evidence` does not exempt the request, as the spec states.

Existing assertions: the test diff only adds lines. `tests/it/spine_run.rs` `tc_450_step_5_no_call_refuses_before_dependent_reads` and `package_member_conflicts_precede_no_call_before_file_reads` are unchanged, and their message and precedence expectations still hold when read against the new code.

Can the tests fail? AC-11, 12, 14 and 15 test precedence. Old code passed them because it had no early refusal at all. They still catch the plausible wrong designs. If `admit` returned `no_run_member()` instead of `Ok`, the AC-11 extra/extensions/requirements tests, the AC-12 sources test and every AC-13 case would fail. If the prepare_run program check sat directly after decode, every AC-14 case would fail. If `read_clause_run_request_bytes` dropped `check_extraction_clause_count` from the new branch, AC-15 would fail at both entries. AC-10 and AC-16 fail on the old code. So each test can fail against its own AC.

Ruling on the extension capability: FR-660 places the unknown-capability check at "the catalog link", which runs after the program and library sources are read. Only the decode rules and conflicting-definition are refused "before any dependent file is read". The code checks `CapabilityId::complete` inside `extension_catalog`, after the program read. So the author's reading is correct: an unknown capability id is not "an extension entry FR-660 refuses from the request alone", and the no-run-member refusal comes first. The FR-1047 wording could state this more plainly. That is recorded as a spec finding in the gap-analysis SR, not as a defect here.

Rust idioms: `selects_no_run_member` and `check_extraction_clause_count` are small private helpers, and adding `PartialEq` on `Missing` is fine. The new code has no new panic, unsafe code or integer conversion.
