// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 exact scalar semantics (FSL #118, AD-005).
//!
//! This typed layer neither widens nor reuses the root crate's historical
//! `checking::NativeType`. Layers:
//!
//! 1. typed values: `filament_exact::Integer`,
//!    `filament_exact::IntegerInterval`/`filament_exact::BoundedInteger`,
//!    [`Rational`](filament_exact::Rational), [`Decimal`](filament_exact::Decimal),
//!    [`Text`](filament_exact::Text), `filament_semantic_value::enumeration::EnumValue` and FR-142
//!    `filament_exact::Quantity` values read against a
//!    `filament_semantic_value::quantity::UnitTable` over an admitted
//!    `filament_semantic_value::unit::UnitGraph`;
//! 2. explicit operation tables: [`evaluate_decimal`](filament_exact::evaluate_decimal)
//!    (FR-140), [`divide`] and `filament_eval::modulo` (FR-147),
//!    [`admit_text`](filament_exact::admit_text),
//!    [`compare_text`](filament_exact::compare_text) and `filament_semantic_value::enumeration::compare_enum`
//!    (FR-141), `filament_semantic_value::quantity`'s `evaluate_quantity` and
//!    `convert_quantity` (FR-142), after
//!    the type-checking [`IllTyped`](filament_exact::IllTyped) refusal;
//! 3. FR-143 records, tuples and finite recursive
//!    [`Value`](filament_exact::Value)s over a
//!    `filament_semantic_value::declaration::TypeEnvironment`, the FR-149 equality matrix
//!    (`TypeEnvironment::check_equality`) and FR-144 bounded
//!    [`CollectionValue`](filament_exact::CollectionValue)s
//!    ([`construct_collection`](filament_exact::construct_collection)) (FSL #119); FR-307
//!    library resolution relocated to the top-level `library` module
//!    (FR-087, #213 S-3a);
//! 4. [`order_numbers`](filament_exact::order_numbers) compares one
//!    [`OrderedOperands`](filament_exact::OrderedOperands)
//!    pair (`Integer`, `Rational` or `Decimal`) under one
//!    [`OrderingOperator`](filament_exact::OrderingOperator);
//!    [`evaluate_integer_arithmetic`](filament_exact::evaluate_integer_arithmetic)
//!    evaluates add/subtract/multiply/negate
//!    over [`IntegerArithmetic`](filament_exact::IntegerArithmetic) operands
//!    (integer division is rational division of `n`/`1`, so it has no
//!    `Divide` variant here) and
//!    [`evaluate_rational_arithmetic`](filament_exact::evaluate_rational_arithmetic)
//!    evaluates add/subtract/multiply/divide/negate over
//!    [`RationalArithmetic`](filament_exact::RationalArithmetic) operands;
//!    [`evaluate_boolean`](filament_exact::evaluate_boolean) evaluates the FSL
//!    connectives `and`/`or`/`implies`/`not` over
//!    [`BooleanConnective`](filament_exact::BooleanConnective) operands — all
//!    under `filament.value.accounting/v1` (FSL #119);
//! 5. the distinct evaluator [`Outcome`](filament_exact::Outcome) with typed
//!    [`Undefined`](filament_exact::Undefined), [`Refusal`](filament_exact::Refusal)
//!    and [`Incomplete`](filament_exact::Incomplete) reasons -- the kernel's own
//!    types (this module's byte-identical copy is deleted);
//! 6. `filament.value.accounting/v1` metering through
//!    [`Meter`](filament_exact::Meter).
//!
//! FR-148 IEEE binary32/binary64 profiles
//! ([`evaluate_ieee`](filament_exact::evaluate_ieee),
//! [`compare_ieee`](filament_exact::compare_ieee)) operate on exact bit patterns
//! with soft-float arithmetic over big integers. This
//! module's byte-identical `decimal`/`ieee`/`numeric`/`text` engine copies;
//! every one of the operation-table items above now names its `filament_exact`
//! definition directly, not a re-export from here.
//!
//! Profile selection and misuse are refused at semantic admission by
//! closed role-specific admission functions. No host floating-point arithmetic or narrowing integer
//! conversion is used by any semantic path.
//!
//! ## FR-078: no `negotiate_*` copies (TC-201)
//!
//! The RT capability-negotiation copies `negotiate_ieee`,
//! `negotiate_integer_division` and their supporting types from `ieee` and
//! `division` (FR-078-AC-1, FR-078-AC-2); only the evaluation functions
//! listed above remain. Each removed name is gone from this crate, not
//! renamed or gated: importing it from `value` is `E0432`, an unresolved
//! import, not a type or borrow error.
//!
//! ```compile_fail,E0432
//! use fsl_semantics::value::negotiate_ieee;
//! ```
//! ```compile_fail,E0432
//! use fsl_semantics::value::negotiate_integer_division;
//! ```
//! ```compile_fail,E0432
//! use fsl_semantics::value::IeeeBackendCapabilities;
//! ```
//! ```compile_fail,E0432
//! use fsl_semantics::value::IeeeItemRequirement;
//! ```
//! ```compile_fail,E0432
//! use fsl_semantics::value::IeeeUnsupportedCause;
//! ```
//! ```compile_fail,E0432
//! use fsl_semantics::value::IeeeDisposition;
//! ```
//! ```compile_fail,E0432
//! use fsl_semantics::value::IntegerDivisionBounds;
//! ```
//! ```compile_fail,E0432
//! use fsl_semantics::value::IntegerDivisionConsumer;
//! ```
//! ```compile_fail,E0432
//! use fsl_semantics::value::IntegerDivisionDisposition;
//! ```
//!
//! A positive control: the module path itself still resolves, and a
//! surviving name imports cleanly, so the failures above are each the named
//! removed item, not a broken crate path.
//!
//! ```
//! use fsl_semantics::value::AdmittedIeeeProfile;
//! fn _use(_: AdmittedIeeeProfile) {}
//! ```

