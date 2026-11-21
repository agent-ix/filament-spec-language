---
id: SR-967175026302197768900474181941935084680
title: "QSL-2743 PR #3398 union producer accessor code-review (Rust lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@d3be81c5f503996212c1d5594f5b9a5d22746c4d; PR #3398; git diff origin/main...HEAD (base ba280846be, 14 files); fsl-semantics check/lowering, model intake/domain_package, fsl-package emit and monitor_plan, fsl-analyze tests/it/monitor_root.rs"
review_set: subset
---

## Summary

Ticket: QSL-2743. I reviewed the frozen PR #3398 diff: the S3/S4 model-union lowering retention, `VariantRecord.position` intake, the `model_union_rows` emit join, the `Emission` storage, the monitor-plan `carry_rows` rekey, and the public `Emission::selected_model_union_rows()` getter. I also reviewed the new monitor-root closure proof and its adverse control. Lanes run: code-review with the rust-review lane (idioms, error paths, integer conversions, resource bounds, test oracles). I made no code edits.

## Verdict

**CONDITIONAL** (one medium and two low findings). The producer is genuine. Rows come from the admitted checked composite (sealed members, canonical rank) and from the intake `VariantRecord` (authored position, original case identity). Nothing is built by the caller or from a graph display name. The emit join refuses a composite, member name or payload node that does not match with `SourceMapInvariant`. The monitor-plan rekey covers both the structural node and the Data payload nodes. The getter borrows retained rows, so it adds no owned storage. The urgent closure proof checks the closure against the actual IR join: rank/position/name/payload are (0,2,Armed,Clock), (1,1,Circle,Data) and (2,0,Idle,None). The IR join independently checks positions, original cases, settled key, Clock identity and Data original type against the package's selected model. The adverse control requires exactly `Exact(UnavailableIdentityWitness)`. FND-001 is the one defect in producer-fact authenticity: the Clock classification is inferred, not read. It fails closed at the IR join, but the row it emits makes a false claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The lowering classifies a payload as `Clock` whenever the checked union member has zero data positions, not when the case's alias-resolved `payloadType` is Clock. S2's `clock_place_type` also drops the payload of an alias whose Sequence or Map values are Clock, and intake admits Map-over-Clock value types (FR-230-AC-23). Such a case would be emitted as `Payload::Clock { payload_type: "ix://filament/native/Clock" }`, a false producer fact. The IR join would then refuse it as a mismatch instead of the producer raising a writer fault. | fsl-semantics/src/check/lowering/model.rs:2958-2970; fsl-semantics/src/check/assemble.rs:2282-2286,2843-2858; fsl-package/src/emit.rs:7917-7920 |
| FND-002 | low | A data payload with more than one checked position, and a case with no matching `VariantRecord`, are both reported as `KeyFault::UnknownDeclaration(<union declaration>)`. The declaration is known. The fault names the wrong cause, so a writer-invariant failure is reported as an unknown declaration. | fsl-semantics/src/check/lowering/model.rs:2944-2970 |
| FND-003 | low | Each union case looks up its `VariantRecord` with a linear `model.records.values().find_map` over every record of the admitted model. Lowering all unions is therefore O(total cases x model records), which grows quadratically with model size. One owner-keyed pass per union, or the model index, would avoid this. | fsl-semantics/src/check/lowering/model.rs:2944-2957 |

## Coverage

Examined (clean unless listed above):
- `SelectedModelUnion`, `SelectedModelUnionCase` and `SelectedModelUnionPayload` (public, with documented fields).
- `SemanticGraph::selected_model_unions` and `select_model_union`.
- The `model_record_declaration` admission of `Enum` records. Only composite handles reach `open_composite`, and payload-free model enums keep the enum-shape path.
- Recursion safety: `composites` is inserted before payload lowering.
- `model_union_rows`: emitted-node skip, composite/shape/length/name checks, settled key from the sealed member, `u64::try_from` rank/position, Clock identity matching the IR `CLOCK_IDENTITY`, and BTreeMap node-key order.
- `Emission` field, Debug and getter; `emit_timed_models` produces empty rows, which matches the existing enum-shape pattern.
- `RetainedRows` carry and `finish`.
- `fsl_package` re-exports.
- The intake `position` from `variants.iter().enumerate()`, before the canonical sort.
- The fixture `position` literals.
- The `variant_type` reorder oracle. It compares records modulo `position` only, asserts both authored position vectors, and keeps the enum member, key and export assertions. It is not weakened.
- The `monitor_root` shared helper. The enum assertions are unchanged; the only change is the shape count, `usize::from(!union)`.

Residual, not findings:
- Clippy for fsl-package never ran, because fsl-semantics failed first in untouched `check/check/typing.rs:2193/2197` and `check/synthesis.rs:148`. The new fsl-semantics code produced no Clippy diagnostics in those runs (logs qsl2743acc-a4/a5).
- Not exercised by a test: tuple-nested unions, alias payloads, and a union payload that is itself a model union.
- A Clock-bearing union under equality was not exercised.

Evidence read, not rerun: arch-test-run receipts qsl2743acc-a1 (1 passed), a2 (2 passed), a3 (30 passed) and a6 (15 passed). Their log mtimes are after the head commit, and the coder report states head d3be81c5f5. No full crate or workspace suite was run, and no repository-wide claim is made.
