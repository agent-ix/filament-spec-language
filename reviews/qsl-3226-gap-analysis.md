---
id: SR-967175026302197768900474181941935084839
title: "gap analysis (manual, scoped) of QSL-3226 verification-plan fixture (FSL PR #3844)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@208c7f909f03c431874cc8d503b1aba3c98383fc; tests/it/request_surface_s1_16.rs; spec/functional/FR-1038-read-the-verification-plan-member-and-run-the-selected-plan.md; FSpec FR-824"
review_set: subset
---
# SR-967175026302197768900474181941935084839: gap analysis (manual, scoped) of QSL-3226 verification-plan fixture (FSL PR #3844)

## Summary

Ticket: QSL-3226. Plan completion: not assessed.

The dispatch brief forbade `quoin matrix` and repo-wide reverse-gap audits, so this is a
**manual, scoped acceptance-to-test check**, not a full gap audit. The matrix, reverse-gap and
repo-wide stub steps of `quoin:gap-analysis` were not run.

Units examined:

- FSL FR-1038-AC-2 (TC-3018), at the branch text and at `origin/main` after spec #3872.
- FSpec FR-824-AC-2 and FR-824-AC-3, from filament-specification at `origin/main`.

The tests checked are `the_verification_plan_member_selects_one_plan_or_refuses` (line 689) and
`the_verification_plan_reader_refuses_a_malformed_member_at_its_pointer` (line 590). The code
checked is `fsl-command/src/command/plan.rs:303-365`.

The branch FR-1038-AC-2 clauses map to tests as follows:

| Clause | Covered by | Notes |
| --- | --- | --- |
| Only plan runs `Release` | Line 692-712 | Runs with an `initial` snapshot, per decision e92. The exact no-initial request belongs to QSL-3259. |
| Two plans refuse `missing-member` | Line 724-752 | |
| `Nope` and `Never` refuse `missing-name` | Line 724-752 | Pointer and code checks are weak, see SR-967175026302197768900474181941935084838 FND-001. |
| `seed` `"7"` refuses `wrong-value-kind` | Line 593 | Exact path. |
| Each refusal exits 20 | Both loops | |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | After this PR, FSpec FR-824-AC-2 has no tagged test in this repository: a grep for `FR-824-AC-2` over `*.rs` finds none. Its `ambiguous_declaration`/`ambiguous-name` branch for verification plans (`A::Release` and `B::Release` matched by bare `Release`) has no test at all, so a regression in that branch ships silently. Dropping the tag is correct, because it over-claimed. But QSL-3259 owns only the no-initial clauses (FR-1038-AC-2 and FR-824-AC-1), not the ambiguous selection, so this gap is unowned. Either add the ambiguous case in this PR, or have the lead file and own a ticket for it. | fsl-command/src/command/plan.rs:351-363, tests/it/request_surface_s1_16.rs:686-689 |

## Verdict

One medium finding, an unowned coverage gap exposed by the correct trace removal.

FR-1038-AC-2's clauses that this PR is responsible for are each mapped to a test above. The one
weakness in that mapping is the assertion strength recorded in the code-review SR, FND-001.

Spec #3872, now on `origin/main` but not in this branch's base, rewrote FR-1038-AC-2's first
clause around `universe_snapshot`. After merge, the `FR-1038-AC-2` tag here covers the selection
and refusal clauses only. QSL-3259 is open and its done-when requires a test tagged FR-1038-AC-2
for the exact no-initial request. That is deferred, owned work, not a defect of this PR. The merge
of HEAD with `origin/main` is clean, since no main commit touches this file after the merge base.
