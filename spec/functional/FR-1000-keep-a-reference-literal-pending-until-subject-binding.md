---
id: FR-1000
title: "Keep a reference literal pending through S1 to S4 and resolve it at subject binding"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-012
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-155
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-107
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-150
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-487
    type: references
  - target: ix://agent-ix/filament-specification/FR-480
    type: implements
  - target: ix://agent-ix/filament-specification/FR-464
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-361
    type: depends_on
---
# FR-1000: Keep a reference literal pending through S1 to S4 and resolve it at subject binding

## Description

FSL SHALL carry a reference literal `M::T#"k"` (FSpec FR-480) as one
subject-independent value from S1 to S4, and SHALL decide whether it is
fixed or created-object, and whether a model-state-barred slot may hold it,
only when a subject is bound. Before binding, no stage knows the subject's
initial states or universes, so no stage before binding can tell the two
kinds apart.

## Rationale

FSpec FR-480 makes the literal's kind a fact of the bound subject: a key an
initial member record admits is fixed, and a key only an operation can create
is created-object. A fixed literal is a constant and may stand in a slot that
bars model state; a created-object literal reads population membership and
may not. One checked package is analyzed under many subjects, so S3 cannot
choose for all of them. Keeping one literal and moving the decision to the
resolver lets the same package be admitted once and resolved per subject,
and keeps the barred-slot rule exact instead of refusing every literal there
or admitting a created-object read.

## Inputs

- A `reference-literal` occurrence (`qualified-name '#' string`) in a unit.
- The unit's model aliases and admitted domain packages (S3).
- At binding, the subject's universes and initial states (FSpec FR-361).

## Outputs

- S1 and S2: one `ReferenceLiteral{type_name, key}` node.
- S3: the literal typed `Reference<M::T>`, held in the checked graph as the
  checked node `NodeKind::ReferenceLiteral{object_type, identity}`
  (`filament-checked`), `object_type` the effective identity of `T` and
  `identity` the key's UTF-8 bytes, unresolved, with its pending marks.
- S4: FSpec FR-480's `literal` term of `value_kind` `reference`.
- Binding: the admitted subject's resolution table `ReferenceBindings`, the
  resolved footprints, or a located refusal before any state is explored.

## Behavior

### S1 and S2

- The parser SHALL parse `qualified-name '#' string` as one
  `ReferenceLiteral` node holding the qualified name and the string token,
  with `#` a punctuation token of this production alone. The production is
  FSpec's complete-V1 `reference-literal = qualified-name, '#', string ;`, a
  `primary` alternative: the parser takes it when the token after a
  primary's qualified name is `#`, and the literal is then a whole primary,
  so postfix selection and operators apply to it as to any other primary.
- The lexer SHALL scan every `#` outside a string or a `//` comment as one
  single-character punctuation token, which joins no neighbouring character
  and starts no comment; a `#` inside a string or comment is text. FSL has no
  `#` comment, so the token collides with no other form.
- The literal SHALL be written solid: the `#` starts at the byte where the
  qualified name ends, and the string starts at the byte after `#`. The
  literal is one value spelling, and a spaced `M::T # "k"` would read as an
  operator FSL does not have. The key SHALL be a string; an integer identity
  is written as its decimal text (`M::Job#"7"`).
- The parser SHALL refuse each malformed spelling `invalid_syntax` at S1,
  as FSpec's reference-literal table states:
  `unexpected-token` at a `#` that follows no primary-leading qualified name
  (`#"k"`, `1#"k"`, `(x)#"k"`, `f(x)#"k"`, `a.b#"k"`, the second `#` of
  `M::T#"a"#"b"`) or that follows whitespace or a comment (`M::T #"k"`);
  `unexpected-token` at the token after `#` when it is not a string
  (`M::T#k`, `M::T#1`, `M::T#)`) or when whitespace or a comment precedes the
  string (`M::T# "k"`); `unexpected-end` when the source ends after `#`; and
  the lexer's own string cause (`invalid-escape`, `invalid-token`) at a
  malformed key string. An empty key and a name that names no object type
  parse, and S3 refuses them.
