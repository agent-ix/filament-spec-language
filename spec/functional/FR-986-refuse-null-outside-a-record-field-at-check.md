---
id: FR-986
title: "Refuse null outside a record field at check"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-001
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: references
  - target: ix://agent-ix/filament-specification/FR-143
    type: depends_on
---
# FR-986: Refuse null outside a record field at check

## Description

When a unit holds `null` outside the whole value of a record field, the
checker SHALL refuse it `ill_typed`/`type-mismatch` at `null`, naming `null`
and the empty-option spellings `none` and `absent`.

## Rationale

`null` is the explicit-null state of a `?` field (`R { f: null }`), and is
never a value of any type, `Option<T>` included (FSpec FR-143; FSpec
FR-143-AC-20 makes `null` for an `Option<Integer>` parameter `ill_typed`). It
is therefore a type error in every other position, and the checker, which
knows the expected type, is the stage that can say so. A forms-stage refusal
(exit 21, "not supported yet") would tell the author the language will one day
accept the construct, which it will not; the type error (exit 20) tells them
what to write instead.

## Inputs

A unit holding `null` outside a record field value.

## Outputs

A `Null` form at S2 and one located check refusal.

## Behavior

- S2 SHALL build `null` outside a record field value as `ExprNode::Null`,
  and SHALL NOT refuse it `UnrepresentedConstruct`. A record field whose
  whole value is `null` keeps building `FieldInitializer::Null` (FR-091).
- The checker SHALL refuse every `Null` node `ill_typed`/`type-mismatch`
  located at `null`, whatever the expected type of its position (an
  `Option<T>`, a non-option type, or none, as in `let x = null in ..`). The
  message names `null` and names `none` and `absent` as the empty-option
  spellings.
- The outcome document of the refused check has category `refusal`, exit 20.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-986-AC-1 | S2 builds `function f using v(): Option<Integer> pure { null }` with a `Null` expression form and no `UnrepresentedConstruct`; check refuses it `ill_typed`/`type-mismatch` at `null`, with a message containing `null`, `none` and `absent`. | Test (TC-2785) |
| FR-986-AC-2 | `function g using v(a: Option<Integer>): Boolean pure { a = null }`, `function h using v(): Integer pure { null }` and `function k using v(): Boolean pure { let x = null in true }` each refuse `ill_typed`/`type-mismatch` at `null`. | Test (TC-2785) |
| FR-986-AC-3 | `record P { a: Integer; b: Integer?; }` with `function p using v(): P pure { P { a: 1, b: null } }` checks; `function o using v(): Option<Integer> pure { none }` and `{ absent }` check. | Test (TC-2785) |

## Dependencies

- FR-091 builds the form; FR-763 no longer lists `null` as a refusal construct.
- FSpec FR-143 owns field presence and the rule that `null` is no option value.
