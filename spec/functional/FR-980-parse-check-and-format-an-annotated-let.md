---
id: FR-980
title: "Parse, check and format an annotated let"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-001
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-002
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-003
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-149
    type: references
  - target: ix://agent-ix/filament-specification/FR-143
    type: references
---
# FR-980: Parse, check and format an annotated let

## Description

FSL SHALL read `let x: T = e in b` as a `let` whose binder `x` is declared
with the type `T`, check `e` with `T` as its expected type, give `x` the type
`T` in `b`, and format the annotation without changing its tokens.

## Rationale

The standard's `expr` production writes `let`, `ident`, an optional `':'
type-ref`, `=`, `expr`, `in`, `expr` (FSpec FR-501 item 3), and FSpec FR-143
and FR-149 name the annotation as a declared-type position. The annotation is
how an author keeps the range of a bound value: an unannotated `let` types
`x` by `e`, so `let line = price * qty in line` is typed `Integer` and is then
refused `unproved-range` where an `Int[0, 99000000]` is expected, while
`let line: Int[0, 99000000] = price * qty in line` states the domain at the
binding and carries the range obligation there, where the author can see it.

## Inputs

A complete-V1 unit whose expressions hold `let` with or without an
annotation.

## Outputs

A CST whose `Expression` node for the `let` holds a `TypeReference` child
when the annotation is written; an S2 `Let` form carrying the annotation as
a type form (FR-091 "Expression mapping"); a checked `let`; formatted source.

## Behavior

### Grammar

- S1 SHALL recognize `let`, an identifier, optionally `:` followed by one
  `TypeReference`, then `=`, the initializer `Expression`, `in` and the body
  `Expression`. Every `type-ref` a parameter admits is admitted as the
  annotation.
- `let x: = e in b` SHALL refuse `invalid_syntax`/`unexpected-token` at the
  `=`, where a type is required; `let x: T e in b` SHALL refuse
  `invalid_syntax`/`unexpected-token` at the first token of `e`, where `=` is
  required; `let x T = e in b` SHALL refuse the same at `T`, where `:` or `=`
  is required.

### Forms

- S2 SHALL build the `let` as `Let { name, annotation, value, body }`, with
  `annotation` the type form of `T` when written and none otherwise; the
  unannotated form builds exactly as before.

### Checking

- With an annotation, the checker SHALL check `e` with `T` as its expected
  type, exactly as it checks an argument against a declared parameter type:
  an integer literal or arithmetic `e` takes `T`'s domain as its required
  interval and carries FSpec FR-149's range obligation, `Some(..)`, `none`
  and `absent` take `T` as their expected option type (FSpec FR-143), and a
  collection or record `e` is checked against `T`'s shape.
- An `e` whose type does not conform to `T` SHALL refuse
  `ill_typed`/`type-mismatch` at `e`; an arithmetic `e` whose proved
  interval is not inside an `Int[lo, hi]` annotation SHALL refuse
  `undefined_expression`/`unproved-range` at the arithmetic operator node,
  under `CheckMode::Linked` as for any declared-type position.
- In `b`, `x` SHALL have the type `T`, not the type of `e`. A `let` name is
  a stable path whatever its annotation (FSpec FR-146).
- An annotation that names no type SHALL refuse as an unresolved type name
  in a parameter type does, at `T`; an annotation of a type the unit's
  selected layers do not admit (for example `Clock` without the temporal
  layer) SHALL refuse `unsupported_construct` at `T` as the same type in a
  parameter does.
- Without an annotation, the checker SHALL type `x` by `e` as before.

### Formatting

- `format` SHALL keep the `:` and every annotation token in order, spacing
  them by the same rules that space a parameter's `name: type`, so formatting
  an annotated `let` and formatting it again give identical bytes, and the
  formatted source checks to the same checked package identity (FR-003).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-980-AC-1 | `function f using v(y: Int[0,2]): Integer pure { let x: Int[0,3] = y + 1 in x }` parses with no diagnostic, its `let` `Expression` node holds one `TypeReference` child whose text is `Int[0,3]`, and S2 builds a `Let` whose annotation is `Int[0, 3]`; the same body without `: Int[0,3]` builds a `Let` with no annotation. | Test (TC-2770) |
| FR-980-AC-2 | `let x: = 1 in x` refuses `invalid_syntax`/`unexpected-token` at the `=`; `let x: Int[0,3] 1 in x` refuses the same at `1`; `let x Int[0,3] = 1 in x` refuses the same at `Int`. | Test (TC-2770) |
| FR-980-AC-3 | Under `CheckMode::Linked`, AC-1's function checks and `x` has type `Int[0, 3]` in the body; with `y: Int[0,3]` it refuses `undefined_expression`/`unproved-range` at the `+`; `function g using v(p: Int[0, 9], q: Int[0, 9]): Int[0, 81] pure { let line: Int[0, 81] = p * q in line }` checks, while the same body with `let line = p * q in line` refuses `undefined_expression`/`unproved-range`. | Test (TC-2771) |
| FR-980-AC-4 | `let x: Boolean = 1 in x` refuses `ill_typed`/`type-mismatch` at `1`; `let o: Option<Int[0, 9]> = none in present(o)` checks; `let x: Undeclared = 1 in x` refuses as an unresolved type name at `Undeclared`. | Test (TC-2771) |
| FR-980-AC-5 | Formatting `function f using v(y: Int[0,2]): Integer pure { let x:Int[0,3]=y+1 in x }` keeps the token order `let`, `x`, `:`, `Int`, `[`, `0`, `,`, `3`, `]`, `=`, ...; a second formatting pass gives identical bytes; the formatted and the original source check to equal checked package identities. | Test (TC-2772) |

## Dependencies

- FR-002 owns the parser; FR-003 the formatter; FR-091 the forms.
- FSpec FR-501 item 3 states the annotated `let`; FSpec FR-149 the range
  obligation; FSpec FR-143 checking at an expected type.
