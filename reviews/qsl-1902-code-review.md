---
id: SR-2026111413443800000001
title: "Code review of QSL-1902 named populations in role predicates"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR 2703 head 1d7d757e30c89e785624ad96eec60bf7b2552867; qsl-semantics/src/check/protocol_obligation.rs, qsl-semantics/src/check/protocol_slots.rs, qsl-analyze/tests/it/fill.rs, qsl-analyze/tests/it/protocol_roles.rs, qsl-analyze/tests/it/protocol_footprints.rs"
review_set: subset
---

## Summary

Ticket: QSL-1902. Code review with the rust-review lane, scoped to
`git diff origin/main...HEAD` (5 files, +206/-29). The review read the code
only. No build or test was run: the coder reports green suites for
qsl-semantics and qsl-analyze, plus clippy and four xtask lints.

`type_named_body` takes a new `Option<NamedScope>`. Both obligation call
sites pass `None`. In `protocol_slots.rs` the slot checker builds
`NamedScope::of(models, scope, unit)` only when `role_bound.is_some()`, and
that holds only for `SlotOwner::Role`, the role `until` slot built at
protocol_slots.rs:1475. Event constraints, `check` conditions, `finish`,
choice/visible/case guards, invariants, variants and related ends keep the
`None` path. That matches FR-208 "p" (the `until` reads model state as a
state clause does) and FR-391's list of population positions, which names no
other protocol slot.

The `MissingSegment { span: None }` branch now records `span_of(slot.form,
location)`. `DeclarationRegions::refusal_region` adds the segment offset to
that span (region.rs:431). When the location path resolves, the result is the
second segment, and the new test pins `absent`. The branch runs for every
slot, not only role `until`. For slots without a `NamedScope`, the only
`MissingSegment` source is a known population written in value position
(typing.rs:1312-1325), and the same span arithmetic is right there too. It is
not over-broad.

The evaluator needs no change, as the coder says. `evaluate_protocol_slot`
(qsl-eval/src/value/expression/mod.rs:459) passes an empty `reads` map and
`pre = None`. `objects_for` (evaluate.rs:3555) then falls through to
`self.objects`, the `current` environment. `named_members` and `named_lookup`
(evaluate.rs:3726, 3749) read through `objects_for`. The retire path
`role_steps` (qsl-eval/src/simulation/protocol/roles.rs:232-247) calls
`self.evaluate`, then `evaluate_in`, then `evaluate_protocol_slot`, over the
state's model observation. An incomplete population halts as a family refusal,
and that becomes an `Undecided` finding, not a crash.

The diff adds no string-comparison branch in source and no compatibility
layer. The new parameter is the smallest seam that reuses the state-clause
typer.

## Verdict

**PASS, mergeable**, with one low robustness finding. Checker scoping,
evaluator reuse and test helpers are correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new `MissingSegment` relocation takes `span_of(slot.form, &location)`, which falls back to the slot's root span, or to `Span{0,0}`, when the location path has no span node. The segment offset is then added to the wrong base. A root fallback mislocates the refusal inside the predicate. `{0,0}` gives start > end, so the region is `None` and drops the location-based region the `span: None` path would have resolved. `missing_segment_at` (typing.rs:163) filters empty spans and timed_model.rs:4644 clamps with `.min(name.end)`. This branch does neither | qsl-semantics/src/check/protocol_slots.rs:535-550, qsl-semantics/src/check/protocol_obligation.rs:1410-1415 |
