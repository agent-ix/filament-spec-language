---
id: SR-2693001
title: "code review of FR-576 operation body build and carrier (PR 2693)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2693 diff origin/main...HEAD at ee377de43: Cargo.lock, qsl-package/src/emit.rs, qsl-package/src/emit/monitor_plan.rs, qsl-package/src/emit/tests/operation_bodies.rs, qsl-package/src/emit/tests/operation_effects.rs, qsl-replay/tests/dispatch_link_compile.rs, qsl-semantics/src/check/checked_dispatch.rs, qsl-semantics/src/check/dispatch_link.rs, qsl-semantics/src/check/mod.rs, src/population_fixtures.rs, tests/it/emit_checked_v2.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
---

## Summary

Ticket: QSL-1899. Code review with the Rust lane folded in, scoped to the
PR diff against `origin/main` (f32450ae9). The PR builds and checks the body
function of every operation with an `operation-body` whether or not a call
reaches it, carries one `operation_bodies` row per FR-496 key through the
checked graph, and writes the rows into the package bytes through the
quire-contract-model `OperationBody`/`OperationBodyRow` types at IR main
ad8f1891.

Focused tests run on the reviewed head: `cargo test -p qsl-package --lib --
emit::tests::operation_bodies emit::tests::operation_effects` (13 passed) and
`cargo test -p qsl-replay --test dispatch_link_compile -- uncalled` (2 passed).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-576-AC-10 is not implemented: an operation with an `operation-body` and no declared result compiles, and the whole `operation_bodies` member is silently withheld instead of refusing `ill_typed`/`type-mismatch` at the body | qsl-semantics/src/check/dispatch_link.rs:255-257 |
| FND-002 | medium | `build_uncalled_bodies` swallows every `body_function` and `with_spans` error with `continue`, so any failure yields a package with no `operation_bodies` member and no diagnostic | qsl-semantics/src/check/dispatch_link.rs:255-270 |
| FND-003 | medium | `emit_timed_models` and the monitor-plan `finish` re-encode the base package with `operation_bodies: None`, dropping rows the base carries, while carrying `operation_effects` from the same base; `CheckedPackageV2::operation_body_rows()` exists for this | qsl-package/src/emit.rs:5683 |
| FND-004 | low | The fixture and test re-encoders set `operation_bodies: None` where the read package's `operation_body_rows()` could be passed; the operation_effects test strips `operation_bodies` from its comparison instead of carrying it | src/population_fixtures.rs:452 |
| FND-005 | medium | `Emission::operation_body_rows` is an in-memory duplicate of the written member used only by tests, so no test reads `operation_bodies` from the package bytes and the write at the encoder call is unasserted | qsl-package/src/emit.rs:660 |
| FND-006 | low | Cargo.lock also moves ix-trace-rs, quire-exact and quire-verification-contracts to new main tips, which the quire-contract-model ad8f1891 bump does not require | Cargo.lock:939 |

### FND-001

