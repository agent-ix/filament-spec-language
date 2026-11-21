// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-740 "Source objects": the package route's probabilistic model over a
//! package whose random parameters and workloads are declared on source
//! object types, read from the emitted bytes alone (QSL-2716). The route is
//! given no source package: it rebuilds them from the package's source-object
//! rows. The model-check input of such a package stays refused (FR-940-AC-6):
//! the rebuilt packages hold no field, population or effect frame.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::simulation::model::{PackageModelInput, PendingPackageForm};
use fsl_package::{
    BarePackage, BareSupply, CheckedPackage, EmitLimits, SourceModelFault, UnreadModelForm,
    V2ReadLimits,
};
use fsl_semantics::check::{CheckedProbabilisticModel, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_units_with_cancel, package_input, PackageInput};
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    profile t = \"filament.temporal.complete/v1\";\n\
    dimension Time; unit ms : Time = rational(1, 1);\n";

/// The units of one package named `package`, through S3: each is the source
/// identity and the text after [`HEADER`]'s declarations, which the first
/// unit alone makes.
fn checked_units(package: &str, units: &[(&str, &str)]) -> CheckedPackage {
    let units: Vec<_> = units
        .iter()
        .enumerate()
        .map(|(at, (identity, text))| {
            let header: String = HEADER
                .lines()
                .take(if at == 0 { usize::MAX } else { 3 })
                .map(|line| format!("{line}\n"))
                .collect();
            let parsed = fsl_cst::parse(
                fsl_foundation::SourceIdentity::new("agent-ix", *identity),
                "program.native",
                format!("{header}{text}").as_bytes(),
                fsl_cst::Limits::default(),
            )
            .expect("the source parses");
            assert_eq!(parsed.diagnostics(), []);
            let unit = fsl_forms::build_unit(&parsed).expect("the forms build");
            (parsed.source().reference().clone(), unit)
        })
        .collect();
    let selections: Vec<_> = units
        .iter()
        .map(|(_, unit)| unit.selections().models.as_slice())
        .collect();
    let selected = admit_units_with_cancel(
        &selections,
        &packages(),
        ModelNormalizationLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("empty selected models admit");
    let mut declarations = PackageDeclarations::assemble_units(
        package.to_owned(),
        units,
        selected,
        Vec::new(),
        fsl_semantics::check::AssemblyLimits::default(),
        &filament_exact::Cancel::new(),
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
        .unwrap_or_else(|refusals| panic!("S3 admits the units: {refusals:?}"))
}

/// `body` as the one unit of the package `source-object-probabilistic`.
fn checked(body: &str) -> CheckedPackage {
    checked_units("source-object-probabilistic", &[("test:source", body)])
}

fn packages() -> PackageInput {
    package_input(std::iter::empty::<&[u8]>()).expect("an empty package input")
}

/// Runs `check` over `package`'s bytes read back with no source package given.
fn through_the_route<T>(
    name: &str,
    package: &CheckedPackage,
    check: impl FnOnce(&BarePackage<'_>) -> T,
) -> T {
    through_doctored_rows(name, package, |rows| rows, check)
}

/// [`through_the_route`] over identity rows whose file text `doctor` changed.
fn through_doctored_rows<T>(
    name: &str,
    package: &CheckedPackage,
    doctor: impl FnOnce(String) -> String,
    check: impl FnOnce(&BarePackage<'_>) -> T,
) -> T {
    let inputs = packages();
    let read = fsl_package::read_evaluation_package(
        package,
        fsl_semantics::library::LibraryName::new(name).expect("a library name"),
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
    let text = String::from_utf8(rows.to_bytes(admitted).expect("the rows write"))
        .expect("the rows are text");
    let mut reread_budget = V2ReadLimits::fixture().native_join_budget();
    let rows =
        fsl_package::ReadIdentityRows::read(doctor(text).as_bytes(), admitted, &mut reread_budget)
            .expect("the doctored rows still join the package");
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
    check(&bare)
}

/// The probabilistic model the package route rebuilds over `package`.
fn rebuilt(
    name: &str,
    package: &CheckedPackage,
) -> Result<CheckedProbabilisticModel, PendingPackageForm> {
    through_the_route(name, package, |bare| {
        PackageModelInput::source_object_probabilistic_model(
            bare,
            &packages(),
            ModelNormalizationLimits::default(),
        )
    })
}

/// Asserts `rebuilt` holds the random parameters and workloads `source`
/// checked: the same operations (by key, declaring type and name), positions,
/// types, support and weights.
fn assert_equal_to_s3(rebuilt: &CheckedProbabilisticModel, source: &CheckedProbabilisticModel) {
    assert_eq!(
        rebuilt.random_parameters().len(),
        source.random_parameters().len()
    );
    for expected in source.random_parameters() {
        let found = rebuilt
            .random_parameter(expected.operation().key(), expected.parameter())
            .expect("the random parameter is rebuilt");
        assert_eq!(found.operation().key(), expected.operation().key());
        assert_eq!(
            found.operation().declaring(),
            expected.operation().declaring()
        );
        assert_eq!(found.operation().name(), expected.operation().name());
        assert_eq!(found.position(), expected.position());
        assert_eq!(found.value_type(), expected.value_type());
        assert_eq!(found.support(), expected.support());
    }
    for expected in source.workloads() {
        let found = rebuilt
            .workload(expected.name())
            .expect("the workload is rebuilt");
        assert_eq!(found.weights().len(), expected.weights().len());
        for (expected, expected_weight) in expected.weights() {
            let (operation, weight) = found
                .weights()
                .iter()
                .find(|(operation, _)| operation.key() == expected.key())
                .expect("the operation is weighted");
            assert_eq!(operation.declaring(), expected.declaring());
            assert_eq!(operation.name(), expected.name());
            assert_eq!(weight, expected_weight);
        }
    }
}

/// FR-740-AC-6: the package route rebuilds the random parameter and the
/// workload declared on a source object from the emitted bytes alone, with
/// the AC's literal `k: Int[1, 3]`: probabilities `1/4` and `3/4`, equal to
/// the records S3 checked.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn the_package_route_rebuilds_a_source_random_parameter_and_workload() {
    let package = checked(
        "object O { n: Integer; operation step(k: Int[1, 3]) modifies self.n; }\n\
         random O::step(k) ~ { 1: 1, 3: 3 };\n\
         workload W on O { weight O::step = 1; }\n",
    );
    let model = rebuilt("source-object-probabilistic", &package)
        .unwrap_or_else(|pending| panic!("the route rebuilds the model: {pending}"));
    assert_equal_to_s3(&model, package.graph().probabilistic_model());
    let [random] = model.random_parameters() else {
        panic!("one random parameter rebuilt");
    };
    let probabilities: Vec<_> = random
        .support()
        .iter()
        .map(|support| support.probability.to_string())
        .collect();
    assert_eq!(probabilities, ["1/4", "3/4"]);
}

/// FR-740-AC-6: a Boolean parameter and a workload on a source subtype that
/// weighs an inherited operation rebuild equal to S3's.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn the_package_route_rebuilds_a_boolean_parameter_and_an_inheriting_workload() {
    let package = checked(
        "object Base { n: Integer; operation clear() modifies self.n; }\n\
         object Switch extends Base { operation choose(bit: Boolean) modifies self.n; }\n\
         random Switch::choose(bit) ~ { true: 1, false: 3 };\n\
         workload Input on Switch { weight Switch::choose = 1; weight Base::clear = 0; }\n",
    );
    let model = rebuilt("source-object-probabilistic", &package)
        .unwrap_or_else(|pending| panic!("the route rebuilds the model: {pending}"));
    assert_equal_to_s3(&model, package.graph().probabilistic_model());
    assert_eq!(model.workload("Input").expect("rebuilt").weights().len(), 2);
}

/// FR-740-AC-6: a package of two units, each declaring objects, rebuilds each
/// unit's random parameter and workload under the source identity S3 gives
/// that unit (`source/<package>` and `source/<package>/1`), for a package
/// name that holds path separators.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn the_package_route_rebuilds_each_unit_of_a_package_under_its_own_source_identity() {
    let name = "acme/v/2";
    let package = checked_units(
        name,
        &[
            (
                "test:first",
                "object A { n: Integer; operation t(k: Int[1, 3]) modifies self.n; }\n\
                 random A::t(k) ~ { 1: 1, 3: 3 };\n\
                 workload WA on A { weight A::t = 1; }\n",
            ),
            (
                "test:second",
                "object B { n: Integer; operation u(k: Int[1, 3]) modifies self.n; }\n\
                 random B::u(k) ~ { 1: 3, 3: 1 };\n\
                 workload WB on B { weight B::u = 1; }\n",
            ),
        ],
    );
    let source = package.graph().probabilistic_model();
    let model = rebuilt(name, &package)
        .unwrap_or_else(|pending| panic!("the route rebuilds the model: {pending}"));
    assert_equal_to_s3(&model, source);
    let identities: Vec<_> = source
        .random_parameters()
        .iter()
        .map(|random| random.operation().key().package.clone())
        .collect();
    assert_eq!(
        identities,
        ["source/acme/v/2".to_owned(), "source/acme/v/2/1".to_owned()]
    );
}

/// FR-740-AC-6: a Text and a Decimal parameter of an operation a workload
/// weighs read back from their renderings, equal to S3's.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn the_package_route_rebuilds_a_workload_over_text_and_decimal_parameters() {
    let package = checked(
        "object O { n: Integer; operation step(t: Text[0, 8; nfc], d: Decimal[0, 100; 2, 2; exact]) \
         modifies self.n; }\n\
         workload W on O { weight O::step = 1; }\n",
    );
    let model = rebuilt("source-object-probabilistic", &package)
        .unwrap_or_else(|pending| panic!("the route rebuilds the model: {pending}"));
    assert_equal_to_s3(&model, package.graph().probabilistic_model());
}

/// FR-740-AC-6: a parameter whose rendering a reader does not rebuild
/// (`Rational`) leaves its operation out of the source package. A workload on
/// another operation still rebuilds; a node that names the operation is
/// refused at the row's node.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn a_probabilistic_node_over_an_unread_parameter_type_is_refused_at_the_row() {
    let unread = "object O { n: Integer; operation odd(r: Rational[0, 4; 1, 4]) modifies self.n; \
                  operation plain(k: Int[1, 3]) modifies self.n; }\n";
    let kept = checked(&format!("{unread}random O::plain(k) ~ {{ 1: 1, 3: 3 }};\n"));
    let model = rebuilt("source-object-probabilistic", &kept)
        .unwrap_or_else(|pending| panic!("the route rebuilds the model: {pending}"));
    assert_equal_to_s3(&model, kept.graph().probabilistic_model());

    let named = checked(&format!(
        "{unread}workload W on O {{ weight O::odd = 1; weight O::plain = 1; }}\n"
    ));
    let Err(PendingPackageForm::SourceModel(fault)) =
        rebuilt("source-object-probabilistic", &named)
    else {
        panic!("the route refuses the unread parameter");
    };
    assert!(matches!(
        &fault,
        SourceModelFault::UnreadParameter { operation, parameter, .. }
            if operation == "O::odd" && parameter == "r"
    ));
    through_the_route("source-object-probabilistic", &named, |bare| {
        let node = bare
            .first_unread_model_form()
            .map(|(_, node)| node.clone())
            .expect("the package holds a source object row");
        assert_eq!(fault.node(), &node);
    });
}

