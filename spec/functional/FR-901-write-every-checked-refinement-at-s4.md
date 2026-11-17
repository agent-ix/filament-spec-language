---
id: FR-901
title: "Write every checked refinement at S4, history and population rows included"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-135
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-138
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-139
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-375
    type: implements
  - target: ix://agent-ix/filament-specification/FR-322
    type: implements
---
# FR-901: Write every checked refinement at S4, history and population rows included

## Description

The S4 emitter SHALL write every refinement that S3 checks (FR-135 to FR-139, FR-147) to
`filament.checked-semantic-graph/v2` as its `relation`/`refinement` node
(FSpec FR-375), and FSL's I2 reader SHALL read it back to the same checked
refinement. S3 and S4 SHALL NOT refuse a checked refinement as
`unsupported_construct`/`not-yet-implemented` because a row has no node form.
The row forms that need a writer are written as FSpec FR-322 "Refinement row
expressions" defines:

| Row form | Written as |
| --- | --- |
| A `history` row (FR-138) | one entry of the body's `histories`: `concrete_type`, `name`, `type`, `initial` as a `literal` or local `reference` term naming the admitted `history-literal`, and one `updates` entry per concrete operation it follows, in source order |
| A read of a history field | the `query` application `filament.op.refinement.history` with member `{kind: "history", declaration, name}` and the receiver as its one argument; `pre(...)` of it is `filament.op.state.pre` over that read |
| A whole concrete population read in a row (FR-139) | `allInstances<T>(C::p)` over the population's node, with the `collection_bounds` `[0, n]` result type, `n` the unit's authored maximum (FR-380) |
| `count`, `exists`, `all`, `sum` and `only` over a population | the FR-322 table's applications, each binder a `value`/`parameter` node |
| `seq(i in a .. b: e)` in a row | the FR-093 lowering of `seq`, which FR-322 admits in row position |

A history field written this way is no member of the concrete type: the
concrete type's declaration node keeps its body and declares no field of
that name, as FSpec FR-375 "History rows" requires. Two refinements that
declare the same history of the same type refer to one history-read node.

A refinement that fails S3 is refused at S3, by its own cause, before
emission. S4 SHALL have no refusal for a checked refinement, and the emitter's
`RefinementNotEmitted` refusal, with S3's matching cause, is removed.

### Decision: S4 writes what S3 checks

A refinement that checks at S3 and is refused at S4 is one the author can
write and can never compile. The standard has a node form for every row form
listed above, so S4 writes each of them. The rule is FR-880's for protocols,
applied to refinements.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-901-AC-1 | Each case of the refinement adverse corpus that refused `not-yet-implemented` for a row without a node form checks and compiles: the `history` row with its update on `attemptUpdate`, the whole-population `count(s in C::config_history where s.versionNumber = 1)` row, and the `seq` row. Each package is admitted, and its refinement node holds the row as the table states. | Test (TC-2540) |
| FR-901-AC-2 | Reading each AC-1 package back gives a checked refinement equal to the compiled one: rows, history entries, populations and keys. Emitting the read-back refinement gives the same bytes. | Test (TC-2540) |
| FR-901-AC-3 | Two refinements in one unit declaring `history C::ConfigVersion.last: Int[0, 1] = 0` refer to one history-read node, and the concrete type's declaration node declares no field `last`. | Test (TC-2541) |
| FR-901-AC-4 | A refinement whose `history` row's update is ill-typed still refuses at S3 with its FR-138 cause, and no package is written. No refinement in the corpus, accepted or refused, is refused with `not-yet-implemented` for a row form. | Test (TC-2541) |

## Dependencies

- [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md)
  to [FR-139](FR-139-type-and-evaluate-population-valued-expressions.md): the
  S3 refinement checks.
- [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md): term
  lowering.
- FSpec FR-322 "Refinement row expressions" and FR-375.
