---
id: SR-2026112010084800000001
title: "Code review of QSL-2463 PR #3171 S4 source-owned probabilistic emission"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@16f5e64e77f9367e718b223185f0665b4853c8a4; git diff 0208154ee1...HEAD (PR 3171); fsl-semantics/src/model/key.rs, fsl-semantics/src/check/probabilistic/node_key.rs, fsl-semantics/src/check/lowering/probabilistic.rs, fsl-package/src/emit/probabilistic.rs, fsl-package/src/emit/tests/probabilistic.rs, fsl-package/tests/source_probabilistic_emission.rs; QSL-2463, QSL-2699"
review_set: subset
date: 2026-11-20
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-740
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: reviews
---

# Code review of QSL-2463 PR #3171

## Summary

Ticket: QSL-2463 (S4 half), with QSL-2699 partial. This is a code review with the rust-review lane, over the six files the PR changes.
The work ran in a throwaway worktree at the head, with its own target dir under /home. The probe test file was never pushed and was deleted afterwards.

Evidence:

- `cargo-slot cargo test -p fsl-package --test source_probabilistic_emission -- --include-ignored` (house skips): 9 passed. The 2 ignored tests fail as documented: `a_source_operation_entry_lists_its_parameters` fails with MalformedClockPlace, and `a_delay_beside_an_untimed_source_clock_is_refused_as_untimed` fails with invalid_timed_model.
- `-p fsl-package --lib -- probabilistic workload delay claim source_objects`: 89 passed and 1 failed, `fixtures_s_s_r_and_s_p_emit_source_objects` (S-P: SourceMapInvariant). On base 0208154ee1 that test fails the same way, so the PR's claim holds. QSL-2678 tracks it.
- `-p fsl-semantics --lib -- probabilistic`: 4 passed.
- `clippy -p fsl-package -p fsl-command --all-targets` with `RUSTFLAGS='--cap-lints warn'`: no warning on a changed line. The warnings in lowering/probabilistic.rs (lines 57, 110, ...) and emit/tests/probabilistic.rs (lines 1111, 1220, 2787) are outside the hunks.
- Probe results, each run through S3, S4, the strict read and the identity-row join:
  - Two source types, each with a delay (uniform, exponential, discrete), a random parameter (Int and Boolean) and a workload, timed on A and then on B: ADMITTED. Every `declaring`/`model` reference equals that type's source node id `{package: source-probabilistic, path: [A], ordinal: 0}`.
  - The QSL-2045 Boolean `Switch` with a reward and a workload: ADMITTED. The support is `true*1,false*3` and the weights are `choose*1,clear*0`.
  - A lineage workload on `Sub` (inherited `Base::a`): ADMITTED. The row's declaring node is Base.
  - A delay on an inherited op of a timed subtype: ADMITTED.
  - A redefinition workload: ADMITTED.
  - A diamond workload (`D extends B, C`, both declaring `x`): STRICT READ REFUSED. See FND-001.
  - A timed source with a `Clock` field: STRICT READ REFUSED with MalformedClockPlace. It is refused with a delay, with only a random parameter and workload, and with no probabilistic declaration at all, so the refusal does not come from this PR. QSL-2347 seam 2 tracks it.
  - A tick time source with a delay: S3 refuses invalid_probabilistic_model/delay-on-untimed-model.
  - Two time sources: S3 refuses invalid_timed_model/duplicate-time-source.
  - So `source_dense_unit` never meets zero or several dense sources after S3. Its `all(equal)` check fails closed with a located S4 refusal.

## Verdict

The identity work is correct. `declaring_node_key` mints `{unit package, [Type], 0}` for a `source/` key and the `model/object_type` key otherwise. These match the source node lowering writes, as the probes and the un-ignored tests show. A domain path always starts `model, object_type` and `model` is a reserved word, so no source type key can collide with a model node key. The probabilistic keys embed the declaring node as `PathSegment::Node`, so they stay injective across packages, and all ordinals are 0 under content-keyed paths. The new function is additive public API. The diff adds no `#[allow]`.