`body_function` calls `require_signature`, which errors with
`MissingClauseField::Result` when the operation declares no result
(`clauses.result` is filled only when `declaration.result()` is `Some`,
dispatch_link.rs:367). `build_uncalled_bodies` then hits `let Ok(..) else {
continue }` and builds no function. `checked_model_operation_bodies`
(qsl-semantics/src/check/mod.rs:8025-8027) sees `has_body` with no function
and returns `None` for the whole member, and the compile succeeds. FR-576 on
this branch (merged by #2694) says `check` SHALL refuse
`ill_typed`/`type-mismatch` located at the body with no package, and the
emitter SHALL NOT withhold the member. The doc comment on
`CheckedOperationBody`/`checked_model_operation_bodies` still describes the
withholding behaviour, and the PR body still calls the case unspecified. No
test covers AC-10 or TC-1011 steps 5-7.

### FND-002

Apart from the resultless case, a missing parameter signature or a
`with_spans` failure is also dropped silently (dispatch_link.rs:255-257 and
268-270). The redundant second `self.operation_body(&key)` lookup
(line 265) is the same shape. A body that cannot be built should refuse the
compile, as the spec says every admitted body is built or the compile refuses.

### FND-003

`emit_timed_models` (emit.rs:5683) rebuilds the package from `base:
&CheckedPackageV2`. It rekeys only timed-model nodes and passes
`operation_effects` through unchanged, so passing `base.operation_body_rows()`
would carry the bodies. Monitor-plan `finish` (monitor_plan.rs:1310) also
re-encodes the base; it rekeys entry functions and renamed record values
through `changes`, so its rows need the same rekey `carry_rows` applies. Both
also set `Emission::operation_body_rows: None`. The result is that a
timed-model or monitor-plan emission over a package with model operations
loses every body, though the IR carrier is optional so nothing refuses. The
brief asked whether `None` at these sites is correct: at these two production
sites it is not.

### FND-004

`population_fixtures::retarget` changes only one parameter's `semantic_type`,
so `base.operation_body_rows()` passes through unchanged next to
`base.operation_effect_rows()`. `tests/it/emit_checked_v2.rs:450` (write,
read, write stability) can pass `read.operation_body_rows()`.
`emit/tests/operation_effects.rs:660` can pass `package.operation_body_rows()`,
and then the `remove("operation_bodies")` at line 693 is not needed.

### FND-005

Every row assertion in `emit/tests/operation_bodies.rs` reads
`emission.operation_body_rows()`, the in-memory field. No test reads the
`operation_bodies` member from the bytes (`raw_wire`/`wire`, or
`CheckedPackageV2::operation_body`). Changing `operation_bodies:
operation_body_rows.as_deref()` at emit.rs:6751 to `None` would leave every
test green. Now that the IR carries the member, the field and accessor
duplicate the package. Delete them and read the rows from the emitted package.

### FND-006

The IR's own lockfile at ad8f1891 still resolves ix-trace-rs at e29cc07 and
quire-verification-contracts at 5769cb8. The PR moves them to 3cd3285 and
1bf72e6, and moves quire-exact from 4fe7df6 to b81e595. These are `branch =
"main"` git deps, so the move is harmless if the final gate is green. It does
widen what this PR changes beyond the body carrier.

## Verdict

Not mergeable at ee377de43 because of FND-001. The spec on the branch
requires a refusal that the code does not raise, and the code withholds a
member the spec forbids withholding. What is sound: the shared `body_function`
helper makes the body node id independent of a call (AC-2 is asserted). The
uncalled body check produces the FR-302 refusals (qsl-replay tests). Rows sort
and key like `operation_effects`, with AC-7 asserting the same keys in the
same order. Aliases dedupe on document. The `emit_checked` path passes real
rows to the encoder. Using the IR `OperationBody`/`OperationBodyRow` types
instead of a local copy is correct and adds no compatibility layer. The PR
adds no pins, digests or file tracking.

## Scope and evidence

- FR-576 (spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md), examined: the Behavior bullets on every-body checking, the no-result refusal, `no_body`, the FR-494 identity and no partial member.
- FR-576-AC-10, examined: Operation `A.touch` declares no result type and has an `operation-body`: the compile refuses `ill_typed`/`type-mismatch` located at that body and emits no package.
- qsl-semantics/src/check/dispatch_link.rs `build_uncalled_bodies`, `link_dispatch` early-return removal, `body_arity_refusals` (`first_new` covers the uncalled functions), examined.
- qsl-semantics/src/check/checked_dispatch.rs `body_function` refactor, examined: behaviour-preserving for dispatch candidates.
- qsl-semantics/src/check/mod.rs `CheckedOperationBody`, `checked_model_operation_bodies`, examined.
- qsl-package/src/emit.rs `operation_body_rows`, `emit_package_inner`, `emit_timed_models`, examined.
- qsl-package/src/emit/monitor_plan.rs `finish`, examined.
- src/population_fixtures.rs `retarget`, tests/it/emit_checked_v2.rs `encode`, examined.
- quire-contract-model ad8f1891 `operation_bodies.rs` (`OperationBody`, `OperationBodyRow`, `operation_body_rows`, `validate`), context only.
- Cargo.lock, examined.

## Dispositions

Round 1, against `git diff c25623b81..81d4e9e93`. Focused tests on 81d4e9e93:
qsl-package operation_bodies, operation_effects and
retained_rows::the_writers_renames_reach_the_rows_it_carries (16 passed); and
qsl-replay dispatch_link_compile `body` (8 passed).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `build_uncalled_bodies` now builds the body clause of a resultless operation and refuses `ill_typed`/`type-mismatch` at `Origin::Body`. Asserted by `a_body_on_an_operation_without_a_result_refuses_type_mismatch` (qsl-package) and `a_body_on_an_operation_without_a_declared_result_refuses_type_mismatch` (qsl-replay), including the `no_body` and function-row variants. |
| FND-002 | fixed | `build_uncalled_bodies` returns `Vec<CheckRefusal>`; a missing view clause set, a `body_function`/`body_clause` error or a `with_spans` error now pushes `dispatch_link(invariant(key))`; the redundant second lookup is gone. |
| FND-003 | fixed | `emit_timed_models` passes `base.operation_body_rows()`; monitor-plan `finish` rekeys `declaring` and the body node through `changes` and re-sorts by key. Asserted by `the_timed_model_writer_carries_the_operation_bodies` and the extended `the_writers_renames_reach_the_rows_it_carries`. |
| FND-004 | fixed | `population_fixtures::retarget`, `tests/it/emit_checked_v2.rs` `encode` and `operation_effects::wire_without_member` pass the read package's `operation_body_rows()`; the `remove("operation_bodies")` strip is deleted. |
| FND-005 | fixed | The `Emission::operation_body_rows` field and accessor are deleted; tests read the member from the package bytes through `body_rows` (`raw_wire`). |
| FND-006 | accepted-no-change | The lead accepted the transitive `branch=main` lockfile moves unchanged: harmless, and the final gate decides. |

After excerpts:

- FND-001: `if !clauses.result.contains_key(&key) { ... functions.push((key, function, None)) ... }` then `refusals.push(CheckRefusal::ill_typed(&Location::root(Origin::Body { function: function.name.clone(), index }), IllTypedCause::TypeMismatch));`
- FND-002: `None => refusals.push(dispatch_link(invariant(key))),`
- FND-003: `operation_bodies: base.operation_body_rows(),` (emit.rs) and `rekey.id(&mut row.declaring); if let OperationBody::Function { node } = &mut row.body { rekey.id(node); }` (monitor_plan.rs)
- FND-004: `operation_bodies: base.operation_body_rows(),`
- FND-005: `pub(super) fn body_rows(emission: &Emission) -> Option<Vec<OperationBodyRow>> { let wire = raw_wire(emission); let member = wire.get("operation_bodies")?; ... }`
