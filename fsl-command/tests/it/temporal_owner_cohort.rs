// SPDX-License-Identifier: AGPL-3.0-or-later
//! The command's temporal dependencies share their owners' public types.

use filament_mltl::request::{ObservationInputs, RequestInput, TemporalInput};
use filament_mltl::{OwnerReadError, OwnerReadErrorCode};
use filament_observation::authority::{availability, clock, closure, completeness, progress};
use tl_mltl::wire::{trace, OwnerLimits};
use tl_mltl::{TraceDocument, TraceSchemaVersion};
use tl_syntax::{FormulaDocument, PropositionId, PropositionMapDocument};

/// Provenance: QSL-2324 dependency compatibility; no observation authority is constructed.
#[test]
fn temporal_owner_types_share_one_dependency_cohort() {
    let _: for<'a> fn(RequestInput<'a>) -> &'a FormulaDocument = |input| input.formula;
    let _: for<'a> fn(RequestInput<'a>) -> &'a PropositionMapDocument =
        |input| input.proposition_map;
    let _: for<'a> fn(RequestInput<'a>) -> &'a clock::View = |input| input.clock;
    let _: for<'a> fn(ObservationInputs<'a>) -> &'a progress::View =
        |input| input.decision_scope_progress;
    let _: for<'a> fn(ObservationInputs<'a>) -> &'a closure::View =
        |input| input.decision_scope_closure;
    let _: for<'a> fn(ObservationInputs<'a>) -> &'a completeness::View = |input| input.completeness;
    let _: for<'a> fn(ObservationInputs<'a>) -> &'a availability::View = |input| input.availability;

    let document = TraceDocument {
        schema_version: TraceSchemaVersion::V1,
        trace_id: "fsl:temporal-owner-cohort".to_owned(),
        closed: true,
        instants: vec![vec![PropositionId(0)], Vec::new()],
    };
    let limits = OwnerLimits::default();
    let produced = trace::derive(&document, limits).expect("owner derives canonical trace");
    let admitted =
        trace::read(produced.bytes(), &document, limits).expect("owner reads its exact trace");
    let TemporalInput::Future(retained) = TemporalInput::Future(&admitted) else {
        panic!("future owner input must retain the trace");
    };
    assert_eq!(retained.document(), &document);
    assert_eq!(retained.bytes(), produced.bytes());
}

/// Provenance: QSL-2324 dependency compatibility; exercise the public error conversion.
#[test]
fn tl_reader_refusals_cross_the_filament_error_boundary() {
    let document = TraceDocument {
        schema_version: TraceSchemaVersion::V1,
        trace_id: "fsl:temporal-owner-cohort".to_owned(),
        closed: true,
        instants: vec![vec![PropositionId(0)]],
    };
    let limits = OwnerLimits::default();
    let produced = trace::derive(&document, limits).expect("owner derives canonical trace");
    let mut other = document.clone();
    other.trace_id = "fsl:another-trace".to_owned();
    for (bytes, expected, code) in [
        (
            produced.bytes(),
            &other,
            OwnerReadErrorCode::ExpectedMismatch,
        ),
        (&b"{}"[..], &document, OwnerReadErrorCode::InvalidJson),
    ] {
        let refusal = trace::read(bytes, expected, limits).expect_err("owner must refuse");
        let field = refusal.field();
        let usage = refusal.usage();
        let label = refusal.code().as_str();
        let converted: OwnerReadError = refusal.into();
        assert_eq!(converted.code(), code);
        assert_eq!(converted.code().as_str(), label);
        assert_eq!(converted.field(), field);
        assert_eq!(converted.usage(), usage);
    }
}
