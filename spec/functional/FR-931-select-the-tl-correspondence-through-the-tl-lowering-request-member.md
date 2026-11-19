---
id: FR-931
title: "Select the TL correspondence of a lowering through the tl_lowering request member"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-757
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-527
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-045
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-329
    type: depends_on
  - target: ix://agent-ix/tl-mltl/FR-031
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-095
    type: depends_on
---
# FR-931: Select the TL correspondence of a lowering through the tl_lowering request member

## Description

A `native-run/1` request asks for FR-757's lowering through its `tl_lowering`
member. The member names the clause, may name the versioned TL correspondence
the lowering is to use, and may ask for cross-evaluation. The correspondence is
part of the claim: `mltl.closed-trace/v1` and `mltl.online-prefix/v1` read the
same formula under a complete and an open surrounding execution, and
`mltl.origin-complete-history/v1` reads past operators back to an
authoritative origin over a finite history, and `mltl.infinite-trace/v1` reads
closed past operators over an infinite lasso trace from its origin. Naming it lets a caller require one reading and get a
typed answer when the clause does not have it, instead of a formula under a
reading the caller did not ask for.

## Inputs

A `native-run/1` request (FR-026, FR-312) carrying a program unit and the
`tl_lowering` member; for cross-evaluation, a trace or a model snapshot.

## Outputs

The run result document's `tl_lowering` record, or a request refusal.

## Behavior

### The member

- `tl_lowering` SHALL be an object decoded under FR-026's closed-envelope
  rules with exactly the members `clause` (required), `correspondence`
  (optional) and `cross_evaluate` (optional). A missing `clause`, a member that
  is `null` or of the wrong JSON kind, or an unknown member SHALL refuse
  `invalid-request` at stage `request`, exit 20, naming the member path
  (`tl_lowering.clause`, `tl_lowering.correspondence`, ...), before any source
  file is read.
- `clause` SHALL name a temporal clause of the program unit by its declared
  name. A name that names no temporal clause SHALL refuse
  `missing_declaration`/`missing-name` naming `tl_lowering.clause` and the
  name, exit 20.

### The correspondence

- `correspondence`, when present, SHALL be exactly one of
  `mltl.closed-trace/v1`, `mltl.online-prefix/v1`,
  `mltl.origin-complete-history/v1` and `mltl.infinite-trace/v1`, compared
  byte for byte; these are the TL targets of FR-045's support table. Any other
  string SHALL refuse `unknown_profile`/`unsupported-selection`, exit 20,
  naming `tl_lowering.correspondence` and the value, before the unit is
  compiled.
- The member fixes the surrounding-execution closure under which FR-527
  classifies the clause: `mltl.online-prefix/v1` classifies it under an open
  surrounding execution, and `mltl.closed-trace/v1`,
  `mltl.origin-complete-history/v1` and `mltl.infinite-trace/v1` under a
  complete one (the infinite-trace row does not read the closure, FR-045).
- When `correspondence` is absent, the clause SHALL be classified under a
  complete surrounding execution and lowered under the target that
  classification names; the default is therefore `mltl.closed-trace/v1` for a
  future-only clause and `mltl.origin-complete-history/v1` for a past-only
  clause under a false-extension profile, and `mltl.infinite-trace/v1` for a
  closed-interval past-only clause under `filament.temporal.infinite-trace/v1`.
- Under `mltl.infinite-trace/v1`, the lowered formula SHALL be a
  `tl-syntax.formula-unbounded/v1` document whose semantic profile is
  `mltl.infinite-trace/v1` and whose clock is the event position, each
  closed past interval `[a, b]` lowered to the same closed interval; it is
  evaluated by `tl-mltl`'s infinite-trace evaluator (`tl-mltl` FR-031), which
  `fsl-command` enables through that crate's `infinite-trace` feature. The
  lowering relies on that evaluator reading a closed past interval that
  reaches before position 0 as FSpec FR-092 "Positions before the origin"
  states, the reading `tl-mltl` adopted in tl-mltl #186; `fsl-command`'s
  `tl-mltl` dependency SHALL be a release or revision that holds it.
