---
id: SR-2026111911313600000006
title: "QSL-2251 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2251; origin/main...HEAD; changed Rust files and named Clock requirements"
review_set: subset
---

## Summary

Independent review of PR 3037, head 5407b07d0dbf2f0c92ef39dc237551a8d06e8b6f. Code-review includes the rust-review lane and rust-style. Findings are statically traced, not runtime reproductions. No repository code or tests changed.

## Verdict

CONDITIONAL. Do not merge until findings receive verified dispositions. Top-level Clock admission, zero data payload assembly and authored tuple data projection are implemented; remaining targeted checks are pending. No applicable AssuranceProfile found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The runtime test checks the count of live clocks across freely enumerated union data values, but has no guarded operation and no Armed(0 ms)/Idle postcondition. It would still pass if the guarded operation ran while Idle or the reset value/drop behavior were wrong. The checker test only counts resets, and the package test only checks paths after readback. Add runtime assertions for inactive-case guard rejection, activation/reset to zero and drop, plus alias/nested and nonterminal tuple paths; assert emitted origin/resolved-path associations rather than relying on trace labels. These AC portions are unverified, not established as implemented by a tagged matrix row. | fsl-eval/tests/it/timed_subject.rs:4391-4450 |

## Coverage

Plan completion: not assessed

Review is restricted to the requested diff and Clock acceptance criteria. No spec/plan edits: spec analyses skipped. Semantic intent/test-oracle review explicitly requested and performed directly without subagents. Each binding above covers a portion, not every conjunct of its AC. All 12 changed files examined, including conformance registration. No new dependency, vendored semantic framework, public API, unsafe code or CI-trigger change.

Computed quoin matrix exited 0: FR-230-AC-63 has five active test binders, no run evidence. Repository-wide static counts: 4328 tagged, 1191 untagged, 258 tagged-by-ignored-test, 104 method-without-symbol, 13 retired. These inherited repository gaps are outside this diff verdict and are not represented as new ticket defects. This is not a whole-repository assurance PASS. No plan selected.

Coder evidence: qsl2251-baseline.log executed zero tests, so establishes no baseline reproduction. qsl2251-clock-tests.log reports 22 passed, 1 failed, 1 ignored; failure is the Clock tuple/union fixture with generated origins. Current diff replaces origins with source spans. The exact-name rerun in qsl2251-tuple-clock.log now passes: 1 passed, 0 failed, 1632 filtered out; test clock_places::a_clock_tuple_position_and_union_case_are_clock_places. Package readback, conformance vectors and runtime tests remain pending. No reviewer cargo, aggregate gate or full-workspace test sweep run. git diff --check passed.

Static follow-through: tuple assembly removes Clock data slots and records projection; typing maps authored indices before TupleProject; Clock union assembly filters payload to arity zero; package emits tuple(index) and retains tuple member identity. Runtime terminal tuple Clock needs no data traversal; inactive top-level union liveness is checked by is_case. Alias resolution exists in clock_place_type/places_below, but alias behavior lacks new end-to-end regression coverage. Origins use tuple position identity; package test checks paths only. quire coverage exited 0; external FSpec-prefixed tags are outside this repository's minted criteria, as expected for cross-repository references, not treated as wrong bindings. Standard AC-399 also includes Set<Slot> and map-key rejection; neither is newly asserted here and no completeness claim is made for them.

## Examined units

