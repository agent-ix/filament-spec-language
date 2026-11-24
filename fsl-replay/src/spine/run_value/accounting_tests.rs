// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine call-world acceptance for canonical conversion accounting.

use crate::reference_codec::ReferenceCodec;
use filament_exact::{Cancel, IntegerDecimalBudget, Reservation, ReservationDenied, Value};
use filament_semantic_value::supplied_stop::{SuppliedLimits, SuppliedPhase, SuppliedRun};
use fsl_eval::execution::{
    ExecuteLimits, ExecuteRequest, ExecuteSelection, PrepareLimits, SuppliedBackends, execute,
};
use fsl_package::ReferenceCatalog;
use fsl_semantics::model::accounting::{NormalizationChargePoint, NormalizationLimitKind};
use fsl_semantics::model::intake::{PackageInput, package_input};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{DocumentRef, ObservationLimits, admit_current_snapshot};
use serde_json::json;

use crate::reference_codec::reference_fixture as domain;

struct Fixture {
    compiled: crate::spine::ComposedUnit,
    objects: ObjectEnvironment,
}

fn fixture(work: Option<u64>) -> Fixture {
    let packages = package_input([domain::DOMAIN_PACKAGE.as_bytes()]).unwrap();
    let mut limits = crate::spine::SpineLimits::default();
    if let Some(work) = work {
        limits.model.work_units = work;
    }
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nmodel Config = {:?};\nfunction keep using v(x: Config::ConfigVersion): Config::ConfigVersion pure {{ x }}\n",
        domain::PACKAGE_IDENTITY
    );
    let compiled = crate::spine::compose(
        fsl_foundation::SourceIdentity::new("test", "canonical-run-accounting"),
        "canonical-run.native",
        text.as_bytes(),
        &packages,
        &crate::spine::DependencyInput::default(),
        limits,
    )
    .expect("genuine source checking and selected model admission");
    let selected = DocumentRef {
        authority: "test".into(),
        identity: "canonical-current".into(),
    };
    let snapshot = json!({
        "format":"filament.state.snapshot/v1", "identity":{"authority": selected.authority,"identity":selected.identity},
        "observation":"current","anchor":{"kind":"handler","name":"read"},
        "model":{"identity":domain::PACKAGE_IDENTITY},
        "populations":[{"population":"ix://example/config-version/config_history","complete":true,
            "objects":[{"key":"雪","type":"ix://example/config-version/ConfigVersion",
                "fields":{"versionNumber":{"integer":"2"},"parent":{"absent":{}}}}]}]
    });
    let provisions = std::collections::BTreeMap::from([(
        selected.clone(),
        serde_json::to_vec(&snapshot).unwrap(),
    )]);
    let checked = &compiled.package;
    let (objects, _) = admit_current_snapshot(
        checked.graph().model_inputs(),
        checked.graph().scope().types(),
        &packages,
        limits.model,
        &provisions,
        &selected,
        ObservationLimits::default(),
    )
    .expect("genuine snapshot admission");
    Fixture { compiled, objects }
}

#[derive(Default)]
struct ConversionWork(u64);
impl Reservation for ConversionWork {
    fn reserve(&mut self, _: usize) -> Result<(), ReservationDenied> {
        Ok(())
    }
}
impl IntegerDecimalBudget for ConversionWork {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        self.0 = self.0.checked_add(units).ok_or(ReservationDenied)?;
        Ok(())
    }
}

