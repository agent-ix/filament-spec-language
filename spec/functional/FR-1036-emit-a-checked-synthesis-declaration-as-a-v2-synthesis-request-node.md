---
id: FR-1036
title: "Emit a checked synthesis declaration as a v2 synthesis-request node"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-754
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-1036: Emit a checked synthesis declaration as a v2 synthesis-request node

## Description

The package writer SHALL emit each checked `synthesis` declaration (FR-754)
as one `filament.checked-semantic-graph/v2` node with `node_tag: "claim"`
and `semantic_form: "synthesis_request"`, whose body, declaration,
occurrence, dependencies and `claim_names` row are those FSpec FR-322
"Synthesis-request nodes" states, and FSL's package reader SHALL read that
node back. FSpec FR-322 owns the body and the reader's refusals; this
requirement states what FSL writes from its checked declaration, so a
synthesizer or planner that holds only the package reads the declaration
the source checked.

## Inputs

A checked `synthesis` declaration: its name and declaration path, its
resolved grammar `G` and domain `D`, the candidate binder, the optional
objective with its direction, and the typed `satisfies` block.

## Outputs

- One declared node per `synthesis` declaration, path the declaration path,
  one `declaration` occurrence at ordinal `0`.
- One `claim_names` row for that occurrence.
- From the reader: the declaration's grammar node, domain node, candidate
  parameter, objective direction, root and text, block root and symmetry
  rule, or FSpec FR-322's refusal with its locus.

## Behavior

- The writer SHALL write `grammar` and `domain` as references to the type,
  `model`/`object_type`, `hybrid`/`hybrid_model` or
  `bounded_domain`/`integer_range` nodes S3 resolved, never to a node it
  mints for the declaration.
- The writer SHALL write `candidate` as a reference to the `value`/`parameter`
  node of the declaration's own name at level `0`, typed `D` for a value
  synthesis and `G` for a plan synthesis, the binder S3 bound inside the
  block and the objective (FR-754).
- The writer SHALL write `objective` as the empty aggregate when the
  declaration has none, and otherwise its `direction`, a reference to its
  root and its `text`, the objective expression's tokens from its first to
  its last joined by one space, comments dropped.
- The writer SHALL write `satisfies` as a reference to the block's root and
  `symmetry` as the text `identity`.
- The writer SHALL write the `claim_names` row from the declaration's name,
  its package-local qualified path and its whole declaration span, as FSpec
  FR-322 "Claim declaration names" states for every claim-bearing node.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1036-AC-1 | `synthesis Pick using v grammar Digit domain Digit satisfies { Pick * Pick = 49 };` over `type Digit = Int[0, 9];` emits one `claim`/`synthesis_request` node with path `["Pick"]`, the six bindings `grammar`, `domain`, `candidate`, `objective`, `satisfies` and `symmetry` in that order, `grammar` and `domain` both the `Digit` node, the candidate a `value`/`parameter` node named `Pick`, `objective` the empty aggregate and `symmetry` `identity`, and one `claim_names` row named `Pick`; with `minimize Pick` the objective's `direction` is `minimize` and its `text` `Pick`. | Test (TC-3016) |
| FR-1036-AC-2 | The plan synthesis `Reach` over `Config::ConfigVersion` with domain `Lengths` emits a node whose `grammar` is the `model`/`object_type` node of `ConfigVersion`, whose `domain` is the `bounded_domain`/`integer_range` node of `Lengths`, and whose candidate is typed by the `ConfigVersion` node; the emitted package is admitted by FSL's reader, which returns the declaration's grammar, domain, objective and block. | Test (TC-3016) |

## Dependencies

- FR-754 (the checked declaration); FR-092 (node keys); FSpec FR-322 (the
  node form and reader).
