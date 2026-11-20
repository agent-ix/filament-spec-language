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
