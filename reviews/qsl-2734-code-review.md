---
id: SR-202611210022442313144108506
title: "QSL-2734 PR3286 code-review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3286 1d666e7daa977b75f02e1bdefbd89b361d0449fd vs merge-base 8afc37d8585d4acb6ebd42e3e129dad6203a432a; Cargo.toml, Cargo.lock, fsl-eval/src/value/expression/evaluate.rs, fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/assemble/equivalence.rs, fsl-semantics/src/check/mod.rs, fsl-semantics/src/check/refusal.rs, fsl-replay/src/spine.rs, fsl-replay/src/spine/refinement.rs, fsl-eval/tests/it/quantity_class_convert.rs, fsl-eval/tests/it/quantity_literals.rs, fsl-eval/tests/it/main.rs, fsl-semantics/tests/it/dimension_equivalence.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-533
    type: references
---

## Summary

Ticket: QSL-2734. Frozen head 1d666e7daa977b75f02e1bdefbd89b361d0449fd, merge-base 8afc37d8585d4acb6ebd42e3e129dad6203a432a. Code review with the rust-review and rust-style lane folded in. Inspection only: no Cargo build, test, clippy, fmt or Kani was run (prohibited by the dispatching brief); runtime evidence is the coder's reported targeted runs, not reproduced here.

## Verdict

CONDITIONAL. The old unmetered Rational class converter from 44829acd89 (`Scope::class_convert`, `across_class`, `units_convert`, `World::class_convert`) is gone at HEAD. Source checking and evaluation now project admitted classes through the shared `UnitGraph::with_dimension_equivalences` at semantic-value 6e809434, which owns conversion edges and meter charges. Identification edges carry a zero offset, nominal identities stay in place, and ordinary and affine nominal conversions are unchanged. Cross-nominal add, subtract, equality and order still refuse. The bare product, quotient and conversion guards run before any shared charge. The manual `Hash` on `DimensionClassProjectionRefusal` agrees with its derived `Eq`. The FR-497-AC-2 oracle change is grounded: carried operands miss both quantity arms of the checker's arithmetic match and reach `OperandTypeMismatch`, whose catalog cause is `type-mismatch`. The test asserts operator, both full carried types, owner/index, root path and the exact source span. Pins match the approved revisions, with core-data left unchanged. The import reordering and line rewraps in assemble.rs, mod.rs and spine.rs are formatter churn with no behavior change. Remaining defects: the integer-power bare guard runs after metered shared evaluation, and the class projection is built twice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Bare integer-power refusal runs after the shared metered power evaluation, so it charges unit edges/arithmetic/retain before refusing and, under a tight meter, returns a limit Incomplete instead of the specified typed unsupported construct, unlike the product/quotient guard which refuses before any charge. | fsl-eval/src/value/expression/evaluate.rs:3426-3446 |
| FND-002 | low | Assembly projects the classes into `names.unit_table`, but `type_units.append` keeps the first (nominal) entry for every declared id, so check() must re-run `EquivalenceClasses::new` + `with_dimension_equivalences` and rebuild the table in reverse order; the overlay depends on append's first-wins ordering and builds the projection twice. | fsl-semantics/src/check/assemble.rs:5197-5213; fsl-semantics/src/check/assemble.rs:5851-5853; fsl-semantics/src/check/mod.rs:2396-2413 |
| FND-003 | low | `convert<Quantity<ua>>(3 ub) = 3 ua` is false under `1 ua = 1000 ub`; the test only asserts it checks, so the asserted expression misstates the conversion. | fsl-semantics/tests/it/dimension_equivalence.rs:1334 |

FND-001: in the `NodeKind::QuantityPower` arm, `evaluate_quantity_unit(QuantityOperation::Power(..), self.meter)` charges `charge_edges`, `charge_rational_target` and `charge_retain` (semantic-value quantity.rs 780-791) before the World::Bytes result-unit comparison. FR-533 "Dimension equivalence reconstruction boundary" requires the same refusal for product, quotient and integer power. Fix: compute the nominal power unit before evaluating, as the Multiply/Divide arm does at 3315-3349, or check the static result unit first.

FND-002: either let the assembly-time projection's entries win in `type_units` (append the projected table first), or project once and pass it to check(). That removes the second `EquivalenceClasses::new` and the comment about deliberate first-entry retention.

## Scope

