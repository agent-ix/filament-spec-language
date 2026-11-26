---
id: SR-9671750263021977689004741869
title: "Code review — QSL-3332 item work accounting"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze/exact.rs, fsl-analyze/tests/it/exact_probabilistic_item.rs, fsl-replay/src/probability_certificate/document/mdp.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Reviewed QSL-3332 PR #3975 at `0795b48aaa18445174488f9e4db576f9132de510`, including the Rust production paths, tests, linked producer/checker routes and the required gates. The analyzer shares the item WorkScope with certificate checking, but MDP proof-document production still starts a fresh accounting meter, and the cumulative tests do not isolate the named phase charges.

## Verdict

**FAIL** — certificate production can spend beyond the item allowance and cumulative phase tests do not detect that reset.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | MDP certificate production uses a fresh meter instead of the item's WorkScope | fsl-analyze/src/analyze/exact.rs:2121-2129; fsl-replay/src/probability_certificate/document/mdp.rs:158-167,438-442 |
| FND-002 | medium | Cumulative-work tests can pass without charging the named certificate phase cumulatively | fsl-analyze/tests/it/exact_probabilistic_item.rs:111-132 |

## Review scope and evidence

- Examined FR-203-AC-29 through FR-203-AC-33; FR-120-AC-35/36, FR-201, and FR-196-AC-5 were context for the shared per-item allowance and outcomes.
- Architecture pair examined: the fsl-analyze producer and fsl-replay checker/replay share one WorkScope.
- The implementation creates `WorkScope` for the item, passes it to the independent certificate checker, but calls `probability_proof_document` without it. That function constructs `Context` with a new `limits.accounting_meter(cancel)` and `work: None`. Thus the producer's certificate arithmetic/retained-storage charges do not consume the remaining item allowance. On a product that spends P units, a certificate production phase needing C units with P+C>B can nevertheless complete under its reset B-unit meter; later checking may fit the original meter's remaining allowance and settle evidence after total work exceeded B.
- The new tests discover the least bound that settles the complete item, then test `complete - 1` and `complete`. That only proves some phase refuses below a threshold. If production gets a fresh meter, the test still passes at `max(P,C)` rather than P+C, so it does not prove the AC-29/30 phase-specific boundary.
- Test command independently run on the reviewed head: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$PWD/target" CARGO_HOME="$PWD/target/cargo-home" cargo test --locked -p fsl-analyze --test it_probabilistic exact_probabilistic_item -- --nocapture` — passed 103/103.
- Coder-reported `cargo check --locked --workspace --all-targets --all-features` passed on this head.
- Strict per-crate clippy remains blocked by the unchanged `dead_code` lint in `fsl-replay/src/hybrid_replay/class.rs`; supplemental clippy passed with that baseline lint allowed. `cargo fmt --check` reports extensive workspace formatting drift, including unchanged code. `cargo deny check` reports existing advisories/duplicate dependency issues; dependency files are unchanged in this PR.

## Criterion scope

