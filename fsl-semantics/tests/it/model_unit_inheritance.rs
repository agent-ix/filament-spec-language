// SPDX-License-Identifier: AGPL-3.0-or-later
//! A model unit's `extends`, field `subsets` and `redefines`, and
//! operation `redefines` (FR-710 grammar) reach the checked model, where
//! FR-081 and FR-082 decide them.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::DomainPackageRecord;
use fsl_semantics::model::intake::{
    PackageInput, UnitIntakeCause, admit, admit_unit, package_input, read_records,
};
use fsl_semantics::model::normalize::ModelRefusalCause;
use ix_trace_rs::trace;

/// Two object types, `Child` specializing `Base` with one subsetting field,
/// one field redefinition and one operation redefinition.
const SHOP: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
object Base identity (id) {
  id: Integer;
  total: Integer;
  items: Integer [0, 5];
  operation bump(): Boolean modifies self.total;
}
object Child extends Base {
  total: Integer redefines Base::total;
  few: Integer [0, 2] subsets items;
  operation bump(): Boolean redefines Base::bump modifies self.total;
}
population children: Child;
";

fn compile(model: &str) -> Result<PackageInput, Vec<fsl_cst::CompleteDiagnostic>> {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "inheritance-model"),
        "model.fsl",
        model.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    fsl_semantics::model::intake::compile_model_unit_with_causes(
        &parsed,
        ModelNormalizationLimits::default(),
    )?;
    Ok(package_input([model.as_bytes()]).expect("a compiled model unit is package input"))
}

/// The causes `compile` refuses `model` with, and the text each is at.
fn compile_refusals(model: &str) -> Vec<(&'static str, String)> {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "inheritance-model"),
        "model.fsl",
        model.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let diagnostics = fsl_semantics::model::intake::compile_model_unit_with_causes(
        &parsed,
        ModelNormalizationLimits::default(),
    )
    .expect_err("the model unit is refused");
    diagnostics
        .iter()
        .map(|diagnostic| {
            let region = diagnostic.region.as_ref().expect("located refusal");
            let span = parsed.source().render(region).expect("region renders");
            (
                diagnostic.cause.as_str(),
                model[span.start.byte..span.end.byte].to_owned(),
            )
        })
        .collect()
}

/// The unit selecting `example/shop` as `M` with an invariant over `M::Child`.
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";
profile P = \"filament.model.complete/v1\";
model M = \"example/shop\";
invariant I using P on M::Child at current { self.total >= 0 }
";

/// Every cause the checker refuses the unit with over `packages`, from model
/// intake through the checked graph.
fn check_causes(packages: &PackageInput) -> Vec<&'static str> {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "inheritance-unit"),
        "unit.native",
        UNIT.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = match admit_unit(
        &built.selections().models,
        packages,
        ModelNormalizationLimits::default(),
    ) {
        Ok(models) => models,
        Err(refusal) => {
            eprintln!("I1 intake refusal: {refusal:#?}");
            return vec![refusal.cause.cause().expect("a catalogued intake refusal")];
        }
    };
    let mut declarations = match PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        models,
        Vec::new(),
    ) {
        Ok(declarations) => declarations,
        Err(refusal) => {
            eprintln!("S3 assembly refusal: {refusal:#?}");
            return refusal
                .errors
                .iter()
                .map(|error| error.cause.catalog_code().cause())
                .collect();
        }
    };
    let pending = match declarations.link_dispatch(ModelNormalizationLimits::default()) {
        Ok(pending) => pending,
        Err(refusals) => {
            eprintln!("S3 dispatch refusals: {refusals:#?}");
            return causes(&refusals);
        }
    };
    match declarations.check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    ) {
        Ok(_) => Vec::new(),
        Err(refusals) => {
            let refusals = pending.report(refusals);
            eprintln!("S3 check refusals: {refusals:#?}");
            causes(&refusals)
        }
    }
}

