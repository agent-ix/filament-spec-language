---
id: FR-172
title: "Check hyper clauses over behaviours and relations over model executions at S3"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-021
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-171
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-361
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-395
    type: depends_on
---
# FR-172: Check hyper clauses over behaviours and relations over model executions at S3

## Description

The S3 TemporalTrace `check` SHALL check a `hyper` clause over `behaviours`
and a `relation` over model executions (ADR-023 §1): bind each trace
variable to a model alias, bind object parameters to population universes,
type indexed atoms and step labels, split the `match` block into its
universal and existential parts, check each variable's fairness set, check
the `align skip` list, and check each execution binding. The checked
clause's identity binds everything that changes its meaning (ADR-023 HV-7).

## Use case

A verification operator writes noninterference over one model, the same
alias twice, and a power comparison over two models. The checker binds each
trace to its model, refuses an atom that reads one model's field through
the other model's trace, and refuses a quantifier with no model, each at its
span.

## Inputs

- `HyperBehavioursForm` and `ModelRelationForm` (FR-171).
- The unit's model aliases and the admitted domain packages, with each
  model's operations, populations and fields (FR-103).
- The unit's resolved temporal profile selection (FR-110).

## Outputs

```rust
pub struct CheckedHyperClause {
    pub aliases: Vec<ModelAlias>,                  // distinct aliases, in first-use order
    pub parameters: Vec<ObjectParameter>,          // (name, alias, declared type, overlapping populations)
    pub quantifiers: Vec<CheckedQuantifier>,       // (kind, variable, alias, fairness set)
    pub mu_universal: Vec<CheckedStepPredicate>,   // μ_U conjuncts
    pub mu_existential: Vec<CheckedStepPredicate>, // μ_E conjuncts
    pub align_skip: Vec<DeclarationKey>,           // empty for lockstep
    pub body: CheckedTemporalFormula,              // over indexed atoms
}

pub struct CheckedModelRelation {
    pub executions: Vec<(Name, DeclarationKey)>,   // each a model operation, in order
    pub body: CheckedExpression,
}
```

- A typed `CheckRefusal` with a span, and no checked clause, on refusal.

## Behavior

### Trace variables and aliases

- S3 SHALL resolve each quantifier's `of` alias to a model alias of the
  unit. An alias that names no model SHALL refuse
  `missing_declaration`/`missing-name` at the alias.
- If a quantifier of a `behaviours` clause has no `of`, then S3 SHALL
  refuse `unsupported_construct`/`expression-form` at the quantifier. If a
  quantifier of a finite trace domain clause has `of`, then S3 SHALL refuse
  the same code at the quantifier (ADR-023 HS-2).
- Two quantifiers that name the same alias SHALL range over the same
  subject; quantifiers that name different aliases SHALL range over
  different subjects.

### Object parameters

- S3 SHALL resolve each object parameter `v: V::T` to the object type `T` of
  alias `V`'s model and to every population of that model that overlaps `T`
  (FR-125 Instancing; FSpec FR-395 "Object parameters", FR-361 "Instancing").
  S3 SHALL NOT refuse a parameter as ambiguous because more than one
  population overlaps `T`.
- S3 SHALL NOT instantiate: the universes and initial states are request
  inputs. FR-125 forms one instance per admitted key of each parameter, and
  the product of those over several parameters (ADR-023 HS-3). A clause with
  a parameter that has no admitted key, an empty universe included, is
  reported `NoInstance` by FR-125 Instancing, the same as a temporal clause
  or state-graph claim.
- This requirement does not change the check of a parameter whose type no
  population of the model overlaps, which S3 refuses. FSpec FR-395 states
  `NoInstance` for it; the difference is open.
- If an object parameter names a population of an alias the clause does not
  bind, then S3 SHALL refuse `ill_typed` at the parameter (FSpec FR-395).

### Indexed atoms and step labels

- Every model member read in a body atom SHALL carry exactly one trace
  variable, `e @ t`. A member read with no trace variable SHALL refuse
  `ill_typed` at the read. One atom MAY read several trace variables.
