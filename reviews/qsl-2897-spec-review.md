---
id: SR-2897003
title: "QSL-2897 part 1 FR-106 implementation-note review"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; QSL-2897; spec/functional/FR-106-admit-snapshots-and-invocations.md implementation notes (lines 492-534 at PR head)"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-106", type: references }
---

## Summary

Ticket: QSL-2897. Review of the FR-106 implementation-boundary notes this PR rewrites, checked against the normative domain contract (lines 434-486) and AC-72..AC-81. The deferral text for Clock-reference enumeration is removed. The QSL-2178 STEP1/STEP2 text is replaced by a QSL-2897 state note, and the Bare route is named as unmet under QSL-2776. Normative requirement text and the AC table are unchanged. The sub-analyses applied are integrity and scope-boundary, limited to the note. EARS and object review do not apply because no requirement statement changed.

## Verdict

**CONDITIONAL.** The note is mostly accurate, and it claims no Bare-route AC. It overstates Source evidence in one place and omits the selector-cause divergence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The note says Source admission validates "as AC-72, AC-73 and AC-76 require, at the actual document and nested value locus". The evidence does not support all of that. AC-73 and AC-76 tests are ignored; their Source halves pass with `--ignored` but are not run by default. AC-76 is a two-route criterion. A selector that is invalid for the template refuses `dangling_reference`, not AC-80's `invalid_runtime_input`/`invalid-value`. The note should state what is proved and name the AC-80 selector cause, or the code should be fixed. | spec/functional/FR-106-admit-snapshots-and-invocations.md:492 |
| FND-002 | low | The note says the package or Bare model route refuses before any snapshot is read. It does not say that `ModelCheckInput.clock_declarations` is `None` on that route, so Clock domains there would hold only seeds if that refusal were lifted. The QSL-2776 handoff should carry this. | spec/functional/FR-106-admit-snapshots-and-invocations.md:500 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new note says each unenumerable case "refuse[s] typed (`ClockDomainRefusal`), never an empty domain". The protocol `values_of` and model-based-test parameter routes (CR FND-008 and FND-009) still produce empty or partial Clock domains. | spec/functional/FR-106-admit-snapshots-and-invocations.md:517 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The note now lists the proved Source causes and says AC-72, AC-73 and AC-76 stay unmet as two-route criteria until Bare passes. |
| FND-002 | fixed | The note now states that the package route has `clock_declarations` `None`, that it refuses `RequiresBound`, and that QSL-2776 must supply the table. |
| FND-003 | fixed | The note now lists every enumeration route that reads the one candidate set or refuses, including the protocol record domains and the compensation enumerability check. It states that the model-based parameter case is refused. This matches the code at b5c45a96. |
