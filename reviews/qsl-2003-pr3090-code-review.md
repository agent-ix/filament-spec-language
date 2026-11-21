---
id: SR-2026112105500001
title: "Code review — PR #3090 QSL-2003 count result ranges"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3090 diff origin/main d84ed5685b...901c5e27ba: fsl-semantics/src/check/facts.rs, fsl-replay/src/outcome.rs, fsl-replay/src/spine/clause/tests/{clause_paths,temporal,witness}.rs, fsl-analyze/tests/it/monitor_inputs.rs, fsl-eval/tests/it/collection_queries.rs, fsl-semantics/tests/it/check_mode.rs"
review_set: subset
evaluated_revision: "901c5e27ba"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-766
    type: reviews
---

## Summary

Ticket: QSL-2003. This review covers PR #3090 at head `901c5e27ba` (10 commits on `origin/main` `d84ed5685b`). The code-review lane and the rust-review lane (dev-tools:rust-review with rust-style) ran by reading the diff, its callers, and the other places in the tree that use `count<N>`. No build or test was run; the findings are traced statically.

The PR adds a Linked range obligation on `Query{visit: Count}` in `fsl-semantics/src/check/facts.rs`, as ruled. It adds a clause-run cause arm for `CallOutcome::Refused` in `fsl-replay/src/outcome.rs`, so the run item keeps `integer_out_of_domain`, as ruled. It enables the QSL-2003 stubs and moves the run-time `count<Zero>` fixtures into called functions. The new obligation now refuses at check three groups of fixtures that the PR did not move: the TC-3121 witness unit, two corpus expectations keyed `flips_with: QSL-2003`, and the shared unit in `tests/it/temporal_capture_run.rs`. All three suites go red on merge.

## Verdict

**FAIL. One fix round is needed: three high, one medium and three low findings.**

What is correct:

- **The obligation.** The proved interval is `[0, m]`, where `m` is the `size(c)` fact when one is in scope and otherwise the collection type's cardinality. This is the same lookup that `Fold` and indexed access use, and the `size` fact is seeded from the type bound (facts.rs:2151), so a guard can only narrow it. The required interval is `N`'s domain. Under `Integer` the domain has no bounds, so the obligation is discharged. A population count is typed `Int[0, max]` from the same bound (typing.rs:4375-4387), so it is discharged by construction, as FR-766 says. The refusal carries `Obligation::Range{required, proved}` with an open upper bound for an unbounded source (AC-3).
- **Mode.** Declared state clauses keep their Linked definedness under both modes, which is what makes AC-8's Kernel half refuse. Function bodies take `with_check_mode(mode)` (mod.rs:9558, family.rs:1662), so the Kernel path is left to S6a.
- **outcome.rs.** The new arm destructures both `CallRefusal` variants with no wildcard and projects `code.code()` and `code.cause()` through the existing `catalog` helper. AC-7's test is enabled.
- **Tests.** `q14_*` cover all four collection kinds (OrderedSet added), the guard, `Int[1, 3]` and the unbounded source. They assert the exact typed `Unproved(Range)` cause. The new kernel test asserts the record's code, cause, `expected` field and location. `an_unannotated_literal_count_source_is_an_ambiguous_literal` implements the ruled AC-9.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Removing `RefusedDomain` from `CLAUSES` also removes it from `tc_3121_unit_text`, which keeps that invariant by filtering `witness_unit_text()` and rewriting its inline count to `z(..)`. The filter and the replace are now no-ops. `tc_3121_public_derivation_refused_and_exhausted_cases_keep_metadata` calls `replay_case(&unit, "RefusedDomain", ..)`, which panics at `graph().state_clause("RefusedDomain").unwrap()`. The lane did not run `witness::`. | fsl-replay/src/spine/clause/tests/witness.rs:144-164; fsl-replay/src/spine/clause/tests/witness.rs:2581-2585; fsl-replay/src/spine/clause/tests/witness.rs:2045 |
| FND-002 | high | Two corpus expectations keyed `"flips_with":"QSL-2003"` were not flipped. `agg-count-narrow-2003` expects `check` exit 0, and `r-count-narrow` expects a `run` refusal at S6a `integer_out_of_domain`. Both hold `count<N>` with `N = Int[0,1]` over `Sequence<Integer>[0,3]` in a function body, which spine compile now refuses under Linked with `unproved-range`. `source_corpus::corpus_collections` fails. | tests/corpus/collections/agg-count-narrow-2003.expected.json:1; tests/corpus/collections/r-count-narrow.expected.json:1; tests/it/source_corpus.rs:991-996 |
| FND-003 | high | The shared unit of `tests/it/temporal_capture_run.rs` declares `temporal Refused { capture z: Zero = count<Zero>(m in ints(sequence[r.level]): m > 0); .. }`. `ints` returns `Sequence<Integer>[0, 3]`, so the Linked CLI compile now refuses the whole unit, and all ten tests in the file fail, not only `a_refused_capture_is_reported_at_the_capture`. | tests/it/temporal_capture_run.rs:38-39; tests/it/temporal_capture_run.rs:259-271 |
| FND-004 | medium | In `monitor_inputs.rs`, `compile_with` now checks any unit containing `count<Zero>` with `PackageDeclarations::check(CheckMode::Kernel, ..)` over the whole unit. That skips spine compile, drops `dependencies` and libraries, and puts every temporal clause in `event_unit` (Small, Gate, Dense, Wide, ...) under Kernel. FR-766 Notes require only the called function to be checked under Kernel, with the clause under Linked, as witness.rs and temporal.rs do through function modes. The event-unit monitor tests no longer run the pipeline the other fixtures use. | fsl-analyze/tests/it/monitor_inputs.rs:173-205 |
| FND-005 | low | `temporal.rs` rewrites fixture source at request time. `request()` string-replaces four spellings of `count<Zero>(m in ints(sequence[..]): m > 0)` with `refusingCount(..)` and appends the function. `report()` then sniffs the bytes for `function refusingCount` to set the Kernel function mode. The fixture constants (temporal.rs:35-141, 448-473) still author inline temporal-atom counts, so the source a reader sees is not the program that runs. | fsl-replay/src/spine/clause/tests/temporal.rs:619-627; fsl-replay/src/spine/clause/tests/temporal.rs:725-730 |
| FND-006 | low | The `if self.mode == CheckMode::Kernel { return Ok(()) }` in the Count arm is redundant. Under Kernel, `discharge()` already swallows every `CheckCause::Unproved` refusal, and `Range` is one. The `Sum` arm beside it has no such guard. | fsl-semantics/src/check/facts.rs:1696-1698 |
| FND-007 | low | `tc_743_the_separation_check_names_its_failing_step` calls the `#[test]` function `count_refusal_in_a_called_kernel_function_fails_the_domain_step()` directly, so the case runs twice and a failure is reported under two names. | fsl-replay/src/spine/clause/tests/witness.rs:2974 |

