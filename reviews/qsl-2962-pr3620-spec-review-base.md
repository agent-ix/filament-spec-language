---
id: SR-3620001
title: "Spec review of the FR-027 table split, export compile-only members and one-unit arrays (filament-spec-language PR 3620)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@736670854b8f6ecedc657733d1a1a9e02f782856; spec/functional/FR-027-export-compiled-native-package.md (call_site program rule, acceptance table, FR-027-AC-21); spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md (compile-only member bullet, FR-1015-AC-18); spec/test-cases/TC-3210 (step 5); spec/test-cases/TC-3211-an-export-request-refuses-a-compile-only-member.md"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: references
---

# Spec review of PR 3620

## Summary

Ticket: QSL-2962. Scope `git diff origin/main...HEAD` at head
736670854b8f6ecedc657733d1a1a9e02f782856: one commit, 4 files.

(1) Table split. The only deleted line is the blank line after
FR-027-AC-13, and both origin/main and HEAD hold 18 `FR-027-AC` rows.
`quire matrix --scope . --format json` over origin/main's spec lists
FR-027-AC-2 to AC-13 only. At HEAD it lists AC-2 to AC-21. No row is lost
or merged.

(2) Export compile-only members. Today fsl-command/src/command/export.rs:219-226
refuses top-level `output` and `handoff` with
`RunCause::ClauseRequest { member, reason: "a compile member is not an
export member; name the file in export.output" }`. That is code
`invalid-request` (command.rs:1259-1264), category refusal, exit 20, and the
message is `invalid clause request member <name>: ...`, which names the
member. The check runs before `target_path` and before either export
branch, so it writes no file and runs no export. The bullet and AC-18 state
this. `call_site` is not in that list. On origin/main, a source-program
export re-reads the remaining members as a `wire::CompileRequest`
(export.rs:292-297), and `deny_unknown_fields` refuses `call_site` there
with serde's unknown-field message. Wiring PR #3617 (origin/QSL-2962-wire
f0f9fc71) adds `call_site` to `CompileRequest` and does not touch
export.rs. With #3617 alone, an export carrying `call_site` is accepted, and
the member is silently ignored by `compile_and_map`. The spec is right
here, and the gap is in the code: #3617 (or the code ticket for this PR)
must add `"call_site"` to the list at export.rs:219. TC-3211 is free on
fresh origin/main. No spec file references the deleted one. The only old
references are the committed review artifacts (integrity FND-001).

(3) Arrays. FR-027's sentence, AC-21 and TC-3210 step 5 agree with each
other. FR-480 (lines 68-71) leaves the request rules to FR-027, so it does
not contradict them. But a one-unit array is inconsistent with how the
command treats a one-unit array everywhere else (FND-001).

`quire validate` passes on all 4 files, with one warning on a changed line
(EARS FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-027 and AC-21 now refuse `call_site` over "an FR-480 array of one unit ... since the program was written as an array". The command does not treat a one-unit array as an array anywhere else. `wire::CompileProgram` keeps only `sources` and `extraction`, not whether the program was an array (wire.rs:183-186, `compile_program` :197-221). `complete` sends a one-source list to `complete_single` (command.rs:3401), and the model-unit check keys on `sources.len() == 1` (:2907), so a one-unit array compiles to the same single-unit package as the object form. FR-665's `handoff` accepts a one-unit array: `validate_handoff_request` refuses only `source_count != 1` (command.rs:3081-3086, "requires exactly one program source"). The wiring PR #3617 refuses `call_site` only when `program_sources > 1`. So the rule needs a new wire distinction, contradicts the handoff precedent for the same question, and refuses a request whose checked package is the one the object form locates over. Candidate: "or the request's `program` is a model unit or holds more than one unit (FR-480)". In AC-21: "an FR-480 array of two units refuses; an array of one unit locates as the object form does". If the one-unit refusal is kept, state it as an FR-480 rule for every single-unit-only member (`handoff` included), so the two members do not disagree. | spec/functional/FR-027-export-compiled-native-package.md:100; spec/functional/FR-027-export-compiled-native-package.md:166 |

## Verdict

Not mergeable as is, on one medium finding. (1) is correct, and the matrix
now sees AC-14 to AC-21. (2) is correct and matches the code for `output`
and `handoff`. Its `call_site` half needs export.rs:219 in the wiring PR:
a code gap, not a spec defect. (3) is consistent with FR-480's text but
inconsistent with `handoff` and the compile path (FND-001). If the owner
keeps the one-unit refusal as a ruling, record FND-001 as accepted-no-change
and the PR is mergeable.
