---
id: FR-766
title: "Prove the result range of count<N> at check"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-003
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-046
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-145
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-044
    type: depends_on
---
# FR-766: Prove the result range of count<N> at check

## Description

`count<N>(x in c: p)` counts the occurrences of `c` for which `p` is true and
returns the count in the integer type `N` the author names (FR-091 assembles
`N` as the form's `type_argument`). The count of a source that can hold `m`
occurrences lies in `[0, m]`. Under `CheckMode::Linked` the checker SHALL prove
that interval lies inside `N`'s domain, and SHALL refuse the `count`
`undefined_expression`/`unproved-range` when it cannot, as it refuses a `sum<N>`
whose running totals are not proved members of `N` (FR-096, FSpec FR-145).

## Rationale

`sum<N>` and `count<N>` both name their result type, and the standard gives
both the same result-domain rule (FSpec FR-145, `state-contract.md`: the count's
domain contains `0..N`). A linked program whose `count` could leave its type
would otherwise pass `fsl check` and be refused only when a run meets a large
enough collection, which is the late failure a linked check exists to prevent.

The lower bound is `0` whatever the predicate is. Proving that a predicate holds
for some occurrence is a separate proof the checker does not attempt, so a
domain that omits `0` is refused. The refusal is
sound: every count the checker admits is a member of `N`.

Under `CheckMode::Kernel` a `count` is not proved, and S6a decides the final
count's membership in `N`. That decision is a refusal, `integer_out_of_domain`,
not the undefined outcome a `sum` meets. A `sum` is a fold whose every running
total must be a member of `N`, so a running total outside `N` leaves the fold
with no value in `N` (FR-096). A `count` has no running total in `N`: its
counter is an unbounded natural number and only the final count is retained into
`N`, which is the ordinary domain retain FSpec FR-044 refuses
`integer_out_of_domain`/`outside-domain`.

## Inputs

- A checked `Query{visit: Count}` node, its source `c`, its predicate and its
  result type `N`.
- The proved size interval of `c` at the node: the bound of `c`'s collection
  type, narrowed by the `size(c)` guard facts in scope, the same facts that
  discharge a `sum`'s prefix obligation.

## Outputs

- Under `CheckMode::Linked`, a checked `count` node typed `N`, or the
  located refusal `undefined_expression`/`unproved-range`.
- Under `CheckMode::Kernel`, a checked node whose S6a evaluation completes with
  a member of `N` or refuses `integer_out_of_domain`/`outside-domain`.

## Behavior

- Under `CheckMode::Linked`, the checker SHALL record a range obligation on every
  `count<N>` node: the proved interval is `[0, m]`, where `m` is the proved
  upper bound of `size(c)` at the node, or no upper bound when `c`'s size has
  none; the required interval is `N`'s domain.
- If the proved interval of a `count` node is not inside its required
  interval, then the checker SHALL refuse the node `undefined_expression`/`unproved-range`.
  The refusal carries both intervals (`Obligation::Range{required, proved}`).
- The checker SHALL narrow `m` by a guard that bounds `size(c)` in scope (for
  example `if size(c) <= 3 then count<Int[0, 3]>(x in c: p) else 0`), as it
  narrows a `sum`'s source size.
- The checker SHALL record the obligation for every collection kind:
  sequence, ordered set, set and bag.
- When `N` is `Integer`, the checker SHALL discharge the obligation, since its
  domain has no bounds.
- A `count` over a named population (`count(M::P: p)`) is typed from the
  population maximum (FR-391), so its type always contains the count and this
  obligation is discharged by construction.
- Under `CheckMode::Kernel`, the checker SHALL NOT record the obligation for a
  `count` outside a declared state clause: in a function body or in a
  temporal clause atom.
- In a state clause a package declares (`invariant`, `pre`, `post`), checked
  through `PackageDeclarations::check`, the checker SHALL record and decide the
  obligation under `CheckMode::Kernel` exactly as under `CheckMode::Linked`,
  and an unproved count range there SHALL refuse
  `undefined_expression`/`unproved-range` at check in both modes (FR-495:
  `Kernel` never admits a declared clause whose obligation
  `Linked` refuses). Such a `count` never reaches S6a outside `N`.
- Under `CheckMode::Kernel`, for a `count` outside a declared state clause, S6a SHALL return `Outcome::Refused(Refusal::IntegerOutOfDomain{target: N})`,
  located at the `count` node, for a final count outside `N` (FR-096-AC-8).
- A clause run compiles its unit under `CheckMode::Linked`, so when a clause's
  `count` is not proved, spine compile SHALL refuse the unit, and the clause
  run item carries category `refusal`, result `declined` and cause
  `{code: undefined_expression, kind: unproved-range}`.
- When a clause run's S6a outcome is a kernel refusal record, the outcome
  projection SHALL give the run item that record's code and cause (FR-286
  "Monitor documents from clause runs"), never a `null` cause.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-766-AC-1 | Under `CheckMode::Linked`, for `q` of `Sequence<Integer>[0, 5]`, `Bag<Integer>[0, 5]` and `Set<Integer>[0, 5]`, `count<Int[0, 3]>(x in q: x > 0)` refuses `undefined_expression`/`unproved-range` at the `count` with required `[0, 3]` and proved `[0, 5]`; the same count into `Int[0, 5]` and into `Integer` checks. | Test (TC-2195) |
