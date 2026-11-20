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

## New findings (disposition pass 1)

Re-checked at head 00ad252b24f14ee7437a7a0ac0b7932755074acf, with a local probe that was never pushed. Edge cases that behave correctly: three levels (`workload on C` requires `A::a`, and leaving it out gives `incomplete-workload A::a`); a mid-level `workload on B` refuses `C::c` as `unknown-operation`; a diamond counts `A::a` once, and `B::a` plus `C::a` is `duplicate-weight`; `Sub::a` plus `Base::a` is `duplicate-weight`; a same-named operation on an unrelated sibling is `unknown-operation`; on a join type `D::x` is ambiguous but `B::x` and `C::x` written out are admitted; `workload on Base { Base::a }` checks.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | A workload on a source subtype that redefines an inherited operation is made to weigh the overridden ancestor operation as well. With `object Base { operation f(p: Int[0, 9]): Integer; }` and `object Sub extends Base { operation f(p: Int[0, 9]): Integer redefines Base::f; }`, `workload W on Sub { weight Sub::f = 1; }` is refused `incomplete-workload` naming `Base::f`. Adding `weight Base::f = 1` makes it pass, and two weights named `f` are recorded. The cause: `model_operations` collects `declared_by` for every type in Sub's lineage, so the redefined `Base::f` stays in the list beside `Sub::f`. On a Sub-only workload `Base::f` is not an operation any Sub instance runs, so the user has to write a weight that means nothing, and S4 will later emit it. A `modifies` operation gives the same result | fsl-semantics/src/check/probabilistic/model.rs:868-890 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `in_lineage` now gates both the weight filter and `model_operations`. Inherited operations are weighed and required under both spellings (`Sub::a`, `Base::a`). The new test `a_workload_on_a_source_subtype_weighs_and_requires_its_inherited_operations` covers this. Redefinitions are still open as FND-004 |
| FND-002 | fixed | The tests now check the weights `1/1` and `0/1` with their operations, that `model_key` equals Switch's key, `UnknownOperation{Other::poke}`, `IncompleteWorkload{Switch::clear}`, `UnresolvedModel{Missing}`, and inheritance |
| FND-003 | fixed | The `UnresolvedModel` doc comment now reads "neither a `model` alias of the unit nor an object type of its source model" |
| FND-004 | fixed | For a source-object workload, `model_operations` now walks `operations().effective(..)`, so a redefinition hides the ancestor op: `Sub::f` is weighed once, and a hidden `Base::f` is `unknown-operation`. The new test `a_workload_on_a_source_subtype_weighs_a_redefined_operation_once` covers both the pure and the `modifies` forms. Ambiguous names regressed and are tracked as FND-005 |
| FND-005 | fixed | Checked at head 6c14e661853010bcb137f1c01c9eeee698762658. `model_operations` collects the declarations of the target type and its ancestors, and drops an ancestor op only when a more derived reachable type declares the same name. On the join type, `D::y` plus `B::x` plus `C::x` is admitted; `D::y` alone is refused `incomplete-workload` naming both `B::x` and `C::x`; `D::x` is `AmbiguousOperation`. Also probed and correct: a redefinition in one parent of a diamond (`D::f` resolves to `B::f`, `A::f` is unknown, a workload on C keeps `A::f`); a redefinition in one parent of a join where the other parent keeps its own `x` (both `B::x` and `C::x` are required, `A::x` is unknown); the diamond's `A::a` is required once (`B::a` plus `C::a` is `duplicate-weight`); and the earlier three-level, redefinition and sibling probes give the same results as before |
| FND-006 | fixed | `weighable` is computed once, before the weights loop |

## New findings (disposition pass 2)

Re-checked at head 954182ff36b0f82c520adca80f91493e9dfb933d, with a local probe that was never pushed. These behave correctly: three levels, including the `A::a` spelling; a mid-level workload refuses the subtype's `C::c`; a diamond (`A::a` counted once, `B::a` plus `C::a` is `duplicate-weight`, leaving it out is `incomplete-workload`); `Sub::a` plus `Base::a` is a duplicate; the `Base::a` spelling on Sub; `workload on Base` with `Base::a` only, refusing `Sub::b`; an unrelated sibling's same-named op; a redefinition on Sub, both `modifies` and query (`Sub::f` once, a hidden `Base::f` is `unknown-operation`); `workload on Base` over a redefined op takes `Base::f`, and `Sub::f` is unknown; a three-level redefinition keeps `B::f` and hides `A::f`. The earlier inherited-op test still passes.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | Regression from round 1. A workload on a source type that inherits same-named operations from two unrelated parents now drops both of them silently. With `object B { operation x() }`, `object C { operation x() }` and `object D extends B, C { operation y() }`, `workload W on D { weight D::y = 1; }` is admitted with no `incomplete-workload`. Writing `weight B::x = 1; weight C::x = 1;` (accepted in round 1) is now refused `unknown-operation` for both. The cause: `OperationTable::effective` leaves out every name whose resolution is ambiguous ("A name whose declarations are ambiguous has no effective operation"). FR-104 refuses that ambiguity at a use site (`D::x`), not at the type, so D is admitted and its instances still carry both ancestors' `x`. On a selected model, both `B::x` and `C::x` are required. Possible fix: list every nearest declaration of an ambiguous name (each with its declaring type) so they are required and weighable by their `B::`/`C::` spelling, or else refuse the workload | fsl-semantics/src/check/probabilistic/model.rs:870-895; fsl-semantics/src/model/operation.rs:171-199 |
| FND-006 | low | `model_operations(model, Some(only))`, which recomputes the whole effective view (`effective` resolves every name over the whole operation table), is now called once per written weight inside the weight loop. That makes it O(weights × names × table) where a single call before the loop would do. It is correct, but it is unmetered work in S3 | fsl-semantics/src/check/probabilistic/model.rs:1506-1513 |