- The CST SHALL hold the literal as one `ReferenceLiteral` node spanning from
  the first byte of the qualified name to the end of the string, whose
  children are, in order, the `QualifiedName` node, the `#` token and the
  string token.
- The formatter SHALL print `#` touching both neighbours, so it renders the
  literal byte for byte (`M::Server#"s1"`), formatting is idempotent and the
  token stream is unchanged (FR-003 "Canonical style").
- S2 SHALL build it as the value form `ReferenceLiteral{type_name, key}`,
  `type_name` the qualified name's spelling as written (`M::Server`) and
  `key` the string's decoded Unicode scalar sequence with no normalization,
  the node's span the CST node's span.

### S3

- The checker SHALL resolve `type_name` and refuse as FSpec FR-480 "Static
  checking" states: no declaration `missing_declaration`/`missing-name` at
  the name; a declaration that is not a model object type
  `ill_typed`/`type-mismatch` at the name with `expected` `object type` and
  `actual` the resolved kind; an empty key `invalid_package`/`invalid-value`
  at the string. In a unit that annotates a population overlapping `T`
  symmetric, FR-150 refuses the literal.
- The checker SHALL type the literal `Reference<M::T>`, the type node a
  `Reference<M::T>` reference names, and keep `{T, identity bytes}` (the
  UTF-8 bytes of `key`) unresolved, as the checked node
  `NodeKind::ReferenceLiteral{object_type, identity}` that
  `filament_checked::construct::reference_literal` builds. That node is
  distinct from a resolved `ObjectReference` value, so no checked value
  holds an object reference before a subject is bound. The constructor
  refuses an empty or non-UTF-8 identity; the checker refuses the empty key
  first, so the constructor's refusal is never the reported one.
