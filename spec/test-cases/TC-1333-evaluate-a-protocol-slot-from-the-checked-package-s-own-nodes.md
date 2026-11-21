---
id: TC-1333
title: "Evaluate a protocol slot from the checked package's own nodes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-533
    type: verifies
---
# TC-1333: Evaluate a protocol slot from the checked package's own nodes

## Description

Verify protocol-slot evaluation from a bare package and its equality with the source routes. Scope: FR-533-AC-1 to FR-533-AC-23. Row 23 verifies the bounded quantity-call refusal while equivalence reconstruction remains unavailable. **R** is the package of FR-517.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | every slot of FR-390's table **R** holds, over the TC-1070 and TC-1315 supplies, on the bare, in-process and reader-backed routes | equal value, category, exit status, catalog code and cause, unavailable binder, location, losses and ordered charge log |
| 2 | 2 | each outcome row of FR-520's map; `retry.relation` at exact usage and one below | the successor outcomes; completes / `Incomplete` naming the counter |
| 3 | 3 | the calls of row 1 in a run that never compiled the source; the in-process `CheckedPackage` held beside the bare package replaced by another unit's with a different expression at the slot | the same results; the bare result is unchanged, the in-process one changes |
| 4 | 4 | from `evaluate_protocol_slot`, `admit_protocol_observation` and `with_supply`, search the bare route's reachable functions for `CheckedPackage`, `CheckedBody`, `source()`, source `Graph` and `protocol_selection::select`; the `NodeKind` match | none; no wildcard arm |
| 5 | 5 | fixture E-P's handoff (TC-1070), documents `M` and `N`: admit the observation state of each of TC-1070 rows 34 to 46 on the bare and source routes, then run each row's slot call with its value and metadata; read the bare host's population types for `nodes`, `otherNodes`, `otherSpare`, `elsewhereNodes` and the bare environment's `attributes` and `conforms` for `Node`, `Other`, `Elsewhere`; admit for model `N`, also over a `BarePackage` with no supply; a snapshot whose `OtherLinks` instance has a `Node` source end | the same admission variant on both routes, with equal `AdmissionUsage`, `populations()`, `environment().references()` and attribute values when admitted, equal records otherwise; each slot call's result, record, locus and charge log equal to the source route's (row 40 `wrong-model-selection`, row 41 `wrong-value-kind`); population types and environment answers equal to the source's; `wrong-model-selection` with no fields, empty path, no kernel record on both routes and with no supply; `wrong-value-kind` with equal `relationship`, `instance`, `end` and path, `type`/`declared` the original identities on the bare route and the alias-qualified names on the source route (recorded unmet parity) |
| 6 | 6 | fixture E-P compiled twice, against its model and against a copy under another identity; the first package's bare observation supplied with row 35's value and metadata to the second's bare slot call; over the first package, the observation of one `BarePackage` value supplied to another `BarePackage` value over the same bytes, then to its own | `wrong-model-selection` at the `Binder` locus of `forward`, at the population check, with the source route's record for the same packages; `wrong-model-selection` at that locus for the other `BarePackage` value and value 5 for its own; meter identical after each refusal |
| 7 | 7 | slots over a plain-Option record, a record with a plain Collection member and a record with an Optional Collection member; a record whose field rows are absent | bare: the first three evaluate with charges equal to the source route's, each binder's value type equal, and the same record value usable unchanged in both environments in original selected declaration-key field order; the last refuses `MissingTypeRepresentation` at the binder, no charge; source route: evaluates |
| 8 | 8 | an `Unavailable` binder that is read | `Unavailable` with the package's location; `None` where the wire has none while the in-process route keeps its source location, recorded as unmet parity |
| 9 | 9 | three slot calls on one meter against three on fresh meters; against the source route | sum for `work_units` and `result_units`, maximum for the other eight; equal to the source route |
| 10 | 10 | `new`, `with_supply`, view creation, selection and supply admission | no counter charged, no `Meter` taken; a refusal leaves the meter identical |
| 11 | 11 | fixture L (TC-1070): select `Decide.visible[0]`, `Decide.cases.guard[0]` and `Decide.cases.guard[1]` by (node, member, index) with a descriptor that carries no span | origins `... decision 2653..2662`, `... Decide case no guard`, `... Decide case yes guard` respectively; one origin per slot; no ambiguity refusal |
| 12 | 12 | every slot of fixtures E, E-C and L on both routes; the fixture L package with the origins `decision 999..1005` and `decision 1000..1004` added, each holding a copy of the `got.ready` row of `decision 2653..2662` numbered on after the existing `term_ordinal` values | equal triple, root, binders and origin; `Got.related.*[0]` have no origin; in the added package `visible[0]` takes `999..1005` (numeric order, not byte order) and the case guards keep their `case` origins |
| 13 | 13 | two package copies of fixture L edited so that `Decide.cases.guard[0]` and `Decide.cases.guard[1]` both have the `got.ready` application as root, retaining the emitted negation node, removing its row under `case no guard` and rewriting that origin's `got.ready` row as the root row with the negation row's `Location`: labels `no` and `yes` kept; labels renamed to `b` and `a` with their origins; a package copy whose `related` array holds two records of one relationship whose `from` ends are one call and whose `to` ends differ, in the emitted sort order, with distinct `Location` values under `related 0 from` and `related 1 from` | actual call values and charge logs of the two guards equal; completed evaluations have no stop location; `guard[0]` takes `case no guard` and `guard[1]` `case yes guard` in the first, `case a guard` and `case b guard` in the second, each reading its own origin's projected-root `Location`; `related.from[0]` and `related.from[1]` read the `Location` of `related 0 from` and `related 1 from` |
| 14 | 14 | fixture L's package with the rows of `decision 2653..2662` removed; with the rows of `case yes guard` removed; the identity-call variant of fixture L; that variant with the rows of `related 0 to` removed | `visible[0]` refuses `MissingLocationProjection` at the `Node` locus of `got.ready`; `cases.guard[1]` refuses the same; `related.from[0]` takes `related 0 from` and `related.to[0]` `related 0 to`; `related.to[0]` refuses `MissingLocationProjection` at the call; no charge |
| 15 | 15 | bare slot calls with no supply, foreign rows, unknown rows, for identity rows and for equality rows; a missing node, a missing member, `MissingTypeRepresentation` | `Package` locus for every witness refusal; `Node`, `Member`, `Node` for the others; exit 21, meter identical |
| 16 | 16 | fixture E's `Refund` retry relation and recovery predicate, the TC-1289 slot and the TC-1290 slot, with equality rows read from the handoff directory; the same slots with the file omitted; `Refund`'s `trigger.guard` with the file omitted | the in-process route's value and charge log; `MissingEqualityWitness`; evaluates |
| 17 | 17 | search the bare route for derivation of equality rows, reads of the three `Emission` row accessors and name-text selection of an origin | none outside the stated rule |
| 18 | 18 | a unit compiled from source in which `trigger.guard` of a copy of fixture E calls a function whose body holds `==`; equality rows omitted, then the rows its handoff writes; a slot that calls no such function, rows omitted | `trigger.guard` refuses `MissingEqualityWitness` at `Package`, meter untouched, then evaluates equal to the in-process route; the other slot evaluates |
| 19 | 19 | **R**'s `Full` and `Partial` `recovery.predicate` over the bare route, with supplies that differ in receipts, target and activation; `Full` with its sum inside `Shape::Circle` and a `Circle(r)` case arm, over matching, mismatching, empty and activated supplies; a `recovered` supply that is an integer; a supply without `target`; receipts and targets supplied as quantities of the source-owned unit `U`; source-unit rows that are absent, repeated, name a non-unit node or carry an identity the unit node does not; a native work allowance that pays the identity join but not the unit read | the values FR-533-AC-19 states, over `U` quantities; the case-arm package is strictly admitted, `r` is not a source on either route, and its Boolean outcomes and ordered charge logs equal the source route's; both malformed supplies refuse as `CallFailure::Admission`; absent unit rows refuse the equality decisions over `U` (`Equality(Unknown)`), and repeated, non-unit or mismatched unit rows refuse the identity witness as `Identity(Unknown)`; the exhausted allowance is `NativeJoinFailure::UnitDefinitions` with the `i2.work_units` setting and its stage limit diagnostic |
| 20 | 20 | Strictly admitted **R** with the source `Shape` union constructed and matched in `Full`'s recovery predicate; vary receipts, target and activation, then add an unrelated union; compare the bare and source type environments | Equal Boolean values and ordered charges; original union type handle, sealed member/declaration identities, ordered members and positional payload types agree; missing required projections refuse before any charge; an unused union with an unrepresented tuple payload leaves the ordinary slot unchanged, while selecting even its nullary variant refuses before charging |
| 21 | 21 | fixture L's two-record variant (TC-1063 row L4e), compiled in each source order of its two clauses, on the in-process and bare routes | `Got.related.from[0]`, `from[1]` and the call `to` end: equal values, charge logs and projected-root read `Location` on both routes; `from[i]` under `related <i> from`, the call `to` end under `related <k> to` with `k` its record's emitted position; the two `from` `Location` values differ; no `Location` under a `related <start> <end>` origin on either route |
| 22 | 22 | fixture E-P with one unread variant (enum) type added to `M`: `with_supply`, `Refund.trigger.guard`, admission for `M` on both routes; fixture E-P with a native budget that pays every earlier `with_supply` join and the declaration projection but not the population catalog | `with_supply` succeeds and the guard equals the source route's value and charge log; bare admission refuses `unknown_required_feature`/`unsupported-feature`, `reason` `missing-type-representation`, no fault, while the source route admits; `NativeJoinFailure::PopulationCatalog` with its setting and a stage limit diagnostic whose bound and attempted total are the catalog error's `limit` and `consumed`, no `BarePackage` |
| 23 | 23 | fixture Q, emitted and read through the actual package reader with identity/equality supply, called through the source package and the resulting `BarePackage`; repeat the five selected bare refusals with unlimited limits and with each tight meter below | source conversions return `1/4 s` and `3/4 s`; product, quotient and both square equalities are true. Bare cross, affine_class, product, quotient and the selected noncanonical square return the exact typed unsupported refusal and retained expression span below, no quantity and none of the five prohibited unit charges. The refusal remains typed unsupported under each tight meter, rather than becoming kernel-limit Incomplete. Bare same returns `1 s`, affine_same returns `3 beat`, and ordinary_power returns true. |

