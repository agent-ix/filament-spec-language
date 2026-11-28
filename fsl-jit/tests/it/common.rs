// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixtures shared by the `fsl-jit` tests.

use filament_eval::{CallFailure, Evaluation};
use filament_exact::{AdmittedCharge, Cancel, Integer, Meter, ScalarLimits, Value};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::supplied_stop::{SuppliedLimits, SuppliedRun};
use fsl_eval::execution::{EntrySelection, ExecutionBackend, PrepareLimits, PreparedEntries};
use fsl_eval::value::S6aRewardEvaluator;
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_jit::{Jit, PreparedJit};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::object_environment::ObjectEnvironment;

pub const UNLIMITED: ScalarLimits = ScalarLimits {
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

/// `declarations` compiled as one complete-V1 unit into a checked package.
pub fn package_of(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n{declarations}"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default()
            .with_tokens(1 << 24)
            .with_nodes(1 << 24)
            .with_source_bytes(1 << 26)
            .with_work_units(1 << 40),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles");
    CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default()
                    .with_nodes(1 << 30)
                    .with_input_bytes(1 << 30)
                    .with_work_budget(1 << 40),
                &S6aRewardEvaluator,
            )
            .expect("the unit checks"),
    )
}

pub fn name(function: &str) -> QualifiedName {
    QualifiedName::unqualified(function).expect("an identifier")
}

pub fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

pub fn select(functions: &[&str]) -> Vec<EntrySelection> {
    functions
        .iter()
        .map(|function| EntrySelection::function(name(function)))
        .collect()
}

pub fn prepare<'p>(package: &'p CheckedPackage, functions: &[&str]) -> PreparedJit<'p> {
    Jit.prepare(
        package,
        &select(functions),
        &PrepareLimits::default(),
        &Cancel::new(),
    )
    .unwrap_or_else(|failure| panic!("the JIT prepares: {failure:?}"))
}

/// One call's evaluation (rendered, since `Evaluation` has no equality), or
/// its failure, with the meter's admitted-charge log.
pub struct Run {
    pub result: String,
    pub charges: Vec<AdmittedCharge>,
}

fn rendered(result: Result<Evaluation, CallFailure>) -> String {
    format!("{result:?}")
}

pub fn interpreted(
    package: &CheckedPackage,
    function: &str,
    arguments: &[Value],
    limits: ScalarLimits,
) -> Run {
    let mut meter = Meter::new(limits);
    let cancel = Cancel::new();
    let mut run = SuppliedRun::new(SuppliedLimits::default(), cancel).expect("a run");
    let result = package.call_supplied(
        &name(function),
        arguments.to_vec(),
        &ObjectEnvironment::default(),
        &mut meter,
        &mut run,
    );
    Run {
        result: rendered(result),
        charges: meter.admitted_charges().to_vec(),
    }
}

pub fn jitted(
    prepared: &PreparedJit<'_>,
    function: &str,
    arguments: &[Value],
    limits: ScalarLimits,
) -> Run {
    let mut meter = Meter::new(limits);
    let cancel = Cancel::new();
    let mut run = SuppliedRun::new(SuppliedLimits::default(), cancel.clone()).expect("a run");
    let result = prepared.call_metered(
        &name(function),
        arguments,
        &ObjectEnvironment::default(),
        &mut meter,
        &mut run,
        &cancel,
    );
    Run {
        result: rendered(result),
        charges: meter.admitted_charges().to_vec(),
    }
}

/// The trait object's `unsupported` list, for assertions.
pub fn unsupported_names(prepared: &impl PreparedEntries) -> Vec<String> {
    prepared
        .unsupported()
        .iter()
        .map(|(selection, _)| selection.name.to_string())
        .collect()
}

/// `call` through the seam trait, under unlimited accounting.
pub fn jitted_call(
    prepared: &PreparedJit<'_>,
    function: &str,
    arguments: &[Value],
) -> Result<Evaluation, CallFailure> {
    prepared.call(
        &name(function),
        arguments,
        &ObjectEnvironment::default(),
        &fsl_eval::execution::CallLimits {
            accounting: UNLIMITED,
            session: Default::default(),
        },
        &mut SuppliedRun::new(SuppliedLimits::default(), Cancel::new()).expect("a run"),
        &Cancel::new(),
    )
}
