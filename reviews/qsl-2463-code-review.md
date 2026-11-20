---
id: SR-2026112007590400000001
title: "Code review of QSL-2463 PR #3160 source-object workloads at S3"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@46bad2ca178dc0150936f3d4b0fe1245d0ad676f; git diff origin/main...HEAD (PR 3160); fsl-semantics/src/check/probabilistic/model.rs, fsl-package/tests/source_probabilistic_emission.rs; QSL-2463"
review_set: subset
date: 2026-11-20
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: reviews
---

# Code review of QSL-2463 PR #3160

## Summary

Ticket: QSL-2463 (partial: the S3 half only). Code review with the rust-review lane, over the two
files the PR changes. With this PR, `workload W on <SourceObject>` resolves the on-name as a model alias first,
then as an object type of the source model (alias `""`). The workload then weighs that one type's
operations.

Evidence, from a throwaway worktree with its own target dir under /home:

- `cargo-slot cargo test -p fsl-package --test source_probabilistic_emission -- --test-threads=1 --include-ignored`: both new tests pass. The two re-keyed ignored tests now pass S3 (they include `workload W on O`) and fail only at S4 with `ProbabilisticModelNotEmitted`, as the PR says. `a_delay_on_an_untimed_source_model_is_refused_at_its_span` (still keyed QSL-2439, which is Done) fails with `invalid_timed_model`, not the expected `invalid_probabilistic_model`. That file region is not in the diff.
- `cargo-slot cargo test -p fsl-package --lib -- workload` (house skips): 9 passed. The pinned `a_delay_on_a_timed_source_model_refuses_emission_with_its_locus`: passed.
- A local probe, never pushed and deleted afterwards. The QSL-2045 repro shape: a source `Switch` with `choose(bit: Boolean)` and `clear()`, `random Switch::choose(bit) ~ { true: 1, false: 1 }`, a reward, and `workload Input on Switch { weight Switch::choose = 1; weight Switch::clear = 0; }`. **S3 now passes** and records weights 1 and 0. S4 then refuses `ProbabilisticModelNotEmitted { RandomParameter, "choose(bit)" }` with its locus, which is the expected pending S4 half. An `Int[0, 1]` variant gives the same result. Duplicate weights, negative weights, all-zero weights, a weight on an unknown operation and a weight on a foreign object type all behave as on a selected model. Inheritance does not: see FND-001.
- `cargo-slot cargo clippy -p fsl-package -p fsl-command --all-targets` with `RUSTFLAGS='--cap-lints warn'`: no warning on a changed line. The model.rs warnings at lines 963, 1194, 1213, 1305, 1368, 1371 and 1381 are outside the diff hunks.

## Verdict

The lookup order follows FR-185 "Workloads": the on-name is resolved as a model alias first and, failing
that, as a source object type. Shadowing cannot happen, because FR-569 refuses a source object type
named like a model alias (`ambiguous_declaration`/`ambiguous-name`). Alias-first is therefore safe and
never silently picks one of two. An on-name that is neither stays `missing_declaration`/`missing-name`.
The loci are correct: unknown-operation is reported at the operation span, incomplete-workload at the
workload span, and missing-name at the on-name span. The PR changes no pub API, adds no `#[allow]` and
adds no panic or unsafe code. S4 refuses a source workload with its locus (probed), so no workload is
emitted without its node. The pinned delay refusal test is still correct at this pin.

Not mergeable as is: FND-001 is a real "same rules" violation. FND-002 and FND-003 are test-quality
findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A workload on a source subtype cannot weigh an inherited operation. With `object Base { operation a() }` and `object Sub extends Base { operation b() }`, `workload W on Sub { weight Sub::b = 1; weight Sub::a = 1; }` is refused `unknown-operation` on `Sub::a`, and so is the spelling `Base::a` (both reproduced). `{ weight Sub::b = 1; }` is admitted with no incomplete-workload refusal. The weight filter compares `operation.declaring_key` (here Base's key) with `only` (Sub's key), and `model_operations` lists only `declared_by(Sub)`. So inherited operations can be neither weighted nor required. Elsewhere `Sub::a` names an operation of Sub (lineage resolution, and the "lineage" in the `workload_model` doc comment), so this breaks FR-185's "same rules as on an operation of a selected model". On a selected model, a workload covers every inherited operation | fsl-semantics/src/check/probabilistic/model.rs:1483-1490; fsl-semantics/src/check/probabilistic/model.rs:858-883 |
| FND-002 | low | The new tests check only part of the workload content. The admission test checks `weights().len() == 2` and `model_key().is_some()`, but not the recorded weights (1 and 0: the zero-weight semantics the QSL-2045 repro stresses), not which operations they belong to, and not that `model_key` is Switch's key. In the refusal test, the foreign-object case checks only the code `invalid_probabilistic_model`, not the cause `unknown-operation` or that `Other::poke` is named. The incomplete-workload case does not check that `Switch::clear` is the omitted operation named. No test covers inheritance (FND-001) | fsl-package/tests/source_probabilistic_emission.rs:237-277 |
| FND-003 | low | The doc comment on `ProbabilisticCause::UnresolvedModel` ("A workload on a name no `model` declaration binds") was not updated. After this PR the cause also means "and no source object type" | fsl-semantics/src/check/probabilistic.rs:302-303 |