- The operand `e` of `e @ t` MAY be a compound expression, such as a `let`,
  an `if` or a parenthesised combination. The `@ t` then scopes over `e`:
  every member read inside `e` that is enclosed by no inner `@` carries `t`.
  Such a read is the field selection of a chain whose root is an object
  parameter, and also a field selection on a `deref`, whatever the root. A
  member read that no `@ t` encloses, as the `.l` of `deref(value(v.next @
  a)).l` where the `@` closes inside the `deref` operand, SHALL refuse
  `ill_typed` at the read. Such a read gets no component, and S3 does not
  give it the component of the reference ([FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)).
- If a member read carries a trace variable bound to a model other than the
  member's, then S3 SHALL refuse `ill_typed` at the read (ADR-023 HS-4).
- S3 SHALL type `t.step.op` as the operation-or-`stutter` label of `t`'s
  model, `t.step.receiver` as its receiver key, and `t.step.O.p` as
  `Option<T>` for parameter `p: T` of operation `O` of `t`'s model.
- The receiver key SHALL be typed `Option<Reference<T>>`, `T` being the one
  object type that declares every operation of `t`'s model that has a
  receiver (FSpec FR-395 "Step labels"). If those operations are declared by
  more than one object type, then S3 SHALL refuse a read of
  `t.step.receiver` with `unsupported_construct`/`expression-form` at the
  read, because no single receiver type exists for it
  ([FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)).

### Match

- S3 SHALL refuse a `match` conjunct that reads state,
  `unsupported_construct`/`expression-form` at the read: `μ` reads step
  labels only (ADR-023 HS-5, FSpec FR-395).
- S3 SHALL check a call of a declared function in a `match` conjunct as a
  value call, not as a state read. Each of its arguments SHALL obey the same
  `match` rules, reading step labels, literals and calls only. An argument
  that reads state SHALL be refused `unsupported_construct`/`expression-form`
  at that read ([FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)).
- S3 SHALL flatten `μ` into its top-level conjuncts and place each conjunct
  that reads only universal variables in `mu_universal` and every other
  conjunct in `mu_existential` (ADR-023 HM-3).
- Inside `match`, S3 SHALL check a qualified name that resolves to an
  operation `O` of the model alias bound to trace variable `t`, written as
  an operand of `=` or `!=` whose other operand is `t.step.op`, as the label
  constant `O` of `t.step.op`'s type, not as a state read. The comparison
  holds at a step exactly when the step is an `O` step; it is false at the
  terminal stutter step, which only `stutter` matches.
- If such a qualified name names an operation of a model alias other than
  the alias bound to the compared `t`, or appears anywhere other than as an
  operand of an `=` or `!=` with a `t.step.op` label, then S3 SHALL refuse
  `ill_typed`/`type-mismatch` at the qualified name.
- A qualified name in `match` that resolves to no operation and no member
  SHALL refuse `missing_declaration`/`missing-name` at the name; one that
  resolves to a model member SHALL keep the state-read refusal above.

### Fairness and profile

- S3 SHALL check each quantifier's fairness set as FR-123 checks a clause's
  fairness set, over the operations of that quantifier's model, with
  `whole` unmarked. A constraint naming an operation of another model SHALL
  refuse `missing_declaration`/`missing-name` at the operation.
- A `behaviours` clause under the infinite-trace profile SHALL check its
  body as an infinite-trace formula over indexed atoms, interval operators
  admitted as FR-123 admits them. A `behaviours` clause under any other
  profile SHALL check and record its profile. Under the event-position
  false-extension profile FR-173 then classifies it into a form read over
  closed finite traces; under every other profile negotiation settles it
  V-8 (FR-173).

### Align skip

- S3 SHALL resolve each `align skip` entry to an operation of a bound
  alias's model. An operation of no bound alias and an operation named twice
  SHALL refuse `unsupported_construct`/`expression-form` at the entry
  (ADR-023 PA-1).
- An empty list, `align skip { }`, is admitted by the grammar (FSpec FR-339
  shared grammar `align-skip`, whose entry list is optional) and parses; it
  is S3's to refuse, not S1's. S3 SHALL refuse it
  `unsupported_construct`/`expression-form` at the list, the span of the
  `align skip { }` item from `align` through its closing `}` (ADR-023
  PA-1). The outcome has `last_stage` S3, category `unsupported` and exit
  code 21; no S1 `invalid_syntax` diagnostic is given for it.