| ID | Role | Excerpt |
| --- | --- | --- |
| FR-203-AC-29 | examined | With `accounting.work_units = B`, an exact item whose product construction consumes `P` units and whose certificate production requires `C` units, where `P <= B`, `C <= B` and `P + C > B`, stops at the first unpaid certificate-production charge with `incomplete`, `Incomplete(LimitReached{work_units, B, accounting.work_units})`; it emits no certificate and has no truth value. The same subject and claim with a bound of `P + C` completes certificate production as before. |
| FR-203-AC-30 | examined | With `accounting.work_units = B`, an exact item whose product construction and certificate production consume `P` units and whose independent certificate check requires `C` units, where `P <= B`, `C <= B` and `P + C > B`, stops at the first unpaid certificate-check charge with `incomplete`, `Incomplete(LimitReached{work_units, B, accounting.work_units})`; it emits no accepted certificate and has no truth value. The same subject and claim with a bound of `P + C` completes certificate checking as before. |
| FR-203-AC-31 | examined | With `accounting.work_units = B`, an exact item whose product construction consumes `P` units and whose counterexample witness production requires `W` units, where `P <= B`, `W <= B` and `P + W > B`, stops at the first unpaid witness-production charge with `incomplete`, `Incomplete(LimitReached{work_units, B, accounting.work_units})`; it emits no witness and has no truth value. The same subject and claim with a bound of `P + W` completes witness production as before. |
| FR-203-AC-32 | examined | With `accounting.work_units = B`, an exact item whose product construction and witness production consume `P` units and whose witness replay requires `W` units, where `P <= B`, `W <= B` and `P + W > B`, stops at the first unpaid replay charge with `incomplete`, `Incomplete(LimitReached{work_units, B, accounting.work_units})`; it does not settle `refuted` or emit a replayed witness. The same subject and claim with a bound of `P + W` completes replay as before. |
| FR-203-AC-33 | examined | For FR-196-AC-5's `Coin` model and `probability >= 0.5 [ always[0,3] holds(c.v = 0) ]` claim over `(c: M::Coin)` under workload `Even`, evidence `exact`, an instrumented run records the `work_units` charges of the first proof-only build (`P`), undefined-path re-exploration (`R`), and all later certificate or witness work (`W`). With `accounting.work_units = P + R - 1`, where `P <= P + R - 1` and `R <= P + R - 1`, the first unpaid re-exploration charge settles `incomplete`, `Incomplete(LimitReached{work_units, P + R - 1, accounting.work_units})` with no truth value. With a bound of `P + R`, the |
| FR-120-AC-35 | context_only | In one EN-4 or EN-5 model-subject run with local `max_candidates` above each expansion's charge, two expansions that each fit `accounting.work_units = B` alone but whose combined candidate and clause work exceeds B stop at the first unpaid cumulative charge with `EvaluationMeter { counter: work_units }`; starting the second expansion does not reset the allowance. The same holds for two clause evaluations that fit separately but exceed B together, and for an expansion followed by a claim predicate. A separate run starts at zero; other accounting counters retain their existing scopes. |
| FR-120-AC-36 | context_only | For an argument vector, candidate and evaluated result pair, a metered fixture observes exactly one additional cumulative work unit apiece, beyond its existing clause work; an operation with no result adds no pair charge. A local `max_candidates` denial remains `MaxCandidates` even when the same candidate unit would also exceed cumulative work; a cumulative denial with local capacity remaining names `EvaluationMeter { counter: work_units }`. Cancellation during candidate generation returns the existing cancelled stop before another candidate is generated, and no stopped expansion supplies a co |
| FR-201-AC-4 | context_only | A proof whose certificate the checker accepts settles `proved`; with a checker `max_states` of 2 the check stops and the item settles `Incomplete(LimitReached{…})` naming `max_states`. The same applies while rederiving R/waiting components or checking randomized observation evidence, with the actual consumed value/setting retained. |
| FR-196-AC-5 | context_only | Over a `Coin` model (object `c` with `v: Int[0, 1]`, initially 0, in a population `coins` bounded to 1; operation `drop(b: Int[0, 1])` with `random Coin::drop(b) ~ { 0: 1, 1: 1 };` and postcondition `count(o in M::coins where true) = b`, so that `drop` with `b = 0` deletes the coin and with `b = 1` keeps it; workload `Even` with weight `drop` 1) and the claim `probability >= 0.5 [ always[0,3] holds(c.v = 0) ]` over `(c: M::Coin)` under `Even`, evidence `exact`, the build stops at the product state reached by `drop` with `b = 0` at position 1, where `c.v` is undefined with cause `absent-key`, a |


## Verdict details

