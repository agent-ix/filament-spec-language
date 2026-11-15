---
id: FR-495
title: "Reach the integer-division, zero-divisor and state-core rows from FSL source"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-003
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-040
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-068
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-335
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-044
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-140
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-147
    type: depends_on
---
# FR-495: Reach the integer-division, zero-divisor and state-core rows from FSL source

## Description

FSpec's scalar vectors (TC-192 integer division, TC-185 decimal, TC-187
quantity) name outcomes that a checked FSL program meets at two different
stages: a refusal the checker issues from source, and a kernel outcome the
evaluator meets only when the checker did not prove the obligation. This
requirement states which FSL source text reaches each row and which
observation each stage gives, so a row is never classified admission-only for
lack of a source route. It owns five things: the range the checker gives an
integer `div`, `rem` or `mod` (QSL-1389), the two observations of a bounded
result domain (QSL-1389), the two observations of a zero divisor (QSL-1393),
the order in which a state-core declaration is refused (QSL-1392), and
the finite-backend negotiation of an integer division over unbounded operands
(QSL-1485).

## Inputs

- A unit whose declarations use `div`, `rem`, `mod`, `/` over decimal,
  rational or quantity operands, or the clause forms `invariant`, `pre` and
  `post`.
- An optional caller-supplied owner-admitted integer division selection for
  `div` and `rem`, obtained through `admit_integer_division`; its three
  profiles are `Euclidean`, `Floor` and `Truncating`. `mod` retains its
  independently defined Euclidean semantics and demands no irrelevant
  `div`/`rem` selection.
- A check mode, `CheckMode::Linked` (the default) or `CheckMode::Kernel`
  (FR-068), supplied to the package-level check (`PackageDeclarations::check`).
- For a Kernel evaluation, the function or expression position whose checked
  `Int[lo, hi]` type is the consumer domain.

## Outputs

- Under `CheckMode::Linked`, a checked package or a located check refusal.
- Under `CheckMode::Kernel`, a checked package whose S6a evaluation returns the
  kernel outcome of FSpec FR-147 and FR-140 (a completed value, `Undefined`,
  `Refused` or `Incomplete`).

## Behavior

### Public compile-facade division selection

The existing FSL public compile facade SHALL accept the caller's owner-admitted integer
division selection and carry it unchanged into package declarations, checking
and lowering.

The facade SHALL NOT replace a supplied selection with `None`, select a default
law, or infer a law from operator spelling or operand values.

The selected `div`/`rem` profile and its original owner definition reference
SHALL agree with the emitted operation's law binding.

The existing source-backed checking, emission and CG agreement path SHALL
retain the supplied admitted capability and its exact owner law binding.

When a source-backed operation's definition binding is foreign to or conflicts
with the admitted selection, the native admission boundary SHALL return its
typed mismatch refusal before CG or source-backed execution.

When `div` or `rem` demands a selection that is absent, the facade SHALL return
the existing typed `MissingSelection` for the `IntegerDivision` role at the
operator.

When definition inputs are unknown, multiple or incompatible, the owner
admission boundary SHALL retain its distinct native refusal.

When a program contains no `div` or `rem`, the facade SHALL NOT require an
integer division selection, including for a `mod`-only program.

This obligation supplies the admitted law to CG and the source-backed
IR-1391 agreement cells through the existing compile facade. A new byte-only
execution surface is outside this scope. Existing CG intake may strict-read
the original emitted checked-package bytes; that is not a requirement for a
new FSL execution entry over bytes alone. Bounded operands and the existing
range/nonzero obligations remain authoritative. Providing a law proves no new
bound and does not authorize a CG-side result-bound workaround.

### Scalar operation behavior