- When the classification is supported and its target equals the named
  correspondence (or none is named), the lowering SHALL proceed as FR-757
  states, and the record SHALL carry `disposition: "supported"`,
  `correspondence` the target, `formula` the lowered `tl-syntax` formula in its
  display form and `atoms` the atom table (each TL atom name mapped to the
  atom text of its `holds` leaf, "Atom text" below). The run exits 0 unless cross-evaluation
  reports a fault.
- A supported record SHALL also carry `classification`, as "Classification
  is preserved by lowering" below states.
- When the classification is unsupported, or is supported under a target other
  than the named correspondence, the record SHALL carry
  `disposition: "unsupported"`, `correspondence` the named or default target
  (absent when the classification names none), `classified` the target the
  classification names (absent when it names none) and `dimensions` the
  classification's unmatched dimensions in FR-045 order (`profile`,
  `infinite-trace`, `direction`, `closure`, ...), and no `formula` or `atoms`;
  the run SHALL exit 21 (unsupported, FR-285). A named correspondence that
  differs from a supported target only by closure SHALL report `closure`, and
  one that differs by operator direction SHALL report `direction`. A named
  correspondence that differs from a supported target by the trace it reads,
  `mltl.infinite-trace/v1` named for a false-extension past clause or a
  finite-trace correspondence named for an infinite-trace past clause, SHALL
  report `profile`.

### Cross-evaluation

- `cross_evaluate`, when present, SHALL be an object with exactly one of two
  forms: `trace` (with an optional `over`), read exactly as FR-312 reads the
  `trace` and `over` members of a `temporal` selection, of which a `finite`
  and a `lasso` trace are admitted (a `timestamped` trace refuses
  `invalid-request` naming `tl_lowering.cross_evaluate.trace`); or `initial`
  (a snapshot file of the model subject) with `max_steps` (an integer of at
  least 1), which evaluates every finite behaviour of the subject's model from
  that snapshot of 1 to `max_steps` steps, in FR-120's enumeration order. Both
  forms, neither form, or `max_steps` 0 SHALL refuse `invalid-request` at stage
  `request`, exit 20, naming the member.
- The trace form SHALL match the correspondence: a `lasso` trace is read only
  under `mltl.infinite-trace/v1`, and that correspondence reads only a
  `lasso` trace, because its claim is about an infinite trace and the other
  three correspondences' claims are about a finite one. A `lasso` trace
  beside a named `correspondence` other than `mltl.infinite-trace/v1`, and a
  `finite` trace or the `initial` form beside a named
  `mltl.infinite-trace/v1`, SHALL refuse `invalid-request` at stage `request`,
  exit 20, naming `tl_lowering.cross_evaluate.trace` (or
  `tl_lowering.cross_evaluate.initial`), before any source file is read. When
  no correspondence is named, a supported lowering whose target and trace
  form do not match SHALL refuse the same way after classification, still at
  stage `request` and with no result document. The `initial` form is not read
  under `mltl.infinite-trace/v1` because FR-120's enumeration yields finite
  behaviours, not lassos.
- In the `lasso` form, the trace SHALL be read as FR-329 reads a
  `Lasso::Observed` (a prefix of `n >= 0` samples and a loop of `L >= 1`), the
  native verdict SHALL be FR-329's exact evaluation, and `tl-mltl` SHALL
  evaluate the lowered formula over a `tl-syntax.lasso-trace/v1` document
  holding the same prefix and loop, each position's atom valuation the native
  value of each atom's `holds` leaf at that sample, with no fairness premises.
  The verdicts SHALL be compared at every activation position among FR-329's
  `n + m × L` unrolled positions, `m` the unroll count FR-329 computes for the
  clause; every later position reads as one of them, so agreement there is
  agreement at every position of the infinite trace. One lasso is one trace in
  `traces`. A clause with a non-empty fairness set over the lasso settles as
  FR-329 settles it (`MissingFairnessPremise`), exit 21, and the record
  carries no `cross_evaluation`, since the native side has no verdict to
  compare. An empty loop SHALL refuse as FR-329 refuses it.