/// FR-940-AC-6: a package whose source objects hold a field, a population and
/// an operation with an effect frame is refused by the model-check input at
/// the row's node, as before the source model was rebuilt: the rebuilt source
/// package holds none of them, so a step over it would change nothing. Its
/// probabilistic model still rebuilds.
#[trace("TC-3012", "FR-940-AC-6")]
#[test]
fn the_model_check_input_over_a_stepping_source_object_is_refused() {
    let package = checked(
        "object O { n: Int[0, 3]; population os; operation step(k: Int[1, 3]) modifies self.n; }\n\
         random O::step(k) ~ { 1: 1, 3: 3 };\n\
         workload W on O { weight O::step = 1; }\n",
    );
    through_the_route("source-object-probabilistic", &package, |bare| {
        let Err(PendingPackageForm::Form { form, node }) =
            PackageModelInput::of(bare, &packages(), ModelNormalizationLimits::default())
        else {
            panic!("the model-check input is refused");
        };
        assert_eq!(form, UnreadModelForm::SourceObject);
        assert_eq!(
            Some(&node),
            bare.first_unread_model_form().map(|(_, node)| node)
        );
    });
    let model = rebuilt("source-object-probabilistic", &package)
        .unwrap_or_else(|pending| panic!("the route rebuilds the model: {pending}"));
    assert_equal_to_s3(&model, package.graph().probabilistic_model());
}

