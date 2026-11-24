// SPDX-License-Identifier: AGPL-3.0-or-later

//! FR-273-AC-51 compile-resolution assertion for the shared evaluation interface.
use filament_semantic_value::location::{ExpressionLocation, ExpressionOwner};
use ix_trace_rs::trace;

#[test]
#[trace("TC-748", "FR-273-AC-51")]
fn evaluation_location_interface_resolves_from_semantic_value_owner() {
    fn resolved_types(_: Option<ExpressionLocation>, _: Option<ExpressionOwner>) {}

    resolved_types(None, None);
}