- In the `initial` form, the subject SHALL be the model subject built from
  the one snapshot: its initial state is that snapshot, and its universes are
  the complete populations the snapshot declares, each the set of object keys
  it holds, as FR-620 builds the default subject's universes and FR-120
  admits them. A population the snapshot marks incomplete SHALL settle
  `incomplete_population` at stage `request`, category incomplete, exit 22
  (ADR-029 CB-4, as FR-109 settles it), naming it, because a universe is a
  complete population and the run cannot enumerate one it lacks. A snapshot the subject builder refuses
  (FR-120's admission refusals) SHALL refuse at stage `request` with that
  refusal, exit 20.
- In the `initial` form, the clause SHALL be evaluated for every instance of
  its `over` parameter: for a reference-typed parameter every object key of
  the universe of a population that admits the parameter's type, in
  ascending key order, as FR-125's instancing enumerates them; for a finite
  non-reference type every value of its domain in canonical order. `over` is
  not admitted beside `initial`, because the instances are the subject's.
  An `over` population with no object SHALL give zero traces, and the record
  SHALL report `traces` 0 and `positions` 0, exit 0, never a vacuous
  agreement over an invented instance.
- In the `initial` form, each behaviour of `j` steps, for each `j` from 1 to
  `max_steps` and each path of FR-120's enumeration from the snapshot, SHALL
  be read as a closed finite trace of `j + 1` states, positions 0 to `j`, the
  snapshot at position 0, under the lowering's correspondence: a shorter
  behaviour is its own trace, not a prefix of a longer one, and a state with
  no successor ends every path through it. One trace is one (instance,
  behaviour) pair, counted in `traces`, in instance order then FR-120's path
  order, and the `trace` index of a `correspondence-loss` fault is that
  position in that order.
- Cross-evaluation SHALL run only on a supported lowering, and SHALL compare
  the native and `tl-mltl` verdicts at every activation position of every
  trace (FR-757). The record's `cross_evaluation` SHALL hold `traces` (the
  number evaluated), `positions` (the number of activation positions
  compared), `agree` (true exactly when `faults` is empty) and `faults`, each a
  `correspondence-loss` entry naming the clause, the trace index, the position
  and both verdicts. A fault SHALL make the run exit 30 (internal-failure,
  FR-285), because two conforming evaluators of one correspondence never
  disagree. A work-budget stop during enumeration SHALL report the counts
  reached and exit 22.
- On an unsupported lowering `cross_evaluate` SHALL be ignored and the record
  SHALL carry no `cross_evaluation`.

### Classification is preserved by lowering

A clause's classification is the pair FR-932's corpus is organised by: the
correspondence (the FR-527 target) and the fragment, `bounded-future` for a
clause whose temporal operators are all future operators and `bounded-past`
for one whose operators are all past operators, every window bounded. The
lowered formula has a classification of its own, read from the `tl-syntax`
formula alone: its semantic profile is the correspondence, and the temporal
family (future or past) of its temporal nodes, every one with a bounded
interval, is the fragment. A formula with no temporal node takes, on both
sides, the fragment its correspondence's direction names: `bounded-past`
under `mltl.origin-complete-history/v1` and `mltl.infinite-trace/v1`,
`bounded-future` otherwise. Under `mltl.infinite-trace/v1` the lowered
formula is the unbounded edition, and its fragment is `bounded-past` exactly
when every temporal node is a past node with a closed interval.

- On a supported lowering, `fsl-command` SHALL compute the lowered
  formula's classification from the formula alone, with no reverse lowering
  and no reference to the native clause, and SHALL compare it with the
  native classification. When they are equal the record SHALL carry
  `classification` `{"correspondence": <target>, "fragment":
  "bounded-future" | "bounded-past"}`.
