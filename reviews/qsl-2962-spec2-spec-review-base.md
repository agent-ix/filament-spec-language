---
id: SR-3618001
title: "Spec review of the reserved-word fixture rename first to pick (filament-spec-language PR 3618)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@18b104b0c1dc0e9432f787b4b60a0bc1b7c8a4d5; spec/functional/FR-027-export-compiled-native-package.md (FR-027-AC-19); spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md (FR-121-AC-36); spec/test-cases/TC-516-locate-a-function-call-site.md (steps 31, expected step 31); spec/test-cases/TC-3210-a-compile-request-call-site-refusal-reaches-the-user-with-its-hint.md (step 3)"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-121
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-516
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3210
    type: references
---

# Spec review of the reserved-word fixture rename (PR 3618)

## Summary

Ticket: QSL-2962. Scope `git diff origin/main...HEAD` at head
18b104b0c1dc0e9432f787b4b60a0bc1b7c8a4d5: one commit, 4 files, 6 lines
changed, a rename and nothing else. The integrity and EARS sub-analyses
were not run, because no statement changed beyond the rename.

Checked:

- The rename is complete. No `units::first`, `function first`,
  `units, first` or `"first"` is left anywhere in spec/. FR-027-AC-19,
  FR-121-AC-36, TC-516 step 31 with its expected result, and TC-3210 step 3
  all say `pick`.
- The fixture matches the code: fsl-replay/tests/imported_function_site.rs:407
  declares `function pick using v(r: R): Boolean pure { true }`, and :428
  selects `("test/units", "pick")`. fsl-replay/tests/imported_function_replay.rs:25
  and :492 use the same `pick`. No fsl-command test pins AC-19 yet; that is
  the code ticket's work.
- The pinned message `ill_typed/operator-ineligible: package test/units
  function test/units::pick has a package-dependent signature` matches the
  format at fsl-replay/src/call_site.rs:504 (`package {package} function
  {selection}`), with `ImportedFunctionName` displayed as `library::function`
  (call_site.rs:81-85).
- `first` is reserved under `1-draft`. It is in the base vocabulary of
  fsl-cst/src/token.rs:174 (`First => "first"`), not the complete-only
  list, so `function first` refuses at S1. `pick` is in neither table.
- No other call-site fixture uses a reserved word as an identifier:
  `big`, `small`, `nope`, `pick`, `q`, `x`, `r`, `R`, `u`, `v` (FR-121-AC-32,
  AC-34, AC-35, AC-36; TC-516 steps 27 to 31; FR-027-AC-15 to AC-21). The
  `absent` in `test/absent` is reserved (token.rs:198, complete-only), but
  it occurs only inside a library-identity string (`import` literal or JSON
  `library`), which is never lexed as an identifier. `other`, `units`,
  `wrap` and `test` are not reserved.
- `quire validate` passes on all 4 files. Its 3 warnings (FR-121 lines 199
  and 293) are on unchanged lines.

The author's open point, that no spec lists `first` as reserved, is not a
gap. FSpec FR-339 (rev 3200a4e, the dependency rev in Cargo.lock) says
"Reserved words ... are fixed by the linked language profile". It
incorporates proposals/filament-v1/shared-grammar.md, whose list of common
reserved words (lines 64-79) includes `first` (with `activate`, `within`,
`attempts`). FSL FR-035 defers to "the shared grammar" for edition-reserved
words. No owner ticket is needed. An optional, non-blocking clarity edit
for FR-035: "Edition-reserved words are those of FSpec FR-339's shared
grammar reserved-word inventory (for example `first`, `by`, `result`,
`retry`); a native binder or declaration name spelled as one refuses at S1
`invalid_syntax`/`unexpected-token`."

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean and mergeable. The rename is consistent across all four restatements,
matches the buildable fixture and the message format, and no other
call-site fixture uses a reserved word as an identifier.
