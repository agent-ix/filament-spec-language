// SPDX-License-Identifier: AGPL-3.0-or-later
//! Replay accounting and recompile-limit behavior for TC-921.

use std::collections::BTreeMap;

use filament_exact::ScalarLimits;
use filament_semantic_value::catalog::Category;
use fsl_foundation::diagnostic::Code;
use fsl_replay::spine::SpineStage;
use fsl_replay::{
    CallSiteUnit, CanonicalAssignment, DependencyInput, DisagreementCause, Identifier,
    QualifiedName, ReplayLimits, ReplayPackage, ReplayRefusal, ReplayRequestWire, ReplaySource,
    RequestClocks, RequestTargetWire, SourceIdentity, StateEnvironment, Verdict, WitnessValue,
    call_site, replay,
};
use ix_trace_rs::trace;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function p using v(x: Int[0, 9]): Boolean pure { x < 5 and x >= 0 and x != 8 }\n";

const S3_LIMIT_SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function small using v(x: Int[0, 9]): Boolean pure { x < 5 }\n\
    function flag using v(b: Boolean): Boolean pure { b }\n\
    function id using v(x: Int[0, 9]): Int[0, 9] pure { x }\n\
    function lt using v(a: Int[0, 9], b: Int[0, 9]): Boolean pure { a < b }\n\
    function maybe using v(t: Option<Boolean>): Boolean pure { true }\n\
    function f using v(x: Int[0, 9]): Integer pure { x + 1 }\n\
    function p using v(x: Int[0, 9]): Boolean pure { f(x) > 3 }\n";

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn request(source_text: &str, function: &str, argument: i64) -> ReplayRequestWire {
    let source = SourceIdentity::new("agent-ix", "test:tc-921");
    let name = QualifiedName::new(vec![
        Identifier::new(function).expect("function identifier"),
    ])
    .expect("qualified function name");
    let site = call_site(
        source.clone(),
        CallSiteUnit::new("fixture".to_owned(), "unit.native", source_text.as_bytes()),
        [],
        &DependencyInput::default(),
        &RequestClocks::default(),
        &name,
    )
    .expect("the fixture compiles");
    ReplayRequestWire {
        certificate: None,
        profile_selections: vec![],
        package: ReplayPackage::read(&site.package, ReplayLimits::default().input_bytes)
            .expect("the checked package reads"),
        dependencies: vec![],
        replay_target: RequestTargetWire::function(name, site.site.declaration.clone()),
        source: ReplaySource::Input(vec![CanonicalAssignment {
            parameter: site.site.parameters[0].1.clone(),
            value: WitnessValue::Integer(i128::from(argument)),
        }]),
        obligation_identity: site.site.obligation_identity().clone(),
        backend: "kani-backend-1".to_owned(),
        state_environment: StateEnvironment::new(vec![]),
        accounting_limits: UNLIMITED,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![(
            "agent-ix".to_owned(),
            "test:tc-921".to_owned(),
            source_text.as_bytes().to_vec(),
        )],
        documents: vec![],
    }
}

/// TC-921 (FR-263-AC-5): accounting work exhaustion is an incomplete
/// no-value result, while an S3 work limit remains a located stage refusal.
#[trace("TC-921", "FR-263-AC-5")]
#[test]
fn accounting_work_limit_stops_execution_and_s3_limit_stays_a_stage_refusal() {
    let mut wire = request(SOURCE, "p", 3);
    wire.stage_limits
        .insert("accounting.work_units".to_owned(), 1);
    let result = replay(wire, ReplayLimits::default()).expect("the request is admitted");
    assert!(result.pre_call_limit().is_none());
    assert_eq!(result.category(), Category::Incomplete);
    assert_eq!(
        result.disagreement(),
        Some(&DisagreementCause::NoValue {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Incomplete),
        })
    );
    let fsl_replay::ReplayResult::Input(input) = &result else {
        panic!("accounting exhaustion returns an input arm, got {result:?}");
    };
    assert_eq!(input.value(), None);
    let limit = result.limit().expect("execution names its denied charge");
    assert_eq!(limit.limit_kind, filament_exact::LimitKind::WorkUnits);
    assert_eq!(limit.limit, 1);
    assert_eq!(limit.consumed, 1);
    assert_eq!(limit.next_charge, filament_exact::Integer::from(1_i64));
    assert_eq!(
        limit.charge_point,
        filament_exact::ChargePoint::OrderingOperands
    );
    let written: serde_json::Value = serde_json::from_str(
        &result
            .to_function_json()
            .expect("the incomplete result serializes"),
    )
    .expect("the result is JSON");
    assert_eq!(written["arm"], "input");
    assert_eq!(written["category"], "incomplete");
    assert_eq!(written["value"], serde_json::Value::Null);
    assert_eq!(
        written["limit"],
        serde_json::json!({
            "kind": "work_units",
            "bound": "1",
            "counter": "2",
            "field": "work_units",
            "setting": "accounting.work_units",
            "consumed": "1",
            "next_charge": "1",
            "charge_point": "ordering.operands"
        })
    );
    let mut wire = request(S3_LIMIT_SOURCE, "small", 7);
    wire.stage_limits.insert("s3.work_units".to_owned(), 0);
    let refused = replay(wire, ReplayLimits::default()).expect_err("zero S3 work refuses");
    let ReplayRefusal::Recompile { refusal, .. } = &refused else {
        panic!("expected an S3 refusal, got {refused:?}");
    };
    assert_eq!(refusal.stage(), SpineStage::Check);
    assert_eq!(refusal.code(), Code::StageLimitExceeded);
    let limit = refusal
        .limit()
        .expect("the stage refusal carries its limit");
    assert_eq!(limit.setting().name(), "s3.work_units");
    assert_eq!(limit.configured_bound(), 0);
    assert_eq!(limit.actual(), 15);
}