- **Range of an integer division.** The checker SHALL give `a div b`, `a rem b`
  and `a mod b` the smallest interval that contains the result of the operator
  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`
  and `y` not zero, taking the retained division law for `div` and `rem`
  (FSpec FR-147) and the Euclidean remainder for `mod`. When both ranges are
  singletons the result range is the singleton of the exact result. The
  operator's exposed member alone decides the range (FSpec FR-147: the
  unexposed member is never checked). A result range inside a bounded
  `Int[lo, hi]` position is proved; otherwise the checker SHALL refuse
  `undefined_expression`/`unproved-range` at the operator. The nonzero
  obligation of the divisor is decided first.
- **Bounded result domain, two observations.** FSpec FR-044 states that an
  expression its check admitted has its result membership proved, so a bounded
  consumer's out-of-domain refusal of FSpec FR-147 is met only by an expression
  that did not pass that check. FSL gives each such row two observations:
  - Under `CheckMode::Linked` the observation is the check refusal of the
    previous bullet, located at the operator, with no evaluation.
  - Under `CheckMode::Kernel` the checker SHALL record no range or nonzero
    obligation for `div`, `rem` and `mod`, as it records none for `sum`
    (FR-100, FR-096-AC-14), and S6a evaluation SHALL return FSpec FR-147's
    outcome for the operands and the consumer domain with FSpec FR-147's
    charges. The consumer domain is the checked result type of the evaluated
    function. The evaluation entries (`call`, `evaluate`, `evaluate_closed_with_frame`)
    take no result-domain parameter.
  - A kernel `DivisionOutOfDomain` and `ModuloOutOfDomain` render as FR-096
    states: `division_out_of_domain` with `quotient-outside-domain` or
    `remainder-outside-domain`, `modulo_out_of_domain`/`outside-domain`, each
    with `expected` the consumer domain and no member exposed.
- **Zero divisor, two observations.** A division carries a nonzero obligation
  (FR-104 "Definedness facts", FR-091), so a zero divisor is not met at
  S6a under `CheckMode::Linked`. For an integer `div`, `rem` or `mod`, a
  decimal or rational `/`, a quantity `/` and a quantity `^` with a negative
  exponent, whose base is its divisor (FR-093 "Integer power"):
  - Under `CheckMode::Linked` a divisor that is a zero literal, or whose range
    contains zero with no exact nonzero guard (FSpec FR-044-AC-7), refuses
    `undefined_expression`/`unproved-nonzero` at the operator before
    evaluation. A divisor whose range excludes zero, a nonzero literal or an
    exact nonzero guard discharges the obligation.
  - Under `CheckMode::Kernel` a zero divisor evaluates to the kernel
    `Undefined::DivisionByZero`, spelled `division-by-zero` (FR-100), at the
    charge FSpec states for the operation, with no value and no loss record.
- **What `CheckMode::Kernel` leaves unrecorded, and what it keeps.** `Kernel`
  drops the unproved definedness obligations of a function body (range, nonzero, presence, rational-range and nonempty-reduction), so the
  evaluator meets them as kernel outcomes. It SHALL NOT change what else the
  check refuses. Four boundaries follow (QSL-1963):
  - The presence obligation of `value(e)` (FR-104 "Definedness facts") is one
    of the obligations `Kernel` drops: a function body whose `value(x)` has no
    proved `present(x)` SHALL refuse `undefined_expression`/`unproved-presence`
    under `Linked` and SHALL check under `Kernel`, where the evaluator meets
    the absent value.
  - `Kernel` selects how a function body is checked. A state clause a package
    declares (`invariant`, `pre`, `post`; FR-104), checked through
    `PackageDeclarations::check`, SHALL refuse its unproved obligation under
    `Kernel` as under `Linked`, so `Kernel` never admits a declared clause
    whose obligation `Linked` refuses. `CheckedGraph::check_clause_expression`
    is a different entry that takes its own `CheckMode` and does drop
    obligations under `Kernel`; this boundary does not govern it.
  - A termination measure is not a definedness obligation of the body. The
    `decreases(..)` expression and the recursion's missing-measure and
    unproved-decrease refusals (FSpec FR-146, FR-091-AC-30) SHALL be checked
    as under `Linked` in both modes, including an unproved obligation inside
    the measure expression.
  - A malformed structure is a fault in what was supplied, not an obligation
    the evaluator could meet. The definedness walk of an already-checked body
    SHALL refuse a rational operation whose operand is not of `Rational`
    type, and `PackageDeclarations::check` SHALL refuse a dispatch declaration
    whose table, operation or candidate function index or signature is
    invalid, under `Kernel` exactly as under `Linked`. The typer builds a
    rational operation only from `Rational` operands, so no FSL source or
    package declaration produces the first fault, and no source text produces
    the second.
- **Declaration form before expression form.** Under a header profile that
  names only `filament.state.core/v1`, S3 SHALL refuse each declaration whose form
  the layer does not admit (the layer admits `invariant`, `pre` and `post`;
  FSpec `state-core.md`) as `unsupported_construct`/`declaration-form` at the
  declaration, naming the layer, and SHALL NOT classify any expression of that
  declaration (FR-110 "Layer selection"). A `function` declaration is therefore
  refused as a declaration, and the operators in its body receive no
  `expression-form` refusal. This order is intended. An integer `div`, `rem`
  or `mod` in an admitted clause form refuses `unsupported_construct`/
  `expression-form` at the operator, naming the layer, including in a dead
  branch (FR-091-AC-39). The state-only subcases of FSpec TC-192 DIV-17 and
  DIV-18 are written as clause bodies; their complete-facet subcases are
  written as function bodies under `filament.value.complete/v1`, except AC-6's,
  which stays a clause body under both profiles.
- **SMT verdict row (FSpec TC-192 DIV-14).** The provider and route are the
  ones FSpec FR-349 and TC-266 already state: the SMT backend (cvc5, with Z3 as
  a cross-check whose absence is a warned outcome, FR-349-AC-6) registered
  under the AD-010 provider contract, an FR-147 mathematical-integer obligation
  carried as a value-validity item (FSpec FR-484), and the verdicts `proved`
  and `refuted`, a `refuted` result carrying a counterexample that replays
  through the FSL complete-V1 executor (FSpec FR-197, FR-098). The row is not
  an FSL evaluation and is not admission-only: until a cvc5 provider is in a
  consumer's lock the row is pending with that missing provider as its owner
  work, and no synthetic verdict counts. FSL adds no verdict route of its own.
  A provider registered through FR-281's external provider arm owns its verdict, and FSL settles `refuted`
  only after the counterexample replays to the violation at the original
  integers, so a provider's `refuted` is never assumed. A test double
  provider exercises that arm and does not make the DIV-14 row `proved` or
  `refuted`: only the registered cvc5 provider does.

- **Finite-backend negotiation (FSpec FR-147-AC-3).** An integer `div`,
  `rem` or `mod` application is a `value-validity` claim site (FR-062), and
  its requirement record's extent is computed from the checked operand types,
  never narrowed. When an operand's domain is a mathematical `Integer`, the
  derived domain is not complete, so a finite (bounded-mode) backend has no
  complete operand, intermediate and result bound to work in. The request
  writer SHALL write the item with its `Unbounded` extent over those roots and
  `finite_bound_available` true, as FR-335 states for any boundable integer
  root. A `requires-bound` disposition SHALL settle it `requires-bound` with
  no harness emitted and no S6a evaluation, and FSL SHALL NOT narrow the
  `Integer` operands to any `Int[..]`. The disposition is the same whether or
  not an SMT backend is in the lock, so the SMT verdict row above leaves it
  unchanged (FSpec FR-147-AC-8's second clause).
- **FSpec FR-147 trace.** FR-495-AC-5 is FSpec FR-147-AC-7 and AC-8 through
  FSL, and FR-495-AC-7 is FSpec FR-147-AC-3 and AC-8's unchanged-disposition
  clause. A test that asserts one of these ACs also carries the FSpec AC's
  trace tag.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-495-AC-1 | Under `filament.value.complete/v1` and each of the three division definitions, `function s1 using v(a: Int[10, 10], b: Int[3, 3]): Int[3, 3] pure { a div b }`, `function s2 using v(a: Int[10, 10], b: Int[7, 7]): Int[3, 3] pure { a rem b }` and `function s3 using v(a: Int[-7, -7], b: Int[3, 3]): Int[2, 2] pure { a mod b }` check, as does `function s4 using v(y: Int[1, 10]): Int[1, 10] pure { 10 div y }` (FSpec TC-192 DIV-15 and DIV-16: the quotient range is `[1, 10]`) and `function s5 using v(x: Int[-10, 5]): Int[-10, 5] pure { x rem (-1) }` (the remainder range is `[0, 0]`, although the quotient of `-10` would be `10`, outside the domain). `function s6 using v(a: Int[10, 10], b: Int[5, 5]): Int[1, 10] pure { a rem b }` refuses `undefined_expression`/`unproved-range` at the `rem` (DIV-15's `10 rem 5`, remainder `0`); `function s7 using v(a: Int[-9223372036854775808, -9223372036854775808], b: Int[-1, -1]): Int[-9223372036854775808, 9223372036854775807] pure { a div b }` refuses `unproved-range` at the `div` (DIV-05, DIV-06), while the same function with `b: Int[1, 1]` and `a` at either signed-64 endpoint checks; `function s8 using v(a: Int[-7, -7]): Int[0, 1] pure { a mod 3 }` refuses `unproved-range` (DIV-12: the Euclidean remainder `2`). `function s9 using v(a: Int[1, 10], b: Int[-2, 2]): Int[-10, 10] pure { a div b }` refuses `undefined_expression`/`unproved-nonzero` at the `div`, not `unproved-range`. | Test (TC-1284) |
| FR-495-AC-2 | Under `CheckMode::Kernel`, under each of the three division definitions and for the checked result type shown as the consumer domain, S6a evaluation of `(-9223372036854775808) div (-1)` over `Int[-9223372036854775808, 9223372036854775807]` returns `refused { code: division_out_of_domain, cause: quotient-outside-domain }` with `expected` that domain and no quotient, while `(-9223372036854775808) rem (-1)` completes with `0` (DIV-05); `MIN div 1` and `MAX div 1` complete and `MIN div -1` is refused as above (DIV-06); `(-7) mod 3` over `Int[0, 1]` with FSpec DIV-10's tuple except `result_units: 0` returns `refused { code: modulo_out_of_domain, cause: outside-domain }` after `integer-modulus.domain` with no `integer-modulus.result-retain` charge, and with `work_units: 2` returns `incomplete` at `integer-modulus.domain` (DIV-12); over `Int[1, 10]`, `10 div 3` returns `3`, `10 rem 7` returns `3` and `10 rem 5` returns `refused { code: division_out_of_domain, cause: remainder-outside-domain }` after `integer-division.domain` (DIV-15); `10 div y` at `y = 5` over `Int[1, 10]` returns `2` and `x rem (-1)` at `x = -10` over `Int[-10, 5]` returns `0` (DIV-16). No refusal exposes a quotient or remainder. | Test (TC-1284) |
| FR-495-AC-3 | `function d1 using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] pure { a div b }`, the same with `rem` and `mod`, `function d2 using v(a: P, b: P): P pure { a / b }` with `type P = Decimal[-1000, 1000; 0, 2; nearest-even];`, `function d3 using v(): Boolean pure { 1 cm / 0 cm = 1 cm / 0 cm }` (with TC-187's units) and `function d4 using v(): P pure { decimal(1, 0) / decimal(0, 3) }` each refuse `undefined_expression`/`unproved-nonzero` at the operator under `CheckMode::Linked`; `function d5 using v(a: Int[0, 9], b: Int[1, 9]): Int[0, 9] pure { a div b }` and `if b != 0 then a div b else 0` over `b: Int[0, 9]` check (the guard is an exact nonzero guard; the second function's result range is inside `Int[0, 9]`). Under `CheckMode::Kernel`, the same bodies evaluate at a zero divisor to `Undefined::DivisionByZero` (`division-by-zero`): `decimal(1, 0)` divided by `decimal(0, 3)` into `Decimal[-1000, 1000; 0, 2; nearest-even]` is undefined with no decimal value or loss record (TC-185 D06), and D15's tuple with `work_units: 1` is undefined after `decimal.operands` with no scale-expansion charge, with `work_units: 0` `incomplete` at `decimal.operands`; `7 div 0` and `7 mod 0` follow TC-192 DIV-01 and DIV-11, and `1 cm / 0 cm` follows TC-187 U27. | Test (TC-1284) |
| FR-495-AC-4 | Under a header profile `filament.state.core/v1` alone, `function f using v(a: Int[0, 9]): Int[0, 9] pure { a div 3 }` refuses `unsupported_construct`/`declaration-form` at the function declaration, naming `filament.state.core/v1`, and no `expression-form` refusal is made for its `div`; the same unit with `rem` and with `mod`, and a function whose only operator is the dead branch of `if true then 1 else (7 div 3)`, each refuse once, as a declaration. The same operators in the body of `invariant`, `pre` and `post` clauses over a model object type (FR-361-AC-5's unit) each refuse `unsupported_construct`/`expression-form` at the operator, naming `filament.state.core/v1`: `1 + 7 div 3 * 2 = 0`, `7 rem 3 = 0` and `-7 mod 3 = 0` (FSpec TC-192 DIV-17, state-only), and `(if true then 1 else (7 div 3)) = 0`, `(if true then 1 else (7 rem 3)) = 0` and `(if true then 1 else (7 mod 3)) = 0` (DIV-18, dead branch), the operator located at its own span. Under `filament.value.complete/v1` the same function bodies check and evaluate. | Test (TC-1284) |
| FR-495-AC-5 | With a cvc5 provider registered, FSpec TC-192 DIV-14's holding obligation settles `proved` and its violated obligation settles `refuted` through the FR-349 route, and the `refuted` counterexample replays through the FSL complete-V1 executor to the violation (FR-098); a lock with no provider reports DIV-14 pending on the provider and never `proved` or `refuted`; Z3 absent leaves cvc5's verdict settled with the absent-cross-check warning. | Test (TC-1284) |
| FR-495-AC-6 | Under a header profile `filament.state.core/v1` alone, an `invariant`, `pre` or `post` clause body over a model object type that states `7 / 3 = rational(7, 3)` refuses `ill_typed`/`type-mismatch` at the `/` operator, because the integer operands are never implicitly converted to a rational (FSpec FR-044 and TC-192 DIV-18: no ambient integer-to-rational conversion), and the same body refuses the same way when no rational type is expected; under `filament.value.complete/v1` the same body checks with the rational type expected (result 7/3) and refuses `ill_typed`/`type-mismatch` when no rational type is expected. This is the state-only subcase of FSpec TC-192 DIV-18's slash row. | Test (TC-1284) |
| FR-495-AC-7 | Finite-backend negotiation (QSL-1485; FSpec FR-147-AC-3, AC-8). Under `filament.value.complete/v1` and each of the three division definitions, `function q using v(a: Integer, b: Integer): Integer pure { if b != 0 then a div b else 0 }` checks, and its `div` application has one `value-validity` requirement record whose extent is `Unbounded` over the parameter nodes of `a` and `b` (domain kind integer); the same with `rem` and with `mod`. First, in a locked run with a cvc5 provider registered beside a test descriptor advertising (`value-validity`, `bounded`) whose disposition for the item is `requires-bound`, the bounded descriptor's item is written with `finite_bound_available` true and settles `requires-bound`, no harness is emitted for it, S6a evaluates nothing, and the request names `a` and `b` as `Integer`, not as any `Int[..]`. Then, in a locked run with the bounded descriptor alone and no SMT provider registered, each item settles the same `requires-bound` disposition with the same request. | Test (TC-1284) |
| FR-495-AC-8 | Through the existing public `fsl-replay` compile facade, compile genuine bounded-operand `div` and `rem` functions with each owner-admitted Euclidean, Floor and Truncating selection. The supplied capability, checked nodes and emitted `operation.laws` retain that exact owner law into CG; the existing source-backed FSL/CG agreement path matches FSpec TC-192 at positive and negative operands, including negative-divisor controls that distinguish the laws. This obligation supplies the admitted law to CG/source-backed IR-1391 agreement; a new byte-only execution surface is outside this scope. Missing demanded selection returns typed `MissingSelection(IntegerDivision)` at the operator; unknown, foreign or conflicting definition bindings retain native typed refusals before execution. A non-division and a `mod`-only control compile without a division selection. No test assigns a private declarations field, invents a law string or substitutes CG behavior for the public-facade selection. | Test |
| FR-495-AC-9 | Complete-facet subcases of FSpec TC-192 DIV-18's slash row, written as function bodies under `filament.value.complete/v1` (the state-only subcases are AC-6's clause bodies, which stay clause bodies under both profiles). `function sl1 using v(): Rational[0, 7; 1, 3] pure { 7 / 3 }` checks, the integer operands promoted to `7/1` and `3/1` by FSpec FR-149, and evaluates to `7/3` after the nonzero and domain checks; `function sl2 using v(): Boolean pure { 7 / 3 = 7 / 3 }` has no expected rational type and refuses `ill_typed`/`type-mismatch` at the first `/`; neither body is read as `div`. Under `filament.state.core/v1` a `function` carrying either body is refused as a declaration first (AC-4), so the slash refusal code of the state-only subcase is only AC-6's clause form. | Test (TC-1284) |
| FR-495-AC-10 | `function p using v(x: Option<Integer>): Integer pure { value(x) }` refuses `undefined_expression`/`unproved-presence` at the `value` under `CheckMode::Linked` and checks under `CheckMode::Kernel`, which records no presence obligation; `function g using v(x: Option<Integer>): Integer pure { if present(x) then value(x) else 0 }` checks under both modes. | Test (TC-1419) |
| FR-495-AC-11 | Over the ConfigVersion package (FR-103-AC-1), checked through `PackageDeclarations::check` (not `CheckedGraph::check_clause_expression`), `invariant NoGuard using v on Config::ConfigVersion at current { deref(value(self.parent)).versionNumber < self.versionNumber }` refuses `undefined_expression`/`unproved-presence` at the `value` under `CheckMode::Linked` and under `CheckMode::Kernel`, one refusal each; the guarded `present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber` (FR-104-AC-1's `ParentOrder`) checks under both. | Test (TC-1419) |
| FR-495-AC-12 | `function w using v(x: Option<Integer>): Int[0, 9] pure decreases(value(x)) { 0 }` refuses `undefined_expression`/`unproved-presence` under `CheckMode::Linked` and under `CheckMode::Kernel`. `function f using v(x: Int[0, 9]): Int[0, 9] pure { f(x) }` refuses `undefined_expression`/`unproved-decrease` with the `missing-measure` obligation at the call `f(x)`, the root of the body, under both modes. `function f using v(x: Int[0, 9]): Int[0, 9] pure { f(x) + 1 }` refuses `unproved-range` at the `+` under `Linked`, as its only refusal, and under `Kernel`, which records no range obligation, refuses `unproved-decrease`/`missing-measure` at the call `f(x)`, path `[0]`, the first operand of the `+`, as its only refusal. | Test (TC-1419) |
| FR-495-AC-13 | Under `CheckMode::Kernel` and under `CheckMode::Linked`, the definedness walk of an already-checked body holding a rational operation (`Rational` or `RationalNegate`) whose operand node has type `Integer` refuses `undefined_expression`/`unproved-range` at that operation. Through `PackageDeclarations::check` under each mode, a package whose dispatch operation names a dispatch table index equal to the table count, and one whose dispatch table names a candidate function index equal to the function count, each refuse `invalid_package`/`invalid-value`. Each outcome is the same refusal under both modes; `Kernel` leaves none of the three to the evaluator. The dispatch refusals are made before any body walk, so today they cannot differ by mode; the criterion guards that they stay independent of the mode. | Test (TC-1420) |
| FR-495-AC-14 | `function r using v(a: Rational[0, 1; 1, 2], b: Rational[0, 1; 1, 2]): Rational[0, 1; 1, 2] pure { a + b }` refuses `undefined_expression`/`unproved-range` at the `+` under `CheckMode::Linked`, as its only refusal (the declared domain bounds the reduced numerator to `[0, 1]` and the denominator to `[1, 2]`; the checker adds the two operand domains and gets numerator `[0, 4]` and denominator `[1, 4]`, which the declared domain does not contain; the refusal is the rational-range obligation), and checks under `CheckMode::Kernel`, which records no rational-range obligation, so the evaluator meets the out-of-domain result. | Test (TC-1421) |
| FR-495-AC-15 | `function q using v(a: Rational[0, 1; 1, 2], b: Rational[0, 1; 1, 2]): Rational[0, 100; 1, 100] pure { a / b }` refuses `undefined_expression`/`unproved-nonzero` at the `/` under `CheckMode::Linked`, as its only refusal (the divisor's range contains zero with no exact nonzero guard), and checks under `CheckMode::Kernel`, which records no nonzero obligation for a rational `/`, so a zero divisor is met by the evaluator as `Undefined::DivisionByZero` ("Zero divisor, two observations"). | Test (TC-1421) |
| FR-495-AC-16 | With `type S = Int[0, 3];`, `function m using v(q: Sequence<S>[0, 2]): S pure { reduce<S>(acc, x in q: acc) }` (a `reduce` with no identity over a source that may be empty) refuses `undefined_expression`/`unproved-range` at the `reduce` under `CheckMode::Linked`, as its only refusal (the nonempty-reduction obligation, `size(q) >= 1` unproved), and checks under `CheckMode::Kernel`, which records no nonempty-reduction obligation, so the evaluator meets the empty source; the same function with `q: Sequence<S>[1, 2]` proves `size(q) >= 1` and checks under both modes. | Test (TC-1421) |


## Dependencies

- FSpec FR-147 and TC-192 own the division laws, the charges and the
  DIV rows; FSpec FR-140 and TC-185 own the decimal D rows; FSpec FR-142 and
  TC-187 own U27. FR-044 owns the proof obligation.
- FR-100 and FR-096 own the kernel outcome mapping and define the Kernel check
  mode for `sum`; this requirement extends that definition to the division
  operators named above.
- FR-107 is the S6a consumer of the clause outcomes.
- FR-110 and FSpec `state-core.md` own the layer's admitted forms.
- Rows that need a source spelling for a Decimal- or Integer-domain quantity
  target (TC-187 U07, U08, U16 to U18, U29) are specified by FR-497
  (`Quantity<u, C>`, its lowering and its kernel conversion, TC-1287); the
  bounded-`Int` result-refusal rows (DIV-05, DIV-06, DIV-12, DIV-15, DIV-16)
  are FR-495-AC-1 and AC-2 here (QSL-1389).
