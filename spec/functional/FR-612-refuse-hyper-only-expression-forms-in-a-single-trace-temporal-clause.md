---
id: FR-612
title: "Refuse hyper-only expression forms in a single-trace temporal clause"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-326
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-023
    type: depends_on
---
# FR-612: Refuse hyper-only expression forms in a single-trace temporal clause

## Description

FSL SHALL refuse an indexed read `e @ t`, a step label `t.step.op` or the
`stutter` label in a `temporal` clause that is not a `hyper` clause, whether
it appears in a `holds` condition of the formula or in an `on each ... when`
guard, with a named refusal located at the form.

## Use case

An author writes `on each (e: Event) when (e.id @ a = e.id @ b)` in an ordinary
temporal clause. The clause reads one trace, and `a` and `b` name no trace. The
refusal says that indexed reads and step labels belong to a `hyper` clause, so
the author writes the property as a `hyper` clause that binds `a` and `b`.

## Inputs

A `temporal` clause's formula and its `on each ... when` guard.

## Outputs

The clause's forms, or a refusal.

## Behavior

- A trace variable is bound only by a `hyper` clause's `forall trace` and
  `exists trace` quantifiers (ADR-023). A single-trace clause evaluates one
  trace position at a time and has no trace variable, so an indexed read or a
  step label has no meaning in it.
- If a `holds` condition of a non-`hyper` temporal clause's formula, or its
  `on each ... when` guard, contains an indexed read, a step label or the
  `stutter` label, then FSL SHALL refuse `unsupported_construct`/
  `expression-form` located at the first such form in source order, with a
  message saying that indexed reads and step labels are admitted in `hyper`
  clauses only.
- The guard is parsed with the same expression tree as a `holds` condition, so
  the two placements refuse with the same code, cause and message instead of a
  syntax error in one and a forms refusal in the other.
- The refusal produces no checked clause. A `hyper` clause keeps admitting the
  forms (FR-488 and the hyper clause requirements).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-612-AC-1 | A non-`hyper` temporal clause over `v: V::ConfigVersion` with formula `always holds(v.versionNumber @ a = 0)` refuses `unsupported_construct`/`expression-form` at `v.versionNumber @ a`, with a message containing `hyper`; with `always holds(a.step.mix)` it refuses the same way at `a.step.mix`. | Test (TC-1543) |
| FR-612-AC-2 | The same clause with activation `on each (w: V::ConfigVersion) when (w.versionNumber @ a = 0)` and formula `always holds(w.versionNumber = 0)` refuses `unsupported_construct`/`expression-form` at `w.versionNumber @ a`, with the same message, and S1 reports no syntax error for it. | Test (TC-1543) |

## Dependencies

- ADR-023 (trace variables are bound by `hyper` clauses only), FR-326
  (temporal clause admission), FR-488 (hyper clause checking).