## Findings detail

- **FND-001.** On main, `CLAUSES` had `RefusedDomain` with the inline count. `tc_3121_unit_text` kept that line, replaced `direct_refusal` with `z(self.versionNumber)` and appended `function z`. With the entry deleted, the unit has `function z` and no `RefusedDomain`. **Fix:** in `tc_3121_unit_text`, push `invariant RefusedDomain using v on Config::ConfigVersion at current { forall(n in ints(sequence[0, z(self.versionNumber)]): n < 500) }` directly, and delete the dead `starts_with("invariant RefusedDomain ")` filter arm and the `direct_refusal` replace. Confidence high: traced to the `unwrap`.
- **FND-002.** FR-766's Notes say spine compile checks under Linked, and a function body checked Linked records the obligation. Proved `[0, 3]` is not inside `[0, 1]`. **Fix:** set `agg-count-narrow-2003.expected.json` to `{"verb":"check","exit_code":20,"category":"refusal","last_stage":"S4","code":"undefined_expression","cause":"unproved-range"}` and `r-count-narrow` to the same check-stage refusal, and drop `flips_with`. If a run-time `integer_out_of_domain` corpus row is still wanted, add it as a separate Kernel-route case. Confidence high.
- **FND-003.** The file's own `DIVISION` comment records the precedent: a clause that Linked refuses is kept out of the shared unit. **Fix:** move `Refused` out of `unit()` into an `extra` passed only by `a_refused_capture_is_reported_at_the_capture`. Then assert that test's check-time `undefined_expression`/`unproved-range`, or route it through a harness with `refusingCount` under Kernel. Confidence medium: this is a CLI run, so the Linked spine path is inferred from FR-766's Notes, not traced through `fsl-command`.
- **FND-004.** **Fix:** compile `event_unit` through the spine entry with function modes `{refusingCount: Kernel}`, the way `compose_in_mode` is used in witness.rs, and delete the `text.contains("count<Zero>")` branch. If no such analyze-side entry exists, check under Linked with only that function's mode overridden.
- **FND-005.** **Fix:** edit the fixture constants to call `refusingCount(..)` and declare it once, then delete the rewrite loop and the byte sniff, and set the mode from the fixture kind.
- **FND-006.** **Fix:** delete the guard, so Kernel behaviour comes from `discharge()` alone, as it does for `Sum`.
- **FND-007.** **Fix:** drop the call from `tc_743`, or turn the body into a non-test helper that both tests call.

## Findings data