- When they differ the record SHALL carry no `classification` and a
  `classification_fault` `{"kind": "classification-loss", "clause",
  "native", "lowered"}` naming the clause and both classifications, and the
  run SHALL exit 30 (internal-failure, FR-285), as a `correspondence-loss`
  fault does, because a lowering that changes the correspondence
  or the direction of a formula hands the TL tools a claim the clause did
  not make.
- The `fsl-command` library entry SHALL let a library caller replace the
  lowered formula's correspondence before it is classified, never through
  the request, so the loss path can be shown, as a mis-mapped atom table
  shows the cross-evaluation loss.
- An unsupported record carries no `classification`, since there is no
  lowered formula to classify.

The lowering is one-way and needs no inverse: each side is classified
directly, so "the lowered form has the clause's classification" is checked
without reconstructing the native clause from the TL formula.

### Atom text

The `atoms` entry of an atom SHALL be its **atom text**: the canonical
text of the atom's `holds(e)` leaf, printed from the leaf's checked
expression `e` by one canonical printer. The source route and the package
route (FR-1039) SHALL call the same printer on the same checked expression,
so the two routes give byte-equal `atoms`. The printer reads the checked
expression and the names the checked package records for it, and never
reads source text or a source span: a package carries no source bytes, and
an atom text sliced from source would differ from the package route's
whenever the author's spacing or grouping differs from the canonical form.
Atom names (`p0`, `p1`, ...) keep their order of first occurrence in the
formula.

The atom text is `holds(` + `T(e)` + `)`, one line, with no line break,
tab or comment, where `T` prints a checked expression as follows.

**Tokens and spacing.** `T` writes the source tokens of the shared
grammar's complete-V1 expression productions, spaced by FR-003's canonical
style ("Canonical style": parentheses, brackets, type arguments,
separators, operators and words), with one change: where FR-003 ends a line
(after `{` and `;`, before `}`), `T` writes one space, so a record value is
`Q::R { a: 1, b: 2 }` and a case is `case x { A: 1; B: 2; }`. No space
follows `(` or `[` or precedes `)` or `]`; every binary operator, `implies`,
`in` and `=` has one space on each side; unary `-` touches its operand;
`not` is followed by one space.

**Operator spelling.** Each checked operation prints as the one source
spelling that denotes it: `implies`, `or`, `and`; `=`, `!=`, `<`, `<=`,
`>`, `>=`; `+`, `-`; `*`, `/`, `div`, `rem`, `mod`; prefix `not` and `-`;
`^` with its signed-integer exponent; `.` for a field or member read and
for an operation call `x.f(a)`; `[i]` for an index or key; each built-in
form by its keyword (`present`, `value`, `deref`, `ref`, `pre`, `size`,
`contains`, `forall`, `exists`, `filter`, `map`, `collect`, `flatMap`,
`flatten`, `dropFirst`, `removeOne`, `fold`, `reduce`, `count`, `sum`,
`only`, `seq`, `reaches`, `lookup`, `allInstances`, `convert`, `Text`);
`let x = e in b` (`let x: T = e in b` when the checked `let` records an
annotation), `if c then a else b` and `case`. A checked node with two
source spellings prints the first one this list gives, so `!=` is never
spelled another way.

**Parenthesisation.** `T` writes a pair of grouping parentheses exactly
where the checked tree needs one to parse back to the same tree under the
shared grammar's "Precedence and bindings" (lowest to highest: `let`/`if`/
`case`; `implies`; `or`; `and`; comparison; `+`/`-`; product; prefix `not`/
`-`; `^`; postfix selection, calls and primaries), and nowhere else:

- an operand whose operator binds less tightly than its position is
  parenthesised;
- for the left-associative `or`, `and`, `+`/`-` and product levels, a right
  operand at the same level is parenthesised and a left operand is not
  (`a - (b - c)`, `a - b - c`);
- for the right-associative `implies`, a left operand that is an `implies`
  is parenthesised and a right one is not;