- If a clause with `align skip` carries a non-empty fairness set on any
  variable, then S3 SHALL refuse `unsupported_construct`/`expression-form`
  at the constraint (ADR-023 PA-3).

### Relations over model executions

- S3 SHALL check an execution binding whose qualified name resolves to an
  operation of a model alias as a model execution binding. A binding to any
  other qualified name SHALL keep FSpec FR-191's finite reading.
- The body SHALL check under the value profile and MAY read, per execution
  `x`, its arguments `x.p`, its receiver's pre-state `pre(x.self.f)`,
  post-state `x.self.f` and result `x.result` (ADR-023 HS-8).

### Body expressions

- S3 SHALL type every expression of a `hyper` body or a `relation` body
  that is not an indexed atom, a step label or an execution read under the
  value typer's rules: expected types, literal typing, and the range and
  definedness obligations a value clause body carries. An expression the
  value typer refuses in a value clause body SHALL refuse in a `hyper` or
  `relation` body with the same code and cause, located at the same
  expression; a `let` that shadows an enclosing `let` is refused at the
  inner `let`, as the value typer refuses it.
- If a `let` in a `hyper` body or a `relation` body binds the name of one of
  the clause's trace variables, object parameters or execution variables,
  then S3 SHALL refuse `ambiguous_declaration`/`ambiguous-name` at that
  `let`, naming the trace variable, object parameter or execution
  variable.

### Identity

- The checked clause's identity SHALL bind every subject alias in order,
  the quantifier prefix with each variable's kind and alias in order, each
  variable's fairness set, `μ`, the `align skip` list and the instance's
  parameter keys (ADR-023 HV-7, ADR-013 O-09).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-172-AC-1 | ADR-023 §1's `NonInterference` checks with one alias `V`, two universal quantifiers over it, one object parameter `v` of type `Vault`, both `μ` conjuncts in `mu_universal` and none in `mu_existential`. `SavesPower` checks with aliases `S` and `N`. §8.2's `Opaque` checks with its one conjunct in `mu_existential`. | Test (TC-597) |
