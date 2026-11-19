---
id: FR-680
title: "Check indexing, tuple positions, source maps, option construction and quantity scaling"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-002
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-104
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-135
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-577
    type: references
  - target: ix://agent-ix/filament-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-143
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-144
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-149
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-680: Check indexing, tuple positions, source maps, option construction and quantity scaling

## Description

When checking a complete-V1 expression, the compiler SHALL parse, type, check,
lower and evaluate the indexing form `c[i]`, the tuple position read `p[n]`,
the source type `Map<K, V>[a, b]` with its reads and equality, the general
option constructor `Some(e)` and quantity scaling by an exact number exactly as
FSpec FR-142, FR-143, FR-144, FR-146 and FR-149 specify them.

## Inputs

S1 parse trees of complete-V1 units, their S2 forms, the S3 type environment
and definedness facts (FR-104, FR-577), and the S4 lowering context (FR-093).

## Outputs

Checked expressions with the FSpec result types, typed refusals with the FSpec
codes and causes at the FSpec loci, and checked-package nodes with the FSpec
operation identities.

## Behavior

**Indexing.** S2 SHALL build every complete-V1 `postfix` selection
`'[', expr, ']'` as an index form, whatever the operand, including a bare
parameter (`s[0]`) and a projected field (`r.xs[0]`); no index form is refused
as unrepresented or as an unknown required feature. S3 SHALL type it by the
operand's type per FSpec FR-144 "Indexing": a `Sequence` or `OrderedSet` with an
`Integer` or `Int[..]` index reads the element type, a `Map<K, V>` with an index
checked against `K` reads `V`, and a tuple with a `uint` literal reads that
position (FSpec FR-143 "Tuple position read"). A set, bag or any other operand
refuses `ill_typed`/`operator-ineligible` at the indexing expression, and a
wrong index type refuses `ill_typed`/`type-mismatch` at the index. S3 SHALL
record the FSpec FR-146 index obligation and, in `CheckMode::Linked`, refuse an
unproved one `undefined_expression`/`unproved-presence` at the index, the
refusal FR-230 already gives a clock-place selector. S4 SHALL lower a collection
or map index to `filament.op.collection.index` and a tuple read to
`filament.op.tuple.project` with its position member.

**Tuple positions.** `p.0` stays `invalid_syntax`/`unexpected-token` at `0`;
the position read is `p[n]` only.

**The key a checked position read carries.** The checked `TupleProject`
node SHALL name its tuple by the tuple type's node key, the content key that
the operand's checked type `Composite(key)` already carries (FR-092 "Source
tuple identity rows": the declaration's key is the type handle), together
with the zero-based position; it SHALL NOT name it by a domain-package
declaration key (`{package, node}`). A source tuple has a node key and no
domain-package identity, so a declaration key can name only a model tuple and
leaves every source tuple unkeyable, while the node key names a source tuple
and a model tuple alike (FR-094 keys model-owned type nodes) and is the key
the S4 lowering already reads for the position member. S3 SHALL set the key
from the operand's checked type, and S4 SHALL lower it to the package's
`{kind: "position", declaration, position}` member, `declaration` the emitted
node of that key (FSpec FR-322 "Position member"). The package wire form is
unchanged; only the checked node form names its tuple by node key.

**Source maps.** S1 SHALL parse `Map<K, V>` and `Map<K, V>[a, b]` as a type
reference wherever a type is written, S3 SHALL type it as the map type a
domain-package map type reads as (FR-056) and refuse a non-scalar key type
`ill_typed`/`operator-ineligible` at the key type. `size(m)`, `contains(m, k)`
and `m[k]` SHALL check and evaluate per FSpec FR-144 "Map values". S3 SHALL
admit `=` and `!=` over two maps of one map type under the FSpec FR-149
`Map` row instead of being refused for want of a relation.

**Map key guards.** S3 SHALL discharge the presence obligation of `m[i]` with
a `contains(m, j)` guard fact whose key is the same key as `i`, by FSpec
FR-146 "Accepted proof forms" item 4 and its index obligation: `i` and `j`
the same stable path at the same observation, or `i` and `j` scalar literals
whose values, checked against `K`, are the same map key. A literal key is
fixed in the source, so `contains(m, 1)` proves `m[1]` as surely as
`contains(m, k)` proves `m[k]`; it proves no read of another literal or of a
path.

**Option construction.** `Some(e)` SHALL be the option constructor in every
expression position, not only in a refinement row (FR-135). S3 SHALL check its
payload against the element type `T` of a unique expected type `Option<T>`, so
an integer literal payload adopts an `Int[..]` element type (`Some(3)` at
`Option<Int[0, 9]>` is admitted and `Some(30)` refuses `ill_typed`/`type-mismatch`
at `30`), and SHALL type it `Option<E>` from the payload's own type where no
unique expected type exists (FSpec FR-143 "The `Some` option constructor").

**Quantity scaling.** S3 SHALL admit `q * n`, `n * q` and `q / n` for a
non-affine `Quantity<u>` and an exact number `n` with result `Quantity<u>`,
record the nonzero obligation on `n` for `q / n`, refuse a float factor and
`n / q` `ill_typed`/`type-mismatch`, and S4 SHALL lower them to
`filament.op.quantity.scale` and `filament.op.quantity.scale_div` (FSpec FR-142
"Quantity scaling").

**Equality against a bounded integer.** FR-104 keeps FSpec FR-149's rule that a
`+`, `-` or `*` operand of `=` against an `Int[lo,hi]` operand is typed at that
range with a range obligation, discharged by FR-104 and FR-577 facts. The
`unproved-range` refusal SHALL name the derived interval of the arithmetic node,
the expected `Int[lo,hi]` and the operand that supplied it.