- a comparison operand of a comparison, and a `^` base that is a `^`, are
  parenthesised, because neither chains;
- a `let`, `if` or `case` that is an operand of any operator or postfix
  form is parenthesised;
- the operand of prefix `not` or `-` that is a binary operation is
  parenthesised (`not (a = b)`, `-(a + b)`), and a `^` operand of prefix
  `-` is not (`-x ^ 2`);
- a grouping the author wrote and the tree does not need is not kept, and
  an argument list, a quantifier body, a bracket or a record field value is
  a delimited position that adds no parentheses of its own.

**Names.** A local binder or parameter prints as its declared name; a field,
operation or relationship-end member as its member name; a declaration (a
function, a type, an enumeration value, a model declaration) as the
qualified spelling the checked package records for it
(`declaration.qualified_name`, the model alias and `::` path of a
model-owned declaration), which both routes read from the same checked
record.

**Literals.** An integer prints in decimal with no leading zero, `0` for
zero and a leading `-` for a negative value; `true`, `false`, `null`,
`none`, `absent` as written; a rational as `rational(n, d)` in lowest terms
with `d > 0`; a decimal as `decimal(c, s)` with its checked coefficient and
scale; a float as `float32(bits: 0x…)` or `float64(bits: 0x…)` with 8 or 16
lowercase hex digits; a quantity as its magnitude, one space and its unit's
qualified spelling (`3 ms`); a text as a JSON string serialized as RFC 8785
serializes a string (`"` and `\` escaped, U+0000 to U+001F as `\b`, `\t`,
`\n`, `\f`, `\r` or `\u00xx` with lowercase hex, every other scalar written
as itself), whatever escapes the source used; a collection value as
`sequence[…]`, `set[…]`, `bag[…]` or `orderedSet[…]` with its elements in
checked order; a record value with its fields in the record's effective
field order; a reference literal as `Q#"key"`.

### Placement

- The lowering and the cross-evaluation SHALL be served by the command adapter
  crate `fsl-command`, which holds ADR-011's only edges to `tl-syntax` and
  `tl-mltl`; no other FSL crate names those crates.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-931-AC-1 | For the event-position clause `Low` (`on origin { always [0, 3] holds(c.versionNumber <= 2) }`), `tl_lowering` `{"clause": "Low"}` and `{"clause": "Low", "correspondence": "mltl.closed-trace/v1"}` each return `disposition` `supported`, `correspondence` `mltl.closed-trace/v1`, `formula` `G[0,3] p0` and an `atoms` entry for `p0`, exit 0; the two records are byte-equal. | Test (TC-2601) |
| FR-931-AC-2 | For `Low`, `"correspondence": "mltl.online-prefix/v1"` returns `supported` under `mltl.online-prefix/v1`; `"correspondence": "mltl.origin-complete-history/v1"` returns `unsupported`, `classified` `mltl.closed-trace/v1`, `dimensions` `["direction"]`, no `formula`, exit 21. For a past-only clause `on origin { historically [0, 2] holds(c.versionNumber <= 2) }` the default is `mltl.origin-complete-history/v1`. The timed form of `Low`, over the ConfigVersion model timed by `dimension Time; unit ms : Time = rational(1, 1); time Config dense unit ms;` (FR-233-AC-5), under `filament.temporal.timed/v1` with no `clock` member (its default binding `model-time`, FR-233) and the body `always [0 ms, 3 ms] holds(c.versionNumber <= 2)`, checks and returns `unsupported` with no `classified` and no `formula`, exit 21. | Test (TC-2601) |
| FR-931-AC-3 | `"correspondence": "mltl.closed-trace/v2"` refuses `unknown_profile`/`unsupported-selection` naming `tl_lowering.correspondence` and the value, exit 20; `"correspondence": 1`, `"correspondence": null`, a missing `clause`, an unknown member `formula`, `cross_evaluate` with both `trace` and `initial`, with neither, with `max_steps` 0, with a `timestamped` trace, and with a `lasso` trace (refused after classification, because `Low`'s default target reads a finite trace) each refuse `invalid-request` at stage `request`, exit 20, naming the member; `"clause": "Nothing"` refuses `missing_declaration`/`missing-name` naming `tl_lowering.clause`. No refusal writes a result document. | Test (TC-2602) |
| FR-931-AC-4 | `Low` with `cross_evaluate` `{"initial": S, "max_steps": 2}`, `S` AC-6's snapshot (config versions `a` and `b`, each with `versionNumber` 0, anchored at an initialization as FR-120 requires of an initial state, over the ConfigVersion model with `versionNumber: Int[0, 3]` and AC-6's `bump`, so each application enumerates at most 16 candidates), returns `cross_evaluation` with `traces` at least 2, `positions` at least `traces`, `agree` true and `faults` `[]`, exit 0; FR-757-AC-2's deliberately mis-mapped atom table (`p0` mapped to the negated leaf, supplied at the `fsl-command` library entry, never through the request) renders through the same record writer as one `correspondence-loss` fault naming `Low`, the trace index, the position and both verdicts, `agree` false, exit 30. | Test (TC-2603) |
| FR-931-AC-5 | Only `fsl-command`'s manifest among the FSL workspace crates names `tl-syntax` or `tl-mltl` (ADR-011). | Inspection (TC-2603) |
| FR-931-AC-6 | `Low` with `cross_evaluate` `{"initial": S, "max_steps": 2}`, `S` a snapshot anchored at an initialization holding config versions `a` and `b`, each with `versionNumber` 0, over the ConfigVersion model with `versionNumber: Int[0, 3]`, whose model steps `bump` (`versionNumber + 1`): the run evaluates both instances, `a` then `b`, and each behaviour of 1 and of 2 steps as its own closed trace of 2 and 3 states, so `traces` is the number of instances, 2, times the number of 1- and 2-step paths from `S` in FR-120's order, `positions` at least `traces`, and `agree` true, exit 0. | Test (TC-2962) |
| FR-931-AC-7 | `cross_evaluate` with `initial` and `over` refuses `invalid-request` naming `tl_lowering.cross_evaluate.over`, exit 20; a snapshot whose `ConfigVersion` population is marked incomplete settles `incomplete_population`, category incomplete, naming it, exit 22 (ADR-029 CB-4); a snapshot whose population holds no object gives `traces` 0, `positions` 0, exit 0. | Test (TC-2962) |
| FR-931-AC-8 | Over every trace of FR-932's corpus, each past case's supported record carries `classification` `{"correspondence": "mltl.origin-complete-history/v1", "fragment": "bounded-past"}` and each future case's `{"correspondence": "mltl.closed-trace/v1", "fragment": "bounded-future"}`, equal to the native classification of the clause (its record's `correspondence` and its operators' direction); the future clause `always[0,3] holds(c.level <= 2)` over FR-932's `Counter` record under `"correspondence": "mltl.online-prefix/v1"` carries `{"correspondence": "mltl.online-prefix/v1", "fragment": "bounded-future"}`; the mixed case `eventually[1,1] once[1,1] not p` returns `unsupported` with no `classification`. A lowered formula stamped with the wrong correspondence (the future clause `always[0,2] holds(c.level = 1)` lowered with its formula's profile replaced by `mltl.origin-complete-history/v1`, supplied at the `fsl-command` library entry and never through the request) yields a record with no `classification` and a `classification_fault` of kind `classification-loss` naming the clause `C`, the native classification `{"correspondence": "mltl.closed-trace/v1", "fragment": "bounded-future"}` and the lowered one `{"correspondence": "mltl.origin-complete-history/v1", "fragment": "bounded-future"}`, exit 30; the same entry with no replacement gives the native classification, exit 0. | Test (TC-2979) |
| FR-931-AC-9 | For the infinite-trace clause `C` (`filament.temporal.infinite-trace/v1`, `on each { once[1,1] not holds(c.level = 0) }` over FR-932's `Counter` record), `tl_lowering` `{"clause": "C"}` and `{"clause": "C", "correspondence": "mltl.infinite-trace/v1"}` each return `supported`, `correspondence` `mltl.infinite-trace/v1`, a formula of the `tl-syntax.formula-unbounded/v1` edition with one closed past node `[1,1]`, and `classification` `{"correspondence": "mltl.infinite-trace/v1", "fragment": "bounded-past"}`, exit 0; with `cross_evaluate` `{"trace": {"lasso": {"prefix": [], "loop": [L0]}}}` (`L0` the sample `level` 0) the record's `cross_evaluation` has `traces` 1, `positions` equal to FR-329's unrolled position count, `agree` true and `faults` `[]`, exit 0. `"correspondence": "mltl.origin-complete-history/v1"` for `C` returns `unsupported`, `classified` `mltl.infinite-trace/v1`, `dimensions` `["profile"]`, exit 21, and `"correspondence": "mltl.infinite-trace/v1"` for the event-position clause `historically [0, 2] holds(c.versionNumber <= 2)` returns `unsupported`, `classified` `mltl.origin-complete-history/v1`, `dimensions` `["profile"]`, exit 21. The infinite-trace clauses `once holds(c.level = 0)` and `eventually[0,2] holds(c.level = 0)` return `unsupported`, `dimensions` `["infinite-trace"]`, with no `classified`, exit 21. | Test (TC-3023) |
| FR-931-AC-10 | A `lasso` trace beside `"correspondence": "mltl.closed-trace/v1"`, and a `finite` trace or the `initial` form beside `"correspondence": "mltl.infinite-trace/v1"`, each refuse `invalid-request` at stage `request`, exit 20, naming `tl_lowering.cross_evaluate.trace` (or `tl_lowering.cross_evaluate.initial`), before the unit is compiled; `C` with no named correspondence and a `finite` trace refuses the same way after classification; a lasso with an empty loop refuses as FR-329 refuses it; the infinite-trace `C` with a weak fairness constraint over an observed lasso settles `MissingFairnessPremise`, exit 21, with no `cross_evaluation`; none of the refusals writes a result document. Through the `fsl-command` library entry, FR-757-AC-2's mis-mapped atom table (`p0` mapped to the negated leaf) over `C` and the lasso of AC-9 gives exactly one `correspondence-loss` fault, naming `C`, trace 0, position 1, native `false` and `tl` `true`, exit 30, and the faithful table agrees. | Test (TC-3023) |
| FR-931-AC-11 | Over the `Low` fixture written `on origin { always [0, 3] holds( ( c.versionNumber<=2 ) ) }`, the `atoms` entry of `p0` is exactly `holds(c.versionNumber <= 2)`, the same bytes as for FR-931-AC-1's spelling. Over a variant whose `c` also has Boolean fields `a`, `b`, `p`, `q`, Integer fields `x`, `y` and a Text field `name`, a leaf written `holds(not(c.a=c.b) and ((c.x+1)-(c.y-2))>=0 implies c.p implies c.q)` prints `holds(not (c.a = c.b) and c.x + 1 - (c.y - 2) >= 0 implies c.p implies c.q)`, and a leaf written `holds(c.name = "aA\t")` prints `holds(c.name = "aA\t")`; and the same requests over the unit's checked package selected as `package` (FR-1039) give byte-equal `atoms`, with the source file removed. | Test (TC-2601) |

## Dependencies

- FR-757 (lowering and cross-evaluation), FR-527 and FR-045 (classification and
  targets), FR-026 and FR-312 (request decoding, trace members), FR-120
  (behaviour enumeration), FR-329 (exact lasso evaluation), FR-285 (exit
  codes), ADR-011 (TL edges).
- `tl-mltl` FR-031 (the infinite-trace evaluator `mltl.infinite-trace/v1`
  names, with its closed-past-before-origin reading).
- FSpec FR-095 (correspondence contract).