/// FR-740-AC-6: an operation identity that is not `<type>::<operation>` is
/// refused at the row's node.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn a_malformed_source_operation_identity_is_refused_at_the_row() {
    let package = checked(
        "object O { n: Integer; operation step(k: Int[1, 3]) modifies self.n; }\n\
         random O::step(k) ~ { 1: 1, 3: 3 };\n",
    );
    let name = "source-object-probabilistic";
    let Err(PendingPackageForm::SourceModel(fault)) = through_doctored_rows(
        name,
        &package,
        |text| {
            // Only the source object's own operation row is malformed: the
            // `source_operations` rows and the operation bodies keep the
            // valid identity, so the supply joins and the refusal lands at
            // the object row.
            let mut file: serde_json::Value =
                serde_json::from_str(&text).expect("the rows are JSON");
            let operations = file["source_objects"]
                .as_array_mut()
                .expect("source object rows")
                .iter_mut()
                .flat_map(|object| object["operations"].as_array_mut().expect("operations"));
            for operation in operations {
                operation["identity"] = "Ostep".into();
            }
            file.to_string()
        },
        |bare| {
            PackageModelInput::source_object_probabilistic_model(
                bare,
                &packages(),
                ModelNormalizationLimits::default(),
            )
        },
    ) else {
        panic!("the route refuses the malformed operation identity");
    };
    assert!(matches!(
        &fault,
        SourceModelFault::MalformedOperation { identity, .. } if identity == "Ostep"
    ));
}

