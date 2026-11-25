---
id: SR-3639005
title: "QSL-3104 PR 3875 code review: strict workspace clippy follow-up (all targets, all features)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@1f5e76ccd40585ca16528e4c9c344733472aff2a; git diff origin/main...HEAD (75 files, +1054/-860) in fsl-analyze, fsl-command, fsl-conformance, fsl-eval, fsl-package, fsl-replay"
review_set: subset
---

## Summary

Ticket: QSL-3104. PR #3875, head 1f5e76ccd40585ca16528e4c9c344733472aff2a, base main.
This review covers the whole diff (`git diff origin/main...HEAD`, 75 files). The Rust lane
(rust-review) ran in this same artifact. No cargo command was run, because the host is
memory-limited. The coder's strict clippy log for this head ends with `EXIT 0`.

What was checked:

- Each lint fix was read hunk by hunk for a behaviour change. The changes are context structs
  for long argument lists, type aliases, boxed variants, struct-update syntax, iterator
  idioms, removal of a never-looping `loop` in `fsl-eval/src/simulation/model/refine.rs`, and
  moving items above test modules.
- The `never_loop` rewrite was checked: the inner `continue`/`break` target the inner loop,
  and every path through the old outer body returned. `check.rs` `model_counterexample` and
  `concretise`, and `fair_summary.rs` `candidate_stutter_relation`, move with identical text.
- The `FairMdp` branch now reads `offer.side`, which is the same as `bound_side`
  (`document.rs:2154`). `Chooser::expand` was dead and is removed.
- Tests: the `#[test]` and `assert` counts are identical on main and head in every changed
  file. `units.rs` and `counter.rs` hold no `#[test]`, so removing `units` from the
  fsl-analyze lib test target drops no test. `units.rs` is still compiled by
  `tests/it/main.rs:220`, and it re-exports `unguarded`, `unread` and `Reads` from the new
  `letters.rs`. The `measure.rs` `partial_lower_tests` module moves with identical text.
  `.map(|line| {..; line})` becomes `.inspect(..)`, which is equivalent.
- Allows: no blanket allow was added. The three new `#[expect(dead_code, reason = ..)]` are
  each on one shared `#[path]` fixture module, and they fail if the code stops being dead.
  Four per-module `#[allow(dead_code)]` and two `#[allow(clippy::duplicate_mod)]` on
  `tc235_k2` were replaced by one shared module. One `#[allow(dead_code, unused_imports)]`
  was narrowed to `unused_imports`, and a duplicated `too_many_arguments` allow was removed.
- Public API changes:
  - `monitor_with_target_preparer` now takes a `MonitorPreparation`.
  - `TargetPreparer` now returns `Box<RunCause>`, unboxed at `monitor.rs:656`.
  - `Written::Applied` now holds `Box<ModelSimState>`.
  - The `probabilistic_wire*` functions now take a `ProbabilisticValueSchema`.

  For each change, the in-repo callers keep the same values. agent-ix consumers on
  origin/main were checked: filament-driver, filament-contract-codegen, filament-contract-ir,
  filament-integration, filament-analyze, filament-eval, filament-examples and
  filament-verification. None calls any of the changed functions. filament-driver calls only
  `command::monitor`, whose signature is unchanged, and it pins fsl-* at 86acfdee.
- rustfmt: each changed file was piped through `rustfmt --edition 2021` on main and on head.
  This PR makes 9 files unformatted that were formatted on main (FND-002).

## Verdict

CONDITIONAL. The lint fixes do not change behaviour, no test or assertion was removed or
weakened, and no blanket allow was added. FND-001 is a new public refusal path, added by the
API reshaping, that has no owning criterion and no test. FND-002 and FND-003 are low. Nothing
here blocks on behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `probabilistic_wire_with_enum_members` used to require an `EnumMemberIndex` by type. It now takes a `ProbabilisticValueSchema` whose `enum_members` is optional, and it adds a runtime refusal `Unspelled("enum member index")` when the index is absent. No FR/AC owns this refusal and no test reaches it, so a compile-time guarantee became an untested runtime error. Once the guard passes, the function is the same as `probabilistic_wire`, because `probabilistic_wire` (and `probabilistic_undefined_wire`) now also use enum members whenever the schema carries them. Failure scenario: a caller that builds the schema without `.with_enum_members` gets an "unspelled" counterexample wire error at run time instead of a compile error. | fsl-replay/src/outcome/counterexample.rs:36-84, fsl-replay/src/outcome/counterexample.rs:2084-2101, fsl-analyze/src/analyze/exact.rs:3965-3985 |
| FND-002 | low | Nine files that rustfmt left unchanged on main are unformatted at head: fsl-analyze/src/analyze/exact.rs, fsl-analyze/tests/it/example_units.rs, fsl-analyze/tests/it/model_check.rs, fsl-eval/src/simulation/model/frames.rs, fsl-eval/src/simulation/model/refine.rs, fsl-package/src/emit/tests/hyper_clause.rs, fsl-replay/src/outcome.rs, fsl-replay/src/outcome/counterexample.rs and fsl-replay/src/probability_certificate/document/refutation.rs. Seven files that were already unformatted gain more drift: measure.rs, undefined.rs, digital.rs, check.rs, search.rs, fsl-replay/src/lib.rs (an unsorted `ProbabilisticValueSchema` export) and mdp.rs. A later `cargo fmt` will churn these lines. | fsl-eval/src/simulation/model/frames.rs:1479-1489, fsl-replay/src/outcome/counterexample.rs:70-82, fsl-replay/src/lib.rs:469 |
| FND-003 | low | Moving `Reads`, `unread` and `unguarded!` from `units.rs` to the new `letters.rs` dropped their doc comments. These recorded why reaching `activation`/`activation_reads` is a defect (the engine reads them only for a clause with a `when` condition) and that only a run with partial-order reduction reads the footprint. | fsl-analyze/tests/it/letters.rs:1-29 |

## Coverage

- Files examined: all 75 in the diff, including the clean ones: fsl-command monitor/request_shape, fsl-conformance tests, the fsl-package emit and test fixtures, fsl-replay zone/limits/mdp/measure/refutation/value_bounds/bounded, the fsl-eval protocol boxing, and the fsl-analyze model_check/zone_check/exact_probabilistic modules.
- Gates not run (no cargo, by brief):
  - `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` (the coder's log says exit 0 on this head)
  - `cargo test --locked -p fsl-command --test it cli_contract::monitor`
  - `cargo test --locked -p fsl-analyze --test it package_probabilistic_refutation::source_enum_scheduler_evidence_encodes_with_the_package_member_index`
  - `cargo test --locked -p fsl-analyze --lib`
  - `cargo test --locked -p fsl-replay --lib partial_lower_tests`
  - `cargo test --locked -p fsl-package --lib emit::tests`
  - `cargo test --locked -p fsl-eval --lib simulation`
  - `cargo test --locked -p fsl-conformance --test it`
  - `cargo fmt --all --check` (fails on main as well, see FND-002)
