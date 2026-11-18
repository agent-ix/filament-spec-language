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
- The checked package SHALL expose each selected population's authored
  bound through
  `CheckedGraph::population_bound(&self, root: &CheckedNodeId, entry:
  &CheckedModelPopulation<'_>) -> Option<&PopulationBound>`. `root` is the
  selected model's root, as `CheckedSelectedModelPopulations::root` gives
  it, and `entry` is one of that model's catalog populations. The key is
  that root and the entry's identity, never the declaring unit's model
  alias, so two units that alias one model differently find one bound.
- `PopulationBound` SHALL carry `label: String`, the `bound` declaration's
  label; `maximum: u64`, the inclusive maximum `N`; and `region:
  SourceRegion`, the region of the declaration's `[N]`. The accessor
  returns `None` exactly when no `bound` names the population. The
  population is then unbounded, and its exported variable carries no
  `Cardinality` or `length` conjunct.
- Each bound SHALL be exported as its own definition named `b_` followed by
  its mangled label (FSpec FR-814's population-bound kind), `b_<label> == Cardinality(DOMAIN v_T) <= N` (TLA+) or `def b_<label> : Prop :=
  v_T.length <= N` (Lean), and `TypeOK` (TLA+) or `WF` (Lean) conjoins it by
  that name. Like every definition, it is preceded by one unindented FSpec
  FR-814 comment line `\* fsl: <node id> <region>` (or `-- fsl: ...`), whose
  node id is the population's catalog node and whose region is the bound's
  `[N]`. Each comment thus names one node and one region, as FR-814
  requires; the `TypeOK` or `WF` comment names that definition's own node
  and region.
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
  such as `Small`, and an invariant, such as `Low`, are never read into
  `Init`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1035-AC-1 | The spine unit with `bound HistoryBound on Config::config_history [8]` and no reachability exports to TLA+ with `Cardinality(DOMAIN v_` and `) <= 8` in the `b_HistoryBound` definition that `TypeOK` conjoins. The same unit without the `bound` has no `Cardinality` in its text. | Test (TC-3003) |
| FR-1035-AC-2 | The spine unit with the invariant `Low` and no `initial` exports `Init == TypeOK`, exit `0`. With `constraint Small using v on Config::ConfigVersion at current { self.versionNumber <= 3 }` added, `Init` holds no `<= 3`. `Braking` with no `initial` exports an `Init` naming the mode `Moving`. | Test (TC-3003) |
| FR-1035-AC-3 | For the spine unit without a `bound`, `CheckedGraph::population_bound` returns `None` for the `config_history` catalog entry under its model root, and the exported `TypeOK` constrains that population by no `Cardinality`. With `bound HistoryBound on Config::config_history [8]`, it returns `maximum` 8, label `HistoryBound` and `region` the bytes of `[8]`. | Test (TC-3003) |
| FR-1035-AC-4 | With that bound, the TLA+ export defines `b_HistoryBound == Cardinality(DOMAIN v_` ... `) <= 8`, and `TypeOK` conjoins `b_HistoryBound`. The line immediately before `b_HistoryBound ==` is an unindented comment matching FSpec FR-814-AC-6, naming the `config_history` population node and the region `<authority>/<identity>:<start>-<end>` of `[8]` in the unit. The comment before `TypeOK ==` names `TypeOK`'s node and region, not that region. | Test (TC-3003) |

## Dependencies

- FSpec FR-812 (the encodings of bounds and `Init`), FR-380 (the `bound`
  declaration), FR-1015 (the export request).