| FR-766-AC-2 | Under `CheckMode::Linked`, `if size(q) <= 3 then count<Int[0, 3]>(x in q: x > 0) else 0` checks for the AC-1 sources, and `if size(q) <= 4 then count<Int[0, 3]>(x in q: x > 0) else 0` refuses `unproved-range` with proved `[0, 4]`. | Test (TC-2195) |
| FR-766-AC-3 | Under `CheckMode::Linked`, `count<Int[1, 3]>(x in q: true)` for `q` of `Sequence<Integer>[0, 3]` refuses `unproved-range` with required `[1, 3]` and proved `[0, 3]`; `count<Int[0, 5]>(x in q: true)` for an unbounded `q` refuses `unproved-range` with a proved interval that has no upper bound. | Test (TC-2195) |
| FR-766-AC-4 | Under `CheckMode::Kernel`, `count<Int[0, 3]>(x in q: x)` for `q` of `Sequence<Boolean>[0, 5]` checks, and evaluating it over five `true` occurrences refuses `IntegerOutOfDomain` with target `Int[0, 3]`, whose record is `integer_out_of_domain`/`outside-domain` with `expected` `Int[0, 3]`. | Test (TC-2195) |
| FR-766-AC-5 | A unit over FR-108's `Config` model declaring `type One = Int[0, 1];`, `function pair using v(a: Int[0, 1000]): Sequence<Int[0, 1000]>[0, 2] pure { sequence[a, a] }` and `invariant AtMostOne using v on Config::ConfigVersion at current { count<One>(m in pair(self.versionNumber): m >= 0) <= 1 }` refuses at compile `undefined_expression`/`unproved-range` at the `count`, exit 20; with `type One = Int[0, 2];` it compiles. | Test (TC-2195) |
| FR-766-AC-6 | The AC-5 `Int[0, 1]` unit run as a clause run of `AtMostOne` returns a compile disposition, and the run item built from it carries category `refusal`, result `declined` and cause `{code: undefined_expression, kind: unproved-range}`. | Test (TC-2195) |
| FR-766-AC-7 | A clause-run report whose disposition is an S6a kernel refusal record `integer_out_of_domain`/`outside-domain` projects to a run item with category `refusal`, result `declined` and cause `{code: integer_out_of_domain, kind: outside-domain}`, never a `null` cause. | Test (TC-2195) |
| FR-766-AC-8 | Through `PackageDeclarations::check` under `CheckMode::Kernel` and under `CheckMode::Linked`, a unit over FR-108's `Config` model declaring `type Zero = Int[0, 0];` and `invariant I using v on Config::ConfigVersion at current { count<Zero>(m in sequence[self.versionNumber]: m >= 0) = 0 }` refuses `undefined_expression`/`unproved-range` at the `count` in both modes, and no checked graph is returned; the same `count<Zero>` in the body `function f using v(q: Sequence<Integer>[0, 1]): Int[0, 0] pure { count<Int[0, 0]>(m in q: m >= 0) }` refuses `unproved-range` under `CheckMode::Linked` and checks under `CheckMode::Kernel`, where S6a meets the out-of-domain count (AC-4). | Test (TC-2240) |

## Notes

Spine compile, and so every clause run, checks under `CheckMode::Linked`, and
no linked integer construct reaches the S6a refusal `integer_out_of_domain`:
every integer retain is proved at check. The kernel run-time refusal applies
only to a `count` outside a declared state clause. A test fixture that needs
an evaluation that meets that refusal at run, such as a `holds` atom refused
at some trace position (FR-327-AC-6) or a refused monitor condition, SHALL
write the `count<Int[0, 0]>` atom in a temporal clause atom (or a function
body evaluated at run) and build its package with
`PackageDeclarations::check(CheckMode::Kernel, ..)` and link it. A fixture
whose `count` sits in a declared `invariant`, `pre` or `post`, such as a
separating-witness domain written in an invariant body (FR-268-AC-3), cannot
reach the run-time refusal under either mode: its unproved count refuses at
check (FR-495), and the fixture SHALL expect that check refusal or move the
atom to a temporal clause. Compiling any such atom from source through spine
compile is refused at check by this requirement.

## Dependencies

- FSpec FR-145 owns the `count` and `sum` result-domain rules and the
  `unproved-range` refusal; FSpec FR-044 owns `integer_out_of_domain`.
- FR-091 assembles `N`; FR-093 lowers the `count`; FR-096 owns the kernel
  refusal record and the `sum` contrast; FR-286 owns the clause-run item cause.
