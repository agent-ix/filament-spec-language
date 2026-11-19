---
id: FR-124
title: "Declare intended terminal states and derive the deadlock-freedom item"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-015
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-391
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-382
    type: traces_to
---
# FR-124: Declare intended terminal states and derive the deadlock-freedom item

## Description

A package SHALL declare the intended terminal states of a state model with a
`terminal` declaration that names the model's alias, at most one per model
alias (ADR-018 DL-1). Two aliases that select one model identity may each
carry one, and both are in force. The request writer SHALL add one derived
deadlock-freedom item for each distinct model subject among a request's
temporal items, unless the package carries a `terminal any` declaration
(ADR-018 DL-3). The item is checked like a reachable-state invariant
(TP-1) whose predicate `deadlocked` is derived, not authored (ADR-018 DL-2).

## Use case

A verification operator models a counter that counts up to 3 and stops. They
write `terminal when` a predicate true at 3, so the model checker treats
state 3 as an intended halt, while a state where the model is stuck by
mistake is still reported. Another model halts on purpose wherever it halts,
so its author writes `terminal any` and gets no deadlock report.

## Semantic authority and boundary

FSpec owns the surface grammar of the `terminal` declaration, the definition of
a deadlock, and the deadlock-freedom item's wire identity and place in the
FSpec FR-331 request (ADR-018 QS-12; References). This requirement specifies what
S3 checks once the shared grammar has parsed the declaration, how FSL classifies
terminal states, and what the request writer adds. The spellings `terminal
when` and `terminal any` follow ADR-018 DL-1 and are illustrative.

## Inputs

- The parsed `terminal` declaration of a package, which names one model alias,
  in one of its two forms:
  one that carries a state predicate `P` (written `terminal when P`) and one
  that marks every terminal state intended (written `terminal any`).
