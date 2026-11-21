---
id: SR-20261121050325
title: "Code review — QSL-2158 remainder / QSL-2159 PR #3261 eight-member product keys, package-bound certificate check, every-scheduler refutation"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3261 diff origin/peter/qsl-2156-digital-completion...2104fccb69 (stacked on #3204): fsl-analyze, fsl-replay, tools/fsl-probability-check"
review_set: subset
evaluated_revision: "2104fccb69"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-876
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-282
    type: references
---

## Summary

Tickets: QSL-2158 (remainder) and QSL-2159. Reviewed the stacked diff `origin/peter/qsl-2156-digital-completion...HEAD` (4 commits, 23 files) by reading it in the worktree at `2104fccb69`. The coder's focused logs `-8` and `-9` were read. One exact-test build was attempted and timed out in the locked-build queue, so the findings below are static, except FND-001, which log `-8` reproduces.

Coder decisions, judged:

- (a) Long-run claim flipped from `>= 1/2` to `<= 1/2`. This is a legitimate oracle correction, not a bent test. In the stub's model, `send_a` writes `attempts := (lost = 0 ? 0 : 2)` and `delivered := (lost = 1)`, and `Stop` (`attempts = 2 and not delivered`) is absorbing. Each step reaches `Stop` with probability 1/3, so every run is absorbed with `delivered` false and the long-run fraction is exactly 0. `>= 1/2` was false, so no proof certificate could exist. The test exists to show the certificate kind (`long-run`), and that is still asserted. Note: this test is FR-201-AC-51, run through the library helper. It is not FR-741-AC-2's long-run item through `fsl analyze` (gap analysis SR-20261121050326).
- (b) Package route. Reading the source reference and name from the emitted package, then requiring the recompiled S4 package to be byte-identical, is a sound binding. The byte-identical package is the subject, so the self-derived `expected_source` adds nothing and loses nothing. Weaknesses are FND-004 to FND-006.
- (c) Eight-member key. The writers (`key.rs` `Product`, `digital_probability_product_key`, `region_probability_product_key`, `timed.rs` `region_key`) and the readers (`decode_key`, `admit_parts`, `product_action::product_key`, `timed_key_progresses`, `timed_clock_count`, `timed_has_deadline`) are consistent. Two test files still read the six-member positions (FND-001, FND-002).
- (d) QSL-2159 retry. The soundness argument holds. A claim `P >= p` (or `<= p`) under every scheduler is a universal statement. Any single deterministic scheduler whose failure paths have mass above `past` (`1 - p`, or `p`) is a counterexample. The witness names every choice (`WitnessScope::EveryScheduler`), and FR-202 replay re-checks the mass independently. The analyzer's choice of scheduler can therefore make the search incomplete, but never unsound. For the eventual form, `accepting` is the non-target dead ends with only `TerminalStutter` edges, and the subject is admitted acyclic (`admit_acyclic_probabilistic_refutation_subject`). So `max P(reach dead end) = 1 - min P(target)`: the retry is complete there and attains the refuting value. For workload claims, the product is a DTMC and `policy_action`/`native_action` ignore the policy, so the retry cannot change a workload witness. The code comment's "may loop" rationale is loose for an acyclic product, where the real cause is an arbitrary tie or prob-0 policy entry, but this does not affect soundness. The retry is skipped whenever `stopped` is set (FND-007).

Of the 17 failures in log `-8`, one is caused by this PR: `bounded_raw_cycle_has_distinct_real_horizons_and_exact_ranks`, bounded.rs:429 `left: 0 right: 2` (FND-001). The other 16 fail with `conflicting_identity: probability certificate` (document.rs:247), or with a `threshold-missed`/`rank-broken` rule of `null`. In every one of these, the test changes the source threshold or statement through `replace_source`/`stronger`, which main's #3121 now puts in the claim node identity. Two exceptions: `bounded_fair::zero_horizon_*` (bounded.rs:80) stops with a compile refusal, `Unproved(Nonzero)`. `reward.rs:447` gets `refused` where it expects `unsupported` after a `replace_source`. Neither touches a key position. One more test, `bounded_altered_horizon_binding_and_rank_never_certify_the_original_claim`, passes only because #3121's identity refusal hides that it is now vacuous (FND-001).

## Verdict

**FAIL — fix round needed.** 2 high, 3 medium, 4 low. Not mergeable until FND-001 and FND-002 are fixed: both are suites this PR turns red. The rest of the diff is sound. The eight-member writers and readers agree, the package byte-identity binding holds, and the QSL-2159 refutation is sound.

## Rust review

| Check | Result |
| --- | --- |
| Behaviour vs spec | FR-196's eight members are written and admitted consistently. `on each` keys are refused by the closed document profile and admitted by the post-state reader, which is consistent. The refutation retry is sound (Summary d). |
| Oracle integrity | The long-run flip is an oracle correction (Summary a). The un-ignored FR-876, FR-196-AC-15 and FR-201-AC-49 tests assert real behaviour. FND-003 is the unasserted value 0. |
| Test fidelity | Stale six-member indices make one test fail and one vacuous (FND-001), and one fsl-analyze test fail (FND-002). The package route has no negative test (FND-004). |
| Errors/API | `ProbabilitySourceError::Package(String)` erases the typed `ReplayRequestRefusal`, and `PackageSources` is labelled `conflicting_identity` (FND-006). The `UnboundedError` to `Limit` mapping is duplicated (FND-009). |
| Safety/concurrency/wire | No unsafe code and no concurrency. The `package` wire member is a byte array, like `byte_provision`. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `bounded.rs` still reads the six-member positions. `key(..)[3][1][2][0] == "bounded"` (now the null `activation` member) never matches. `bounded_raw_cycle_has_distinct_real_horizons_and_exact_ranks` therefore fails (`running.len()` 0 against 2, log -8 bounded.rs:429), and its `first[2]`/`first[3]` comparisons are stale. In `bounded_altered_horizon_binding_and_rank_never_certify_the_original_claim`, the same guard gates every alteration ("binding" `state[3][1][1][3]`, "local-id", "absent"). Only "horizon" was moved to `[5]`, and it is behind the dead guard, so no alteration is applied. The test passes today only because main's #3121 identity refusal makes the unaltered request fail. Fix: the monitor is `[5]` and the model is `[4]`. | tools/fsl-probability-check/tests/it/bounded.rs:427-436, 557-562 |
| FND-002 | high | `a_formula_product_embeds_the_real_complete_monitor_without_inner_transport` (active) asserts `parsed[2]` is the model and `parsed[3]` the monitor of a `for_formula_state` key. With eight members these are the null `instance`/`activation`, so the test fails. It was not run in the focused set. Fix: `parsed[4]`/`parsed[5]`, and assert that `[2]`/`[3]` are null. | fsl-analyze/tests/it/bounded_closure.rs:501-514 |
| FND-003 | medium | QSL-2159 / FR-876-AC-2: `every_scheduler_without_random_is_refuted` asserts only `refuted` and `replay.reproduced`. The AC requires "its value 0", and also never `proved` and never `unsupported`. The value is neither asserted nor shown in the item. Fix: assert the reported exact value 0 (or the counterexample's value member). If the item has no value member, that is the gap to close. | fsl-analyze/tests/it/probability_from_source.rs:713-721 |
| FND-004 | medium | The new package route has no negative test. Nothing shows `PackageMismatch` (an emitted package from another source or threshold), `PackageSources` (a multi-source package), `Package` (unreadable bytes), or the request rule "exactly one of source and package". FR-282-AC-20's test would still pass with the byte-identity comparison deleted. Fix: add one refused case per arm, at least the mismatched package. | tools/fsl-probability-check/tests/it/certificate_kinds.rs:153-209; fsl-replay/src/probability_certificate/source.rs:142-198, 300-307 |
| FND-005 | low | On the package route, a request that also carries `package_name` is accepted and the member is ignored. The test's `later` request keeps the source request's `package_name`, so a wrong name passes silently. The wire comment says exactly one of `source`/`package` is named, but `package_name` belongs only to the source route. Fix: refuse `package_name` alongside `package`. | tools/fsl-probability-check/src/main.rs:66-76, 148-182 |
| FND-006 | low | `ProbabilitySourceError::Package(String)` stringifies `ReplayRequestRefusal` and drops the typed cause. `PackageSources` is spelled `conflicting_identity` although a multi-source package is a shape refusal. Fix: carry the refusal as a typed `#[source]`/`#[from]`, and label `PackageSources` as a package-shape error. | fsl-replay/src/probability_certificate/source.rs:94-103, 151-160 |
| FND-007 | medium | QSL-2159: the maximizing retry runs only when `stopped.is_none()`, so the direct-state `until`/`always` forms never retry. An every-scheduler `always` claim over a unit without `random` lines can still settle `unsupported` on the same tie. QSL-2159's diagnosis names `StaysUp` (`always holds(s.up)`) and `CheckoutAvailable` (`always[0,10]`) among the affected claims. The retry is equally sound with `produce_witness_stopping`, because stopped states end the paths. Fix: retry in both arms, and add an `always` case without `random` to FR-876's tests. If this is out of scope, note the limitation on QSL-2159. | fsl-analyze/src/analyze/exact.rs:1884-1912 |
| FND-008 | low | `product_action::product_key` admits instance and activation independently: a null `instance` with `"active"`, or an instance with a null activation, passes. FR-196 pairs them: an `on origin` key has both null, and an `on each` key has both set. Fix: require both null or both set. | fsl-replay/src/product_action.rs:355-362 |
| FND-009 | low | The retry repeats the caller's `UnboundedError` to `ExactProbBuildIssue::Limit` mapping verbatim (exact.rs:1028-1050). Fix: factor one `unbounded_issue(error, limits)` helper. | fsl-analyze/src/analyze/exact.rs:1888-1910 |

## Dispositions

Disposition pass 1 at `955174db08` (fix commits `8fa61c25ec`, `955174db08`), read from git objects; no build or test run.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | bounded.rs reads the monitor at `[5]` and the model at `[4]` in both the raw-cycle filter and the alteration guard, so every alteration now applies. |
| FND-002 | fixed | bounded_closure.rs asserts `parsed[4]` is the model and `parsed[5]` the monitor. |
| FND-003 | fixed | New `the_refutation_without_random_has_value_zero` asserts the replayed witness mass is 1, so the event's value under that scheduler is 0. |
| FND-004 | fixed | Three new refused cases: a provision recompiling to other bytes (`PackageMismatch`), zero sources and unreadable bytes, a wrong package name, and neither/both subjects. |
| FND-005 | fixed | A stated `package_name` must equal the package's own name, otherwise `PackageName` is refused; tested. |
| FND-006 | fixed | `PackageRead(ReadError)` and `Package(Box<ReplayRequestRefusal>)` keep typed causes; `PackageSources` is no longer labelled `conflicting_identity`. |
| FND-007 | fixed | The retry runs in both arms; with `stopped` it maximizes `decide_until` over the open states; an `always` case without `random` is tested. |
| FND-008 | fixed | `product_key` requires `instance` and `activation` both null or both set; tested by TC-3060. |
| FND-009 | fixed | One `unbounded_failure` helper serves the solve and the retry. |