The reviewer confirms the analyzer/replay architectural pairing and the focused integration test ran. The test does not establish the certificate producer/checker phase boundaries specified by AC-29 and AC-30; production currently violates that shared-boundary requirement.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Analyzer proof-document production and writing now receive the item's WorkScope. |
| FND-002 | fixed | AC-29 through AC-33 tests measure phase checkpoints and assert first-unpaid and sufficient bounds. |
| FND-003 | fixed | Phase tests now use hand-counted fixed work constants and assert measured checkpoints against them; limits are derived from constants rather than checkpoint values. |

## Current-head review (disposition pass 5)

Reviewed exact PR head `d705e3b591df76eba9378b66dad7772e21a2b829`; REST head and `git ls-remote` agree. The PR base is `dc71332c3dd343a59ca181ff059f1da40867061a`. FND-003 is fixed: the AC-29..33 thresholds use hand-counted constants (Coin product 14,364; certificate production/check 5,130/4; witness production/replay 7/8; undefined proof-only/re-exploration/materialization/replay 35,976/52/2/7; package-owner replay 51). Assertions compare measured checkpoints to those independently stated values.

The bounded-formula route now creates `CandidateBudget` with the item's existing `WorkScope` when `bounded_formula.is_some()`, and passes a clone of that scope through the budgeted proof-only product path and undefined re-exploration. For bounded formulas yielding `FhResult::Bounds`, the branch guard still routes non-fair bounded values through interval handling; focused coverage for value-bound proof and rejected-interval precision fallback remains in the passing integration module. The exact route sweep found standard and partial-lower certificates, timed/mean-time, measure/fraction, long-run, instances, source and package-owner witness replay, certificate checking, undefined replay, and materialization sharing the item scope; standalone unscoped wrappers are not called by these item paths. No meter reset was found.

The required architecture pair is `fsl-analyze` production with independent `fsl-replay` certificate checking and witness replay, all using the same item `WorkScope`. Current-head filtered run: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$PWD/target" CARGO_HOME="$PWD/target/cargo-home" cargo test --locked -p fsl-analyze --test it_probabilistic --features test-support 'exact_probabilistic_item::' -- --nocapture` — **104 passed, 0 failed**. Workspace check passed. Strict clippy fails on the newly changed `Ok::<_, ProbabilityDocumentError>(check?)` at `exact.rs:2589` (`clippy::needless_question_mark`). `rustfmt --edition 2024 --check --config skip_children=true` on the two changed Rust files reports formatting differences in new helper/assertion and candidate-budget hunks, as well as pre-existing file drift.

## New findings (disposition pass 5)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | New phase assertion helpers and candidate-budget expression are not rustfmt-formatted | fsl-analyze/tests/it/exact_probabilistic_item.rs:121,130-137; fsl-analyze/src/analyze/exact.rs:1599-1604 |
| FND-005 | medium | The new certificate-check checkpoint closure fails strict clippy with needless_question_mark | fsl-analyze/src/analyze/exact.rs:2589 |

The focused rustfmt check reports formatting differences in these newly added/changed hunks (for example it expands `assert_eq!` at test line 121 and wraps `assert!` at lines 130-137). The check also reports substantial baseline drift in unchanged code, so FND-004 is limited to the touched hunks. FND-005 is a required per-crate clippy gate failure introduced by the checkpoint wrapper; removing the redundant `Ok(...?)` is the suggested fix. Overall code-review disposition is **not mergeable** until these new findings are resolved or explicitly dispositioned.

## Current-head review (disposition pass 2)

Reviewed the current PR head `90e6811b1befb8c776506b0647f48f1c81485514`, verified against both the PR REST head and `git ls-remote`; its base is current `origin/main` `30305e69c1dc2f1d8f2cca605aaf11ca160aeb5a`. Rechecked the prior fixes and the newly merged main changes overlapping the PR: the partial-lower-measure certificate producer passes the existing `WorkScope` into document production, including fallback after whole-graph production; the shared meter and cancellation flow remain intact. The exact-item phase-isolation tests remain present. No new code-review findings were identified in the changed-file interactions.