### Q — Dimension-equivalence reconstruction boundary

Compile this source using the normal source checker. The symbols `s` and `beat`
are nominal roots of their own declared dimensions, not an assumed spelling of
the equivalence class's canonical representative.

```fsl
language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
dimension Time;
unit s : Time = rational(1, 1);
unit ms : Time = rational(1, 1000) * s;
dimension Beat;
unit beat : Beat = rational(1, 1);
unit bar : Beat = rational(4, 1) * beat;
dimension Beat ~ Time by 1 beat = rational(1, 4) s;
unit shifted : Beat = rational(1, 1) * beat + rational(2, 1);
dimension Length;
unit m : Length = rational(1, 1);
function cross using v(): Quantity<s> pure { convert<Quantity<s>>(1 beat) }
function affine_class using v(): Quantity<s> pure { convert<Quantity<s>>(1 shifted) }
function product using v(): Boolean pure { 6 s * 3 beat = rational(9, 2) s * 1 s }
function quotient using v(): Boolean pure { 6 s / 3 beat = 8 s / 1 s }
function power_s using v(): Boolean pure { 2 s ^ 2 = 4 s * 1 s }
function power_beat using v(): Boolean pure { 2 beat ^ 2 = rational(1, 4) s * 1 s }
function same using v(): Quantity<s> pure { convert<Quantity<s>>(1000 ms) }
function affine_same using v(): Quantity<beat> pure { convert<Quantity<beat>>(1 shifted) }
function ordinary_power using v(): Boolean pure { 2 m ^ 2 = 4 m * 1 m }
```