```yaml
clean: false
scope:
  - {id: FR-766, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "Under `CheckMode::Linked` the checker SHALL prove that interval lies inside `N`'s domain, and SHALL refuse the `count` `undefined_expression`/`unproved-range` when it cannot"}
  - {id: FR-766-AC-7, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "A clause-run report whose disposition is an S6a kernel refusal record `integer_out_of_domain`/`outside-domain` projects to a run item with category `refusal`, result `declined` and cause `{code: integer_out_of_domain, kind: outside-domain}`, never a `null` cause."}
  - {id: FR-268-AC-3, path: "spec/functional/FR-268-check-a-state-clause-witness-on-replay.md", role: context_only, excerpt: "refused separating-witness domain through a called function z under CheckMode::Kernel"}
findings:
  - {fnd: FND-001, severity: high, confidence: high, method: code-review, check: code-bug, artifact_id: FR-268-AC-3, related_ids: [FR-766], path: fsl-replay/src/spine/clause/tests/witness.rs, lines: "144-164"}
  - {fnd: FND-002, severity: high, confidence: high, method: code-review, check: code-bug, artifact_id: FR-766, related_ids: [], path: tests/corpus/collections/agg-count-narrow-2003.expected.json, lines: "1"}
  - {fnd: FND-003, severity: high, confidence: medium, method: code-review, check: code-bug, artifact_id: FR-766, related_ids: [], path: tests/it/temporal_capture_run.rs, lines: "38-39"}
  - {fnd: FND-004, severity: medium, confidence: medium, method: code-review, check: test-intent, artifact_id: FR-766, related_ids: [FR-495], path: fsl-analyze/tests/it/monitor_inputs.rs, lines: "173-205"}
  - {fnd: FND-005, severity: low, confidence: high, method: rust-review, check: test-intent, artifact_id: FR-766, related_ids: [], path: fsl-replay/src/spine/clause/tests/temporal.rs, lines: "619-627"}
  - {fnd: FND-006, severity: low, confidence: high, method: rust-review, check: other, artifact_id: FR-766, related_ids: [FR-495], path: fsl-semantics/src/check/facts.rs, lines: "1696-1698"}
  - {fnd: FND-007, severity: low, confidence: high, method: rust-review, check: other, artifact_id: FR-268-AC-3, related_ids: [], path: fsl-replay/src/spine/clause/tests/witness.rs, lines: "2974"}
```

## Dispositions

Disposition pass 1, against head `09b834b010` (fix commits `2211d3844f`, `09b834b010`; branch base `3c7b98bebb`). Static re-check of the fix diff `791571fa4c..09b834b010`; no build or test was run.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `tc_3121_unit_text` now pushes `invariant RefusedDomain ... z(self.versionNumber) ...` itself. The dead `RefusedDomain` filter arm and the `direct_refusal` replace are gone. |
| FND-002 | fixed | Both rows are now exit 20, category `refusal`, `undefined_expression`/`unproved-range`, last_stage `S3`, with no `flips_with`. S3 is correct and my S4 was wrong: 32 other check-stage `unproved-range` rows use S3, and run rows refused at check also report S3. `r-count-narrow` correctly stays `verb: run`, because the row exercises a run request that now refuses at compile. |
| FND-003 | fixed | `Refused`, `type Zero` and `ints` moved out of `unit()` into `REFUSED`, which only `a_refused_capture_is_reported_at_the_capture` passes through `run_with`. That test asserts a compile-stage `undefined_expression`/`unproved-range` refusal. The FR-860-AC-4 conflict this exposes is SR-2026112105500003 FND-006. |
| FND-004 | fixed | `compile_with` now compiles through spine with `check_with_execution_points_and_source_inputs`, passing `dependencies`, keeping libraries, and setting `check_mode: Some(Kernel)`. The `SourceInputs.check_mode` doc (lifecycle.rs:1987-1990) says declared clauses stay Linked under it. Function bodies are the only part that goes Kernel, and the fixtures' only functions are `ints` and `ok`, which carry no obligation, plus `refusingCount`. So in effect only the called function is Kernel, which meets FR-766 Notes. Adding a per-function field to `SourceInputs` is not needed. |
| FND-005 | fixed | The fixture constants now write `refusingCount(..)`, and the string rewrite is gone, so the source shown is the program run apart from one appended helper declaration. The conditional append and the `refusingCount(` byte check that remain are accepted as test plumbing. A smaller optional tidy-up is to declare `refusingCount` in the fixture `concat!` strings that use it and set the mode from the `Unit` kind. |
| FND-006 | fixed | The Kernel early return in the Count arm is deleted. |
| FND-007 | fixed | The body is now the helper `assert_count_refusal_fails_the_domain_step`, called by both `tc_743` and the `#[test]`. |