- fsl-eval/src/value/expression/evaluate.rs (examined): bytes-route guards in the Quantity, QuantityPower and ConvertScalar arms; `quantity_unit` helper.
- fsl-semantics/src/check/assemble.rs (examined): runtime unit projection into `Names::unit_table`; `AssemblyCause::DimensionClassProjection` code and catalog cause.
- fsl-semantics/src/check/assemble/equivalence.rs (examined): `EquivalenceClasses::memberships`.
- fsl-semantics/src/check/mod.rs (examined): check-time reprojection and type-table unit replacement.
- fsl-semantics/src/check/refusal.rs (examined): `DimensionClassProjectionRefusal` manual Hash; `KeyFault::DimensionClassProjection` invariant id.
- fsl-replay/src/spine.rs, fsl-replay/src/spine/refinement.rs (examined): exhaustive-match arms and message.
- Cargo.toml, Cargo.lock (examined): five approved revisions; core-data unchanged.
- fsl-eval/tests/it/quantity_class_convert.rs, fsl-eval/tests/it/quantity_literals.rs, fsl-semantics/tests/it/dimension_equivalence.rs (examined): oracles for FR-093-AC-46, FR-533-AC-23, FR-497-AC-2.
- filament-semantic-value 6e809434 src/unit/equivalence.rs, src/quantity.rs (context_only): shared projection, `converts_to`, `result_unit`, metered `convert`.

## Dispositions

Round 1, fix head 4219c63abebbce1dd776d930e126e778c4bd5d96 (original head 1d666e7daa977b75f02e1bdefbd89b361d0449fd). The reason column records the evidence; after-excerpts are in the round-1 dispositions comment.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At 4219c63abebbce1dd776d930e126e778c4bd5d96 the bytes route probes the result unit with the public evaluate_quantity_unit(QuantityOperation::Power(unit_base, &exponent)) on a separate all-zero ScalarLimits meter before the caller-metered call. Shared evaluate_quantity_unit runs type_check and IdentifiedUnit::new, then evaluate stops at its first UnitIdentityRead charge, so no value arithmetic runs and nothing reaches the caller meter. This is the checker's Typer::power NO_CHARGE pattern. On a match the real operation runs once on self.meter. |
| FND-002 | fixed | Assembly now appends types.units() after the projected names.unit_table, so the projected entries win. check() no longer reprojects, and the dead KeyFault::DimensionClassProjection / DimensionClassProjectionRefusal wrapper is removed. The nominal PackageDeclarations.units stays for TimeUnits. |
| FND-003 | fixed | The predicate is now convert<Quantity<ua>>(1000 ub) = 1 ua, which is true under 1 ua = 1000 ub. |

Round 2, final head 7eb36c927d92a4b7637c05c608d09db5e819f2b8 (delta from 4219c63abebbce1dd776d930e126e778c4bd5d96). Each round-1 fix was re-verified at this head; none of the fixed files or spec rows changed in the delta. Delta review, no new findings. Pins: the lock source changes only IR 0e089f6e -> dacfac6b and FSpec 6ad873d5 -> 247a3941; SV, checked/eval, observation and core-data are unchanged. Reader quantity.pow projects the emitted Operands::Two(base, exponent) to the existing NodeKind::QuantityPower(base, exponent), the same two-child shape as quantity arithmetic, with no type or arithmetic reconstruction, so Bare admission reaches the early power guard. QSL-2728 producer sub-scope: lowering takes the let binder from the just-pushed scope entry (binder.parameter); MemberTerm::let_binding carries Some(binder); map_refs and the identity walk include it; BodyNames adds it as a dependency; emit member_term maps it with admitted_id(...).transpose(); every non-let binding emits None, and the 28 mechanical patterns only add `..`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. At 7eb36c927d92a4b7637c05c608d09db5e819f2b8 the bytes route probes the result unit with the public evaluate_quantity_unit(QuantityOperation::Power(unit_base, &exponent)) on a separate all-zero ScalarLimits meter before the caller-metered call. Shared evaluate_quantity_unit runs type_check and IdentifiedUnit::new, then evaluate stops at its first UnitIdentityRead charge, so no value arithmetic runs and nothing reaches the caller meter. This is the checker's Typer::power NO_CHARGE pattern. On a match the real operation runs once on self.meter. |
| FND-002 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. Assembly now appends types.units() after the projected names.unit_table, so the projected entries win. check() no longer reprojects, and the dead KeyFault::DimensionClassProjection / DimensionClassProjectionRefusal wrapper is removed. The nominal PackageDeclarations.units stays for TimeUnits. |
| FND-003 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. The predicate is now convert<Quantity<ua>>(1000 ub) = 1 ua, which is true under 1 ua = 1000 ub. |