1. Verify the projected unit graph retains distinct declared unit identities for
   `s` and `beat`. The class representative is selected by the admitted least
   dimension key, not the spelling or declaration order. Inspect the projected
   canonical root of each nominal unit: exactly one differs from that unit's
   original declared identity. Select `power_s` if that unit is `s`, or
   `power_beat` if it is `beat`. Assert this root difference before evaluation.
   If the representative is Beat, one `s` is four class roots and one `beat` is
   one; if it is Time, one `s` is one class root and one `beat` is `1/4`.
2. Evaluate the source functions. `cross` is exact `1/4 s`, `affine_class` is
   exact `3/4 s`, and `product`, `quotient`, `power_s` and `power_beat` are true.
   The product is `72 beat^2` if Beat is representative, or `9/2 s^2` if Time
   is representative; the quotient is dimensionless `8`. The selected square
   is `64 beat^2` for a noncanonical `s`, or `1/4 s^2` for a noncanonical `beat`.
   Check that the product's nominal roots `s * beat`, the quotient's nominal
   roots `s / beat`, and the selected square's nominal root squared differ
   from their checked class-root result units. A noncanonical operand alone
   does not trigger refusal if those result units are equal.
3. Emit the checked package, read its actual emitted representation using the
   normal package reader, join its identity/equality supply and construct the
   resulting `BarePackage`. Do not substitute a mock world or an evaluator
   retaining the source unit graph for the bare call. The source package may
   remain beside the bare value solely as an independent expected-value and
   retained-location oracle, as the existing source/bare test helper does.