fn causes(refusals: &[fsl_semantics::check::CheckRefusal]) -> Vec<&'static str> {
    refusals
        .iter()
        .filter_map(|refusal| refusal.cause.catalog_code())
        .map(|code| code.cause())
        .collect()
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_writes_supertypes_subsets_and_redefinitions_into_the_document() {
    let packages = compile(SHOP).expect("the inheritance model unit compiles");
    let (_, document) = admit(
        "example/shop",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the compiled inheritance");
    let records = read_records("example/shop", &document).expect("records are read");
    let object = records.iter().find_map(|record| match record {
        DomainPackageRecord::ObjectType(object) if object.key.node.ends_with("/Child") => {
            Some(object)
        }
        _ => None,
    });
    let object = object.expect("Child is an object type");
    assert_eq!(object.supertypes.len(), 1);
    assert!(object.supertypes[0].node.ends_with("/Base"));
    let few = records.iter().find_map(|record| match record {
        DomainPackageRecord::FieldMember(field) if field.key.node.ends_with("/Child/few") => {
            Some(field)
        }
        _ => None,
    });
    let few = few.expect("Child.few is a field");
    assert_eq!(few.subsets.len(), 1);
    assert!(few.subsets[0].node.ends_with("/Base/items"));
    let bump = records.iter().find_map(|record| match record {
        DomainPackageRecord::OperationMember(op) if op.key.node.ends_with("/Child/bump") => {
            Some(op)
        }
        _ => None,
    });
    let redefines = bump.and_then(|op| op.redefines.as_ref());
    assert!(redefines.is_some_and(|key| key.node.ends_with("/Base/bump")));
    let total = records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::FieldMember(field) if field.key.node.ends_with("/Child/total") => {
                Some(field)
            }
            _ => None,
        })
        .expect("Child.total is a field");
    assert!(
        total
            .redefines
            .as_ref()
            .is_some_and(|key| key.node == "ix://example/shop/Base/total")
    );
}

#[trace("FR-082-AC-2", "TC-1942")]
#[test]
fn a_conforming_specialization_subsetting_and_redefinition_checks() {
    for model in [
        SHOP.to_owned(),
        SHOP.replace("modifies self.total;", "modifies self.total, self.items;"),
    ] {
        let packages = compile(&model).expect("the inheritance model unit compiles");
        assert_eq!(check_causes(&packages), Vec::<&str>::new(), "{model}");
    }
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn a_supertype_that_names_no_declaration_refuses_at_the_name() {
    let model = SHOP.replace("extends Base", "extends Ghost");
    assert_eq!(
        compile_refusals(&model),
        [("missing-name", "Ghost".to_owned())]
    );
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn a_subsets_or_redefines_that_finds_no_member_refuses_at_the_name() {
    for (from, to, at) in [
        ("subsets items", "subsets ghost", "ghost"),
        ("redefines Base::bump", "redefines Base::ghost", "ghost"),
        ("redefines Base::bump", "redefines Ghost::bump", "Ghost"),
        ("redefines Base::total", "redefines Base::ghost", "ghost"),
    ] {
        let model = SHOP.replace(from, to);
        assert_eq!(
            compile_refusals(&model),
            [("missing-name", at.to_owned())],
            "{to}"
        );
    }
}

#[trace("FR-082-AC-2", "TC-1942")]
#[test]
fn a_redefinition_naming_a_type_that_is_not_a_supertype_refuses_the_target() {
    let model = SHOP
        .replace(
            "object Child extends Base",
            "object Other { operation bump(): Boolean; }\nobject Child extends Base",
        )
        .replace("redefines Base::bump", "redefines Other::bump");
    let packages = compile(&model).expect("the model unit compiles");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "inheritance-unit"),
        "unit.native",
        UNIT.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the selecting unit parses");
    let built = fsl_forms::build_unit(&parsed).expect("the selecting unit builds");
    let Err(refusal) = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    ) else {
        panic!("an unrelated operation cannot be a redefinition target");
    };
    let UnitIntakeCause::Refused(found) = refusal.cause else {
        panic!("the refusal is semantic: {refusal:?}");
    };
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].code,
        fsl_foundation::diagnostic::Code::InvalidModelBinding
    );
    let ModelRefusalCause::RedefinitionTarget { redefiners, target } = &found[0].cause else {
        panic!("the unreachable target retains its typed cause: {found:?}");
    };
    assert_eq!(target.node, "ix://example/shop/Other/bump");
    assert_eq!(
        redefiners
            .iter()
            .map(|key| key.node.as_str())
            .collect::<Vec<_>>(),
        ["ix://example/shop/Child/bump"]
    );
}

#[trace("FR-082-AC-2", "TC-1942")]
#[test]
fn a_supertype_cycle_refuses_specialization_cycle() {
    let model = SHOP.replace(
        "object Base identity (id)",
        "object Base extends Child identity (id)",
    );
    let packages = compile(&model).expect("the model unit compiles");
    assert!(
        check_causes(&packages).contains(&"specialization-cycle"),
        "{:?}",
        check_causes(&packages)
    );
}

