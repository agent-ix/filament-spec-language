// SPDX-License-Identifier: AGPL-3.0-or-later
//! Production accounting meters keep fixed-size state.

use fsl_semantics::model::accounting::{ModelNormalizationLimits, NormalizationMeter};
use fsl_semantics::model::population::{AdmissionMeter, PopulationAdmissionLimits};
use ix_trace_rs::trace;

/// Production xtask dependencies do not enable `test-support`, so these are
/// production meter types. Their size remains fixed; the production
/// normalization counters and admission meter also have compile-time no-drop
/// assertions in their owning crate.
#[test]
#[trace("NFR-012-M-10")]
fn production_accounting_meters_have_no_per_charge_heap_state() {
    assert!(!std::mem::needs_drop::<AdmissionMeter>());
    let normalization = NormalizationMeter::new(ModelNormalizationLimits::default());
    let admission = AdmissionMeter::new(PopulationAdmissionLimits::default());
    assert_eq!(normalization.admission_count(), 0);
    assert_eq!(admission.admission_count(), 0);
    assert!(std::mem::size_of_val(&normalization) < 256);
    assert!(std::mem::size_of_val(&admission) < 256);
}