| FR-172-AC-2 | Refusals at the named span: `forall trace a` with no `of` in a `behaviours` clause (`unsupported_construct`, the quantifier); `of V` in a finite trace domain clause (the quantifier); `of W` naming no model (`missing_declaration`, the alias); `v.l` with no trace variable and `d.used @ n` with `d: S::Device` (`ill_typed`, the read); a `match` conjunct reading `v.l @ a` (`unsupported_construct`/`expression-form`, the read); an object parameter `d: S::Device` in a clause that binds only `V` (`ill_typed`, the parameter); `fair weak N::Device::tick` on a variable of `S` (`missing_declaration`). | Test (TC-597) |
| FR-172-AC-3 | §13's clause with `align skip { V::Vault::mix }` checks with `align_skip` holding `mix`; `align skip { }`, `align skip { V::Vault::mix, V::Vault::mix }`, `align skip { N::Device::tick }` in a clause over `V` alone, and `align skip` with `fair weak V::Vault::step` each refuse `unsupported_construct`/`expression-form`. | Test (TC-597) |
| FR-172-AC-4 | FR-171-AC-3's `Det` checks with two model execution bindings to `step`. `NonInterference` and the same clause with its quantifiers' fairness sets swapped between `a` and `b` (one empty, one `weak step`) have different node identities, as do the clause and the same clause with an extra `match` conjunct. | Test (TC-597) |
| FR-172-AC-5 | In a unit that declares alias `V` (the vault model, object type `Vault` with fields `h` and `l` and operation `step`) and alias `N` (the device model, object type `Device` with operation `tick`), `NonInterference` over `V` with `match { a.step.op = V::Vault::step and b.step.op = stutter }` checks with both conjuncts in `mu_universal`. `match { V::Vault::step != a.step.op }` checks with its one conjunct in `mu_universal` and holds at exactly the joint steps at which `match { a.step.op != V::Vault::step }` holds. Refusals at the qualified name: `a.step.op = N::Device::tick` with `a` of `V` (`ill_typed`/`type-mismatch`); `a.step.receiver = V::Vault::step` and a bare conjunct `V::Vault::step` (`ill_typed`/`type-mismatch`); `a.step.op = V::Vault::nope` (`missing_declaration`/`missing-name`); `a.step.op = V::Vault::l`, naming a field of `Vault` (`unsupported_construct`/`expression-form`, the state-read refusal). | Test (TC-597) |
| FR-172-AC-6 | FR-171-AC-3's `Det` with body `half(x.i) < rational(3, 4)`, for a function `half` over `x.i`'s type whose declared result is `Rational[0, 1; 1, 2]`, refuses `undefined_expression`/`unproved-range` at `rational(3, 4)`, as a value clause over a parameter `x` of a record type with one field `i` of `x.i`'s type refuses the same body. `Det` with body `rational(1, 2) < rational(3, 4)`, and `NonInterference` with that comparison in its body, each refuse `ill_typed`/`ambiguous-literal` at the literal a value clause with that body is refused at. `NonInterference` with `let a = 1 in a = 1` in its body refuses `ambiguous_declaration`/`ambiguous-name` at that `let`, naming trace variable `a`; with `let v = 1 in v = 1` it refuses the same at that `let`, naming object parameter `v`. `Det` with body `let x = 1 in x = 1` refuses the same at that `let`, naming execution variable `x`. | Test (TC-597) |
| FR-172-AC-7 | Over a model whose operations with a receiver are declared by two object types, a `match` conjunct reading `a.step.receiver` refuses `unsupported_construct`/`expression-form` at that read; over the vault, `a.step.receiver = b.step.receiver` checks. `NonInterference` with the conjunct `f(a.step.Vault::step.i)`, for a declared `f(x: Option<Int[0, 1]>): Boolean`, checks with that conjunct in `mu_universal`. With `f(v.l @ a)` it refuses `unsupported_construct`/`expression-form` at `v.l @ a`. | Test (TC-1264) |
| FR-172-AC-8 | With the `Linked` clause of FR-487's Acceptance Criteria, `B` = `(let r = v.next in present(r) and deref(value(r)).l = 1) @ a` checks: `@ a` scopes over the `let`, so the read of `next` and the read of `l` carry `a`. `B` = `deref(value(v.next @ a)).l = 1` refuses `ill_typed` at the `.l` read, which no `@` encloses, and `B` = `present(v.next @ a) and present(v.next)` refuses `ill_typed` at the bare `v.next`. | Test (TC-1264) |
| FR-172-AC-9 | Through `command::compile` (`native-compile/1`) with the counter-step model selected, the source clause `hyper IgnoreTick using infinite over behaviours (item: CounterModel::Counter) forall trace left of CounterModel forall trace right of CounterModel align skip {} match { left.step.op = right.step.op } { always holds(item.value @ left = item.value @ right) }` gives a `filament-outcome/1` document with `last_stage` `S3`, category `unsupported`, exit code 21, and one diagnostic `unsupported_construct` with cause `expression-form` whose locus is the span of `align skip {}`; no diagnostic has code `invalid_syntax` and `last_stage` is never `S1`. The same source with the list holding one qualified name of an operation of the counter-step model gives no diagnostic at its `align skip` item. | Test (TC-597) |
| FR-172-AC-10 | Over a model with two populations that overlap `Thing`, one of member type `Lamp` and one of member type `Switch`, both subtypes of `Thing`, a `hyper` clause with the parameter `v: V::Thing` checks, with no `ambiguous_declaration` refusal, and the checked parameter holds the type `Thing` and both populations. A population whose member types are all unrelated to `Thing` is not among them. | Test (TC-3219) |

## Dependencies

- ADR-023 §1 HS-1 to HS-9, §2 HM-3, §6 HV-7, §13 PA-1 and PA-3; ADR-018
  FA-1, FA-6, QS-5; ADR-013 O-09.
- [FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)
  (the lowering of the checked atoms and conjuncts; the receiver and call
  rules above).
- [FR-171](FR-171-build-forms-for-hyper-clauses-over-behaviours.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (fairness and interval checking),
  [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md),
  [FR-110](FR-110-resolve-header-profile-selections-at-e3.md).
- FSpec owns the grammar, FR-191's extension to model subjects and the
  identity members of the checked-package v2 `hyperproperty` node (ADR-023
  QS-1, QS-2, QS-7).

## References

- ADR-023. FSpec half: FSpec FR-395 and FR-402 (Linear STD-136; ADR-023 QS-1, QS-2, QS-7).
