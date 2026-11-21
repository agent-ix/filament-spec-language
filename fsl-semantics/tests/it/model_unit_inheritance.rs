// SPDX-License-Identifier: AGPL-3.0-or-later
//! A model unit's `extends`, field `subsets` and `redefines`, and
//! operation `redefines` (FR-710 grammar) reach the checked model, where
//! FR-081 and FR-082 decide them.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::DomainPackageRecord;
use fsl_semantics::model::intake::{
    admit, admit_unit, package_input, read_records, PackageInput, UnitIntakeCause,
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

fn assert_redefinition_target(model: &str, expected_target: &str, expected_redefiner: &str) {
    let packages = compile(model).expect("the model unit compiles");
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
    assert_eq!(target.node, expected_target);
    assert_eq!(
        redefiners
            .iter()
            .map(|key| key.node.as_str())
            .collect::<Vec<_>>(),
        [expected_redefiner]
    );
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
    assert!(total
        .redefines
        .as_ref()
        .is_some_and(|key| key.node == "ix://example/shop/Base/total"));
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
    assert_redefinition_target(
        &model,
        "ix://example/shop/Other/bump",
        "ix://example/shop/Child/bump",
    );
}

#[trace("FR-082-AC-2", "TC-1942")]
#[test]
fn same_owner_and_self_operation_redefinitions_refuse_the_original_owner() {
    let self_target = SHOP.replace("redefines Base::bump", "redefines Child::bump");
    assert_redefinition_target(
        &self_target,
        "ix://example/shop/Child/bump",
        "ix://example/shop/Child/bump",
    );
    let same_owner = SHOP
        .replace(
            "object Child extends Base {",
            "object Child extends Base { operation other(): Boolean;",
        )
        .replace("redefines Base::bump", "redefines Child::other");
    assert_redefinition_target(
        &same_owner,
        "ix://example/shop/Child/other",
        "ix://example/shop/Child/bump",
    );
    let mixed = SHOP.replace("  operation bump(): Boolean modifies self.total;",
        "  operation bump(): Boolean modifies self.total;\n  operation local(): Boolean redefines Base::bump modifies self.total;");
    assert_redefinition_target(
        &mixed,
        "ix://example/shop/Base/bump",
        "ix://example/shop/Base/local",
    );
    assert_eq!(
        check_causes(&compile(SHOP).expect("the proper ancestor form compiles")),
        Vec::<&str>::new()
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
machine BaseMachine { active: Boolean; operation toggle(); states (Ready); transition Ready -> Ready on toggle; }
machine ChildMachine extends BaseMachine { operation turn(); states (Done); transition Done -> Done on turn; }
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

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn an_object_extending_another_meaning_refuses_at_its_declaration() {
    for parent in ["Row", "BaseMachine", "BaseProcess"] {
        let declaration = format!("object Child extends {parent} {{ }}");
        let source = MACHINES_AND_PROCESSES.replace("object Child extends Base { }", &declaration)
            + "record Row { n: Integer; }\n";
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("test", "inheritance-model"),
            "model.fsl",
            source.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("source metadata admitted");
        let diagnostics = fsl_semantics::model::intake::compile_model_unit_with_causes(
            &parsed,
            ModelNormalizationLimits::default(),
        )
        .expect_err("a wrong-meaning supertype never produces a document");
        let [diagnostic] = diagnostics.as_slice() else {
            panic!("one wrong-meaning refusal for {parent}: {diagnostics:#?}");
        };
        assert_eq!(
            diagnostic.code,
            fsl_cst::CompleteCode::InvalidModelBinding,
            "{parent}: {diagnostic:#?}"
        );
        assert_eq!(diagnostic.cause.as_str(), "malformed-declaration");
        let span = parsed
            .source()
            .render(diagnostic.region.as_ref().expect("source-located refusal"))
            .expect("source span renders");
        let start = source.find(&declaration).expect("the authored declaration");
        assert_eq!(span.start.byte, start, "{parent}: {diagnostic:#?}");
        assert_eq!(span.end.byte, start + declaration.len());
    }
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn non_object_headers_without_extends_in_the_grammar_refuse_the_keyword() {
    for declaration in [
        "record Row extends Ghost { n: Integer; }",
        "event Notice extends Ghost { timestamp: Timestamp; occurs timestamp; }",
        "interface Service extends Ghost { operation update(); }",
        "store Repository extends Ghost persists Node { operation write_record(); }",
    ] {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             domain \"example/shop\";\n\
             object Node identity (id) {{ id: Integer; }}\n\
             {declaration}\n"
        );
        assert_eq!(
            compile_refusals(&source),
            [("unexpected-token", "extends".to_owned())],
            "{declaration}"
        );
    }
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn non_object_record_fields_refuse_domain_field_narrowing_syntax() {
    for modifier in ["subsets items", "redefines Row::items"] {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             domain \"example/shop\";\n\
             record Row {{ items: Integer; small: Integer {modifier}; }}\n"
        );
        let keyword = modifier.split_whitespace().next().expect("a modifier");
        assert_eq!(
            compile_refusals(&source),
            [("unexpected-token", keyword.to_owned())],
            "{modifier}"
        );
    }
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn non_object_event_and_interface_local_subsets_survive_compilation_and_intake() {
    for (owner, declaration) in [
        (
            "Notice",
            "event Notice { small: Integer [0, 2] subsets items; items: Integer [0, 5]; timestamp: Timestamp; occurs timestamp; }",
        ),
        (
            "Service",
            "interface Service { small: Integer [0, 2] subsets items; items: Integer [0, 5]; operation update(); }",
        ),
    ] {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             domain \"example/shop\";\n{declaration}\n"
        );
        let packages = compile(&source)
            .unwrap_or_else(|diagnostics| panic!("{owner} source compiles: {diagnostics:#?}"));
        let (_, document) = admit(
            "example/shop",
            &packages,
            ModelNormalizationLimits::default().intake,
        )
        .expect("the compiler produced an admissible domain document");
        let records = read_records("example/shop", &document)
            .expect("the event/interface fields are retained");
        let field_identity = format!("ix://example/shop/{owner}/small");
        let field = records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::FieldMember(field) if field.key.node == field_identity => {
                    Some(field)
                }
                _ => None,
            })
            .expect("the subsetting field remains a declared member");
        assert_eq!(field.owner.node, format!("ix://example/shop/{owner}"));
        assert_eq!(field.subsets.len(), 1);
        assert_eq!(
            field.subsets[0].node,
            format!("ix://example/shop/{owner}/items")
        );
        assert!(field.redefines.is_none());
    }
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn non_object_event_and_interface_field_links_refuse_unresolved_names_at_source() {
    for declaration in [
        "event Notice { items: Integer; small: Integer LINK; timestamp: Timestamp; occurs timestamp; }",
        "interface Service { items: Integer; small: Integer LINK; operation update(); }",
    ] {
        for (link, name) in [
            ("subsets ghost", "ghost"),
            ("redefines Ghost::items", "Ghost"),
            ("redefines Notice::ghost", "ghost"),
        ] {
            // Both owner spellings are valid declarations in their own row.
            let owner_link = if declaration.starts_with("interface") {
                link.replace("Notice::", "Service::")
            } else {
                link.to_owned()
            };
            let source = format!(
                "language \"ix:native\" edition \"1-draft\";\n\
                 domain \"example/shop\";\n{}\n",
                declaration.replace("LINK", &owner_link)
            );
            assert_eq!(
                compile_refusals(&source),
                [("missing-name", name.to_owned())],
                "{declaration}: {owner_link}"
            );
        }
    }
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn non_object_interface_and_store_operation_links_refuse_unresolved_names_at_source() {
    for declaration in [
        "interface Service { operation update() redefines LINK; }",
        "store Repository persists Node { operation write_record() redefines LINK; }",
    ] {
        for (link, name) in [("Ghost::update", "Ghost"), ("Other::ghost", "ghost")] {
            let source = format!(
                "language \"ix:native\" edition \"1-draft\";\n\
                 domain \"example/shop\";\n\
                 object Node identity (id) {{ id: Integer; }}\n\
                 interface Other {{ operation update(); }}\n{}\n",
                declaration.replace("LINK", link)
            );
            assert_eq!(
                compile_refusals(&source),
                [("missing-name", name.to_owned())],
                "{declaration}: {link}"
            );
        }
    }
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn non_object_redefinitions_retain_the_shared_refusal_at_the_source_declaration() {
    for declaration in [
        "event Notice { items: Integer; small: Integer redefines Notice::items; timestamp: Timestamp; occurs timestamp; }",
        "interface Service { items: Integer; small: Integer redefines Service::items; }",
        "interface Service { operation a(); operation b() redefines Service::a; }",
        "store Repository persists Node { operation a(); operation b() redefines Repository::a; }",
    ] {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\ndomain \"example/shop\";\nobject Node identity (id) {{ id: Integer; }}\n{declaration}\n"
        );
        let Err(diagnostics) = compile(&source) else {
            panic!("a non-ancestor redefinition cannot produce a document: {declaration}");
        };
        assert!(!diagnostics.is_empty());
        for diagnostic in diagnostics {
            assert_eq!(
                diagnostic.code,
                fsl_cst::CompleteCode::InvalidModelBinding,
                "{diagnostic:#?}"
            );
            assert_eq!(
                diagnostic.cause.as_str(),
                "malformed-declaration",
                "{diagnostic:#?}"
            );
            let span = diagnostic
                .byte_span()
                .expect("a declaration-located refusal");
            assert_eq!(
                &source[span.start..span.end],
                declaration,
                "{diagnostic:#?}"
            );
        }
    }
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn non_object_disallowed_members_preserve_the_exact_parser_diagnostic() {
    for (declaration, offending) in [
        (
            "event Notice { operation bump(); timestamp: Timestamp; occurs timestamp; }",
            "bump",
        ),
        ("store Repository persists Node { n: Integer; }", "n"),
    ] {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\ndomain \"example/shop\";\nobject Node identity (id) {{ id: Integer; }}\n{declaration}\n"
        );
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("test", "inheritance-model"),
            "model.fsl",
            source.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("source metadata admits");
        assert!(!parsed.is_admissible());
        let compiler = fsl_semantics::model::intake::compile_model_unit_with_causes(
            &parsed,
            ModelNormalizationLimits::default(),
        )
        .expect_err("invalid source produces no model");
        assert_eq!(compiler.as_slice(), parsed.diagnostics());
        let [diagnostic] = compiler.as_slice() else {
            panic!("one parser refusal: {compiler:#?}");
        };
        assert_eq!(diagnostic.phase, fsl_foundation::Phase::Parse);
        assert_eq!(diagnostic.code, fsl_cst::CompleteCode::InvalidSyntax);
        assert_eq!(diagnostic.cause, fsl_cst::CompleteCause::UnexpectedToken);
        let span = diagnostic
            .byte_span()
            .expect("the offending token is source located");
        assert_eq!(&source[span.start..span.end], offending);
    }
}

#[trace("FR-710-AC-10", "FR-710-AC-3", "TC-1942")]
#[test]
fn non_object_contextual_operation_is_an_event_field_name() {
    let source = "language \"ix:native\" edition \"1-draft\";\ndomain \"example/shop\";\nevent Notice { operation: Integer; timestamp: Timestamp; occurs timestamp; }\n";
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "inheritance-model"),
        "model.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admits");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let packages = compile(source).expect("a contextual field name compiles");
    let (_, document) = admit(
        "example/shop",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("the compiled event admits");
    let records = read_records("example/shop", &document).expect("the event fields are retained");
    let field = records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::FieldMember(field)
                if field.key.node == "ix://example/shop/Notice/operation" =>
            {
                Some(field)
            }
            _ => None,
        })
        .expect("operation is a field, not a discarded operation declaration");
    assert_eq!(field.owner.node, "ix://example/shop/Notice");
    assert_eq!(
        field.value_type,
        fsl_semantics::model::domain_package::ValueTypeRef::Native(
            fsl_semantics::model::domain_package::NativeValueType::Integer
        )
    );
}