The removed pinned-refusal test was justified: a delay on a clock-free timed source model now emits. With a Clock field, the strict read refuses the whole package whatever its declarations, so no delay is silently dropped or mis-emitted.

There is one real defect. It is FND-001: a diamond workload that S3 admits now yields an unreadable package with no source locus, where S4 used to refuse it with a locus. The other two findings are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A workload on a source subtype with an ambiguous ancestor operation, such as `object D extends B, C` where B and C both declare `x` and the weights are `D::y = 1; B::x = 1; C::x = 3;`, passes S3 (as #3160 requires both rows) and S4 emits rows `[B] x`, `[C] x`, `[D] y`. The strict read then refuses `invalid_package`/InvalidValue at `/semantic_graph/nodes/8/body/members/2/value/members/1` with `source_region: None`. The cause is contract-ir 4c91eff `probabilistic_join.rs` `source_workload`: `named.contains(&name)` refuses a repeated operation name (FSpec FR-640 Reader rule 3 "repeat"). Before this PR, S4 refused every source workload with its locus. Now this S3-admitted unit gives an unlocated read refusal. No ticket tracks the FSL S3 / IR rule 3 mismatch. Fix: S4 refuses a workload whose rows repeat an operation name, as `ProbabilisticModelNotEmitted{Workload}` at the workload span. Then file a ticket for FSpec FR-640 rule 3 / IR to key repeats by (declaring, name). | fsl-package/src/emit/probabilistic.rs:296-340; fsl-package/tests/source_probabilistic_emission.rs:404-440 |
| FND-002 | low | `declaring_node_key` re-implements the source recipe of `Lowering::model_node_key` (`strip_prefix("source/")`, `type_identity_segment`, `declared_node_key(.., vec![name], 0)`) in a second place. The emitter's lookup and lowering's node creation now depend on two copies staying identical. Extract one helper that both call. | fsl-semantics/src/model/key.rs:124-136; fsl-semantics/src/check/lowering/model.rs:1440-1457 |
| FND-003 | low | `a_boolean_source_switch_with_a_workload_and_a_reward_emits` asserts only the node counts and the declaring references. It does not check the Boolean support (`true: 1, false: 3`) or the weights (`choose 1`, `clear 0`), and the QSL-2045 repro asks for the zero clear weight to survive. A body that dropped or reordered the weights would still pass. The delay test's "typed by the unit node of `ms`" (FR-740-AC-5) is likewise checked only by substrings `"1"` and `"3"` in the body. | fsl-package/tests/source_probabilistic_emission.rs:466-487; :161-166 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | A delay, random parameter or workload on an object type that the second unit of a multi-unit package declares passes S3 but is refused at S4. The refusal has its locus, but FR-740 says S4 SHALL emit each checked declaration whatever the object type's origin. Reproduced at head 7c4b133d9a with `PackageDeclarations::assemble_units`. Unit a declares `time A dense unit ms`. Unit b declares `time B dense unit s` and then `delay C::poke ~ uniform`, `delay B::u ~ uniform` (B is its own timed root), or `random B::u(k) ~ { 1: 1, 3: 3 }` with a workload. Each of these gets `ProbabilisticModelNotEmitted` (for example, "delay `u` cannot be written to a checked-package/v2 graph yet"). The single-unit forms of the same declarations emit. A likely cause, not verified: the second source model's identity is `source/pkg/1`, so `source_object_node_key` mints the node package `pkg/1`, which no emitted node carries. Separately, `source_dense_unit` gathers the timed roots of every source owner, not only the declaring type's own, so two units with different units can never agree. Fix: key a non-first source model's declaring node as its own lowering does, and restrict `source_dense_unit` to the declaring type's owner. Add a two-unit emission test. | fsl-semantics/src/model/key.rs:138-146; fsl-package/src/emit/probabilistic.rs:396-413; fsl-semantics/src/check/assemble.rs:4561-4565 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified at head 7c4b133d9a. `workload_node` now refuses rows that repeat an operation name, located at the workload span and citing QSL-2707 (which exists, state Specced). The diamond probe now gets "workload `W` cannot be written to a checked-package/v2 graph yet" in place of the unlocated strict-read refusal. The new test `a_workload_weighing_two_operations_of_one_name_is_refused_at_its_span` asserts the declaration name, the feature, the label region in the unit text and the message. |
| FND-002 | fixed | `source_object_node_key` (pub(crate), key.rs) is now the one recipe. `declaring_node_key` and `AdmittedModel::model_node_key` (lowering/model.rs:1446) both call it. |
| FND-003 | still-open | The delay half is fixed: the discrete delays are now matched as (magnitude, type) pairs against the `ms` unit node id. The Boolean half is not. The new support and weight assertions read S3's `package.graph().probabilistic_model()`, which emission cannot change, so an emitted `random_parameter`/`workload` body that dropped or reordered its support or weights would still pass. Assert on `random[0]["body"]` and `workload[0]["body"]` (or read them back) instead. |
| FND-003 | fixed | Verified at head 50d1cb9e2f. `a_boolean_source_switch_with_a_workload_and_a_reward_emits` now reads the emitted `random[0]["body"]` support as (value_kind, value, weight) and gets (boolean, true, "1") and (boolean, false, "3"). It reads the `workload[0]["body"]` rows as (declaring target, operation, weight) for `choose` 1 and `clear` 0. |
| FND-004 | fixed | Two-unit packages now emit. `the_declarations_of_each_unit_of_a_package_are_emitted_on_its_own_types` checks each unit's delay, random parameter and workload on its own type, with the quantities typed by `ms` and by `s`. My two-unit probe and `a_refusal_in_a_later_unit_is_located_in_that_unit` both pass. A same-named type in two units is still refused at assembly (`AmbiguousTypeName`), so the per-package source node keys `{pkg, [T], 0}` cannot collide. The merge brought regressions, which are FND-005 and FND-006. |
| FND-005 | fixed | Verified at a1973b0e9b. `ProbabilisticInput::visible` (over `AdmittedModel::declares` and `visible_in`, with each declaration's `probabilistic N` source owner) now gates `operation`, `workload_target`, claim fairness and workload coverage. My two-unit probe refuses all 8 cases with `missing_declaration`/`missing-name`: random, reward, workload and delay, each from unit b on A and from unit a on B. A weight on A's op in unit b's workload on B is refused `unknown-operation`. Each unit's own declarations, in two timed units, still emit. `a_declaration_on_another_units_source_type_is_refused` covers both directions. `release` is not covered and still leaks: see FND-008. |
| FND-006 | fixed | `region_of` now writes `source_artifact(region.source())`. Unit b's delay, random, reward and workload `source_map` entries name test:second at unit b's offsets (287, 329, 362, 385), and unit a's name test:first. The new source-map test asserts the second unit's delay region. Timed-model nodes are a separate defect: FND-009. |
| FND-007 | fixed | `source_package_name` is removed. `AdmittedModel::of_package` stores the admitted package name (assemble.rs:4689), and `model_node_key` reads it. `a_package_name_ending_in_a_number_keys_its_source_types_under_that_name` (`acme/v/2`, two units) passes. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | The `type_tables` merge per alias (check/mod.rs:8583-8589) makes one unit's source types resolvable from another unit's probabilistic declarations. FR-569 scopes a source object type to the unit that declares it. Reproduced in a two-unit package, unit a = `object A { .. s() .. }`. At 7c4b133d9a, unit b's `reward cost on A::s = 1;`, `random A::t(k) ~ ..` and `workload W on A { .. }` were each refused `missing_declaration`/`missing-name`. At 50d1cb9e2f, S3 admits all three. The random parameter and the workload emit on A's node. The reward emits too, but the strict read then refuses `invalid_package`/InvalidValue at `/semantic_graph/nodes/8/body/members/4/value/target` with no locus. The reverse, unit a naming unit b's `B`, was already admitted at S3 before this round (the last unit's table was visible) and now emits. Fix: resolve a probabilistic declaration's source type through the writing unit's own source model, not a package-wide union. Then refuse the cross-unit names with `missing-name` and add a two-unit test for each direction. | fsl-semantics/src/check/mod.rs:8583-8589 |
| FND-006 | medium | In a two-unit package, the second unit's probabilistic nodes get `source_map` regions under the first unit's source. Reproduced with FIRST_UNIT/SECOND_UNIT: the `delay_distribution`, `random_parameter` and `workload` nodes on B carry `{"identity":"test:first"}` with test:second offsets (256-297, 298-330, 331-368). `region_of` (emit/probabilistic.rs:203-214) writes `source_artifact(graph.source())` with the offsets of `graph.region(location)`, so it drops the region's own source. B's own declaration node is correctly located at test:second. The strict read does not catch this. Fix: build the `CheckedSourceRegion` from the resolved region's source, and assert the source of a second-unit node's `source_map` entry in a test. | fsl-package/src/emit/probabilistic.rs:203-214 |
| FND-007 | low | `source_package_name` (key.rs) recovers the package name from `source/<name>[/<position>]` by dropping a trailing all-digit segment. A first unit of a package whose name ends in an all-digit segment, such as `acme/v/2`, has identity `source/acme/v/2`. That identity is read as package `acme/v`, so lowering's `model_node_key` and the emitter's `declaring_node_key(.., graph.package_name())` disagree. Lowering could take the checked package's name the way the emitter does, so both callers share one value. Not reproduced. | fsl-semantics/src/model/key.rs:139-152; fsl-semantics/src/check/lowering/model.rs:1446 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | `release` now resolves another unit's source operation; this regresses from main. The two-unit probe has unit a = `object A { .. t() .. }` and unit b = `object B ..; taskset Ctl { task a .. }; release Ctl.a on A::t;`. On main 1c9b02db90 and at 7c4b133d9a it is refused `missing_declaration`/`missing-name` (`UnresolvedReleaseOperation` "A::t"). At a1973b0e9b it is admitted. `check_releases` (task_set/release.rs:97) calls `input.operation(.., None, None)`, so `visible` returns true and the per-alias merged table resolves A. This is the FR-569 unit-scoping class of FND-005; FR-569 refuses another unit's source type in a clause, role or `over`. Fix: pass the release's own unit to `visible`, and add a two-unit release test in both directions. (State clauses are not affected: `pre Q using v on A::t` from unit b is still refused at assembly, `UnresolvedTypeName` "A".) | fsl-semantics/src/check/task_set/release.rs:97; fsl-semantics/src/check/probabilistic/model.rs:760-775 |
| FND-009 | medium | Two units that each declare a time member now emit, and the second unit's timed nodes are mis-located in the source map. On main and at 7c4b133d9a the strict read refused such a package (`TimedNodeReference` at `/semantic_graph/nodes/8/body/members/0/value/target`). At a1973b0e9b it is admitted, but unit b's `["B","Timed"]` node and its `timed_model`/`time_source` node carry source `test:first` with unit b's offsets (266). Unit a's sit at test:first@275. This is the same defect FND-006 fixed for probabilistic nodes, in the timed-model emitter's region code. Fix it in this PR, which newly lets these packages through, or file a ticket and refuse with a locus until then. | fsl-package/src/emit (timed model source map); probe `time A dense unit ms` (unit a) + `time B dense unit s` (unit b) |
| FND-010 | high | Commit a1973b0e9b adds 502 untracked temp files: 251 each under `tmp/.tmp7T5Oht/` and `tmp/.tmpAHrgeC/`, copies of `proposals/**` fixtures (checked-package-v2, filament-v1, planning-v1, backend-provider-v1, ...). They come to about 543k inserted lines, 519 files in `origin/main...HEAD`. They look like a test's TMPDIR written inside the worktree. `.gitignore` does not cover `tmp/`. Fix: drop them from the branch and ignore `/tmp/`. | tmp/.tmp7T5Oht/**; tmp/.tmpAHrgeC/**; .gitignore |
| FND-011 | low | The new `declares` took over the first doc line of `record_keys` ("The key of every declaration record of the package, of any kind."), so `declares` has a two-sentence doc that starts with the wrong sentence and `record_keys` has none. | fsl-semantics/src/check/lowering/model.rs:2472-2480 |
