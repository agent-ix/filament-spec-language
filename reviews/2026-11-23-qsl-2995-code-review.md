---
id: SR-967175026302197768900474181941935084803
title: "code-review of PR3641 on-each profile boundary and cyclic-always regression"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3641 head bef20c929aac956e9d0bec9128105cef4a469dd5; fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/analyze/exact/instances.rs, fsl-analyze/src/analyze/items.rs, fsl-replay/src/lib.rs, fsl-replay/src/probability_certificate/document.rs, fsl-replay/src/probability_certificate/document/instances.rs, fsl-analyze/tests/it/probability_from_source.rs, fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs"
review_set: subset
---

## Summary

Tickets: QSL-2995 (commit bef20c929a), QSL-2923 (commit 50b1afcf3e), QSL-2884 (no commit). Rust-review lane folded in. The reorder that forms the instance set before the profile check is equivalent to the old `roots` emptiness test (roots is initial x over x instances), so non-empty on-each items keep their previous settlement. The statistical intercept settles before any sample is drawn. Defects: an admission failure on the new statistical path and on the re-routed on-each quantile reports an internal failure; a substring value check in the control test; formatting-only hunks in exact.rs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Statistical on-each (and the on-each quantile now routed to instances::run) maps a model-admission failure to broken() internal failure instead of build_refusal; elapsed.rs mapped it through build_refusal | fsl-analyze/src/analyze/exact/instances.rs:135-137 |
| FND-002 | low | statistical() maps population/identity failures with ExactProbLimits::default(), so a limit settlement can name default exact_prob limits rather than the request's | fsl-analyze/src/analyze/exact/instances.rs:122 |
| FND-003 | low | on_each_has_instance runs before subject(), so the empty path skips the claims-identity ptr::eq check (IdentityMismatch) and every non-empty item expands initial states and populations twice | fsl-replay/src/probability_certificate/document/instances.rs:675-714 |
| FND-004 | medium | TC-3211 step-1 control asserts the value by substring "391", not the exact 391/400, and does not assert ratio mode or no-instance cause on the empty side | fsl-analyze/tests/it/probability_from_source.rs:1993-1999 |
| FND-005 | low | Formatting-only hunks in exact.rs unrelated to the change; origin/main's form is already rustfmt-clean, so they are pure noise to revert | fsl-analyze/src/analyze/exact.rs:405-408 |

## Scope and evidence

- FR-875 Behavior "Instances" and "Profile boundary" (spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md): examined. Profile rejection after instance formation and no-instance precedence match the reorder.
- FR-875-AC-1, FR-875-AC-4 to FR-875-AC-8: examined against the new TC-3211 tests.
- TC-3211 (spec/test-cases/TC-3211-on-each-settlement-states-the-profile-boundary.md): examined; Expected step 1 names exact value 391/400.
- FR-202-AC-36 (spec/functional/FR-202-replay-a-probabilistic-witness.md): examined for the QSL-2923 test. The test asserts kind lower, x(up)=1, x(down)=1, ranks 1/0, no scheduler, no path, and replays; it discriminates against the 0.2.0 candidate, which predates QSL-2921 (5b9c3ec3d1) and ended incomplete.
- Reorder (exact.rs:1094-1101): non-Each quantiles still reach elapsed::run; Each quantiles now settle unsupported via subject() Unsupported (FR-875-AC-7 quantile case).
- fsl-replay/src/lib.rs hunks: origin/main bbac9b9130 is rustfmt-dirty at lines 441, 451 and 492 of this file; the PR's reflow is what rustfmt produces, so these hunks must stay (reverting makes the file fmt-red). Only the exact.rs hunks at 405-408, 416-419 and 3478-3479 are revertible noise.
- Tests run (focused, -j 8, worktree target): fsl-probability-check it on_each 9/9 passed; fsl-analyze it on-each and rarely_failing filters 14/14 passed.

## Verdict

Not mergeable as is: fix FND-001 and FND-004; FND-005 is a trivial revert. Others low.
