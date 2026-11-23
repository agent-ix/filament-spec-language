---
id: SR-967175026302197768900474181941935084797
title: "Code review of QSL-2951 probability-check red test and merge-gate routing"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3570 head 28472bfdbfd5086742d13900bef6f8a21bbd82b8; tools/fsl-probability-check/tests/it/cyclic_mdp.rs, xtask/src/merge_gate_recipe.rs, docs/merge-gate-recipe.md"
review_set: subset
---

## Summary

Ticket: QSL-2951. Code review with the rust-review lane folded in, scoped to `git diff origin/main...HEAD` (three files, one commit 28472bfdbf).

The test half is correct. `git log -S` and `git show a3588a41ad` confirm the breaking commit: QSL-2820 (#3474) replaced `cyclic_mdp.rs`'s `replace_source` body, which used to swap only `source.content` and `byte_provision[0][2]`, with `bounded::replace_source_and_rebind`, which recompiles the edited source through `compile_with_mode(...).unwrap()`. With threshold 0 or 1 that compile is refused at S3, so the helper panicked. FR-186 ("If a probability ... threshold is 0 or 1, S3 SHALL refuse the claim [`qualitative-threshold`], naming the qualitative form"), FR-186-AC-2, ADR-024 PF-8, ADR-028 XF-8 and FR-201's cyclic-MDP threshold clause ("Existing S3 endpoint admission is unchanged: source thresholds 0 and 1 are refused") all require that refusal. The front end is right and the test was stale. The rewrite asserts the exact S3 refusal in-process: one `CheckRefusal`, `ProbabilisticCause::QualitativeThreshold`, claim `Likely`, `Possible` for 0 and `Always` for 1, which matches `fsl-semantics/src/check/probabilistic.rs:954-961`. It also rebuilds the checker request exactly as before a3588a41ad (content plus byte provision) and keeps `assert_ne!(run(&outside).0, 0)`. Assertion by assertion, the test makes every assertion it made before (five malformed mutations, foreign binding, `rank-broken` boundary, and checker non-zero exit at 0 and 1), plus the new S3 assertions. No assertion was lost. The checker-level 0/1 behaviour is still covered by the same assertion as before. S3 `qualitative-threshold` is also tagged FR-186-AC-2 in five other tests (fsl-semantics and fsl-replay spine), so there is no coverage loss.

The merge-gate half adds `fsl-probability-check` IT modules to the `fsl-analyze/src/analyze/exact.rs`, `exact/**` and `fsl-replay/src/**` routes, and a special case for the `tools/` location of the integration root. The paths exist (`fsl-analyze/src/analyze/exact.rs`, `fsl-analyze/src/analyze/exact/` with 7 modules). No workflow file is touched. The doc table matches the code. The recipe test is not tautological: without `integration_root`'s `tools/` arm, `add_all_it_modules` would fall back to a whole-library selector and the test would fail. But the routing still misses the paths that caused this regression, and it misses the exact-probabilistic producer (FND-001, FND-002).

## Verdict

**CONDITIONAL**. The test fix is correct and loses nothing. The gate change works as far as it goes, but it would not have caught the QSL-2820 break it is meant to prevent. There are two medium routing gaps and two low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No route reaches `tools/fsl-probability-check/**`, and these are the paths that broke. a3588a41ad changed only `tools/fsl-probability-check/tests/it/*.rs`, `fsl-semantics/src/check/**` and `fsl-analyze/tests/it/*`. None of them matches the new exact*/fsl-replay rules. `crate_name` returns None for a `tools/` path. A changed `tools/fsl-probability-check/tests/it/cyclic_mdp.rs` therefore routes through `integration_path` to package `filament-spec-language` with filter `cyclic_mdp`, but the root crate has no such module. A changed `tests/it/main.rs` resolves `test_package` to `tools`, and `tools/fsl-probability-check/src/main.rs` gets "no targeted cargo tests". A PR that edits the checker or its tests still never prints the checker's IT commands | xtask/src/merge_gate_recipe.rs:1080-1088; xtask/src/merge_gate_recipe.rs:1120-1131; xtask/src/merge_gate_recipe.rs:1013-1095 |
| FND-002 | medium | `fsl-analyze/src/exact_probabilistic.rs` and `exact_probabilistic/**` are the engine whose `ExactProbLimits`/`ProductRequest` proofs the probability-check ITs round-trip (11 imports across the IT modules). They fall to the generic `fsl-analyze/src/` rule, which adds no `fsl-probability-check` modules. The new test is named `exact_probabilistic_and_certificate_paths_...`, but it only exercises `analyze/exact` and `fsl-replay` paths | xtask/src/merge_gate_recipe.rs:407-432; xtask/src/merge_gate_recipe.rs:1891-1896 |
| FND-003 | low | All 11 checker IT modules (77 tests, each compiling a source through the spine and the analyze producer and spawning the checker binary) now run for every `fsl-replay/src/**` change, including unrelated hybrid, refinement, monitor and model-check modules (71 entries). Narrower prefixes would fit the intent: `certificate*`, `probability_certificate*`, `exact_probabilistic*`, `spine.rs`. The added cost was not measured | xtask/src/merge_gate_recipe.rs:109-111; xtask/src/merge_gate_recipe.rs:458-466; docs/merge-gate-recipe.md:23 |
| FND-004 | low | The recipe test hard-codes the 11 module names and checks them with `contains`. A module added to `tools/fsl-probability-check/tests/it/main.rs` is never required. The existing `top_level_integration_modules` helper (used at line 2022) would derive the list | xtask/src/merge_gate_recipe.rs:1897-1919 |