**Two-path term guards.** A comparison of a two-path `+` or `-` term against
an integer literal, such as `x + y <= 9`, SHALL add the sum or difference
bound FSpec FR-146 "Accepted proof forms" item 8 defines, and S3 SHALL
intersect the interval of a `+` or `-` node over the same two paths with it
wherever the node stands, including a call argument checked against an
`Int[lo,hi]` parameter. So `if x + y <= 9 then narrow(x + y) else 0`, for
`x`, `y` of `Int[0, 9]` and a parameter `Int[0, 9]` of `narrow`, checks.
Item 8 is the whole of this rule: a term of three or more operands, `!=`, and
any combination of two bounds add no fact.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-680-AC-1 | `s[0]` over a parameter `s: Sequence<Integer>[1, 3]`, `r.xs[0]` over a field `xs: Sequence<Integer>[1, 2]`, and `o[1]` over `o: OrderedSet<Integer>[0, 3]` under the guard `size(o) >= 2` check with result type `Integer` and lower to one `filament.op.collection.index` each; `s[0]` over `Sequence<Integer>[0, 3]` with no fact refuses `undefined_expression`/`unproved-presence` at `0`; `t[0]` over `t: Set<Integer>[0, 3]` and `n[0]` over `n: Integer` refuse `ill_typed`/`operator-ineligible`; `s[true]` refuses `ill_typed`/`type-mismatch` at `true`. | Test (TC-1820) |
| FR-680-AC-2 | For `tuple Pair(Boolean, Integer);`, `p[0]` checks as `Boolean` and `p[1]` as `Integer`, each lowered to `filament.op.tuple.project`; `p[2]` and `p[i]` refuse `ill_typed`/`operator-ineligible` at the index; `p.0` refuses `invalid_syntax` at S1. | Test (TC-1821) |
| FR-680-AC-3 | A parameter of type `Map<Int[0, 3], Integer>[0, 4]` parses and checks; `m[k]` under `contains(m, k)` checks as `Integer` and without the fact refuses `undefined_expression`/`unproved-presence` at `k`; `Map<Cfg, Integer>` over a record `Cfg` refuses `ill_typed`/`operator-ineligible` at `Cfg`; `=` over two `Map<Int[0, 3], Integer>[0, 4]` operands is admitted by `TypeEnvironment::check_equality`. | Test (TC-1822) |
| FR-680-AC-4 | `Some(3)` as the body of a function whose result is `Option<Int[0, 9]>`, and as the `=` operand against an `Option<Int[0, 9]>` parameter, checks; `Some(30)` there refuses `ill_typed`/`type-mismatch` at `30`; `Some(1)` in a state clause outside any refinement is the option constructor. | Test (TC-1823) |
| FR-680-AC-5 | With `ms` of scale `1/1000` over `s`, `d * 2`, `2 * d` and `d / 2` for `d: Quantity<ms>` check as `Quantity<ms>` at a `Quantity<ms>` result; `d / k` for `k: Int[0, 9]` with no fact refuses `undefined_expression`/`unproved-nonzero`; `d * f` for `f: Float64` and `2 / d` refuse `ill_typed`/`type-mismatch`. | Test (TC-1824) |
| FR-680-AC-6 | `c = a + b` over `a`, `b`, `c` of `Int[0, 10]` refuses `undefined_expression`/`unproved-range` at `a + b` with a message naming `[0, 20]`, `Int[0, 10]` and `c`; `convert<Integer>(c) = a + b` checks. | Test (TC-1825) |
| FR-680-AC-7 | For a source `tuple Pair(Boolean, Integer);` and for a model tuple `M::Slot` of a selected domain package, `p[1]` checks to a `TupleProject` node whose tuple key equals the node key of the operand's checked type `Composite(key)` and whose position is 1, for the source tuple as for the model tuple, never a `{package, node}` declaration key; the checked package of each carries `filament.op.tuple.project` with the member `{kind: "position", declaration, position: 1}`, `declaration` the emitted tuple node of that key. | Test (TC-2973) |
| FR-680-AC-8 | Over `m: Map<Int[0, 9], Integer>[0, 3]` and `k: Int[0, 9]`, `fsl check` of a function whose body is `if contains(m, 1) then m[1] else -1` succeeds at S4 with exit 0, as does `if not contains(m, 1) then -1 else m[1]`; `if contains(m, 1) then m[2] else -1`, `if contains(m, 1) then m[k] else -1` and `if contains(m, k) then m[1] else -1` each refuse `undefined_expression`/`unproved-presence` at the index, exit 20. | Test (TC-3063) |
| FR-680-AC-9 | With `function narrow using v(x: Int[0, 9]): Integer pure { x }` and `x`, `y`, `z` of `Int[0, 9]`, `fsl check` of `if x + y <= 9 then narrow(x + y) else 0` and of `if x - y >= 0 then narrow(x - y) else 0` succeeds at S4 with exit 0; `if x + y <= 10 then narrow(x + y) else 0`, `if x + y != 10 then narrow(x + y) else 0`, `if x + y + z <= 9 then narrow(x + y + z) else 0` and `if x + y <= 9 and y >= 2 then narrow(x + 2) else 0` each refuse `undefined_expression`/`unproved-range` at the argument, exit 20. | Test (TC-3064) |

## Dependencies

- FSpec FR-142, FR-143, FR-144, FR-146 and FR-149 own the typing, obligations,
  refusals and evaluation; FSpec FR-322 owns the checked-package carriers.
- FR-093 owns lowering; FR-104 and FR-577 own the definedness facts.