/// Trace: FR-650-AC-20
#[test]
fn canonical_run_conversion_and_execution_share_one_limit() {
    let fixture = fixture(None);
    let models = fixture.compiled.models.selected_models();
    let cancel = Cancel::new();
    let mut catalog_budget = super::NormalizationReferenceBudget::source(
        models,
        NormalizationChargePoint::ReferenceCatalog,
    );
    let catalog =
        ReferenceCatalog::from_checked_call(&fixture.compiled.package, models, &mut catalog_budget)
            .unwrap();
    let codec = ReferenceCodec::for_checked_call(
        &catalog,
        &fixture.compiled.package,
        models,
        &fixture.objects,
    )
    .unwrap();
    let [reference] = fixture.objects.references() else {
        panic!("one genuine object")
    };
    let mut wire_budget = ConversionWork::default();
    let wire = codec
        .encode_reference(reference, &mut wire_budget, &cancel, 64)
        .unwrap();
    let scope = fixture.compiled.package.graph().scope();
    let forms = super::Forms::new(scope.types(), scope.enums());
    let callable = fixture.compiled.package.graph().callable("keep").unwrap();
    let declared = &callable.parameters[0].1;
    let value = Value::Reference(reference.clone());
    let mut membership_only = SuppliedRun::new(SuppliedLimits::default(), cancel.clone()).unwrap();
    forms
        .admit_supplied(&mut membership_only, 0, declared, &value)
        .unwrap();
    let request = ExecuteRequest {
        selection: ExecuteSelection::Function {
            name: fsl_eval::value::QualifiedName::unqualified("keep").unwrap(),
            arguments: vec![Value::Reference(reference.clone())],
            environment: fixture.objects.clone(),
        },
        backend: filament_eval::ExecutionBackendChoice::Interpreter,
    };
    let execution = ExecuteLimits {
        accounting: crate::spine::default_accounting(1_000_000),
        prepare: PrepareLimits::default(),
        session: filament_eval::SessionLimits::default(),
    };
    let mut execution_only = SuppliedRun::new(SuppliedLimits::default(), cancel.clone()).unwrap();
    execute(
        &fixture.compiled.package,
        &request,
        &SuppliedBackends::default(),
        &execution,
        &mut execution_only,
        &cancel,
    )
    .expect("genuine interpreter execution");
    let admitted = membership_only.admission_spent();
    let executed = execution_only.admission_spent();
    assert!(admitted > 0 && executed > 0);
    let ceiling = admitted.max(executed);
    let limits = SuppliedLimits::default().with_admission_work_units(ceiling);
    let mut execution_only = SuppliedRun::new(limits, cancel.clone()).unwrap();
    execute(
        &fixture.compiled.package,
        &request,
        &SuppliedBackends::default(),
        &execution,
        &mut execution_only,
        &cancel,
    )
    .expect("execution alone fits the same admission ceiling");
    let mut combined = SuppliedRun::new(limits, cancel.clone()).unwrap();
    let mut admission = super::SuppliedReferenceAdmission::new(&mut combined, 0, 0);
    assert_eq!(
        codec
            .restore_reference(&wire, &mut admission, &cancel, 64)
            .unwrap(),
        *reference
    );
    assert!(
        admission.stopped.is_none(),
        "reference conversion is admitted"
    );
    drop(admission);
    forms
        .admit_supplied(&mut combined, 0, declared, &value)
        .expect("canonical boundary membership alone fits the same ceiling");
    let failure = execute(
        &fixture.compiled.package,
        &request,
        &SuppliedBackends::default(),
        &execution,
        &mut combined,
        &cancel,
    )
    .expect_err("boundary admission and execution exceed the unchanged cumulative ceiling");
    let filament_eval::CallFailure::Supplied(stop) = failure else {
        panic!("typed supplied stop: {failure:?}")
    };
    assert_eq!(stop.context.phase, SuppliedPhase::Admission);
    assert_eq!(stop.context.consumed, ceiling);
}

/// Trace: FR-650-AC-21
#[test]
fn canonical_run_catalog_preserves_normalization_work_denial() {
    let baseline = fixture(None);
    let consumed = baseline
        .compiled
        .models
        .selected_models()
        .consumed(NormalizationLimitKind::WorkUnits);
    let fixture = fixture(Some(consumed));
    let models = fixture.compiled.models.selected_models();
    let mut budget = super::NormalizationReferenceBudget::source(
        models,
        NormalizationChargePoint::ReferenceCatalog,
    );
    ReferenceCatalog::from_checked_call(&fixture.compiled.package, models, &mut budget)
        .expect_err("catalog cannot replace the original normalization allowance");
    let denial = budget
        .stopped
        .take()
        .expect("original typed normalization denial");
    assert_eq!(denial.limit_kind, NormalizationLimitKind::WorkUnits);
    assert_eq!(denial.limit, consumed);
    assert_eq!(denial.consumed, consumed);
    assert_eq!(
        denial.charge_point,
        NormalizationChargePoint::ReferenceCatalog
    );
    assert!(denial.next_charge > 0);
}

