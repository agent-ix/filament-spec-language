---
id: FR-983
title: "Read Some(e) as the present-option constructor in every expression"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-001
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-specification/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-375
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-143
    type: depends_on
---
# FR-983: Read Some(e) as the present-option constructor in every expression

## Description

FSL SHALL read an unqualified call spelled exactly `Some` with one argument
as the present-option constructor in every expression position (a function
body, a state clause, a temporal `holds`, a refinement row), and SHALL leave
every other call name, including `some`, `Something` and a qualified
`M::Some`, an ordinary call.

## Rationale

The standard first wrote `Some(e)` only inside a refinement row, while
`absent` and `none` denote the empty option everywhere; without a general
present constructor a function could not return a present option. The
standard now makes `Some(e)` the constructor in every position (FSpec FR-501
item 4, FSpec FR-375), which is what the parser already accepts. This
requirement fixes the boundary of the spelling so an author's own function
named `some` or `Something`, or a model function `M::Some`, keeps its
meaning.

## Inputs

A complete-V1 unit holding calls spelled `Some`, `some`, `Something` or a
qualified name ending in `Some`.

## Outputs

A checked option constructor, an ordinary call, or a located refusal.

## Behavior

- An unqualified call whose name is exactly the three bytes `Some` SHALL be
  the present-option constructor, checked as FR-680 "Option construction"
  states: typed `Option<E>` for an argument of type `E` where no option type
  is expected, and checked at `P` where `Option<P>` is expected.
- A function a unit declares with the name `Some` SHALL NOT be callable by
  that name; the declaration itself is admitted, and every `Some(..)` call
  is the constructor.
- `Some` with zero or two or more arguments SHALL refuse
  `ill_typed`/`type-mismatch` at the call; the outcome document of the check
  is a refusal, exit 20.
- Every other call name, whatever its case or prefix (`some(..)`,
  `Something(..)`), and every qualified call (`M::Some(..)`), SHALL resolve
  as an ordinary call; an unresolved one refuses as any unresolved call
  does.
- Inside a refinement row `Some(e)` keeps the meaning FSpec FR-375 gives it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-983-AC-1 | `function s using v(b: Int[0,9]): Option<Int[0,9]> pure { Some(b) }` checks; `function p using v(b: Integer): Boolean pure { present(Some(b)) }` checks; `invariant I using v on Config::ConfigVersion at current { present(Some(self.versionNumber)) }` over FR-108's `Config` model checks. | Test (TC-2778) |
| FR-983-AC-2 | `function some using v(x: Integer): Integer pure { x }` with `function g using v(): Integer pure { some(1) }`, and `function Something using v(x: Integer): Integer pure { x }` with `function h using v(): Integer pure { Something(1) }`, each check and `g()` and `h()` return `1`; `function k using v(): Integer pure { Somex(1) }` with no such declaration refuses as an unresolved call. | Test (TC-2778) |
| FR-983-AC-3 | `Some(b, b)` and `Some()` refuse `ill_typed`/`type-mismatch` at the call, and the `check` outcome document of either unit has category `refusal`, exit 20, with that code and cause. | Test (TC-2779) |

## Dependencies

- FR-680 owns option construction checking; FR-286 the outcome document.
- FSpec FR-501 item 4 and FSpec FR-375 make `Some(e)` general; FSpec FR-143
  owns checking at an expected type.