- Each occurrence in a slot that bars model state (FSpec FR-464 "Slots that
  bar model state") SHALL be marked `pending-reference`, and S3 SHALL NOT
  refuse it there. Each read footprint (FR-155) that holds a literal,
  including a hyper clause's per-trace footprint, SHALL be marked
  `pending-reference` for that literal and left without its membership
  location. Nothing at S3 decides fixed or created-object.

### S4

- Lowering SHALL emit FSpec FR-480 "Lowering"'s
  `{term: "literal", type: R, value_kind: "reference", value: h}`, `h` the
  lowercase hexadecimal of the identity bytes. Two literals with one type and
  one identity are one term, whatever alias or spelling wrote them.

### Resolution at subject binding

- Subject binding SHALL be FR-120's subject admission: `fsl-eval`'s
  `ModelSystem` admission (`ModelSystem::new` and its siblings) resolves the
  package's literals after it has checked the universes and admitted the
  initial states, and before it returns the system, so no expansion,
  exploration or clause evaluation can see an unresolved literal. Admission
  is the one stage that holds both inputs of the decision, the admitted
  initial member records and the universe key sets, and every analysis path
  over a subject reaches its states through an admitted `ModelSystem`.
  `fsl-analyze` admits the request's subject once before it runs any item,
  so a resolution refusal is the request's binding refusal
  (`AnalyzeRefusal::Binding`) and no item runs.
- The resolution table SHALL be data the admitted `ModelSystem` holds beside
  the checked package, `ReferenceBindings`, keyed by the checked node's
  `(object_type, identity)` and mapping each key to
  `ReferenceBinding::Fixed(r)` or `ReferenceBinding::CreatedObject`.
  Resolution SHALL NOT change the checked package: its graph keeps every
  literal pending, so one checked package is admitted under many subjects,
  each with its own table. A system admitted over one model of a package
  that selects several (a refinement's two subjects) resolves the literals
  whose `T` that model declares; the whole-subject admission above has
  already decided the refusals for the rest.
- Before any state is explored or any clause is evaluated, the resolver
  SHALL build one table from each `(T, identity)` of
  the checked package's literals to `Fixed(r)`, `r` the FSpec FR-181
  reference of the object an initial member record admits, or to
  `CreatedObject`, for a key that occurs in an overlapping population's
  universe and that no initial record admits.
- A key that occurs in the universe of no population overlapping `T` SHALL
  refuse `missing_declaration`/`missing-name` at the literal, naming `T` and
  the key's hexadecimal identity, and nothing over the subject is evaluated.
  A fixed key whose record's type does not conform to `T` refuses the same.
- A `CreatedObject` literal at an occurrence marked `pending-reference` in a
  barred slot SHALL refuse `ill_typed`/`operator-ineligible` at the literal.
  A `Fixed` literal there is admitted as a constant. The check runs in the
  same admission, over every S3 `pending-reference` mark of the package's
  barred slots, once the table exists; S3 never runs it.
- Each footprint marked `pending-reference` SHALL be resolved: a
  `CreatedObject` literal adds the membership of the populations that
  overlap `T`, and a `Fixed` literal adds nothing. The admitted
  `ModelSystem`'s footprints are the resolved ones, and every footprint
  consumer (FR-155's frame and independence checks, state-space reduction)
  SHALL read them, never the pending S3 footprint.

### Evaluation

- The clause evaluator (FR-107, `fsl-eval`) SHALL evaluate a
  `NodeKind::ReferenceLiteral` node by reading the admitted system's
  `ReferenceBindings`, which every evaluation over that subject receives
  with the state's object environment. The node is a lookup in the table,
  never a value the graph holds.
- A `Fixed(r)` literal evaluates to `r` at every state of every behaviour.
  A dereference through it whose object is not a member of the state follows
  the rules for an absent target.
- A `CreatedObject` literal evaluates at each state by a key lookup of the
  state's member with that key in the overlapping populations: that member's
  FSpec FR-181 reference when it is present and its type conforms to `T`,
  and otherwise the undefined value with reason `absent-key`. The lookup is
  the evaluator's own `lookup` query with undefined absence over the
  state's object environment, so the undefined value is the one that query
  gives a key with no member.

### In a hyper atom

A `holds` atom of a hyper clause over `behaviours` is evaluated by one
`Machine` over one component state per trace variable, each component
observed through its own component's admitted `ModelSystem` and so carrying
that system's `ReferenceBindings`
([FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)
"Evaluating an atom"). A model read of the atom names its component through
FR-487's `component_reads`; a reference literal is not a model read and
carries no component index of its own. Which component's table and state a
literal uses therefore follows from its kind.

- **Fixed.** A literal whose `(T, identity)` resolves `Fixed(r)` SHALL be
  admitted anywhere in a hyper atom's condition, whatever components the
  atom reads, and SHALL evaluate to `r` in every component. `r` is the
  FSpec FR-181 reference `(universe, object type, key)`, which names no
  component, and FR-107 compares references ignoring the component, so the
  literal is the same constant in every trace. Every component of a hyper
  clause whose model declares `T` is a system admitted over the same
  subject, with the same initial member records and universes, so every such
  component's table gives the same binding for `(T, identity)`; a component
  whose model does not declare `T` resolves no literal of `T`.
- **Agreement.** `CheckedHyperLetters::new` SHALL compare, for each
  `(T, identity)` of a literal in the clause's atom conditions, the binding
  that each component system whose model declares `T` holds. When two of
  them differ (one `Fixed` and one `CreatedObject`, or `Fixed` of two
  references), it SHALL refuse
  `LettersRefusal::ReferenceBindingsDisagree { object_type, identity,
  components }`, naming the literal's type and hexadecimal identity and the
  trace variable indices whose bindings differ, before any component state
  is observed. Only a caller that assembles the components from systems
  admitted over different subjects can reach this; `fsl-analyze` admits one
  subject per request and never does.
- **CreatedObject.** A literal whose `(T, identity)` resolves
  `CreatedObject` reads population membership at a state, so it SHALL be
  read in exactly one component: the one component that the atom's model
  reads read, the single distinct value of `component_reads(atom)`. It
  evaluates by the per-state key lookup above, over that component's object
  environment and that component system's table. The atom's per-trace read
  footprint (FR-155) for that component SHALL gain the membership of the
  populations overlapping `T` in that component's model, and no other
  component's footprint gains it.
- **Spanning or no component.** A `CreatedObject` literal in an atom whose
  `component_reads(atom)` holds more than one distinct component, or none,
  SHALL be refused `ill_typed`/`operator-ineligible` at the literal, naming
  the literal's type, its hexadecimal identity and the components the atom
  reads (empty for none). It is the cause a created-object literal gets in a
  slot that bars model state: the literal is admitted only where one
  population state is in scope, and an atom over two traces, or over none,
  offers no single state to look the key up in. No component is chosen by
  default, since any choice would silently decide the property over one
  trace. The refusal SHALL be decided at binding, in the same admission
  pass as the barred-slot check, once the table exists: `fsl-analyze`
  reports it as the request's binding refusal before any item runs, and
  `CheckedHyperLetters::new` refuses
  `LettersRefusal::ReferenceLiteralComponent` with the same names for a
  caller that builds the letters directly, before any state is observed.
  A `Fixed` literal in the same atom is not refused.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1000-AC-1 | `M::T#"s1"` in a clause body parses as one `ReferenceLiteral` node spanning the literal, renders back byte for byte and formats idempotently with its token stream unchanged; `M::T#s1`, `M::T#` and `#"s1"` each refuse `invalid_syntax`/`unexpected-token` at the token after `#`, at the token after `#` and at `#`. | Test (TC-2833) |
| FR-1000-AC-2 | Over TC-1076's `Counter` unit, `invariant R using v on M::Counter at current { self = M::Counter#"a" }` checks with the literal typed `Reference<M::Counter>`; `M::Nothing#"a"` refuses `missing_declaration`/`missing-name` at `Nothing`, `M::Level#"a"` refuses `ill_typed`/`type-mismatch` at `M::Level` with expected `object type`, and `M::Counter#""` refuses `invalid_package`/`invalid-value` at the string. A protocol `choice` guard comparing a `receive` record's reference field `got.target` (record `Pick { target: Reference<M::Counter>; }`, received before the choice) to `M::Counter#"z"` checks at S3 with no refusal: the occurrence is pending, not refused. | Test (TC-2834) |
| FR-1000-AC-3 | `M::Counter#"a"` lowers to one `literal` term of `value_kind` `reference` whose `value` is `61`; two occurrences, one through a second alias of the package, share the term. | Test (TC-2834) |
| FR-1000-AC-4 | Over the analyze Counter subject (the `test/counter` domain package whose `Counter` has the field `value`) whose initial state holds counter `a` and whose universe holds `a` and `b`: a clause reading `M::Counter#"a"` resolves `Fixed` and is decided; `M::Counter#"zz"` refuses `missing_declaration`/`missing-name` at the literal naming `Counter` and `7a7a` as the request's binding refusal at subject admission, before any item runs or any state is explored; and a protocol `choice` guard comparing a `receive` record's reference field `got.target` (record `Pick { target: Reference<M::Counter>; }`, received before the choice) to the created-object `M::Counter#"b"` refuses `ill_typed`/`operator-ineligible` at the literal at subject admission, while the same guard over `M::Counter#"a"` is admitted. | Test (TC-2835) |
| FR-1000-AC-5 | Over the AC-4 subject, a clause `always holds(deref(M::Counter#"b").value = 0)` is undefined with reason `absent-key` at every state, since the per-state lookup finds no `b`, and its footprint holds the membership of `counters`; `always holds(deref(M::Counter#"a").value <= 3)` is decided with a footprint that holds no membership location for the literal. | Test (TC-2835) |
| FR-1000-AC-6 | In a clause body, `M::T #"s1"`, `(x)#"s1"`, `f(x)#"s1"` and `M::T#"a"#"b"` refuse `invalid_syntax`/`unexpected-token` at the first, first, first and second `#`; `M::T# "s1"`, `M::T#1` and `M::T#)` refuse `invalid_syntax`/`unexpected-token` at `"s1"`, `1` and `)`; a source ending in `M::T#` refuses `invalid_syntax`/`unexpected-end`; `M::T#"\q"` refuses `invalid_syntax`/`invalid-escape` at the string; `"a#b"` and `// a#b` parse as a string and a comment; `M::T#""` parses. The admitted `M::T#"s1"`'s `ReferenceLiteral` node has the children `QualifiedName` (`M::T`), the `#` token and the string token, in that order. | Test (TC-2952) |
| FR-1000-AC-7 | S2 builds the body `M::T#"s1"` of a function as the one value form `ReferenceLiteral{type_name: "M::T", key: "s1"}` whose span is the literal's source span, and `M::T#"é"` as the key `é` (one scalar, no normalization). | Test (TC-2953) |
| FR-1000-AC-8 | Over the AC-4 subject, the hyper clause `hyper HF using t0 over behaviours (c: M::Counter) forall trace a of M forall trace b of M { always holds(c.value @ a <= 3 or c.value @ b <= 3 or c = M::Counter#"a") }` holds the fixed literal in an atom that reads components 0 and 1 and in an atom position that is no model read: it is admitted with no binding refusal and analyzed, exit 0, and no `operator-ineligible` refusal is reported. | Test (TC-2957) |
| FR-1000-AC-9 | Over the AC-4 subject, `always holds((deref(M::Counter#"b").value = 0) @ a)`, whose one atom reads only component 0, is admitted and resolves the created-object literal against component 0: the atom is undefined with reason `absent-key`, since no state of component 0 holds `b`. | Test (TC-2957) |
| FR-1000-AC-10 | Over the AC-4 subject, `always holds((deref(M::Counter#"b").value) @ a = c.value @ b)`, whose atom reads components 0 and 1, and `always holds(c = M::Counter#"b")`, whose atom makes no model read, each refuse `ill_typed`/`operator-ineligible` at the literal, naming `Counter`, `62` and the components `[0, 1]` and `[]`, as the request's binding refusal (exit 20) before any item runs; the same two atoms over `M::Counter#"a"` are admitted. | Test (TC-2957) |
| FR-1000-AC-11 | For FR-487's vault and a hyper clause over `forall trace a of V forall trace b of V` whose atom compares `v` with `V::Vault#"w"`, `CheckedHyperLetters::new` given one system admitted with vaults `v` and `w` in its initial state and one admitted with only `v` initially and universe `{v, w}` refuses `LettersRefusal::ReferenceBindingsDisagree` naming `Vault`, `77` and components `[0, 1]`; given the first system for both components, it admits. | Test (TC-2957) |

## Dependencies

- FSpec FR-480 (the literal's syntax, typing, lowering and resolution),
  FR-464 (the slots that bar model state), FR-361 (universes and initial
  states), FR-181 (references).
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  (value forms), [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  (lowering), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (subject admission), [FR-107](FR-107-evaluate-state-clauses-at-s6a.md)
  (clause evaluation), [FR-155](FR-155-derive-and-enforce-read-and-write-footprints.md)
  (footprints), [FR-150](FR-150-check-the-symmetric-population-annotation.md)
  (symmetric populations) and
  [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md)-AC-9,
  whose refusal names a reference literal.
- [FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)
  (a hyper atom's components, `component_reads` and `CheckedHyperLetters`).