The focused current-head run `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$PWD/target" CARGO_HOME="$PWD/target/cargo-home" cargo test --locked -p fsl-analyze --test it_probabilistic exact_probabilistic_item -- --nocapture` passed **105/105**. The reviewer confirms that the `fsl-analyze` producer and independent `fsl-replay` checker/replay are the paired paths that depend on this shared accounting change. Prior findings remain fixed. Code-review disposition for this head: **mergeable**, subject to the separate final integration/gate run owned by the lead.

## Current-head review (disposition pass 3)

Reviewed PR head `f2775940f9ee7e229e9d0d087ebdb1de7f841b7e`, verified against both the PR REST head and `git ls-remote`; its base is current `origin/main` `070824b9fae55641811c6c4c933c2b699b58ce9a`. The latest merge adds bounded protocol-refinement closure accounting and exports its new transition-limited checker entry points from `fsl-replay/src/lib.rs`. These additions do not alter or shadow the probability-document WorkScope APIs used by QSL-3332. The producer, independent checker/replay, and phase-isolation tests remain intact; no new code-review finding was identified.

The focused current-head command `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$PWD/target" CARGO_HOME="$PWD/target/cargo-home" cargo test --locked -p fsl-analyze --test it_probabilistic exact_probabilistic_item -- --nocapture` passed **105/105** (exit 0). The `fsl-analyze` producer and independent `fsl-replay` checker/replay remain the architecture pair that mirrors this change. Prior code findings remain fixed; this head is **mergeable** from the code-review perspective.

## Current-head review (disposition pass 4)

Reviewed PR head `fe4c8eee75e3e2b502948165ae3a50cab791ad76`, verified against both the PR REST head and `git ls-remote`; current `origin/main` base is `dc71332c3dd343a59ca181ff059f1da40867061a`. The QSL-3402 merge changes semantic conformance/index/refusal handling and review documents, not the `WorkScope` producers/checkers. In the exact-item routes, standard and partial-lower certificate production, certificate serialization/checking, timed/mean-time, measure/fraction, long-run, instance, witness production/replay (both source and package-owner routes), and undefined replay all use the scoped APIs with the item `WorkScope`; the legacy unscoped replay helper has no call site in this analyzer path. Standalone public wrappers that create their own meters are not used by the exact-item analyzer routes. No meter-reset production regression was found.

The focused current-head test `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$PWD/target" CARGO_HOME="$PWD/target/cargo-home" cargo test --locked -p fsl-analyze --test it_probabilistic product_and_certificate_production_share_the_item_remainder -- --nocapture` passed **1/1**. The broader 105-test filter passed on the immediately preceding head. The new owner-approved QSL-2846 review rule requires paired producer/checker and witness-production/replay tests with hand-computed expected work values. The current FR-203-AC-29..32 tests instead derive P/C/W from `ExactWorkCheckpoint.consumed` in successful product runs and use those observed values as thresholds. This is not an independent hand-computed oracle, so it is a new test-gap finding and blocks merge pending a fixed-work fixture/assertion.

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | Cumulative-work phase tests derive P/C/W thresholds from the product's own checkpoints instead of hand-computed expected values | fsl-analyze/tests/it/exact_probabilistic_item.rs:110,123-129,182,213-214,259-267 |

The owner-approved QSL-2846 review rule requires paired paths to be tested with hand-computed expected values. The current tests instrument a successful run, set `P`, `C` and `W` from `ExactWorkCheckpoint.consumed`, and then derive the insufficient/sufficient bounds from those same observations. They can self-calibrate if a phase's charge accounting is incomplete, so they do not independently establish the expected cumulative boundary. Add a small fixture with charge values calculated from the spec/code path and assert fixed expected limits and outcomes across certificate production/checking and witness production/replay.
