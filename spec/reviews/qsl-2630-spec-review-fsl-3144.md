---
id: SR-20261120051601
title: "Base review of completion spec batch 2, FSL half (PR #3144)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR #3144 at 3753974de9, diff origin/main...HEAD: FR-065, FR-931, FR-961, FR-1039, NFR-140, TC-376, TC-519, TC-2601, TC-2701, TC-3019, TC-3033"
review_set: base
---

## Summary

Ticket: QSL-2630 (batch also covers QSL-2636, QSL-2577, QSL-2598, QSL-2614, QSL-2638, QSL-2343, QSL-2627, QSL-2624). Base checklist over the FSL half of completion spec batch 2. The rulings are written down as requirement text: FR-931 always prints `count`, `sum`, `fold`, `reduce` and `only` type arguments from the checked node; FR-065 widens tuple positions as union payloads; FR-961 gains the overrides entry and AC-6; NFR-140 binds a bare id to the holding member and reports an unminted bare id in `untracked_symbols`; FR-1039 gains `tl_lowering` (QSL-2627); TC-519 Scope gains FR-124-AC-11 (QSL-2624). No high finding.

## Verdict

PASS with one medium and three lows. New AC ids FR-065-AC-10/11, FR-1039-AC-6, FR-931-AC-16/17, FR-961-AC-6 and NFR-140-AC-4 collide with nothing on origin/main or any open PR. quire validate exits 0 on every changed file. FR-961-AC-6 matches FR-869 (s1.* is a parse-stage limit) and execute_prepared (a compile failure returns before settle_call), so the lead's point 1 holds. Nothing assumes the driver FR-002 AC exists.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-065 tuple bullet refuses every non-integer argument of another type, but Contract IR FR-347 "Tuple positions" admits a record conforming to a record position, so producer and reader rules differ | spec/functional/FR-065-migrate-function-application-to-checked-family.md:135-137 |
| FND-002 | low | FR-931 table row and FR-931-AC-17 spell the population count type as `Int[0, n]`, which can be read as requiring a concrete `n` | spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md:441,525 |
| FND-003 | low | FR-931 parenthetical says FSpec FR-322 carries the type as `type_argument` and `result_type` for every listed form; FR-322 states `type_argument` only for the population forms | spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md:450-451 |
| FND-004 | low | FR-961 step 3 says only a refusal before the call settles no item; AC-6 relies on a compile-stage limit stop (incomplete, exit 22) also settling none | spec/functional/FR-961-settle-a-run-call-s-requirement-items-through-run-with-providers.md:153-154 |
