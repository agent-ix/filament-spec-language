// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-285: an admission record of code `resource_exhausted` reports the
//! incomplete exit on both the analyze and the monitor refusal routes.

use std::collections::BTreeMap;

use filament_semantic_value::admission::AdmissionRecord;
use filament_semantic_value::catalog::CatalogCode;
use fsl_analyze::analyze::AnalyzeRefusal;
use fsl_analyze::monitor::MonitorRefusal;
use fsl_semantics::model::observation::AdmissionFailure;
use ix_trace_rs::trace;

fn exhausted() -> AdmissionFailure {
    AdmissionFailure::Incomplete(Box::new(AdmissionRecord {
        catalog: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
        fields: BTreeMap::new(),
        path: Vec::new(),
        kernel: None,
    }))
}

/// FR-285: a limit exits 22. The admission failure `Incomplete` carrying
/// `resource_exhausted` exits 22, not the refusal 20, on `analyze` and on
/// `monitor` alike. Only the monitor half discriminates: `analyze` already
/// takes its category from the `AdmissionFailure::Incomplete` variant, so it
/// reads 22 with or without the `resource_exhausted` arm of `exit_category_of`;
/// `monitor` goes through the code map and reads 20 without it. The analyze
/// half pins the two routes to one answer.
#[trace("TC-769", "FR-285-AC-3")]
#[test]
fn an_incomplete_resource_exhausted_admission_exits_22_on_analyze_and_monitor() {
    let analyze = AnalyzeRefusal::Document(exhausted())
        .document()
        .category()
        .exit_code();
    let monitor = MonitorRefusal::Admission {
        position: 0,
        failure: Box::new(exhausted()),
    }
    .document()
    .category()
    .exit_code();
    assert_eq!((analyze, monitor), (22, 22));
}
