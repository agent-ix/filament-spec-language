---
id: FR-982
title: "End a window-aggregate threshold at the next temporal connective"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-002
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-600
    type: references
  - target: ix://agent-ix/filament-specification/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-784
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-414
    type: depends_on
---
# FR-982: End a window-aggregate threshold at the next temporal connective

## Description

FSL's parser SHALL read the threshold of an `AggregateAtom` as a `sum`
(the operand a comparison takes), so that a following `and`, `or` or
`implies` is a temporal connective joining the atom to the next temporal
operand, and the checker SHALL type a parenthesized threshold against the
aggregate's value.

## Rationale

The parser read the threshold as a full `Expression`, so it consumed every
connective after it. `count where holds(p) over past[0,1] >= 1 and holds(q)`
was refused as a syntax error, and `... >= s.k and s.ok` was accepted as the
Boolean threshold `(s.k and s.ok)`: the unit checked and meant something the
author did not write. The standard makes the threshold a `sum` (FSpec FR-501
item 1, FSpec FR-784), which ends it at the first connective and lets an
aggregate atom stand in any temporal operand position. There is no switch
that keeps the old reading: it was wrong, not a variant.

## Inputs

A temporal or hyper clause formula holding aggregate atoms.

## Outputs

A CST in which each `AggregateAtom` spans its term, comparison and `sum`
threshold, and each connective after it is a `TemporalConjunction`,
`TemporalDisjunction` or `TemporalImplication` node; or a located refusal.

## Behavior

- The `AggregateAtom` production SHALL be `AggregateTerm`, a comparison
  operator, and the `sum` level of the expression grammar. A threshold
  holding `and`, `or`, `implies`, `let`, `if` or a comparison is written in
  parentheses.
- When `and`, `or` or `implies` follows the threshold, it SHALL be read at the
  temporal level with their precedence and associativity (FSpec FR-784
  item 1).
- The operand after such a connective is a temporal operand; a bare value
  expression there SHALL refuse `invalid_syntax`/`unexpected-token` at its
  first token, and no CST holds a threshold spanning a connective.
- An aggregate atom SHALL parse as the first, a middle or the last operand of
  a connective and as the operand of a temporal operator.
- `holds(count where ..)` SHALL refuse `invalid_syntax`/`unexpected-token` at
  `where` (FSpec FR-784 item 4).
- The checker SHALL refuse a parenthesized threshold whose type is not
  numeric `ill_typed`/`type-mismatch` at the threshold, and keep FSpec
  FR-414's `unit-mismatch` for a threshold of another dimension.
- The complete-grammar corpus SHALL hold an aggregate atom in a non-last
  position.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-982-AC-1 | `count where holds(s.ok) over past[0,1] >= 1 and holds(s.done)`, `... >= 1 or holds(s.done)` and `... >= 1 implies holds(s.done)` each parse with one `AggregateAtom` whose text ends at `1`, inside one `TemporalConjunction`, `TemporalDisjunction` and `TemporalImplication` node respectively; `... >= s.k + 1` parses with the atom's text ending at `s.k + 1`. | Test (TC-2776) |
| FR-982-AC-2 | `... >= s.k and s.ok`, `... >= s.k or s.ok` and `... >= s.k implies s.ok` each refuse `invalid_syntax`/`unexpected-token` at the `s` of `s.ok`, and none yields an `AggregateAtom` spanning the connective. | Test (TC-2776) |
| FR-982-AC-3 | `... >= (s.k and s.ok)` parses with the parenthesized threshold inside the `AggregateAtom`; `holds(s.done) and count where holds(s.ok) over past[0,1] >= 1 and holds(s.ok)` parses with the atom as the middle operand; `always holds(count where holds(s.ok) over past[0,1] >= 1)` refuses `invalid_syntax`/`unexpected-token` at `where`. | Test (TC-2777) |
| FR-982-AC-4 | Over a record trace subject `s` with `k: Int[0, 9]`, `ok: Boolean` and `done: Boolean`, under the event-position false-extension profile, the clause formula `count where holds(s.ok) over past[0,1] >= (s.k + 1) and holds(s.done)` checks, and `count where holds(s.ok) over past[0,1] >= (s.ok and s.done)` refuses `ill_typed`/`type-mismatch` at `(s.ok and s.done)`. Under the timestamped-event finite-window profile, on a clock `"events"` over a unit declaring `dimension Time;` and `unit ms : Time = rational(1, 1000);`, the same two formulas with the window `past[0 ms, 1000 ms]` check and refuse identically. No formula carries an interval-less temporal operator, which neither profile admits (FSpec FR-090-AC-7, FR-048-AC-9). | Test (TC-2777) |

## Dependencies

- FSpec FR-501 item 1 and FSpec FR-784 state the threshold rule; FSpec FR-414
  owns aggregate atoms and their threshold dimension.
- FR-002 owns the parser; FR-600 keeps its production inventory equal to the
  standard's.