/// FR-740-AC-6: a collection-valued scalar parameter rebuilds with the
/// multiplicity, presence and key S3's record gives it, which the rendering
/// of the element type does not carry: the row spells it as a collection
/// layer.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn the_package_route_rebuilds_the_multiplicity_of_a_collection_parameter() {
    use fsl_semantics::model::domain_package::DomainPackageRecord;
    let package = checked(
        "object O { n: Integer; operation step(ks: Set<Int[1, 3]>[0, 2], k: Int[1, 3]) \
         modifies self.n; }\n\
         workload W on O { weight O::step = 1; }\n",
    );
    let parameters = |packages: &[fsl_semantics::model::domain_package::DomainPackage]| {
        packages
            .iter()
            .flat_map(|package| &package.records)
            .filter_map(|record| match record {
                DomainPackageRecord::OperationMember(operation) => Some(
                    operation
                        .parameters
                        .iter()
                        .map(|parameter| {
                            (
                                parameter.key.clone(),
                                parameter.multiplicity,
                                parameter.presence,
                            )
                        })
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let expected = parameters(package.graph().source_packages());
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].len(), 2);
    assert_ne!(expected[0][0].1, expected[0][1].1);
    through_the_route("source-object-probabilistic", &package, |bare| {
        let model = bare
            .source_model()
            .expect("the rows describe the source model");
        assert_eq!(parameters(&model.packages), expected);
    });
}

/// FR-740-AC-6: each collection kind and element type a source operation
/// parameter spells rebuilds, from the emitted bytes, with the key,
/// multiplicity, presence and value type S3's record gives it.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn the_package_route_rebuilds_every_collection_kind_of_a_parameter() {
    use fsl_semantics::model::domain_package::DomainPackageRecord;
    let parameters = |packages: &[fsl_semantics::model::domain_package::DomainPackage]| {
        packages
            .iter()
            .flat_map(|package| &package.records)
            .filter_map(|record| match record {
                DomainPackageRecord::OperationMember(operation) => Some(
                    operation
                        .parameters
                        .iter()
                        .map(|parameter| {
                            (
                                parameter.key.clone(),
                                parameter.multiplicity,
                                parameter.presence,
                                parameter.value_type.to_string(),
                            )
                        })
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    for (spelling, unbounded) in [
        ("Bag<Int[1, 3]>[0, 2]", false),
        ("Sequence<Int[1, 3]>[1, 4]", false),
        ("OrderedSet<Int[1, 3]>[0, 3]", false),
        ("Sequence<Text[0, 8; nfc]>[0, 2]", false),
        ("Set<Decimal[0, 100; 2, 2; exact]>[0, 2]", false),
        ("Sequence<Integer>", true),
    ] {
        let package = checked(&format!(
            "object O {{ n: Integer; operation step(ks: {spelling}) modifies self.n; }}\n\
             workload W on O {{ weight O::step = 1; }}\n"
        ));
        let expected = parameters(package.graph().source_packages());
        assert_eq!(expected.len(), 1, "{spelling}");
        assert_eq!(expected[0].len(), 1, "{spelling}");
        assert_eq!(expected[0][0].1.upper.is_none(), unbounded, "{spelling}");
        through_the_route("source-object-probabilistic", &package, |bare| {
            let model = bare
                .source_model()
                .expect("the rows describe the source model");
            assert_eq!(parameters(&model.packages), expected, "{spelling}");
        });
    }
}