4. On the bare route, call `cross`, `affine_class`, `product`, `quotient` and
   the selected square. Each returns `unsupported_construct` /
   `not-yet-implemented` with
   `what = dimension-equivalence-package-reconstruction` and no quantity.
   Assert the exact source substring of its retained stop location:
   `convert<Quantity<s>>(1 beat)`, `convert<Quantity<s>>(1 shifted)`, the first
   `6 s * 3 beat`, the first `6 s / 3 beat`, and the first `2 s ^ 2` or
   `2 beat ^ 2` for the selected member. Assert the occurrence start/end as
   well as text so the repeated right-hand product or square cannot satisfy
   the location oracle accidentally. Conversion roots have depth 0; the
   selected arithmetic operands of the Boolean equality have depth 1.
5. Inspect each bare call's actual admitted charge log: it contains no
   `UnitIdentityRead`, `UnitEdge`, `UnitRationalArithmetic`, `UnitTargetDomain`
   or `UnitResultRetain` for the refused operation. A function-entry charge is
   permitted. Repeat each refusal on fresh meters whose other allowances are
   generous, setting separately `work_units = 1`, `unit_edges = 0`,
   `integer_bits = 0` and `result_units = 0`. These no-argument literal bodies
   require only the earlier `FunctionCall` work charge to reach the guard;
   `work_units = 0` would instead stop at that entry and is not a boundary
   test. Each selected call still returns the same typed unsupported refusal,
   location and absence of unit charges, rather than entering the shared
   kernel and returning limit Incomplete.
6. Call `same`, `affine_same` and `ordinary_power` on source and bare routes
   under generous meters: they return `1 s`, `3 beat` and true respectively.
   The integer-power guard is conditional on differing nominal and checked
   result units; it is not a refusal of every integer power. Reader recovery
   of equivalence definitions remains an unmet dependency; these calls do not
   claim source/package parity for that deferred reconstruction.

Tag each test with `TC-1333` and the criteria it actually verifies; row 23 carries `FR-533-AC-23`. Rows 1 and 2 also carry FR-390-AC-11 and FR-520-AC-9.

## Expected Results

Each row's Expected column.
