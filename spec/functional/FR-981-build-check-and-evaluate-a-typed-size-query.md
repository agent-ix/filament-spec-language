---
id: FR-981
title: "Build, check and evaluate a typed size query"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-003
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-766
    type: references
  - target: ix://agent-ix/filament-specification/FR-783
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-145
    type: depends_on
---
# FR-981: Build, check and evaluate a typed size query

## Description

FSL SHALL build `size<N>(c)` as the `Size` form carrying its result type `N`,
check it as FSpec FR-783 states, and evaluate it to the same length as
`size(c)`.

## Rationale

`size<N>(c)` is in the standard grammar and FSpec FR-783 gives it a meaning:
the length of `c` typed at the integer domain the author names. The parser
already reads it; S2 refused it as a construct no form represents, so the
documented spelling, used for example to compare a population's size in a
model's `Count` type, could not be checked or run. It is built on the
existing `Size` form, so it adds a type to a query FSL already evaluates
rather than a second query.

## Inputs

A complete-V1 unit holding `size<N>(c)`.

## Outputs

An S2 `Size` form with `type_argument` `N`; a checked node typed `N` or a
located refusal; at run, the length of `c`.

## Behavior

### Forms

- S2 SHALL build `size<N>(c)` as `Size { operand: c, type_argument: Some(N) }`
  and `size(c)` as `Size { operand: c, type_argument: None }`. Neither refuses
  `UnrepresentedConstruct` (FR-091, FR-763).
- `size<>(c)` SHALL refuse at S1 `invalid_syntax`/`unexpected-token` at the
  `>`, and `size<N>()` SHALL refuse the same at the `)`, because the grammar
  requires one type name and one operand.

### Checking

- The checker SHALL apply FSpec FR-783: `N` is a dimensionless integer type
  (`Integer`, `Int[lo, hi]` or an alias of one) and the node's type; any
  other `N` refuses `ill_typed`/`type-mismatch` at `N`; a non-collection
  operand refuses `ill_typed`/`type-mismatch` at `c`.
- Under `CheckMode::Linked` the checker SHALL record a range obligation on
  the node: the proved interval is `c`'s size interval (the collection
  type's bound narrowed by the `size(c)` guard facts in scope, the facts
  FR-766 uses for `count<N>`), the required interval is `N`'s domain; an
  obligation not discharged refuses `undefined_expression`/`unproved-range`
  at the `size` call with `Obligation::Range{required, proved}`. `N` =
  `Integer` discharges it.
- In a declared state clause the obligation is recorded and decided under
  `CheckMode::Kernel` as under `Linked` (FR-495), as for `count<N>` (FR-766).
  Under `CheckMode::Kernel` outside a declared clause no obligation is
  recorded, and S6a refuses a length outside `N` with
  `Refusal::IntegerOutOfDomain{target: N}` at the node.

### Evaluation

- S6a SHALL evaluate `size<N>(c)` exactly as `size(c)` (FSpec FR-145's bound
  count, with the same charges) and return the length as a member of `N`.
- `size(c)` with no type argument is unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-981-AC-1 | S2 builds `function f using v(c: Sequence<Integer>[0, 4]): Int[0, 4] pure { size<Int[0, 4]>(c) }` with one `Size` form whose type argument is `Int[0, 4]`, and `size(c)` with one `Size` form with no type argument; neither refuses `UnrepresentedConstruct`. | Test (TC-2773) |
| FR-981-AC-2 | `size<>(c)` refuses at S1 `invalid_syntax`/`unexpected-token` at the `>`, and `size<Int[0, 4]>()` refuses the same at the `)`. | Test (TC-2773) |
| FR-981-AC-3 | Under `CheckMode::Linked`, AC-1's function checks with the node typed `Int[0, 4]`; with `size<Int[0, 3]>(c)` and result `Int[0, 3]` it refuses `undefined_expression`/`unproved-range` with required `[0, 3]` and proved `[0, 4]`; `if size(c) <= 3 then size<Int[0, 3]>(c) else 0` checks; `size<Boolean>(c)` refuses `ill_typed`/`type-mismatch`; `size<Int[0, 4]>(3)` refuses `ill_typed`/`type-mismatch`. | Test (TC-2774) |
| FR-981-AC-4 | Through `command::run` over FR-108's ConfigVersion spine, the function claim `function three using v(): Boolean pure { size<Int[0, 4]>(sequence[7, 8, 7]) = 3 and size(sequence[7, 8, 7]) = 3 }` reports that the claim holds, exit 0. | Test (TC-2775) |
| FR-981-AC-5 | Under `CheckMode::Linked`, a unit declaring `type Count = Int[0, 1000];` and `function few using v(c: Sequence<Integer>[0, 4]): Boolean pure { size<Count>(c) >= 0 }` checks. | Test (TC-2774) |

## Dependencies

- FSpec FR-783 owns the typed size query; FSpec FR-145 the length.
- FR-091 maps the form; FR-763 no longer lists it as a refusal construct;
  FR-766 is the matching `count<N>` rule.