- For the request writer: the request's temporal items over model subjects
  (FR-125's `ModelSubject`), each with its checked package.

## Outputs

- On the checked state model, per model alias: `TerminalDeclaration::{None,
  When(clause), Any}`, where `clause` is a checked state predicate.
- For the package: its terminal declarations in FSpec FR-476 terminal row
  order, each keyed by (the selection identity of its model, its ordinal
  among that model's terminal nodes), the alias kept as a display label.
- A `DeadlockFreedom` request item per subject: requirement (`temporal-
  satisfaction`, `Unbounded{domains}`), property form `ReachableInvariant`,
  and an obligation identity made of the subject and the fixed item kind
  `deadlock-freedom`.
- A typed `CheckRefusal` with a span on refusal.

## Behavior

### The `terminal` declaration

- S1 SHALL parse the `terminal` declaration as the shared grammar's `terminal-decl`,
  `terminal <name> using <alias> on <model> ( when <block> | any ) ;`, into one
  `TerminalDeclaration` node, and `terminal` SHALL be a reserved word of the
  complete grammar. S2 builds that node into a `TerminalDeclarationForm`
  (FR-382).
- The checker SHALL admit at most one `terminal` declaration per model alias.
  S2 builds a second declaration for one model alias (FR-382), and S3
  refuses it (FR-124 terminal check): the checker SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at its span, naming both
  declarations (FSpec FR-366).
- The checker SHALL check the predicate `P` of a `When` declaration as a state
  predicate over the model's state, through the one clause checker that
  invariants use (FR-104). If `P` reads a parameter, a result or a
  pre-state, then the checker SHALL refuse as FR-104 refuses that read in
  an invariant.
- Where the member types of the model's populations (the types its
  populations cover, not every declared object type) are exactly one object
  type `T` and exactly one population covers `T`, the checker SHALL type
  `self` in `P` as `Reference<M::T>`.
- Where the model's populations cover exactly one object type `T` and `T` is
  a member type of two or more populations, the checker SHALL refuse `self` in `P` with
  `ambiguous_declaration`/`ambiguous-name` at `self`, naming those populations
  in ascending `DeclarationKey` order, as FR-104 refuses a clause whose `on`
  type several populations cover.
- Where the model's populations cover no object type or several, the checker SHALL
  refuse `self` in `P` with `missing_declaration`/`missing-name` at `self`:
  no single type is the one `self` stands for, so `self` is not bound (the
  ambiguity is the declaration's, not a name resolving to several
  declarations).
- The checker SHALL admit in `P`, for a model with any number of object
  types, a quantifier or population form over a population `M::P` of the
  model (`forall(o in M::P : ...)`, `exists(o in M::P : ...)`, FR-391), and
  SHALL type each bound variable by that population.
- Where `P` does not read `self`, the checker SHALL check `P` whatever the
  model's population member types are.
- Where `P` reads `self`, the deadlock classifier SHALL evaluate `P` once for
  each object of `T`'s population at the terminal state, and SHALL treat `P`
  as satisfied when it is true for every object, so a terminal state with no
  object of `T` satisfies `P` (as an invariant on `T` holds for no object,
  FR-104). It SHALL treat `P` as not satisfied when it is false for any
  object, even when it is `Undefined` for another, and otherwise, when it is
  `Undefined` for some object, as `Undefined`.
- Where `P` does not read `self`, the deadlock classifier SHALL evaluate `P`
  once at the terminal state (FSpec FR-366).
- When a state model has no `terminal` declaration, the checker SHALL check it
  to `TerminalDeclaration::None`.
- The checker SHALL admit one `terminal` declaration on each of two model
  aliases that select one model identity. It SHALL neither refuse the second
  nor merge it into the first: both are checked, both are carried by the
  package (FR-411), and both take part in classification.
- A `When` predicate that reads no state, such as `when { true }` or
  `when { false }`, is a valid Boolean condition: it checks to `When`, and
  it intends every terminal state (`true`) or none (`false`).
- The checker SHALL include the `TerminalDeclaration` in the checked
  package, so two models that differ only in it have different package
  identities.

### Deadlocked states

- The deadlock classifier SHALL classify a state of a subject as
  **terminal** when no transition identity is enabled at it, which is when
  FR-120's successor relation gives it no successor (ADR-018 FA-2, DL-2).
- The deadlock classifier SHALL read every `terminal` declaration of the
  package (FSpec FR-366 "Several declarations"). Each declaration reads a
  terminal state as intending it (`Any`, or `When(P)` with `P` `true` at
  the state's observation by FR-107), not intending it (`P` `false`),
  `Undefined`, or refused (the evaluation of `P` is refused, so its value is
  not known).
- The classifier SHALL classify the terminal state as **intended** when at
  least one declaration intends it; otherwise as **undecided** when at
  least one is refused, since that declaration may intend it; otherwise as
  neither intended nor deadlocked when at least one is `Undefined`;
  otherwise, and with no declaration, as **deadlocked**. The class SHALL NOT
  depend on the order in which the declarations are read, the model
  selection identities or the aliases.
- An undecided state settles the item `inconclusive`, cause
  `undecided-successor` (ADR-018 V-6). An `Undefined` state makes the
  deadlock-freedom item's letter at that state undefined: the item settles
  `refuted` with cause `UndefinedEvaluation{where, cause}` (ADR-018 UE-1,
  FR-126).
- The classifier SHALL read the declarations in FSpec FR-476 terminal row
  order (by model selection identity, then terminal node key, then `claim`
  occurrence ordinal) on both routes, and an undecided or `Undefined`
  report SHALL name the first refused or `Undefined` declaration in that
  order, by its terminal key (selection identity, ordinal), with its first
  undefined cause. The source route SHALL compute that order from the
  terminal node keys the S4 emitter mints for its declarations (FR-411),
  not from alias or source order, so both routes report the same
  declaration; the alias is a display label only.
- The deadlock classifier SHALL evaluate `P` only at terminal states, so a
  state that satisfies `P` and has a successor is not terminal.
- The deadlock classifier SHALL read the state graph alone; fairness
  constraints play no part in whether a state is deadlocked.

### The deadlock-freedom item

- The request writer SHALL add one `DeadlockFreedom` item per distinct model
  subject among the request's temporal items, so two items over equal
  subjects share one deadlock-freedom item.
- When the package carries a `terminal any` declaration on any of its
  models, the request writer SHALL add no deadlock-freedom item for the
  subject, since every terminal state is then intended.
- The request writer SHALL give the item the subject and the property
  `always holds(not deadlocked)`, routed, negotiated and settled as a
  `ReachableInvariant` item (FR-126, FR-127).
- The request writer SHALL make the item's obligation identity the subject
  and the kind `deadlock-freedom`, so it differs from every authored
  claim's identity over the same subject.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-124-AC-1 | The `Counter` unit (population `counters`, field `value: Int[0, 3]`, operation `inc` with precondition `self.value < 3` and postcondition `self.value = pre(self.value) + 1`) checks with each of: no `terminal` declaration (`None`), `terminal when` a predicate that holds exactly when every counter is at 3 (`When`), and `terminal any` (`Any`); the three packages have pairwise different identities. A second `terminal` declaration refuses `ambiguous_declaration`/`ambiguous-name` naming both declarations; a `terminal when` predicate that reads `pre(self.value)` refuses as FR-104 refuses it in an invariant. | Test (TC-519) |
| FR-124-AC-2 | A request with two temporal items over the same `Counter` subject (universe `{c}`, initial value 0) and the `None` declaration carries exactly one `DeadlockFreedom` item, with property form `ReachableInvariant`, requirement (`temporal-satisfaction`, `Unbounded`) and an obligation identity distinct from both authored items. With the `When` declaration it carries one; with the `Any` declaration it carries none. | Test (TC-519) |
| FR-124-AC-3 | Over the `Counter` subject, state `value = 3` is terminal; it is deadlocked under `None` and intended under `When` and `Any`. States `value = 0` to `2` are not terminal under any declaration. Adding `fair weak inc` to a request's items changes none of these classifications. | Test (TC-519) |
| FR-124-AC-4 | `terminal Done using Complete on M when { true };` and `terminal Halt using Complete on M any;` each parse to one `TerminalDeclaration` node whose text is the declaration; only the `when` declaration holds a `Block`. `terminal Done using Complete on M;` fails at `;`, `... on M when true;` fails at `true`, `terminal Done using Complete when { true };` fails at `when`, and `terminal using Complete on M any;` fails at `using`, each as `invalid_syntax`. | Test (TC-1060) |
| FR-124-AC-5 | In a model with one object type `Counter` (`value: Int[0, 3]`, population `counters`, universe `{c1, c2}`), `terminal Done using v on M when { self.value = 3 };` checks to `When`, and a terminal state is intended exactly when both counters hold 3. | Test (TC-519) |
| FR-124-AC-6 | In a model with two object types `Counter` (`value: Int[0, 3]`, population `counters`, universe `{c1}`) and `Gate` (`open: Boolean`, population `gates`, universe `{g1}`), operations `inc` on `Counter` (as `Counter` in AC-1) and `release` on `Gate` (precondition `not self.open`, postcondition `self.open`), `terminal Done using v on M when { forall(c in M::counters : c.value = 3) and forall(g in M::gates : g.open) };` checks to `When`; the terminal state with `c1.value = 3` and `g1.open` is intended, and no terminal state is reachable otherwise. | Test (TC-519) |
| FR-124-AC-7 | In the AC-6 model, `terminal Done using v on M when { self.value = 3 };` refuses `missing_declaration`/`missing-name` at `self`; in a model whose populations cover no object type, `terminal Done using v on M when { self.value = 3 };` refuses the same way and `terminal Done using v on M when { true };` checks to `When`. | Test (TC-519) |
| FR-124-AC-8 | In the AC-5 model with universe `{}`, the single state is terminal and intended under `terminal Done using v on M when { self.value = 3 };`; in the AC-5 model with universe `{c1}`, `terminal Done using v on M when { exists(c in M::counters : c.value = 3) };` is evaluated once at each terminal state: with universe `{c1}` and `c1.value = 3` the terminal state is intended, and with universe `{}` it is false there, so the state is deadlocked. | Test (TC-519) |
| FR-124-AC-9 | In a model with one object type `Counter` covered by two populations, `counters` and `archive`, `terminal Done using v on M when { self.value = 3 };` refuses `ambiguous_declaration`/`ambiguous-name` at `self`, naming `archive` and `counters`; the same predicate written `forall(c in M::counters : c.value = 3)` checks to `When`. In a model with `Counter` in `counters` and `archive` and `Gate` in `gates` (several object types), `self.value = 3` refuses `missing_declaration`/`missing-name` at `self`. | Test (TC-519) |
| FR-124-AC-10 | In the AC-5 model with no operations and `terminal Done using v on M when { 3 mod (2 - self.value) = 1 or self.value = 3 };` (true at 0 and 3, false at 1, undefined at 2), universe `{c1, c2}`: with `c1.value = 2` and `c2.value = 1` the terminal state is deadlocked (one object false, one `Undefined`); with `c1.value = 2` and `c2.value = 3` it is neither intended nor deadlocked and the deadlock-freedom item settles `refuted` with cause `UndefinedEvaluation`; with `c1.value = 0` and `c2.value = 3` it is intended. | Test (TC-519) |
| FR-124-AC-11 | Over FSpec FR-366-AC-12's unit (`V` selects `test/vault`, `N` selects `test/vault-other`, each vault terminal at `l = 1`, `q(x)` is `6 mod x`), on the source route and on the package route, with equal results: `terminal Done using v on V when { q(1 - self.l) = 0 };` (undefined at `l = 1`) with `terminal Halt using v on N any;` classifies the terminal state intended and the request carries no deadlock-freedom item, and so does the pair with `Done` on `N` and `Halt` on `V`; `Done` on `V` with `terminal Stop using v on N when { false };` settles the item `refuted`, `UndefinedEvaluation`, naming the declaration keyed (`test/vault`, 0) with label `V`, and `Stop` on `V` with `Done` on `N` names (`test/vault-other`, 0) with label `N`; `Done` on `V` with `N`'s `when { self.l = 1 }` evaluated under an evaluation meter too small for it (refused) settles `inconclusive`, `undecided-successor`, naming (`test/vault-other`, 0). With `model M = "test/vault"; model N = "test/vault";`, `terminal Done using v on M when { self.l = 1 };` and `terminal Stop using v on N when { false };` both check (no `ambiguous-name`), the package carries two terminal nodes on one model node, and the item settles `proved`; swapping the two `model` lines or the two declarations leaves the package and every report equal. `terminal Done using v on V when { true };` intends every terminal state on both routes. | Test (TC-519) |

## Dependencies

- ADR-018 §10 DL-1 to DL-3 and DL-6.
- [FR-104](FR-104-check-state-clauses.md) (state predicate checking),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (evaluation at an
  observation), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (successor relation), [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md)
  (the request writer), [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (state model members),
  [FR-391](FR-391-name-a-population-in-a-state-clause.md) (a population name
  in a terminal predicate).

## References

- FSpec FR-366 (deadlock freedom: the `terminal` declaration, the deadlock
  definition and the deadlock-freedom item), the FSpec half of ADR-018 QS-12
  (Linear STD-131).
