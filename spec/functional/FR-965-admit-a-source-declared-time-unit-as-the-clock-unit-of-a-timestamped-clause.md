---
id: FR-965
title: "Admit a source-declared time unit as the clock unit of a timestamped-event clause"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-015
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-337
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-194
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-933
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-370
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-778
    type: depends_on
---
# FR-965: Admit a source-declared time unit as the clock unit of a timestamped-event clause

## Description

The clock unit of a timestamped-event clause SHALL be the unit its selected
clock's `timestamp_unit` `T` names, whether a selected domain package
declares that unit or the clause's own source unit does. A unit that writes
`dimension Time; unit s : Time = rational(1, 1); unit ms : Time =
rational(1, 1000) * s;` and selects a clock `{"kind": "timestamped_event",
"timestamp_unit": "ms"}` has the clock unit `ms`, and a `duration` window
such as `past[0 ms, 300 s]` emits with both bounds converted to `ms`.

A window aggregate is most often written over latencies the author's own
unit declares, as in a p95 latency bound (FR-194). Requiring a domain package
for the clock unit would refuse that whole use, while the source unit already
holds a unit node every other quantity of that unit names. The bounds name
that node, so a window in a source unit and one in a model unit follow one
rule: the bounds are typed by the one node the clock unit has.

## Inputs

- The selected clock's `timestamp_unit` `T` (FR-640), given to S3 and to the
  emitter (FR-337).
- The clause's source unit's declared units and dimensions, and the units of
  the admitted domain packages.

## Outputs

The clause's clock unit node, or none.

## Behavior

- When `T` is the FCD identity of a unit an admitted domain package declares
  (such as `ix://acme/rpc/unit/ms`), the clock unit node SHALL be that
  model-owned node, path `["scalar_type", "unit", T]`, as FR-337 states.
- When `T` is the name of a unit the clause's own source unit declares (the
  program, or the library that declares the clause), the clock unit node
  SHALL be that source-owned unit node: its `node_id.path` is its declaration
  path `[T]`, it carries that source unit's `SourceOwner`, and it is the node
  every other quantity of the unit names. An FCD identity and a declared unit
  name never coincide, because a declared name is an identifier and an FCD
  identity is an `ix://` URI, so `T` selects at most one node.
- The clock unit node SHALL be a linear unit of a dimension named `Time`.
  These refusals apply only to a clause that holds a `duration` window, the
  one form that needs a clock unit node; a clause without one keeps any
  non-empty `timestamp_unit` (FR-640), such as the default `step`. In such a
  clause, a `T` that names no declared unit SHALL refuse the clause with cause
  `unknown-clock-unit`, and a `T` that names a unit of another dimension with
  cause `clock-unit-dimension`, each naming the clock and the unit, as FR-933
  refuses a role clock's binding unit (through the request path,
  `invalid-request` at stage `emit`, exit 20); an affine time unit refuses
  `ill_typed`/`type-mismatch` as FR-337 states. No package is written.
- S3 SHALL convert both bounds of a `duration` window exactly to the clock
  unit and refuse a bound that does not convert to it, as FR-337 states for
  a model-owned clock unit; a source `Time` and a model `Time` are distinct
  dimensions with no equivalence unless one is admitted (FR-194-AC-7).
- The emitter SHALL write no second unit node for a source-owned clock unit,
  and the reader SHALL match a bound's `type` against it by `node_id.path`
  and `semantic_form` (FSpec FR-370, FR-778).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-965-AC-1 | A unit declaring `dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;` whose timestamped-event clause over `e: Quantity<ms>` is `always[0,3] (quantile 0.5 of e over past[0 ms, 300 s] <= 5 ms)`, checked and emitted under a clock whose `timestamp_unit` is `ms`, emits; its aggregate's `lower` and `upper` are the quantity literals `0` and `300000` whose `type` is the source-owned `ms` unit node (its declaration `ms`, owner kind `source`); the package holds no node whose path is `["scalar_type", "unit", "ms"]`; under `timestamp_unit` `s` the bounds are `0` and `300` typed by the source `s` node; FSL's checked-package reader admits both packages. | Test (TC-2720) |
| FR-965-AC-2 | Negative: AC-1's clause under `timestamp_unit` `us` (no unit of that name), and with the unit also declaring `dimension Length; unit m : Length = rational(1, 1);` under `timestamp_unit` `m` (a unit not of `Time`), refuse with cause `unknown-clock-unit` and `clock-unit-dimension` respectively, each naming the clock and the unit (FR-933), and write no package. | Test (TC-2720) |
| FR-965-AC-3 | A unit that declares AC-1's source units and also selects the domain package `example/config-version` (base dimension `Time`, units `s` and `ms`) as `Config`: under `timestamp_unit` `ms`, `past[0 ms, 300 s]` types both bounds by the source `ms` node; under `timestamp_unit` `ix://example/config-version/unit/ms`, `past[0 Config::ms, 300 Config::s]` types both bounds by the model-owned node `["scalar_type", "unit", "ix://example/config-version/unit/ms"]`, and no other unit node is written for either clock unit. | Test (TC-2720) |

## Dependencies

- FR-337: the aggregate emission whose clock unit this requirement widens.
- FR-194: the window aggregates; FR-194-AC-13 runs the p95 case through
  `command::run` with source-declared units.
- FR-640: the clock selection.
- FSpec FR-370, FR-778: the reader's clock-unit match.