/// Review FND-001: unmigrated certificate consumers retain their original IO.
#[test]
fn review_fix_legacy_certificate_reference_roundtrip() {
    let fixture = fixture(None);
    let [reference] = fixture.objects.references() else {
        panic!("one genuine object")
    };
    let value =
        fsl_eval::simulation::model::value_json(&Value::Reference(reference.clone())).unwrap();
    let text = serde_json::to_string(&value).unwrap();
    let transition = filament_semantic_value::model_transition::ModelTransition::new(
        r#"{"name":"keep"}"#.into(),
        Some(text.clone()),
        vec![text],
        quire_canonical::Limits::new(100_000),
    )
    .unwrap();
    let template = crate::TaskTransitionTemplate {
        action: crate::TaskTransitionAction::Release,
        task_set: "tasks".into(),
        task: "task".into(),
        job_rank: 0,
        slot: 0,
        release: transition.clone(),
        source_transition: Some(transition),
        priority: Some(1),
    };
    let wire = template
        .to_wire()
        .expect("existing certificate writer accepts native references");
    let restored = crate::TaskTransitionTemplate::from_wire(&wire)
        .expect("existing certificate reader accepts its reference spelling");
    assert!(template.same_identity(&restored));
}

/// Review FND-003: zero conversion allowance stops before collection staging.
#[test]
fn review_fix_canonical_staging_admits_conversion_before_allocation() {
    let packages = PackageInput::new();
    let compiled = crate::spine::compose(fsl_foundation::SourceIdentity::new("test", "conversion-before-copy"),
        "conversion.native", b"language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\"; function keep using v(flag: Boolean, x: Sequence<Integer>[0, 50000]): Sequence<Integer>[0, 50000] pure { x }",
        &packages, &crate::spine::DependencyInput::default(), crate::spine::SpineLimits::default()).unwrap();
    let scope = compiled.package.graph().scope();
    let forms = super::Forms::new(scope.types(), scope.enums());
    let declared = &compiled
        .package
        .graph()
        .callable("keep")
        .unwrap()
        .parameters[1]
        .1;
    let input =
        json!({"type":"sequence", "elements": vec![json!({"type":"integer","value":"5"}); 50_000]});
    let run = std::sync::Mutex::new(
        SuppliedRun::new(
            SuppliedLimits::default().with_conversion_work_units(0),
            Cancel::new(),
        )
        .unwrap(),
    );
    let stop = forms
        .read_canonical(&input, declared, 1, &run)
        .expect_err("conversion must be admitted before the additional tree exists");
    let super::ReadStop::Refusal(refusal) = stop else {
        panic!("typed conversion refusal")
    };
    let crate::spine::RunRefusal::Supplied(stop) = *refusal else {
        panic!("original supplied stop")
    };
    assert_eq!(stop.context.phase, SuppliedPhase::Conversion);
    assert_eq!(stop.context.parameter, Some(1));
    assert_eq!(
        stop.context.event,
        Some(filament_semantic_value::supplied_stop::SuppliedEvent::WitnessPreflight)
    );
    assert_eq!(stop.context.consumed, 0);
    assert_eq!(run.lock().unwrap().admission_spent(), 0);
    let allowed =
        std::sync::Mutex::new(SuppliedRun::new(SuppliedLimits::default(), Cancel::new()).unwrap());
    assert!(matches!(
        forms
            .read_canonical(
                &json!({"type":"sequence","elements":[{"type":"integer","value":"5"}]}),
                declared,
                1,
                &allowed
            )
            .unwrap(),
        Value::Collection(_)
    ));
}

/// Review FND-002: a genuine catalog obeys the caller rather than a hidden64.
#[test]
fn review_fix_codec_depth_policy_has_no_implicit_ceiling() {
    let fixture = fixture(None);
    let models = fixture.compiled.models.selected_models();
    let mut owner = super::NormalizationReferenceBudget::source(
        models,
        NormalizationChargePoint::ReferenceCatalog,
    );
    let catalog =
        ReferenceCatalog::from_checked_call(&fixture.compiled.package, models, &mut owner).unwrap();
    let codec = ReferenceCodec::for_checked_call(
        &catalog,
        &fixture.compiled.package,
        models,
        &fixture.objects,
    )
    .unwrap();
    let mut value = json!({"type":"integer","value":"5"});
    for _ in 0..100 {
        value = json!({"type":"sequence","elements":[value]});
    }
    let cancel = Cancel::new();
    let mut denied = ConversionWork::default();
    assert!(matches!(
        codec.encode_value(&value, &mut denied, &cancel, 63),
        Err(crate::ReferenceCodecError::Refused {
            reason: crate::ReferenceFailure::DepthLimit,
            ..
        })
    ));
    let mut allowed = ConversionWork::default();
    assert_eq!(
        codec
            .encode_value(&value, &mut allowed, &cancel, usize::MAX)
            .unwrap(),
        value
    );
}