```yaml
{
  "bindings": [
    {
      "ac_id": "FR-230-AC-63",
      "test_id": "clock_places::a_clock_tuple_position_and_union_case_are_clock_places",
      "trace": "correct"
    },
    {
      "ac_id": "FR-230-AC-63",
      "test_id": "timed_subject::a_model_union_clock_is_live_only_in_its_active_case",
      "trace": "correct"
    },
    {
      "ac_id": "FR-230-AC-63",
      "test_id": "timed_subject::a_model_tuple_clock_keeps_the_authored_data_position_at_run_time",
      "trace": "correct"
    },
    {
      "ac_id": "FSpec-FR-322-AC-399",
      "test_id": "model_tuple_and_union_clock_places_read_back",
      "trace": "correct"
    }
  ],
  "scope": [
    {
      "excerpt": "A selected timed document whose tuple `Slot` has position `p0` typed `ix://filament/native/Clock` and `p1` typed `Integer`, and whose variant type `Phase` has the cases `Armed` with `payloadType` `ix://filament/native/Clock` and `Idle` with none, is admitted (`TK-08` and `TK-09` admitted): a field `s: Slot` has the clock place whose path is `field(s)`, `tuple(0)` and a field `p: Phase` the clock place whose path is `field(p)`, `union(Armed)`, each with a `resolved_path` row per step ending at the `Clock` terminal; no node stands for either clock; `Phase::Armed` is a `union_value` with the empt",
      "id": "FSpec-FR-322-AC-399",
      "path": "/Users/peter/dev/filament-specification/spec/objects/interfaces/FR-322-checked-package-artifact.md",
      "role": "examined"
    },
    {
      "excerpt": "A variant case whose `payloadType` resolves to an admitted value type, such as `ix://filament/native/Boolean` or a declared tuple type, is accepted as the member's payload. A `payloadType` of `ix://filament/native/Clock`, or an alias of it, is accepted as a clock payload (FR-322 \"Clock positions and payloads\", FR-415). An unresolved reference or a reference to a type not admitted as a value type, such as `ix://filament/unit/ms` or a namespace, refuses `unsupported_construct`/`declaration-form` at `Variant.payloadType`, never `malformed-declaration`.",
      "id": "FSpec-FR-154-AC-25",
      "path": "/Users/peter/dev/filament-specification/spec/functional/type-model/FR-154-admit-domain-package-model.md",
      "role": "examined"
    },
    {
      "excerpt": "With tuple `Slot(Clock, Integer)` and `union Phase { Armed(Clock), Idle }` as fields `s` and `p`: `self.s.0 >= 2 ms` is a guard on the place `s.0`, which exists in every state; `present(self.p.Armed) and value(self.p.Armed) >= 1 ms` is a guard on the place `p.Armed`, false while `p` is `Idle`, and `value(self.p.Armed) >= 1 ms` alone refuses `undefined_expression`/`unproved-presence`; `self.p = Phase::Armed(0 ms)` resets `p.Armed` and makes it present, `self.p = Phase::Idle` drops it, and neither place holds a presence bit; `s1 = s2` over `Slot` and `p1 = p2` over `Phase` refuse `invalid_timed_",
      "id": "FSpec-FR-415-AC-28",
      "path": "/Users/peter/dev/filament-specification/spec/functional/temporal/FR-415-declare-clocks-and-timed-models.md",
      "role": "examined"
    },
    {
      "excerpt": "Under `time dense` with tuple `Slot(Clock, Integer)` and `union Phase { Armed(Clock), Idle }` as fields `s` and `p` of a domain package whose tuple position and case `payloadType` are `ix://filament/native/Clock`: the package is admitted; the checker lists the places `s.0` (unguarded) and `p.Armed` (guarded by `present(self.p.Armed)`); the guard `present(self.p.Armed) and value(self.p.Armed) >= 1 ms` checks to one convex part carrying that data predicate, and `value(self.p.Armed) >= 1 ms` alone refuses `undefined_expression`/`unproved-presence`; `self.p = Phase::Armed(0 ms)` records a reset of",
      "id": "FR-230-AC-63",
      "path": "/Users/peter/dev/qsl-2251-code/spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md",
      "role": "examined"
    },
    {
      "excerpt": "//! FR-231 (TC-686): the timed subject of ADR-026 §11's `Rpc` model, its",
      "id": "fsl-eval/tests/it/timed_subject.rs",
      "path": "fsl-eval/tests/it/timed_subject.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! FR-230-AC-49/AC-50 (TC-685 step 33), FSpec FR-322-AC-148..150: a dense",
      "id": "fsl-package/src/emit/tests/clock_places.rs",
      "path": "fsl-package/src/emit/tests/clock_places.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! The FR-091 forms assembler, the entry of E3 (ADR-011 §2.1 E3 owner: FSL",
      "id": "fsl-semantics/src/check/assemble.rs",
      "path": "fsl-semantics/src/check/assemble.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! Name resolution and typing of Complete-V1 value expressions and function",
      "id": "fsl-semantics/src/check/check.rs",
      "path": "fsl-semantics/src/check/check.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! The typing loop: [`Typer::check_as`] and [`Typer::infer`] type",
      "id": "fsl-semantics/src/check/check/typing.rs",
      "path": "fsl-semantics/src/check/check/typing.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! FR-094: model-owned nodes, the `Reference<T>` and",
      "id": "fsl-semantics/src/check/lowering/model.rs",
      "path": "fsl-semantics/src/check/lowering/model.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! FR-230, FSpec FR-322 \"Clause nodes\": lower the terms a timed model's",
      "id": "fsl-semantics/src/check/lowering/timed_clause.rs",
      "path": "fsl-semantics/src/check/lowering/timed_clause.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! FR-230: a timed state model's clocks, guards, resets, time invariants",
      "id": "fsl-semantics/src/check/timed_model.rs",
      "path": "fsl-semantics/src/check/timed_model.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! FR-154: lift a spec-bundle to Semantic IR 2.0.0 bytes, admit them as a",
      "id": "fsl-semantics/src/model/intake.rs",
      "path": "fsl-semantics/src/model/intake.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! FR-230 \"Places\": a clock below an object's field, through a package",
      "id": "fsl-semantics/tests/it/clock_places.rs",
      "path": "fsl-semantics/tests/it/clock_places.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! FR-056 \"Variant types\" (QSL-734): a variant type is admitted as one `enum`",
      "id": "fsl-semantics/tests/it/variant_type.rs",
      "path": "fsl-semantics/tests/it/variant_type.rs",
      "role": "examined"
    },
    {
      "excerpt": "//! The local FSpec vector gate. Path resolution and child environments live here,",
      "id": "xtask/src/conformance.rs",
      "path": "xtask/src/conformance.rs",
      "role": "examined"
    }
  ]
}
```
