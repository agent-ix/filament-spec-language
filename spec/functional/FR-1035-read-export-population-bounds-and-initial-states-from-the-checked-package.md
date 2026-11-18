---
id: FR-1035
title: "Read an export's population bounds and initial states from the checked package"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-812
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-380
    type: references
---
# FR-1035: Read an export's population bounds and initial states from the checked package

## Description

When FSL exports a model to TLA+ or Lean, it SHALL take each population's
bound and the model's `Init` from the checked package it compiled, as
FSpec FR-812 states, and the checked package SHALL expose each selected
population's authored bound to the mapper.

## Behavior

### Population bounds

- A population's bound SHALL be the maximum `N` of the `bound L on M::p [N]`
  declaration that names it (FR-380). That is the only source a population
  maximum has. Domain-package populations and source `population` members
  declare none. The FR-610 identity rows carry `{name, symmetric}` only and
  are not read for bounds. A population that no `bound` names exports with
  no `Cardinality` (TLA+) or `length` (Lean) conjunct.
- The checked package SHALL expose, for each population of its selected
  model population catalog, that population's authored bound, or its
  absence. The bound is found by the population's model root and population
  identity, as the catalog names them, and it carries the maximum and the
  source region of the bound's `[N]`. It is not found by the declaring
  unit's model alias. Today the catalog entry exposes only the population's
  identity and members, and `CheckedGraph::population_bounds` lists the
  unit's `bound` declarations by label, alias, population and maximum with
  no join to a catalog entry and no region. The join and the region are this
  requirement's code, and the `fsl:` comment of the conjunct's `TypeOK`
  definition names that region.
- The bound is read the same way whether or not a reachability walks the
  population. FR-380 emits the `bounded_domain`/`model_population` node
  only for a walked bound, so the mapper never infers a bound from the
  presence of that node.

### Initial states

- With the request's `initial`, `Init` SHALL state that snapshot's state.
- Without it, a hybrid's `Init` SHALL hold one disjunct per `hybrid-initial`
  entry, or the implicit `mode M when { true }`, with `M` its first mode. A
  state model's `Init` SHALL be its well-typedness, `Init == TypeOK` and
  `def Init (s : State) : Prop := WF s`. A `constraint` clause (FR-158),
  such as the spine model's `Low`, and an invariant are never read into
  `Init`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1035-AC-1 | The spine unit with `bound HistoryBound on Config::config_history [8]` and no reachability exports to TLA+ with `Cardinality(DOMAIN v_` and `) <= 8` in `TypeOK`. The same unit without the `bound` has no `Cardinality` in its text. | Test (TC-3003) |
| FR-1035-AC-2 | The spine unit with the invariant `Low` and no `initial` exports `Init == TypeOK`, exit `0`. `Braking` with no `initial` exports an `Init` naming the mode `Moving`. | Test (TC-3003) |

## Dependencies

- FSpec FR-812 (the encodings of bounds and `Init`), FR-380 (the `bound`
  declaration), FR-1015 (the export request).