#[trace("FR-082-AC-1", "TC-1942")]
#[test]
fn a_redefinition_that_widens_the_frame_refuses_effect_escape() {
    let model = SHOP.replace(
        "redefines Base::bump modifies self.total",
        "redefines Base::bump modifies self.total, self.items",
    );
    let packages = compile(&model).expect("the model unit compiles");
    let found = check_causes(&packages);
    assert!(found.contains(&"effect-escape"), "{found:?}");
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn inheritance_resolves_forward_declarations_and_transitive_members() {
    let model = SHOP.replace("object Child extends Base", "object Child extends Middle")
        + "object Middle extends Base {}\n";
    let packages = compile(&model).expect("the later supertype is resolved");
    assert_eq!(check_causes(&packages), Vec::<&str>::new());
    let (_, document) = admit(
        "example/shop",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("the compiled document admits");
    let records = read_records("example/shop", &document).expect("records are read");
    let few = records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::FieldMember(field)
                if field.key.node == "ix://example/shop/Child/few" =>
            {
                Some(field)
            }
            _ => None,
        })
        .expect("the subset field is declared");
    assert_eq!(few.subsets.len(), 1);
    assert_eq!(few.subsets[0].node, "ix://example/shop/Base/items");
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_field_can_subset_a_field_on_its_own_type() {
    let model = SHOP.replace(
        "  items: Integer [0, 5];",
        "  items: Integer [0, 5];\n  some: Integer [0, 2] subsets items;",
    );
    let packages = compile(&model).expect("the local subset compiles");
    assert_eq!(check_causes(&packages), Vec::<&str>::new());
}

const MACHINES_AND_PROCESSES: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
machine BaseMachine { active: Boolean; states (Ready); }
machine ChildMachine extends BaseMachine { states (Done); }
process BaseProcess { id: Integer; operation go(); step start: begin; }
process ChildProcess extends BaseProcess { operation stop(); step last: begin; }
object Child extends Base { }
object Base { total: Integer; }
population children: Child;
";

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn machine_and_process_inheritance_keeps_the_declared_supertype() {
    let packages =
        compile(MACHINES_AND_PROCESSES).expect("machine and process inheritance compiles");
    let (_, document) = admit(
        "example/shop",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the inherited constructs");
    let records = read_records("example/shop", &document).expect("records are read");
    let machine = records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::StateMachine(machine)
                if machine.key.node == "ix://example/shop/ChildMachine" =>
            {
                Some(machine)
            }
            _ => None,
        })
        .expect("the child machine is retained");
    assert_eq!(machine.supertypes.len(), 1);
    assert_eq!(machine.supertypes[0].node, "ix://example/shop/BaseMachine");
    let process = records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Process(process)
                if process.key.node == "ix://example/shop/ChildProcess" =>
            {
                Some(process)
            }
            _ => None,
        })
        .expect("the child process is retained");
    assert_eq!(process.supertypes.len(), 1);
    assert_eq!(process.supertypes[0].node, "ix://example/shop/BaseProcess");
    assert!(process.identity_fields_declared);
    assert_eq!(process.identity_fields.len(), 1);
    assert_eq!(
        process.identity_fields[0].node, "ix://example/shop/BaseProcess/id",
        "the retained subtype key is the inherited original field"
    );
    let wire: serde_json::Value =
        serde_json::from_slice(&packages["example/shop"]).expect("the compiled document is JSON");
    let child = wire["types"]
        .as_array()
        .expect("the document has types")
        .iter()
        .find(|node| node["identity"] == "ix://example/shop/ChildProcess")
        .expect("the child process is emitted");
    assert_eq!(
        child["identityFields"],
        serde_json::json!([]),
        "the source-derived document explicitly inherits its process key"
    );
    assert_eq!(check_causes(&packages), Vec::<&str>::new());
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn missing_machine_and_process_supertypes_refuse_at_the_name() {
    for parent in ["BaseMachine", "BaseProcess"] {
        let model = MACHINES_AND_PROCESSES.replace(&format!("extends {parent}"), "extends Ghost");
        assert_eq!(
            compile_refusals(&model),
            [("missing-name", "Ghost".to_owned())]
        );
    }
}
