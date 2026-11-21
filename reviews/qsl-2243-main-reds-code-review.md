---
id: SR-202611210022442313144108557
title: "code-review of PR3386 realigning two main-red timed-refinement test oracles"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3386 head ae223c69996b7e55c15d826976397b47469f5764 against base abcb1c5ad45882b9100e1a917ef3ccbb23f32d86; fsl-analyze/tests/it/timed_refinement.rs, fsl-analyze/tests/it/timed_refinement_liveness.rs"
review_set: subset
---

## Summary

Ticket: QSL-2243. This review covers the two test-only changes in PR3386, with the rust-review lane folded in. No production or spec file changed.

Test 1 (`a_fair_loop_of_a_timed_component_does_not_hide_an_unfair_one`) no longer pins `prefix[0].delay == 1`. It now requires a non-terminal lasso and a non-empty loop `prefix[entry..]`. Each loop delay must be at most 1 ms, and the total loop delay must be positive. The poll-only names, `HiddenUnfair{idle}` and replay `Reproduced{HiddenUnfair}` assertions are kept. This matches FR-241 ("Lassos": the loop's linear constraints are solved over rational delays, and no fixed representative is mandated) and FR-899 "Timed subjects" ("a loop of total delay 0 refutes nothing"). A zero-delay loop, a terminal stutter, a missing lasso or a wait over 1 ms still fails the test.

Test 2 (`the_liveness_half_of_a_timed_abstract_hidden_field_is_decided`) now expects `Checked(Holds)` instead of `Undecided(LassoNotConcretized)`. The fixture justifies this:

- Concrete `send` has `x >= 1 ms` and `x` is never reset, so `send` stays enabled once taken.
- `assume fair weak M::Call::send` removes the quiescent tail.
- Every concrete-fair loop therefore takes `send`, and `send` maps to `A::Call::send`.
- `A::Call::send` is guarded only by `y >= 1 ms`. With no reset, `y` tracks `x`, so `y >= 1 ms` holds at every concrete send.
- The hidden `late` does not constrain `send`, so every candidate run takes the class.

That is FR-899's hidden-field rule ("for every concrete behaviour fair under F_C, some candidate run along it satisfies F_A"). `TimedLivenessOutcome` has only one holding variant, `Holds`.

One defect: removing the two unused imports left the `use` list unwrapped. `rustfmt --check --edition 2021` fails on the PR file and passes on the base file, and CI runs `cargo fmt --all -- --check`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Removing TimedRefinementRefusal and ZoneCheckRefusal from the zone_check import list left it unwrapped, so rustfmt --check fails on timed_refinement.rs (the base file passes) and CI's cargo fmt --all -- --check goes red | fsl-analyze/tests/it/timed_refinement.rs:7-12, .github/workflows/ci.yml:28 |

## Scope and evidence

I read the whole diff, both fixtures (`two_loop_world`, `timed_hidden_fair`, `refinement`, `document`) and the helpers `violation`, `names` and `hidden_unfair_name`. I also read `Lasso` (`entry`, `terminal_stutter`) in fsl-replay/src/model_check/counterexample.rs and `TimedLivenessOutcome` in fsl-analyze/src/zone_check/refine.rs. Governing text: FR-241 (Lassos, AC-3) and FR-899 (Hidden fields, Timed subjects, AC-9).

Evidence:

- I ran `rustfmt --check` on the two PR files and on the base copy of timed_refinement.rs. The PR copy shows a diff; the base copy is clean.
- No Cargo, clippy or test run was made, as the brief required. Runtime results for the two tests are pending with root/Titan and are not claimed here.

Examined and found sound:

- In Test 1, the old `== 1` assertion was an over-pin. FR-241's least/midpoint/+1 rule applies to finite paths only, and the existing log shows the real value is 1/2.
- In Test 1, the `x <= 1 ms` invariant while not in flight, with `poll` resetting `x`, justifies the per-step bound.
- In Test 1, mutation reasoning: a total-zero loop, a terminal stutter, an absent lasso, a delay above 1 ms, a wrong failure name or a non-reproducing replay each fail an assertion.
- In Test 2, the strict `Holds` follows from the fixture as set out in the Summary. The removed comment cited a test that no longer exists. FR-899-AC-9's own fixture is bound to the TC-2536 tests at timed_refinement_liveness.rs:624-682, which this PR leaves unchanged.
- Rust lane: no panics on untrusted input, no tautological assertion, and trace tags are kept (TC-2536/FR-899). `Rational::add` and `PartialOrd` are used by value and reference correctly; the coder's `cargo check --test it` exited 0.

## Verdict

The oracle changes are normatively justified and do not weaken either test: Test 1 is strengthened from a single representative to the FR-241/FR-899 total-loop property, and Test 2's strict Holds follows from the fixture under FR-899. FND-001 (a rustfmt regression from the import removal) must be fixed before merge.

## Dispositions

Round 1: head 84b94fb546bb0314a4251f310d7f2c85ab83a938, rebased onto origin/main dab8c60624b4427e2290efe7ebaa3827f5ae0842. Against ae223c69996b7e55c15d826976397b47469f5764, the two test files differ only by the three-line re-wrap of the zone_check import list, so the test bodies are unchanged. Main changed neither file between abcb1c5ad45882b9100e1a917ef3ccbb23f32d86 and dab8c60624b4427e2290efe7ebaa3827f5ae0842. `rustfmt --check --edition 2021` is clean on both files at the new head. The committed reviews/qsl-2243-main-reds-code-review.md is byte-identical to this SR's review-pass version. Titan ran the two tests at ae223c69996b7e55c15d826976397b47469f5764 (rc 0: 2 passed, 0 failed, 0 ignored). No runtime was run at 84b94fb546bb0314a4251f310d7f2c85ab83a938, and none is claimed.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The import list is re-wrapped to rustfmt's layout and `rustfmt --check` is clean at 84b94fb546bb0314a4251f310d7f2c85ab83a938 |
