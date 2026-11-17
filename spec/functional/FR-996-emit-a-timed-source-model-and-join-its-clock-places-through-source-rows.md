---
id: FR-996
title: "Emit a timed source model and join its clock places through source rows"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-012
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-088
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-995
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/filament-specification/FR-322
    type: implements
  - target: ix://agent-ix/filament-specification/FR-789
    type: implements
---
# FR-996: Emit a timed source model and join its clock places through source rows

## Description

When a unit's source model is timed (FR-230 "The source model's time
member"), S4 SHALL emit its FSpec FR-322 timed-model nodes exactly as for a
selected timed model, with these source forms: the `timed_model` root's
`model` is the source-declared `model`/`object_type` node of the type the
time member names; each `clock_place`'s `resolved_path` row names its
declaring type and member by FR-995's source declaration keys; and each row's
declared member type is the rendering the source-object identity row of the
declaring type (FR-569 "Source-object identity rows") gives that member. A
strict reader SHALL resolve those rows through the source rows, never
through a selected model declaration (FSpec FR-789).

## Rationale

A clock place's `resolved_path` is checked package data that a reader joins
to the declaration of each path step, so that a forged path refuses without
reopening a document (FSpec FR-322-AC-124). For a selected model the
declaration is the selected domain document. A source model has no document:
its declarations are the source-declared nodes, their field bindings and
their identity rows, which already carry every fact a row states (member
name, presence and type). Joining through those rows keeps one rule for both
routes, a row joins the declaration that owns the step, and keeps FR-230's
separation: a source model's time member never times a selected model's
clock, so a source root's row never resolves through a selected document.
FR-088-AC-13 already requires every type a source member names to have a
spelling, so each row's rendering is defined.

## Inputs

A checked source model with one `dense` time member and its clock places.

## Outputs

The `timed_model` root, `time_source` and `clock_place` nodes, or a refusal.

## Behavior

- The root's `model` and the `time_source`'s model SHALL be the node of the
  source object type the time member names (FR-230-AC-65).
- For each `field(name)` path step, the row's `declaring_type` SHALL be the
  source declaration key `node` of the type or native record whose member it
  is (`ix://{identity}/{type}`), its `member_identity`
  `ix://{identity}/{type}/{name}`, and its `member_type` the identity row's
  rendering of that member for an object type, or the rendering of the
  record field binding's type for a native record, in the FSpec FR-322
  rendered type form; `tuple`, `union` and `slot` steps keep their FR-322
  rows.
- A source model's places SHALL list every clock of the source model,
  whichever source object type declares it, directly or below a record.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-996-AC-1 | Over FR-230-AC-65's unit (`record W { c: Clock; }`, `object O { w: W; d: Clock; operation step() modifies self.d; }`, `time O dense unit ms;`), S4 emits one `timed_model` root whose `model` is `O`'s source-declared node, and two `clock_place` nodes: `d` with one row `{declaring_type: "ix://I/O", member_identity: "ix://I/O/d", member_type: "ix://filament/native/Clock"}`, and `w.c` with rows `ix://I/O`/`ix://I/O/w` of type `ix://I/W`, then `ix://I/W`/`ix://I/W/c` of type `ix://filament/native/Clock`, the terminal `Clock`, `I` the `SourceOwner` `identity` of `O`'s node. | Test (TC-2818) |
| FR-996-AC-2 | FSL's strict I2 read admits the FR-996-AC-1 package. Each refuses `invalid_package`/`malformed-clock-place` at the row: `d`'s `member_identity` replaced by `ix://I/O/e` (no such member), its `declaring_type` by `ix://J/O` for another unit's identity `J`, its `member_type` by `Integer`, and `w.c`'s second row's `declaring_type` replaced by a selected model's `ix://acme/orders/Order`. | Test (TC-2819) |

## Dependencies

- [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md)
  "The source model's time member" and FR-569 "Timed source models".
- [FR-995](FR-995-spell-source-declaration-keys-from-the-source-owner.md):
  the key spelling.
- FSpec FR-322 "Timed-model nodes" and FSpec FR-789: the row join.