// ADR-011 §6.1 layer 3: the §6.2 `semantic_value` submodules and
// `value::model_query` (§6.2: `model`), with the flat re-exports of their own items. The layer-5 S6a
// evaluator, `value::expression`, is in the `fsl-eval` crate's own `value`
// module and imports these by their
// `fsl_semantics::value::<submodule>` path. The `pub` submodules are those
// it imports by submodule path: `enumeration`, `environment_stage` and
// `model_query`; `declaration`, `containment`, `quantity` and `stop` are in
// the `filament-semantic-value` crate (ADR-011 layer SV). `definition` and `semantic_node` stay
// `pub(crate)`: their consumers outside `value` are `check`, `model` and
// `library`, all in this crate. `decimal`, `ieee`,
// `numeric` and `text` are deleted; their former items are imported from `filament_exact`
// directly.

pub(crate) mod definition;
mod diagnostics_catalog;
pub mod enumeration;
pub mod environment_stage;
pub(crate) mod layer;
pub(crate) mod member;
pub mod model_query;
pub mod nominal_payload;
pub(crate) mod semantic_node;
pub(crate) mod unit;

// `ChargePoint`, `Incomplete`, `InjectedDenial`, `LimitKind`,
// `Meter`, `ScalarLimits`, `Charge` and `length_amount` were this module's
// own `accounting` submodule, re-exported from here. That submodule
// duplicated `filament_exact`'s accounting types byte-for-byte and is deleted;
// every former consumer (`crate::model::population` included, which reused
// this crate-wide re-export the same way FR-153 reuses this crate's own
// `filament.value.accounting/v1` meter for `lookup.*`/`population.visit`/
// `collection.*` charges) now imports `filament_exact::{..}` directly. One
// definition, one import path -- no re-export shim stands in for the
// deleted module.
// `CardinalityBound`/`EmptyCardinalityBound` are `filament_exact`'s own types; this
// module does not re-export them, so consumers import them from
// `filament_exact` directly.
// The kernel's `Value`, `ValueType`, collection, equality and rational items
// (ADR-011 §6.1's K row) are not re-exported here: every consumer imports
// them from `filament_exact`, the one definition (ADR-011 §7.2).
// `DecimalType`, `DecimalLoss`, `DecimalResult` and
// `evaluate_decimal` were this module's own `decimal` submodule, a
// byte-identical duplicate of `filament_exact`'s (V4 had already moved unit
// placement onto the kernel's `DecimalType::placement`; only the type and
// its evaluation engine were left). That submodule is deleted; every former
// consumer now imports `filament_exact::{DecimalType, DecimalLoss,
// DecimalResult, evaluate_decimal}` directly -- one definition, one import
// path, no re-export standing in for the deleted module.
// `enumeration` and `environment_stage` are `pub` modules, so their
// items have one public path, the submodule one
// (`value::environment_stage::stage_failure`); this module does not re-export
// them flat as well (the same one-path rule as for the kernel's items).
// `SelectionRefusalCode` and the `PackageRefusal` vocabulary are
// `filament_semantic_value::definition`'s (ADR-011 §6.1 layer SV); consumers
// import them from there, not through a re-export here.
pub use definition::{
    admit_ieee_profile, admit_integer_division, admit_selection, always_roles, divide,
    qualification_roles, AdmittedIeeeProfile, AdmittedSelection, DefinitionReference,
    DefinitionRole, ModelRole, TemporalRole, Trigger,
};
pub use diagnostics_catalog::{native_diagnostics_catalog, NATIVE_DIAGNOSTICS_IDENTITY};
pub use layer::{HeaderLayer, LayerForm, WitnessKind, WitnessRequest};
// `ExactScalar`, `IeeeOperand`, `IeeeProvenance`, `IeeeResult`,
// `IeeeExact`, the five entry points (`evaluate_ieee`/
// `compare_ieee`/`convert_ieee_width`/`ieee_to_exact`/`exact_to_ieee`) and
// the private rounding/arithmetic engine beneath them were this module's
// own `ieee` submodule, a byte-identical duplicate of `filament_exact`'s
// (`IeeeWidth`, `IeeeValue`, `IeeeFlag`, `IeeeFlags`, `IeeeComparison`,
// `IeeeOperationKind`, `ieee_intrinsic_identities`, `IeeeExactLoss`,
// `IeeeOperation` and `IEEE_DEFINITION` were already re-exported straight
// from `filament_exact`, not duplicated). That submodule is deleted; every
// former consumer now imports the whole set from `filament_exact` directly.
pub use member::{AcceptedPoint, FairnessTarget, Member};
// `evaluate_boolean`, `evaluate_integer_arithmetic`,
// `evaluate_rational_arithmetic` and `order_numbers` were this module's own
// `numeric` submodule, a byte-identical duplicate of `filament_exact`'s
// (`ArithmeticOperator`, `OrderingOperator`, `OrderedOperands`,
// `IntegerArithmetic`, `RationalArithmetic` and `BooleanConnective` were
// already re-exported straight from `filament_exact`). That submodule is
// deleted; every former consumer now imports `filament_exact::{evaluate_boolean,
// evaluate_integer_arithmetic, evaluate_rational_arithmetic, order_numbers}`
// directly.
// `Outcome`, `Undefined` and `Refusal` were this module's own
// `outcome` submodule, a byte-identical duplicate of `filament_exact`'s O-16
// kernel types (ADR-011 §6.1's K row), re-exported from here. That
// submodule is deleted; every former consumer now imports
// `filament_exact::{Outcome, Undefined, Refusal}` directly -- one definition,
// one import path, no re-export standing in for the deleted module.
// `InvalidSemanticGraph` and `SemanticGraphCause` are
// `filament_semantic_value::semantic_node`'s (ADR-011 §6.1 layer SV); consumers
// import them from there, not through a re-export here.
pub use semantic_node::{
    identity_limit, ModelSubject, NodeOwner, NominalRefusal, OwnerSelection, OwnerSubject,
};
// `admit_text`, `compare_text`, `Text`, `TextPayload` and
// `InvalidTextLiteral` were this module's own `text` submodule, a
// byte-identical duplicate of `filament_exact`'s except for
// `TextPayload::from_source_literal`'s JSON-decode convenience, which the
// kernel deliberately excludes (a source-lexer concern, not a kernel one --
// `filament_exact::text`'s module doc). That submodule is deleted; every
// former consumer now imports `filament_exact::{admit_text, compare_text, Text,
// TextPayload}` directly. `InvalidTextLiteral` has no successor: nothing
// outside this crate's own now-deleted tests called
// `TextPayload::from_source_literal`'s fallible JSON decode, so
// `tests/it/text_enum_identity.rs` keeps that one decode step as a
// test-local helper, calling the kernel's new
// `filament_exact::TextPayload::from_source_literal(text, spelling)`
// constructor (infallible, over already-decoded text) to tag the result.
// `value::unit` keeps the compile-side half of the unit graph: the preimage
// readers and admission. The runtime half (`Dimension`, `Unit`, `UnitEdge`,
// `UnitGraph`, `CompoundUnit`, `compound_unit_id`, `InvalidCompoundUnit`,
// `CompoundUnitCause`, `NotAUnitKey`) is `filament_semantic_value::unit`, which
// every caller names directly.
pub use unit::{
    admit_unit_graph, CompoundUnitPreimage, DimensionEquivalencePreimage, DimensionPreimage,
    UnitPreimage,
};
