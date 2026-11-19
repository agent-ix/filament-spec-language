---
id: FR-1040
title: "Check and lower clock-place references and presence reads in population forms"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-018
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-139
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-391
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-829
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-772
    type: depends_on
---
# FR-1040: Check and lower clock-place references and presence reads in population forms

## Description

A population form (`count`, `sum`, `exists`, `all` and `only` of FR-139 in a
refinement row, and the forms FR-391 admits in a state clause) may read the
clock places of the objects it visits through `ref(p)` and `present(p)`. When
a population form reads a clock place, FSL SHALL check each such read at S3 and lower it at S4 to the terms FSpec
FR-829 and FR-322 "Presence" define, over a timed model as over any model.
The same holds for every other position a data term stands in. The
meaning is FR-230's: `ref(p)` is the place id of the instance `p` denotes and
`present(p)` its presence. Neither reads a clock value.

### Decision: a place reference is the place id, per instance

Inside a population form the owner term of `ref(x.f)` and `present(x.f)` is
the binder `x`. The read is evaluated at each object the binder visits and
gives that object's place id or presence. It is never the clock's value at
the current valuation: a clock holds no data value (FSpec FR-322 "Data part
of a clock-bearing record, tuple or union"), and the place id is the only
data that names a clock. This is what lets an author count the objects whose
clock a field names, or pick one object's clock with `only`, without reading
a clock outside a clock constraint. The binder visits only the objects the
population holds, so there is no absent owner to test.

## Behavior

- **Places with an id.** S3 SHALL check `ref(p)`, for `p` a place
  expression whose path names a clock place of its owner's type with no
  `Map` level, as a `PlaceReference` node over the owner, the place and one
  selector term for each slot step, typed `Reference<C>` with `C` the clock
  type the place's declared type names. S4 SHALL lower it to FSpec FR-829's
  `filament.op.clock_reference.ref` application, typed by the
  `scalar_type`/`clock_reference` node. A `Reference<Clock>` field,
  parameter, result or element is typed by that node, so a function whose
  signature holds one has a node like any other function.
- **Paths through a reference.** A place path that crosses an object
  reference (`ref(self.r.x)`, `ref(self.r.ts[0].x)`, `present(self.r.v)`)
  the checker SHALL check with the data read up to the last object reference on the path
  (`self.r`) as the owner, carried as the `filament.op.model.deref`
  application over that read (FSpec FR-829 "Owner terms"), and the remaining steps as a place path of that
  object's type. Several reference hops chain the same way. An owner read of
  type `Option<Reference<T>>` is not of the owner type and refuses
  `ill_typed`/`type-mismatch` at the `ref` or `present`; `value(self.r)` reads
  it under FR-146's presence obligation.
- **Presence reads.** `present(p)` of a guarded place SHALL lower to FSpec
  FR-322's `presence_read` over the place's `clock_place` template, the owner
  term and the slot terms, whichever owner term it has: `self`, another
  object, a parameter, a population binder or a data read through a
  reference. `forall(e in p: present(e))` and `exists` over a clock
  collection place SHALL lower to the quantifier over FSpec FR-772's
  slot-domain application at that owner term, with the `presence_read` of
  `e` as its body, in a population form and in every other data position. The
  `clock_place` node a presence read or a place reference names is the one
  the model's timed model lists (FR-230 "Places"). A clock place outside a
  dense timed model is refused at its field (`clock-without-dense-time`), so
  a place the timed model does not list is a checker defect. It is reported
  as an internal fault, never as a user refusal.
- **No presence obligation on `ref`.** `ref` of a guarded `Option<Clock>`
  or union-case place SHALL check whether or not the place is present. A slot
  place keeps FR-230 "Selection"'s obligation on its selector: `ref(x.q[i])`
  checks where `i < size(x.q)` is established by the population form's
  `where` condition, by a conjunct guarding the read under FR-146's rules,
  by the precondition or by the time invariant's predicate. Otherwise it
  refuses `undefined_expression`/`unproved-presence` at the selector.
- **Refusals that remain.** These forms are wrong, and each SHALL refuse with
  the typed cause shown:
  - a population form whose condition or body reads a clock value (`x.c` in
    a value position, `value(x.v)` outside a clock constraint, a
    clock-bearing value read whole) refuses
    `invalid_timed_model`/`clock-read-outside-constraint` at the read
    (FSpec FR-415);
  - `ref` of a place under a `Map` value refuses `ill_typed`/`type-mismatch`
    at the `ref` (FR-230 "Place identity");
  - `ref` of a path that names no clock place of the owner's type refuses
    `ill_typed`/`type-mismatch` at the `ref`;
  - a `sum` whose summand is a `Reference<Clock>`, an object reference, a
    population or any other non-summable type refuses `ill_typed`/`type-mismatch`
    at the summand at S3, so S4 never sees such a summand;
  - `<` and the other orderings over two place ids refuse
    `ill_typed`/`operator-ineligible`;
  - a call to a clock-reading declaration (FSpec FR-415) in a population
    form, or in any other data position, refuses
    `invalid_timed_model`/`clock-read-outside-constraint` at the call.
- **Calls lower after their callee.** S4 SHALL lower every function before a
  body that calls it, a recursive group through placeholders, so a call
  always finds its callee's node. A callee whose own lowering is refused
  leaves the unit refused with that refusal, and the call adds no second
  refusal.

With these rules no `unknown_required_feature`/`unsupported-feature` or
`unsupported_construct`/`not-yet-implemented` refusal remains on a
place reference, a presence read, a population sum operand or a call. Each
form either lowers or refuses with one of the typed causes above.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1040-AC-1 | Over FSpec FR-415-AC-15's `Call` model, `ref(self.x)` and `ref(self.q[0])` (under `size(self.q) >= 1`) in a postcondition write `self.r = ref(self.x)` and in a precondition `self.r = ref(self.q[0])` check as `PlaceReference` nodes typed `Reference<Clock>` and lower to `filament.op.clock_reference.ref` applications with `result_type` the `scalar_type`/`clock_reference` node, and the unit compiles with no `unsupported-feature` refusal. | Test (TC-3084) |
| FR-1040-AC-2 | With `peer: Reference<Call>`, `ref(self.peer.x)` and `ref(self.peer.ts[0].x)` (where `ts: Sequence<Timer>[0, 2]`, `size(self.peer.ts) >= 1` established) check and lower with the owner term the `filament.op.model.deref` application over the `peer` field read; with `peer: Option<Reference<Call>>`, `ref(self.peer.x)` refuses `ill_typed`/`type-mismatch` at the `ref` and `ref(value(self.peer).x)` under `present(self.peer)` checks. | Test (TC-3084) |
| FR-1040-AC-3 | In a refinement row over `calls = {c1, c2}`, `only(c in M::calls where c.n = 2: ref(c.x))` and `count(c in M::calls where ref(c.x) = self.r)` check, lower with the dereferenced binder `c` as owner term, and evaluate as FSpec FR-829-AC-2 states; the same `count` in a state invariant (FR-391) checks and lowers. | Test (TC-3084) |
| FR-1040-AC-4 | `count(c in M::calls where present(c.v))`, `sum(c in M::calls: if present(c.v) then 1 else 0)`, `all(c in M::calls: exists(e in c.q: present(e)))` and `exists(c in M::calls: forall(e in c.q: present(e)))` check, lower to `presence_read` nodes whose owner term is the dereferenced binder `c` and, for the slot quantifiers, to FSpec FR-772's slot-domain application at `c`, and evaluate as FSpec FR-829-AC-3 states. | Test (TC-3084) |
| FR-1040-AC-5 | `ref(self.v)` over `v: Option<Clock>` with no presence premise checks; `ref(c.q[i])` in `count(c in M::calls where i < size(c.q) and ref(c.q[i]) = self.r)` checks; `ref(c.q[i])` in `count(c in M::calls where ref(c.q[i]) = self.r)` refuses `undefined_expression`/`unproved-presence` at `i`. | Test (TC-3084) |
| FR-1040-AC-6 | Negative: `sum(c in M::calls: c.x)` and `only(c in M::calls: c.x)` refuse `invalid_timed_model`/`clock-read-outside-constraint` at `c.x`; `sum(c in M::calls: ref(c.x))` and `sum(c in M::calls: M::calls)` refuse `ill_typed`/`type-mismatch` at the summand; `ref(c.m[k].x)` over a `Map` value refuses `ill_typed`/`type-mismatch` at the `ref`; `ref(c.x) < ref(c.y)` refuses `ill_typed`/`operator-ineligible`; and a call `late(c.x)` to a function `late(t: Clock): Boolean` in a `count` condition refuses `invalid_timed_model`/`clock-read-outside-constraint` at the call. | Test (TC-3084) |
| FR-1040-AC-7 | A pure function `same(a: Reference<Clock>, b: Reference<Clock>): Boolean { a = b }` has a function node whose parameters are typed by the `scalar_type`/`clock_reference` node, and a refinement row or clause calling it lowers its call to that node; with the function's body made ill-typed, the unit refuses once at the function body and not at the call. | Test (TC-3084) |

## Dependencies

- FR-230: clock places, place identity, selection and presence.
- FR-139 and FR-391: the population forms in refinements and state clauses.
- FR-093: lowering of checked value expressions to FSpec FR-322 terms.
- FSpec FR-829: the place-reference application and the `clock_reference` type node.
- FSpec FR-322 "Presence" and FSpec FR-772: the presence read and the slot domain.
