// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-740 "Source objects": the package route's input over a package whose
//! random parameters and workloads are declared on source object types, read
//! from the emitted bytes alone (QSL-2716). The route is given no source
//! package: it rebuilds one from the package's source-object rows.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::simulation::model::{PackageModelInput, PendingPackageForm};
use fsl_package::{BarePackage, BareSupply, CheckedPackage, EmitLimits, V2ReadLimits};
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input, PackageInput};
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    profile t = \"filament.temporal.complete/v1\";\n\
    dimension Time; unit ms : Time = rational(1, 1);\n";

/// `body` after [`HEADER`], with no selected domain package, through S3.
fn checked(body: &str) -> CheckedPackage {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:source-objects"),
        "program.native",
        format!("{HEADER}{body}").as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the source parses");
    assert_eq!(parsed.diagnostics(), []);
    let unit = fsl_forms::build_unit(&parsed).expect("the forms build");
    let inputs = packages();
    let selected = admit_unit(
        &unit.selections().models,
        &inputs,
        ModelNormalizationLimits::default(),
    )
    .expect("empty selected models admit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "source-object-probabilistic".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("the package assembles");
    let _pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .map(CheckedPackage::link)
        .unwrap_or_else(|refusals| panic!("S3 admits the unit: {refusals:?}"))
}

fn packages() -> PackageInput {
    package_input(std::iter::empty::<&[u8]>()).expect("an empty package input")
}

/// Runs `check` over the package route's input for `package`'s emitted bytes,
/// which hold no source package.
fn through_the_route<T>(
    package: &CheckedPackage,
    check: impl FnOnce(Result<PackageModelInput<'_>, PendingPackageForm>) -> T,
) -> T {
    let inputs = packages();
    let read = fsl_package::read_evaluation_package(
        package,
        fsl_semantics::library::LibraryName::new("source-object-probabilistic")
            .expect("a library name"),
        &inputs,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("the package emits and its bytes are admitted");
    let admitted = read.admitted();
    let mut rows_budget = V2ReadLimits::fixture().native_join_budget();
    let rows = read
        .emission()
        .identity_rows(admitted, &mut rows_budget)
        .expect("the identity rows join the package");
    let mut join_budget = V2ReadLimits::fixture().native_join_budget();
    let bare = BarePackage::new(admitted)
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut join_budget,
        )
        .expect("the bytes view admits its supply");
    check(PackageModelInput::of(
        &bare,
        &inputs,
        ModelNormalizationLimits::default(),
    ))
}

/// FR-740-AC-6: the package route rebuilds the random parameter and the
/// workload declared on a source object from the emitted bytes alone: the
/// records equal the ones S3 checked.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn the_package_route_rebuilds_a_source_random_parameter_and_workload() {
    let package = checked(
        "object O { n: Integer; operation step(x: Integer) modifies self.n; }\n\
         random O::step(x) ~ { 1: 1, 3: 3 };\n\
         workload W on O { weight O::step = 1; }\n",
    );
    let source = package.graph().probabilistic_model();
    through_the_route(&package, |input| {
        let input = input.unwrap_or_else(|pending| panic!("the route builds its input: {pending}"));
        let rebuilt = input.input().probabilistic;
        let [random] = rebuilt.random_parameters() else {
            panic!("one random parameter rebuilt");
        };
        let [expected] = source.random_parameters() else {
            panic!("one random parameter checked");
        };
        assert_eq!(random.operation().name(), expected.operation().name());
        assert_eq!(
            random.operation().declaring(),
            expected.operation().declaring()
        );
        assert_eq!(random.position(), expected.position());
        assert_eq!(random.value_type(), expected.value_type());
        assert_eq!(random.support(), expected.support());
        let probabilities: Vec<_> = random
            .support()
            .iter()
            .map(|support| support.probability.to_string())
            .collect();
        assert_eq!(probabilities, ["1/4", "3/4"]);
        let workload = rebuilt.workload("W").expect("the workload rebuilds");
        let checked = source.workload("W").expect("the workload is checked");
        assert_eq!(workload.weights().len(), checked.weights().len());
        for ((found, found_weight), (expected, expected_weight)) in
            workload.weights().iter().zip(checked.weights())
        {
            assert_eq!(found.name(), expected.name());
            assert_eq!(found.declaring(), expected.declaring());
            assert_eq!(found_weight, expected_weight);
        }
    });
}

/// FR-185-AC-14: a workload on a source subtype weighs the operations it
/// inherits, read back with the Boolean support of its random parameter.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn the_package_route_rebuilds_a_boolean_parameter_and_an_inheriting_workload() {
    let package = checked(
        "object Base { n: Integer; operation clear() modifies self.n; }\n\
         object Switch extends Base { operation choose(bit: Boolean) modifies self.n; }\n\
         random Switch::choose(bit) ~ { true: 1, false: 3 };\n\
         workload Input on Switch { weight Switch::choose = 1; weight Base::clear = 0; }\n",
    );
    let source = package.graph().probabilistic_model();
    through_the_route(&package, |input| {
        let input = input.unwrap_or_else(|pending| panic!("the route builds its input: {pending}"));
        let rebuilt = input.input().probabilistic;
        let [random] = rebuilt.random_parameters() else {
            panic!("one random parameter rebuilt");
        };
        assert_eq!(random.support(), source.random_parameters()[0].support());
        let workload = rebuilt.workload("Input").expect("the workload rebuilds");
        let checked = source.workload("Input").expect("the workload is checked");
        let spelled = |weights: &[(fsl_semantics::check::ProbabilisticOperation, _)]| {
            let mut spelled: Vec<_> = weights
                .iter()
                .map(|(operation, weight): &(_, filament_exact::Rational)| {
                    (operation.name().to_owned(), weight.to_string())
                })
                .collect();
            spelled.sort();
            spelled
        };
        assert_eq!(spelled(workload.weights()), spelled(checked.weights()));
        assert_eq!(workload.weights().len(), 2);
    });
}
