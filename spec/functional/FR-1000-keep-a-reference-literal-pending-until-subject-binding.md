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
  - target: ix://agent-ix/filament-spec-language/FR-150
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-185
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
- S3: the literal typed `Reference<M::T>`, holding `{T, identity bytes}`
  unresolved, with its pending marks.
- S4: FSpec FR-480's `literal` term of `value_kind` `reference`.
- Binding: the resolution table, the resolved footprints, or a located
  refusal before any state is explored.

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
  UTF-8 bytes of `key`) unresolved.
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

- When a subject is bound, before any state is explored or any clause is
  evaluated, the resolver SHALL build one table from each `(T, identity)` of
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
  A `Fixed` literal there is admitted as a constant.
- Each footprint marked `pending-reference` SHALL be resolved: a
  `CreatedObject` literal adds the membership of the populations that
  overlap `T`, and a `Fixed` literal adds nothing. Every footprint consumer
  (FR-155's frame and independence checks, state-space reduction) SHALL read
  the resolved footprint, never the pending one.

### Evaluation

- A `Fixed(r)` literal evaluates to `r` at every state of every behaviour.
  A dereference through it whose object is not a member of the state follows
  the rules for an absent target.
- A `CreatedObject` literal evaluates at each state by a key lookup of the
  state's member with that key in the overlapping populations: that member's
  FSpec FR-181 reference when it is present and its type conforms to `T`,
  and otherwise the undefined value with reason `absent-key`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1000-AC-1 | `M::T#"s1"` in a clause body parses as one `ReferenceLiteral` node spanning the literal, renders back byte for byte and formats idempotently with its token stream unchanged; `M::T#s1`, `M::T#` and `#"s1"` each refuse `invalid_syntax`/`unexpected-token` at the token after `#`, at the token after `#` and at `#`. | Test (TC-2833) |
| FR-1000-AC-2 | Over TC-1076's `Counter` unit, `invariant R using v on M::Counter at current { self = M::Counter#"a" }` checks with the literal typed `Reference<M::Counter>`; `M::Nothing#"a"` refuses `missing_declaration`/`missing-name` at `Nothing`, `M::Level#"a"` refuses `ill_typed`/`type-mismatch` at `M::Level` with expected `object type`, and `M::Counter#""` refuses `invalid_package`/`invalid-value` at the string. A protocol `choice` guard comparing a `receive` record's reference field `got.target` (record `Pick { target: Reference<M::Counter>; }`, received before the choice) to `M::Counter#"z"` checks at S3 with no refusal: the occurrence is pending, not refused. | Test (TC-2834) |
| FR-1000-AC-3 | `M::Counter#"a"` lowers to one `literal` term of `value_kind` `reference` whose `value` is `61`; two occurrences, one through a second alias of the package, share the term. | Test (TC-2834) |
| FR-1000-AC-4 | Over the analyze Counter subject (the `test/counter` domain package whose `Counter` has the field `value`) whose initial state holds counter `a` and whose universe holds `a` and `b`: a clause reading `M::Counter#"a"` resolves `Fixed` and is decided; `M::Counter#"zz"` refuses `missing_declaration`/`missing-name` at the literal naming `Counter` and `7a7a` before any state is explored; and a protocol `choice` guard comparing a `receive` record's reference field `got.target` (record `Pick { target: Reference<M::Counter>; }`, received before the choice) to the created-object `M::Counter#"b"` refuses `ill_typed`/`operator-ineligible` at the literal when the subject is bound, while the same guard over `M::Counter#"a"` is admitted. | Test (TC-2835) |
| FR-1000-AC-5 | Over the AC-4 subject, a clause `always holds(deref(M::Counter#"b").value = 0)` is undefined with reason `absent-key` at every state, since no state holds `b`, and its footprint holds the membership of `counters`; `always holds(deref(M::Counter#"a").value <= 3)` is decided with a footprint that holds no membership location for the literal. | Test (TC-2835) |
| FR-1000-AC-6 | In a clause body, `M::T #"s1"`, `(x)#"s1"`, `f(x)#"s1"` and `M::T#"a"#"b"` refuse `invalid_syntax`/`unexpected-token` at the first, first, first and second `#`; `M::T# "s1"`, `M::T#1` and `M::T#)` refuse `invalid_syntax`/`unexpected-token` at `"s1"`, `1` and `)`; a source ending in `M::T#` refuses `invalid_syntax`/`unexpected-end`; `M::T#"\q"` refuses `invalid_syntax`/`invalid-escape` at the string; `"a#b"` and `// a#b` parse as a string and a comment; `M::T#""` parses. The admitted `M::T#"s1"`'s `ReferenceLiteral` node has the children `QualifiedName` (`M::T`), the `#` token and the string token, in that order. | Test (TC-2952) |
| FR-1000-AC-7 | S2 builds the body `M::T#"s1"` of a function as the one value form `ReferenceLiteral{type_name: "M::T", key: "s1"}` whose span is the literal's source span, and `M::T#"é"` as the key `é` (one scalar, no normalization). | Test (TC-2953) |

## Dependencies

- FSpec FR-480 (the literal's syntax, typing, lowering and resolution),
  FR-464 (the slots that bar model state), FR-361 (universes and initial
  states), FR-181 (references).
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  (value forms), [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  (lowering), [FR-155](FR-155-derive-and-enforce-read-and-write-footprints.md)
  (footprints), [FR-150](FR-150-check-the-symmetric-population-annotation.md)
  (symmetric populations) and
  [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md)-AC-9,
  whose refusal names a reference literal.
