---
id: SR-2026111922154500000002
title: "spec-review of QSL-2566 FR-531 function body locations in the bytes"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3114 at ba0e3547; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md (table row, 'Function body locations in the bytes', location spelling, FR-531-AC-36, FR-531-AC-37), spec/test-cases/TC-3133-function-body-losses-are-located-alike-on-both-routes.md; context: FSL origin/main 8db8981d (fsl-semantics check.rs float_ordering and dispatch_link.rs, fsl-package emit.rs and emit/protocol_locations.rs, reader_protocol_projection.rs, fsl-eval evaluate.rs record, fsl-eval/tests/it/ieee_ordering_route_parity.rs), FR-383, FR-397, FSpec PR #760"
review_set: subset
---

## Summary

Spec review of FSL PR #3114. Ticket: QSL-2566 (implementation: QSL-2625). FR-531 now says that a value loss or stop inside a function body is located at the checker's `Body{function,index}` location plus path, on both routes and never at a call site. S4 writes one FSpec `body_locations` row for each term of each body. The bare route reads its locations from these rows, with no fallback to structure or to source. The PR adds FR-531-AC-36/37 and TC-3133. It depends on FSpec #760 and should merge after it.

## Examined scope

- The rule (examined, holds). `evaluate.rs` `record` faults `value-loss-site-is-located` when a node has no location. The source route records at the checker location. `float_ordering` binds the right operand of `r <= e` with a `let` at lowered `[0]`, while the checker located `e` at `[1]`. So AC-36 (`[0]`/`[1]` against a derived `[1, 0]`) and AC-37 (`[1]` against a derived `[0]`) both fail against today's `locate_functions`. They can fail.
- Keying (examined). FR-383's walk and `emit/protocol_locations.rs` key a row by (application, occurrence, term_ordinal), with ordinals numbered per visit. `EmitRefusal::UnlocatedOccurrence` exists (emit.rs:103). Model-clause functions take `Origin::Body` with `index` = position in `functions` (dispatch_link.rs:295-310), which matches "index among the package's functions".
- AC-36 (examined, sound). It uses the same `function f using v(...)` unit as `ieee_ordering_route_parity.rs`. `sqrt(2.0)` is inexact twice, and `g`'s losses stay in `f`'s body.
- AC-37 inputs (examined). `1 + 2^-60` is inexact at both widths. The largest finite value squared overflows. `1/3` is inexact and `1/0` divides by zero. Conversions must be written explicitly as `convert<T>(e)` (FR-397).
- Ids (examined). FR-531-AC-36/37 and TC-3133 are on neither main nor any of the 25 other open FSL PRs.
- `quire validate` on both changed files exits 0.

## Verdict

Mergeable: no, until FND-001 is fixed. As written, AC-37 cannot pass. The rule itself and AC-36 are sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-531-AC-37 is not runnable as written. (a) "the body `e` alone gives the same loss at path `[]`" keeps the declared result `Boolean`, so a float body `e` is `ill_typed`. The AC must say that this variant's result type is `T`. (b) The conversion cases conflict with the stated signature `p: T, q: T, r: T`: they need a `Rational[..]` parameter (bounds unstated, and FR-397 lists `Rational[..]`, not bare `Rational`) and a `Float64` parameter for the `Float32` case, written `convert<T>(p)`. (c) The signature lacks the `function` keyword | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:1011 |
| FND-002 | medium | The rule covers the underflow and invalid flags, a decimal rounding loss and the location of a stop. No AC exercises any of them: AC-37 has only inexact, overflow and divide-by-zero cases, and no stop. Add an underflow case (for example the smallest normal value times `0.5` with an inexact result), an invalid case (`sqrt(-1.0)`), a decimal rounding case and one stop located in a body | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:218-235 |
| FND-003 | medium | "S4 SHALL write, for every term of every function body it emits" also binds `emit_timed_models`. That path re-encodes the base package with `checked_term_data: None`, so after FSpec #760 every timed-model package that has a function body with terms is refused by the strict reader. Neither the FR nor QSL-2625's emitter bullet names this path. The FSL test `tests/it/dispatch_emission.rs:404-418` ("graph-only package remains admitted") also drops the sidecar from a package with operation-body function nodes | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:236-245 |
| FND-004 | low | Clause functions (model `pre`/`requires` bodies) are `Origin::Body` functions in the checker, but per FR-322 they carry no `body` binding on the wire. So FR-322's "function node that carries a body" excludes them, and a loss in a precondition evaluated on the package route would still have no row. State whether they are in scope | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:218-222 |
