---
id: FR-611
title: "Refuse previous, next and weak until as keywords, naming the derived form"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-327
    type: references
  - target: ix://agent-ix/filament-specification/FR-092
    type: depends_on
---
# FR-611: Refuse previous, next and weak until as keywords, naming the derived form

## Description

FSL's S1 parser SHALL refuse the words `previous`, `next` and `weak` written as
temporal operators, with a message that names the derived form that expresses
the operator, because the temporal grammar has one spelling per operator
(FSpec FR-092 "Derived forms").

## Use case

An author used to LTL writes `previous holds(p)`. The parser refuses the word
and says to write `once[1,1] holds(p)`, which is strong previous, and the
author gets the meaning they wanted on the next try.

## Inputs

A temporal formula: the body of a `temporal`, `hyper` or `hybrid` clause.

## Outputs

An `invalid_syntax` diagnostic, or the parsed formula.

## Behavior

- If the identifier `previous` or `next` is followed by a formula where a
  temporal formula admits a unary temporal operator, then the S1 parser SHALL
  refuse `invalid_syntax` at the word with a message that names `once[1,1]`
  for `previous` and `eventually[1,1]` for `next`.
- If the identifier `weak` followed by `until` stands where a temporal formula
  admits a binary temporal operator, then the S1 parser SHALL refuse
  `invalid_syntax` located at `weak`, with a message naming
  `(p until[a,b] q) or always[a,b] p` and, under the infinite-trace profile,
  `(p until q) or always p`.
- The derived forms keep the meanings of FSpec FR-092: `once[1,1] p` is strong
  previous, read at position 0 through the pre-origin extension;
  `eventually[1,1] p` is next; the weak-until forms hold when `q` is reached
  within the interval with `p` until then, or `p` holds throughout it.
- `previous`, `next` and `weak` remain ordinary identifiers everywhere else
  (field names, parameter names, enum members), so `r.next` and a parameter
  named `previous` parse as before.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-611-AC-1 | `previous holds(c.level = 0)`, `always (holds(c.level = 1) implies previous holds(c.level = 0))` and `next holds(c.level = 1)` each refuse `invalid_syntax` at the word, with a message containing `once[1,1]` for `previous` and `eventually[1,1]` for `next`. | Test (TC-1542) |
| FR-611-AC-2 | `holds(c.level = 0) weak until holds(c.level = 2)` refuses `invalid_syntax` at `weak`, with a message containing `or always`. A record with a field named `next` and a formula `holds(c.next = 1)` parse with no diagnostic. | Test (TC-1542) |
| FR-611-AC-3 | Over every `Counter` trace of length 1 to 4 with `c.level` in 0..2, under the event-position false-extension profile, `holds(c.level = 1) implies once[1,1] holds(c.level = 0)` `on each` and `eventually[1,1] holds(c.level = 1)` `on each` equal a reference that reads the operand at position `i - 1` (false at position 0 for the atom) and at `i + 1` (false past closure); `(holds(c.level = 0) until[0,2] holds(c.level = 2)) or always[0,2] holds(c.level = 0)` equals a reference bounded weak until. | Test (TC-1542) |

## Dependencies

- FSpec FR-092 ("Derived forms", "Positions before the origin").
