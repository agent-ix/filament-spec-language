---
id: FR-701
title: "Admit a set or bag fold only through the commutative step catalog"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-001
    type: implements
  - target: ix://agent-ix/filament-specification/FR-145
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-144
    type: references
---
# FR-701: Admit a set or bag fold only through the commutative step catalog

## Description

`fold<A>(acc, x in c: step, identity: i)` and `reduce<A>(acc, x in c: step)`
over a `Sequence` or an `OrderedSet` admit any pure total `step` of type
`A`. Over a `Set` or a `Bag`, `check` admits a step only from FSpec FR-145's
closed syntactic catalog: a body `acc OP t` or `t OP acc` where `acc` does
not occur in `t`, `t` has type `A`, and `OP` with `A` is `+` or `*` on
`Integer`, or `and` or `or` on `Boolean`.

The rule is by design. A set or bag has no authored order, so a fold over it
is a function of its members only when the step is commutative and
associative, and only a step from the catalog is known to be so without a
proof. A bounded accumulator (`Int[..]`, `Rational`, `Decimal`, a quantity)
is refused because whether an intermediate accumulator stays inside its
bounds would depend on the visit order. The restriction is on the step and
the accumulator, not on the element: the element may have any type, as long
as `t` has type `A` (for example `acc + convert<I>(x)` over a `Set<Int[0, 9]>`
with `type I = Integer`, or `acc and x.ok` over a set of references whose
`ok` field is `Boolean`).

## Inputs

- A checked `fold` or `reduce` over a collection, its accumulator type `A`,
  its step and, for `fold`, its identity.

## Outputs

- The admitted `fold`/`reduce` node, or a typed refusal.

## Behavior

- `check` SHALL type the form first (`A`, `identity:` presence, the step and
  identity types), refusing `ill_typed`/`type-mismatch` there, and only then
  apply the catalog to a set or bag source, as FSpec FR-145 orders it.
- Over a `Set` or `Bag`, `check` SHALL admit exactly the catalog steps above
  and SHALL refuse every other step `ill_typed`/`operator-ineligible`, naming
  the failed commutativity and associativity obligation, including a step
  that calls a function and a step over a bounded accumulator.
- Over a `Sequence` or `OrderedSet`, `check` SHALL admit any pure total step
  of type `A`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-701-AC-1 | With `type I = Integer;` and `type B = Boolean;`, `fold<I>(acc, x in s: acc + convert<I>(x), identity: 0)` over `s: Set<Int[0, 9]>[0, 5]`, `fold<I>(acc, x in s: convert<I>(x) * acc, identity: 1)` over the same set and `fold<B>(acc, x in t: acc and x, identity: true)` over `t: Set<Boolean>[0, 2]` each check. | Test (TC-1907) |
| FR-701-AC-2 | `fold<I>(acc, x in s: acc - convert<I>(x), identity: 0)` over the set, a step `add(acc, convert<I>(x))` that calls the pure function `function add using v(a: I, b: I): I pure { a + b }`, and `fold<R>(acc, x in r: acc + x, identity: rational(0, 1))` over `r: Set<Rational[0, 1; 1, 2]>[0, 2]` with `type R = Rational[0, 4; 1, 2]` each refuse `ill_typed`/`operator-ineligible`; the subtraction step over `q: Sequence<Int[0, 9]>[0, 5]` checks. | Test (TC-1907) |

## Dependencies

- FSpec FR-145 "Fold and reduce admission" owns the catalog; FSpec FR-144
  owns the collection kinds and their order.
