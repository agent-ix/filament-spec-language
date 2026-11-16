// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569: the unit's own `object` declarations (TC-1408), checked on the
//! source route.

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::{
    AssemblyCause, AssemblyRefusal, CheckCause, CheckedGraph, PackageDeclarations,
    SourceObjectFault,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::DomainPackage;
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::model::normalize::ModelRefusalCause;
use ix_trace_rs::trace;

/// TC-1408 fixture S's header.
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"filament.model.complete/v1\";\n\
    profile Pp = \"filament.protocol.finite-global/v1\";\n";

/// TC-1408 fixture S: the three object declarations.
const OBJECTS: &str = "\
object Order identity (total) {
  total: Int[0, 9];
  note: Text[0, 8; nfc]?;
  population orders;
  operation size(): Integer;
  operation count(): Integer;
  operation scaled(n: Integer): Integer;
  operation bump(): Boolean modifies self.total;
}
object Sub extends Order {
  operation size(): Integer redefines Order::size;
  operation scaled(m: Integer): Integer redefines Order::scaled;
}
abstract object Shape { population shapes; }
";

/// TC-1408 fixture S: the clauses.
const CLAUSES: &str = "\
invariant I using P on Order at current { self.total >= 0 }
pre Q using P on Order::bump { self.total < 9 }
post R using P on Order::bump { self.total > pre(self.total) }
body B using P on Sub::size() { 7 }
body C using P on Order::size() { 1 }
pre Qs using P on Order::size { self.total >= 0 }
invariant Sized using P on Order at current { self.size() >= 0 }
";

/// One refusal as its catalog code and cause.
type Refused = (Code, &'static str);

fn assemble(declarations: &str) -> Result<PackageDeclarations, AssemblyRefusal> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "tc-1408"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        SelectedModels::fixture_entries(Vec::new()),
        Vec::new(),
    )
}

/// The unit's refusals as catalog codes, with the span text each is at.
fn refusals(declarations: &str) -> Vec<(Refused, String)> {
    let text = format!("{HEADER}{declarations}\n");
    let refusal = assemble(declarations).expect_err("the unit is refused");
    refusal
        .errors
        .iter()
        .map(|error| {
            (
                (error.cause.code(), error.cause.catalog_code().cause()),
                text[error.span.start..error.span.end].to_owned(),
            )
        })
        .collect()
}

/// The unit's refusals as catalog codes, with the byte offset each is at,
/// relative to the start of `declarations`.
fn refusals_at(declarations: &str) -> Vec<(Refused, usize)> {
    let refusal = assemble(declarations).expect_err("the unit is refused");
    refusal
        .errors
        .iter()
        .map(|error| {
            (
                (error.cause.code(), error.cause.catalog_code().cause()),
                error.span.start - HEADER.len(),
            )
        })
        .collect()
}

/// The offset in `text` of the `occurrence`th (from 0) `needle`, plus `skip`
/// bytes into it.
fn offset(text: &str, needle: &str, occurrence: usize, skip: usize) -> usize {
    text.match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("`{needle}` occurs {} times", occurrence + 1))
        .0
        + skip
}

/// Links dispatch and checks `declarations`, as the compile driver does.
fn finish(
    mut declarations: PackageDeclarations,
) -> Result<CheckedGraph, Vec<fsl_semantics::check::CheckRefusal>> {
    let pending = declarations.link_dispatch(ModelNormalizationLimits::default())?;
    declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .map_err(|refusals| pending.report(refusals))
}

fn check(declarations: &str) -> CheckedGraph {
    finish(assemble(declarations).expect("the unit assembles")).expect("the unit checks")
}

/// A bare type name resolves to the source object type; an operation of a
/// source type, a population and a `Reference` of a subtype resolve too.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn fixture_s_clauses_resolve_to_the_source_declarations() {
    let graph = check(&format!(
        "{OBJECTS}{CLAUSES}\
         invariant V4 using P on Order at current {{ forall(o in orders : deref(o).total >= 0) }}\n\
         function sub using P (r: Reference<Sub>): Integer pure {{ deref(r).total }}\n"
    ));
    for name in ["I", "Q", "R", "Qs", "V4"] {
        assert!(graph.state_clause(name).is_some(), "{name} is checked");
    }
}

/// An operation `T` declares nothing of, a source record and an unknown name
/// each refuse `missing-name` at the name.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn an_unresolved_context_refuses_missing_name_at_the_name() {
    let missing = (Code::MissingDeclaration, "missing-name");
    assert_eq!(
        refusals(&format!(
            "{OBJECTS}pre X using P on Order::reset {{ true }}"
        )),
        [(missing, "reset".to_owned())]
    );
    assert_eq!(
        refusals(&format!(
            "{OBJECTS}record Receipt {{ total: Integer; }}\n\
             invariant X using P on Receipt at current {{ true }}"
        )),
        [(missing, "Receipt".to_owned())]
    );
    assert_eq!(
        refusals(&format!(
            "{OBJECTS}invariant X using P on Nothing at current {{ true }}"
        )),
        [(missing, "Nothing".to_owned())]
    );
}

mod orders_model {
    include!("../fixtures/orders_model.rs");
}
use orders_model::model_package;

/// Source members retain one outer collection as multiplicity, while an
/// optional collection retains its whole checked type.
#[trace("FR-569-AC-15", "FR-088-AC-8", "TC-1940")]
#[test]
fn retained_source_member_types_separate_only_the_outermost_collection() {
    use fsl_semantics::model::domain_package::{DomainPackageRecord, Multiplicity, ValueTypeRef};
    let source = "object Order {\n\
        f: Decimal[0, 100; 2, 2; exact];\n\
        s: Sequence<Set<Integer>[0, 2]>[0, 2];\n\
        r: Reference<Order>; c: Clock;\n\
        o: Option<Sequence<Integer>[0, 3]>;\n\
        operation copy(s: Sequence<Set<Integer>[0, 2]>[0, 2]): Option<Sequence<Integer>[0, 3]>;\n\
    }\n";
    let declarations = assemble(source).expect("all source member forms assemble");
    let models = &declarations
        .model_inputs
        .as_ref()
        .expect("retained source model")
        .0;
    let view = &models
        .iter()
        .find(|model| model.alias.is_empty())
        .expect("source model")
        .view;
    let records = &view.domain_package().records;
    let field = |name: &str| {
        records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::FieldMember(field)
                    if field.key.node.ends_with(&format!("/Order/{name}")) =>
                {
                    Some(field)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Order.{name} is retained"))
    };
    let one = Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    };
    let sequence = Multiplicity {
        lower: 0,
        upper: Some(2),
        ordered: true,
        unique: false,
    };
    assert_eq!(field("s").multiplicity, sequence);
    let ValueTypeRef::Resolved(element) = &field("s").value_type else {
        panic!("resolved nested set");
    };
    let filament_exact::ValueType::Collection(set) = element.as_ref() else {
        panic!("one set element, not another sequence");
    };
    assert_eq!(set.kind(), filament_exact::CollectionKind::Set);
    assert_eq!(set.element(), &filament_exact::ValueType::Integer);
    assert_eq!(
        set.bound(),
        Some(filament_exact::CardinalityBound::new(0, 2).expect("bound"))
    );
    assert_eq!(field("o").multiplicity, one);
    assert!(
        matches!(&field("o").value_type, ValueTypeRef::Resolved(ty) if matches!(ty.as_ref(), filament_exact::ValueType::Option(_)))
    );
    assert_eq!(field("f").multiplicity, one);
    assert!(
        matches!(&field("f").value_type, ValueTypeRef::Resolved(ty) if matches!(ty.as_ref(), filament_exact::ValueType::Decimal(_)))
    );
    assert_eq!(field("c").value_type, ValueTypeRef::Clock);
    assert_eq!(field("c").multiplicity, one);
    let ValueTypeRef::Package(order) = &field("r").value_type else {
        panic!("reference names Order's key");
    };
    assert_eq!(order, &field("r").owner);
    let operation = records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation) => Some(operation),
            _ => None,
        })
        .expect("copy operation");
    assert_eq!(operation.parameters[0].multiplicity, sequence);
    assert_eq!(operation.parameters[0].value_type, field("s").value_type);
    let result = operation.result.as_ref().expect("copy result");
    assert_eq!(result.multiplicity, one);
    assert_eq!(result.value_type, field("o").value_type);
    let graph =
        finish(assemble(&source.replace(" c: Clock;", "")).expect("the data fields assemble"))
            .expect("the data fields check");
    for name in ["f", "o"] {
        let ValueTypeRef::Resolved(value_type) = &field(name).value_type else {
            panic!("resolved {name}");
        };
        assert_eq!(
            value_type.as_ref(),
            &source_field_type(&graph, "Order", name)
        );
    }
    let checked_sequence = source_field_type(&graph, "Order", "s");
    let filament_exact::ValueType::Collection(sequence_type) = &checked_sequence else {
        panic!("checked outer sequence");
    };
    assert_eq!(sequence_type.element(), element.as_ref());
}

#[trace("FR-569-AC-15", "TC-1940")]
#[test]
fn source_model_owned_value_types_have_the_same_resolved_representation() {
    use fsl_semantics::model::domain_package::{DomainPackageRecord, ValueTypeRef};
    let declarations = assemble_model(
        "object Z { total: M::Total; operation f(p: M::Total): M::Total; }",
        model_package(false),
    )
    .expect("model types assemble");
    let models = &declarations.model_inputs.as_ref().expect("models").0;
    let source = models
        .iter()
        .find(|model| model.alias.is_empty())
        .expect("source model");
    let mut types = Vec::new();
    for record in &source.view.domain_package().records {
        match record {
            DomainPackageRecord::FieldMember(field) => types.push(&field.value_type),
            DomainPackageRecord::OperationMember(operation) => {
                types.extend(
                    operation
                        .parameters
                        .iter()
                        .map(|parameter| &parameter.value_type),
                );
                types.extend(operation.result.iter().map(|result| &result.value_type));
            }
            _ => {}
        }
    }
    assert_eq!(types.len(), 3);
    for ty in &types {
        assert!(
            matches!(ty, ValueTypeRef::Resolved(value) if matches!(value.as_ref(), filament_exact::ValueType::Int(_))),
            "{ty:?}"
        );
    }
    assert_eq!(types[0], types[1]);
    assert_eq!(types[1], types[2]);
    let ValueTypeRef::Resolved(expected) = types[0] else {
        panic!("resolved Total");
    };
    let expected = expected.clone();
    let graph = finish(declarations).expect("model-owned source signatures check");
    assert_eq!(*expected, source_field_type(&graph, "Z", "total"));
}

fn override_causes(source: &str, model: Option<DomainPackage>) -> Vec<&'static str> {
    let declarations = match model {
        Some(model) => assemble_model(source, model),
        None => assemble(source),
    }
    .expect("override forms assemble");
    finish(declarations)
        .expect_err("the incompatible override is refused")
        .iter()
        .filter_map(|refusal| refusal.cause.catalog_code())
        .map(|code| code.cause())
        .collect()
}

#[trace("FR-569-AC-15", "TC-1940")]
#[test]
fn adverse_source_overrides_do_not_expand_the_closed_conformance_relation() {
    for (source, expected) in [
        ("object Base { operation f(p: Int[0, 9]): Integer; } object Child extends Base { operation f(p: Int[0, 3]): Integer redefines Base::f; }", "variance-parameter"),
        ("object Order {} object Base { operation f(): Reference<Order>; } object Child extends Base { operation f(): Option<Reference<Order>> redefines Base::f; }", "variance-result"),
        ("object Base { operation f(p: Decimal[0, 100; 2, 2; exact]): Boolean; } object Child extends Base { operation f(p: Decimal[0, 200; 2, 2; exact]): Boolean redefines Base::f; }", "variance-parameter"),
    ] {
        let causes = override_causes(source, None);
        assert!(causes.contains(&expected), "{expected}: {causes:?}");
    }
}

#[trace("FR-569-AC-15", "FSpec-FR-151-AC-2", "TC-1940")]
#[test]
fn model_object_reference_variance_recurses_through_options_and_collections() {
    let mut model = model_package(false);
    model.records.extend([
        orders_model::object("Node", &[], false),
        orders_model::object("Leaf", &["Node"], false),
    ]);
    for (base, child) in [
        ("Option<Reference<M::Node>>", "Option<Reference<M::Leaf>>"),
        (
            "Option<Set<Reference<M::Node>>[0, 4]>",
            "Option<Set<Reference<M::Leaf>>[1, 3]>",
        ),
    ] {
        let source = format!("object Base {{ operation f(): {base}; }} object Child extends Base {{ operation f(): {child} redefines Base::f; }}");
        finish(assemble_model(&source, model.clone()).expect("model reference overrides assemble"))
            .unwrap_or_else(|refusals| panic!("{base} to {child}: {refusals:?}"));
    }
    let source = "object Base { operation f(): Option<Reference<M::Leaf>>; } object Child extends Base { operation f(): Option<Reference<M::Node>> redefines Base::f; }";
    assert!(override_causes(source, Some(model)).contains(&"variance-result"));
}

/// The unit that selects fixture D as `M`, with `declarations`.
fn assemble_model(
    declarations: &str,
    package: DomainPackage,
) -> Result<PackageDeclarations, AssemblyRefusal> {
    let text = format!("{HEADER}model M = \"test/orders\";\n{declarations}\n");
    assemble_model_text(&text, package)
}

fn assemble_model_text(
    text: &str,
    package: DomainPackage,
) -> Result<PackageDeclarations, AssemblyRefusal> {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "tc-1408-d"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let selected = SelectedModels::fixture(
        "M",
        fsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        selected,
        Vec::new(),
    )
}

/// Fixture S's clauses written against the model: `on M::Order`.
const MODEL_CLAUSES: &str = "\
invariant I using P on M::Order at current { self.total >= 0 }
pre Q using P on M::Order::bump { self.total < 9 }
post R using P on M::Order::bump { self.total > pre(self.total) }
body B using P on M::Sub::size() { 7 }
body C using P on M::Order::size() { 1 }
pre Qs using P on M::Order::size { self.total >= 0 }
invariant Sized using P on M::Order at current { self.size() >= 0 }
";

const AMBIGUOUS: Refused = (Code::AmbiguousDeclaration, "ambiguous-name");
const MISSING: Refused = (Code::MissingDeclaration, "missing-name");
const MALFORMED: Refused = (Code::InvalidModelBinding, "malformed-declaration");

/// The text of `declarations` as the model unit `assemble_model` assembles.
fn model_text(declarations: &str) -> String {
    format!("{HEADER}model M = \"test/orders\";\n{declarations}\n")
}

/// The refusals of the model unit with `declarations`, as catalog codes with
/// the offset each is at in [`model_text`].
fn model_refusals_at(declarations: &str) -> Vec<(Refused, usize)> {
    assemble_model(declarations, model_package(false))
        .expect_err("the unit is refused")
        .errors
        .iter()
        .map(|error| {
            (
                (error.cause.code(), error.cause.catalog_code().cause()),
                error.span.start,
            )
        })
        .collect()
}

/// FR-569-AC-3: a source object type or population whose name another
/// declaration of the unit's namespace takes is refused at every declaration
/// of the name.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_name_two_declarations_share_is_refused_at_each() {
    // `record Order` beside `object Order`: both declarations.
    let text = format!("record Order {{ total: Integer; }}\n{OBJECTS}");
    assert_eq!(
        refusals_at(&text),
        [
            (AMBIGUOUS, offset(&text, "Order", 0, 0)),
            (AMBIGUOUS, offset(&text, "object Order", 0, 7)),
        ]
    );
    // Population `orders` beside a function `orders`: the population's name
    // and the function's declaration.
    let text = format!("{OBJECTS}function orders using P (): Integer pure {{ 1 }}\n");
    assert_eq!(
        refusals_at(&text),
        [
            (AMBIGUOUS, offset(&text, "population orders", 0, 11)),
            (AMBIGUOUS, offset(&text, "function orders", 0, 0)),
        ]
    );
}

/// `object M` in a unit with `model M`: both declarations.
#[trace("FR-569-AC-3", "TC-1408", "FSpec-FR-610-AC-2", "FSpec-TC-1440")]
#[test]
fn an_object_named_like_a_model_alias_is_refused_at_both() {
    let refused = model_refusals_at("object M { }");
    let text = model_text("object M { }");
    assert_eq!(
        refused,
        [
            (AMBIGUOUS, offset(&text, "model M", 0, 0)),
            (AMBIGUOUS, offset(&text, "object M", 0, 7)),
        ]
    );
}

/// A quantifier binder named like a source population is ambiguous at the
/// use, naming both.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_binder_named_like_a_population_is_ambiguous() {
    let declarations = assemble(&format!(
        "{OBJECTS}invariant Bad using P on Order at current {{ forall(orders in orders : true) }}\n"
    ))
    .expect("the unit assembles");
    let refusals = finish(declarations).expect_err("the binder is ambiguous");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::AmbiguousName { name, loci } = &refusal.cause else {
        panic!("an ambiguous name: {refusal:?}");
    };
    assert_eq!(name, "orders");
    assert_eq!(loci.len(), 2, "the use and the binder: {loci:?}");
    // A binder of another name over the same population checks.
    check(&format!(
        "{OBJECTS}invariant Good using P on Order at current {{ forall(o in orders : true) }}\n"
    ));
}

/// Members of one object type: two fields of a name (the second), a field
/// named like an inherited field (the field), an operation named like an
/// inherited one without `redefines` (the operation).
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_repeated_member_name_is_refused_at_the_later_member() {
    let text = format!("{OBJECTS}object Twin {{ total: Integer; total: Integer; }}\n");
    assert_eq!(
        refusals_at(&text),
        [(AMBIGUOUS, offset(&text, "total: Integer; total", 0, 16))]
    );
    let text = OBJECTS.replace(
        "object Sub extends Order {",
        "object Sub extends Order {\n  total: Integer;",
    );
    assert_eq!(
        refusals_at(&text),
        [(AMBIGUOUS, offset(&text, "total: Integer;", 0, 0))]
    );
    let text = OBJECTS.replace(
        "  operation size(): Integer redefines Order::size;",
        "  operation size(): Integer;",
    );
    assert_eq!(
        refusals_at(&text),
        [(
            AMBIGUOUS,
            offset(
                &text,
                "operation size(): Integer;\n  operation scaled(m",
                0,
                10
            )
        )]
    );
}

/// `deletes Order` in an operation whose parameter is also named `Order`.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_deletes_name_that_is_a_parameter_and_a_type_is_ambiguous() {
    let text = OBJECTS.replace(
        "  operation count(): Integer;",
        "  operation drop(Order: Reference<Order>) deletes Order;",
    );
    assert_eq!(
        refusals_at(&text),
        [(AMBIGUOUS, offset(&text, "deletes Order", 0, 8))]
    );
}

/// FR-569-AC-4: each declaration check refuses once, with its code and cause,
/// at the offending name.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_declaration_check_refuses_with_its_code_at_the_offending_name() {
    let cycle = (Code::InvalidPackage, "definition-cycle");
    // (declarations, the text the refusal is at, its code and cause)
    let cases: [(&str, &str, Refused); 8] = [
        (
            "object X extends Receipt { }\nrecord Receipt { total: Integer; }",
            "Receipt",
            MALFORMED,
        ),
        ("object X extends Ghost { }", "Ghost", MISSING),
        (
            "object X identity (ghost) { total: Integer; }",
            "ghost",
            MISSING,
        ),
        (
            "object X { total: Integer; operation f() modifies self.ghost; }",
            "ghost",
            MISSING,
        ),
        (
            "object X { operation f() creates Ghost; }",
            "Ghost",
            MISSING,
        ),
        (
            "object X { operation f(n: Integer) deletes n; }",
            "n;",
            MALFORMED,
        ),
        (
            "object X { operation f() redefines Order::reset; }",
            "Order::reset",
            MISSING,
        ),
        (
            "object A extends B { }\nobject B extends A { }",
            "B {",
            cycle,
        ),
    ];
    for (declarations, at, expected) in cases {
        let text = format!("{OBJECTS}{declarations}");
        // The needle's first occurrence inside the case's own declarations.
        let start = text.find(declarations).unwrap()
            + declarations
                .find(at)
                .unwrap_or_else(|| panic!("`{at}` in {declarations}"));
        let at_extends = if at == "B {" {
            // The cycle is reported at the first `extends` entry.
            text.find(declarations).unwrap() + declarations.find("B").unwrap()
        } else {
            start
        };
        assert_eq!(
            refusals_at(&text),
            [(expected, at_extends)],
            "{declarations}"
        );
    }
}

/// `extends M::Order` names a model type: `malformed-declaration` at the
/// entry.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn extending_a_model_type_is_refused_at_the_entry() {
    let declarations = "object X extends M::Order { }";
    let text = model_text(declarations);
    assert_eq!(
        model_refusals_at(declarations),
        [(MALFORMED, offset(&text, "M::Order", 0, 0))]
    );
}

/// A redefinition whose parameter type is not contravariant refuses
/// `variance-parameter` once, naming the redefining member.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_redefinition_with_a_narrower_parameter_refuses_variance_parameter() {
    let text = OBJECTS.replace(
        "operation scaled(m: Integer): Integer redefines",
        "operation scaled(m: Text[0, 8; nfc]): Integer redefines",
    );
    let declarations = assemble(&format!("{text}{CLAUSES}")).expect("the unit assembles");
    let refusals = finish(declarations).expect_err("the parameter is not contravariant");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::ModelConformance(failure) = &refusal.cause else {
        panic!("a conformance failure: {refusal:?}");
    };
    assert!(failure.member.node.ends_with("/Sub/scaled"), "{failure:?}");
    assert!(
        failure.parent.node.ends_with("/Order/scaled"),
        "{failure:?}"
    );
    assert!(
        matches!(&failure.cause, ModelRefusalCause::VarianceParameter { .. }),
        "{failure:?}"
    );
}

/// `redefines Order::reset`: `Order` is a proper supertype that exposes no
/// `reset`, so the refusal is at the member.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_redefinition_of_an_operation_the_supertype_lacks_is_refused_at_the_operation() {
    let text = OBJECTS.replace(
        "operation size(): Integer redefines Order::size;",
        "operation size(): Integer redefines Order::reset;",
    );
    assert_eq!(
        refusals_at(&text),
        [(MISSING, offset(&text, "Order::reset", 0, 7))]
    );
}

/// `A extends B` with `B extends A` is one `definition-cycle` naming both
/// edges.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_supertype_cycle_is_one_definition_cycle_naming_both_edges() {
    let refusal = assemble("object A extends B { }\nobject B extends A { }")
        .expect_err("the cycle is refused");
    let [error] = &refusal.errors[..] else {
        panic!("one refusal: {refusal:?}");
    };
    let AssemblyCause::SourceObject(SourceObjectFault::SupertypeCycle { edges }) = &error.cause
    else {
        panic!("a cycle: {error:?}");
    };
    assert_eq!(
        edges,
        &[
            ("A".to_owned(), "B".to_owned()),
            ("B".to_owned(), "A".to_owned())
        ]
    );
    assert_eq!(
        (error.cause.code(), error.cause.catalog_code().cause()),
        (Code::InvalidPackage, "definition-cycle")
    );
}

/// Every checked node type under `node`, with the package identity written
/// `P`, so that the source and model routes compare equal.
fn type_names(node: fsl_semantics::check::CheckedNode<'_>, out: &mut Vec<String>) {
    out.push(
        format!("{:?}", node.value_type())
            .replace("source/fsl-semantics-integration-tests", "P")
            .replace("test/orders", "P"),
    );
    for child in node.children() {
        type_names(child, out);
    }
}

/// The checked types of the clause `name`'s expression.
fn clause_types(graph: &CheckedGraph, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    type_names(
        graph.state_clause(name).expect("the clause").body(),
        &mut out,
    );
    out
}

/// The checked types of the function `name`'s body.
fn function_types(graph: &CheckedGraph, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let body = (0..)
        .map_while(|index| graph.function_state(index))
        .find(|function| function.name == name)
        .expect("the function")
        .body;
    type_names(body, &mut out);
    out
}

/// The receiver type names of a dispatch table's entries.
fn tables_receivers(table: &fsl_semantics::check::DispatchTable) -> Vec<String> {
    table
        .entries()
        .iter()
        .map(|(receiver, _)| {
            format!("{receiver:?}")
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches("\" }])")
                .to_owned()
        })
        .collect()
}

/// A checked expression's node kinds, package identities written `P`.
fn shape_of(node: fsl_semantics::check::CheckedNode<'_>, out: &mut String) {
    out.push_str(
        &format!("{:?};", node.kind())
            .replace("source/fsl-semantics-integration-tests", "P")
            .replace("test/orders", "P"),
    );
    for child in node.children() {
        shape_of(child, out);
    }
}

/// One dispatch entry without its receiver's package identity: the receiver
/// type's name, body, precondition and precondition clauses.
type CandidateShape = (String, usize, Option<usize>, Vec<usize>);

/// The dispatch tables' candidates, by receiver type name.
fn tables(graph: &CheckedGraph) -> Vec<Vec<CandidateShape>> {
    graph
        .dispatch_tables()
        .iter()
        .map(|table| {
            table
                .entries()
                .iter()
                .map(|(receiver, candidate)| {
                    let receiver = format!("{receiver:?}");
                    let name = receiver
                        .rsplit("/")
                        .next()
                        .unwrap_or_default()
                        .trim_end_matches("\" }])")
                        .to_owned();
                    (
                        name,
                        candidate.body,
                        candidate.precondition,
                        candidate.precondition_clauses.clone(),
                    )
                })
                .collect()
        })
        .collect()
}

/// The source `Sub`-typed and `Order`-typed expressions of AC-5, with the
/// names `object` qualifies by `qualifier`.
fn ac5_extras(qualifier: &str) -> String {
    format!(
        "bound B on {q}orders [3]\n\
         invariant Cap using P on {q}Order at current {{ count(o in {q}orders where o.total = 0) < 4 }}\n\
         invariant Every using P on {q}Order at current {{ forall(o in allInstances<{q}Order>({q}orders) : deref(o).total >= 0) }}\n\
         invariant SizedSub using P on {q}Sub at current {{ self.size() >= 0 }}\n\
         function totalOf using P (s: Reference<{q}Sub>): Int[0, 9] pure {{ deref(s).total }}\n",
        q = qualifier
    )
}

/// FR-569-AC-5 (S-R): a clause on `Sub::size` joins `Sub.size`'s effective
/// precondition as `Qs2 or Qs` with the own clause first, and S and D link
/// the same dispatch tables and check the same types.
#[trace("FR-569-AC-5", "TC-1408")]
#[test]
fn the_source_and_model_twins_link_equal_tables_and_check_equal_types() {
    let source = check(&format!(
        "{OBJECTS}{CLAUSES}pre Qs2 using P on Sub::size {{ self.total >= 1 }}\n{}",
        ac5_extras("")
    ));
    let model = finish(
        assemble_model(
            &format!(
                "{MODEL_CLAUSES}pre Qs2 using P on M::Sub::size {{ self.total >= 1 }}\n{}",
                ac5_extras("M::")
            ),
            model_package(false),
        )
        .expect("the model twin assembles"),
    )
    .expect("the model twin checks");

    // Dispatch tables: equal, with `Sub.size` carrying its own clause first.
    let (source_tables, model_tables) = (tables(&source), tables(&model));
    assert_eq!(source_tables, model_tables);
    assert!(source_tables.iter().any(|table| {
        table
            .iter()
            .all(|entry| entry.0 == "Order" || entry.0 == "Sub")
    }));
    let precondition_of = |graph: &CheckedGraph, name: &str| -> String {
        let table = graph
            .dispatch_tables()
            .iter()
            .find(|table| tables_receivers(table).iter().any(|r| r == "Sub"))
            .expect("the size table");
        let (_, candidate) = table
            .entries()
            .iter()
            .find(|(receiver, _)| format!("{receiver:?}").contains(&format!("/{name}\"")))
            .unwrap_or_else(|| panic!("{name} has an entry"));
        let function = graph
            .function_state(candidate.precondition.expect("an effective precondition"))
            .expect("the precondition function");
        let mut shape = String::new();
        shape_of(function.body, &mut shape);
        shape
    };
    // `Sub.size`'s effective precondition is `Qs2 or Qs`, the own clause
    // first (`>= 1` is Qs2, `>= 0` is Qs); `Order.size`'s is `Qs` alone.
    let sub = precondition_of(&source, "Sub");
    assert!(sub.starts_with("Connective(Or"), "{sub}");
    assert!(
        sub.find("Integer(1)").unwrap() < sub.find("Integer(0)").unwrap(),
        "Qs2 first: {sub}"
    );
    assert!(!precondition_of(&source, "Order").contains("Connective"));
    assert_eq!(sub, precondition_of(&model, "Sub"));
    assert_eq!(
        precondition_of(&source, "Order"),
        precondition_of(&model, "Order")
    );

    // Checked types: equal on the two routes, and the expected ones.
    for clause in ["Cap", "Every", "SizedSub"] {
        assert_eq!(clause_types(&source, clause), clause_types(&model, clause));
    }
    assert_eq!(
        function_types(&source, "totalOf"),
        function_types(&model, "totalOf")
    );
    let cap = clause_types(&source, "Cap");
    assert!(
        cap.contains(&"Int(IntegerInterval { lower: Integer(0), upper: Integer(3) })".to_owned())
    );
    assert!(cap.iter().any(|t| t.contains("kind: Set")
        && t.contains("bound: Some(CardinalityBound { minimum: 0, maximum: 3 })")
        && t.contains("ix://P/Order")));
    assert!(cap.contains(&"Population(Some(3))".to_owned()));
    let every = clause_types(&source, "Every");
    assert!(every
        .iter()
        .any(|t| t.contains("kind: Set") && t.contains("ix://P/Order")));
    let total_of = function_types(&source, "totalOf");
    assert_eq!(
        total_of[0], "Int(IntegerInterval { lower: Integer(0), upper: Integer(9) })",
        "deref(s).total is the field's Int[0, 9]"
    );
    assert!(total_of[1].contains("ix://P/Sub"), "s is a Reference<Sub>");
}

/// FR-569-AC-5 (S-R'): `Sub` redefines `bump` and also modifies `note`, which
/// `Order.bump` does not; S and D each refuse one `effect-escape` on the
/// `note` field of the redefining member.
#[trace("FR-569-AC-5", "TC-1408")]
#[test]
fn a_redefinition_that_widens_the_frame_refuses_effect_escape_on_both_routes() {
    let text = OBJECTS.replace(
        "operation scaled(m: Integer): Integer redefines Order::scaled;",
        "operation scaled(m: Integer): Integer redefines Order::scaled;\n  \
         operation bump(): Boolean redefines Order::bump modifies self.total, self.note;",
    );
    let source = finish(assemble(&format!("{text}{CLAUSES}")).expect("S-R' assembles"))
        .expect_err("the frame escapes");
    let model = finish(assemble_model(MODEL_CLAUSES, model_package(true)).expect("D-R' assembles"))
        .expect_err("the frame escapes");
    for (route, refusals) in [("S-R'", source), ("D-R'", model)] {
        let [refusal] = &refusals[..] else {
            panic!("{route}: one refusal: {refusals:?}");
        };
        let CheckCause::ModelConformance(failure) = &refusal.cause else {
            panic!("{route}: a conformance failure: {refusal:?}");
        };
        assert_eq!(
            (failure.code, failure.axis),
            (Code::IllTyped, "effect"),
            "{route}"
        );
        assert!(
            failure.member.node.ends_with("/Sub/bump"),
            "{route}: {failure:?}"
        );
        assert!(
            failure.parent.node.ends_with("/Order/bump"),
            "{route}: {failure:?}"
        );
        assert!(
            matches!(&failure.cause, ModelRefusalCause::EffectEscape { field } if field.node.ends_with("/Order/note")),
            "{route}: {failure:?}"
        );
    }
}

/// FR-569-AC-2: `on Order` and `on M::Order` check against different field
/// sets when the source `Order` declares a field the model's lacks.
#[trace("FR-569-AC-2", "TC-1408", "FSpec-FR-610-AC-2", "FSpec-TC-1440")]
#[test]
fn a_bare_and_a_qualified_context_see_different_field_sets() {
    let objects = OBJECTS.replace(
        "  total: Int[0, 9];",
        "  total: Int[0, 9];\n  extra: Integer;",
    );
    // `extra` is a field of the source `Order` only.
    let both = |clause: &str| {
        finish(
            assemble_model(&format!("{objects}{clause}"), model_package(false))
                .expect("the unit assembles"),
        )
    };
    both("invariant OnSource using P on Order at current { self.extra >= 0 }\n")
        .expect("`on Order` sees `extra`");
    let refusals = both("invariant OnModel using P on M::Order at current { self.extra >= 0 }\n")
        .expect_err("`on M::Order` does not");
    assert!(
        matches!(&refusals[..], [refusal] if matches!(&refusal.cause, CheckCause::StateModel(_) | CheckCause::MissingName(_))),
        "{refusals:?}"
    );
    // The model's `note` is seen by both.
    both(
        "invariant A using P on Order at current { present(self.note) or true }\n\
         invariant B using P on M::Order at current { present(self.note) or true }\n",
    )
    .expect("both see `note`");
}

/// A protocol `Sell` over `over_type`, with `w` the role on the whole model
/// `M` and `Seller` a static role on the source `Order`; its `finish` reads
/// `view.extra`, a field of the source `Order` only.
fn sell(over_type: &str) -> String {
    format!(
        "protocol Sell using Pp over (view: {over_type}) on origin {{\n\
           role w on M;\n\
           role Seller on Order;\n\
           run sequence Main {{\n\
             attempt A by Seller on Order::bump contracts [] as (b: Boolean) {{ true }};\n\
           }}\n\
           finish Done as (d: Boolean) {{ view.extra >= 0 }};\n\
         }}\n"
    )
}

/// FR-218-AC-39 (S-M): `w` is the role on the whole model `M` and `Seller` a
/// static role on the source `Order`. `over (view: Order)` has the source
/// `Order` as its model context, so it reads the source-only field `extra`;
/// `over (view: M::Order)` has the model's, which has none.
#[trace(
    "FR-218-AC-39",
    "FR-569-AC-2",
    "TC-1408",
    "FSpec-FR-610-AC-2",
    "FSpec-TC-1440"
)]
#[test]
fn a_protocol_roles_and_over_resolve_to_the_source_or_the_model_by_spelling() {
    let objects = OBJECTS.replace(
        "  total: Int[0, 9];",
        "  total: Int[0, 9];\n  extra: Integer;",
    );
    let on_source = finish(
        assemble_model(&format!("{objects}{}", sell("Order")), model_package(false))
            .expect("the S-M unit assembles"),
    )
    .unwrap_or_else(|refusals| panic!("over Order: {refusals:?}"));
    let [protocol] = on_source.protocols() else {
        panic!("one protocol");
    };
    let role = |name: &str| {
        protocol
            .roles
            .iter()
            .find(|role| role.name == name)
            .unwrap_or_else(|| panic!("role {name}"))
    };
    // `w` is on the whole model `M`: it names the model's package and no
    // object type. `Seller` is on the source `Order`: an object type of the
    // source package.
    assert_eq!(role("w").model_selection.as_deref(), Some("test/orders"));
    assert_eq!(role("w").context, None);
    assert!(role("Seller").context.is_some());
    assert_eq!(role("Seller").model_selection, None);

    // The same protocol over the model's `Order` cannot read `extra`.
    let refusals = finish(
        assemble_model(
            &format!("{objects}{}", sell("M::Order")),
            model_package(false),
        )
        .expect("the S-M unit assembles"),
    )
    .map(|_| ())
    .expect_err("M::Order has no `extra`");
    assert!(!refusals.is_empty());
    // And over the model's `Order` without reading `extra`, it checks: the
    // two contexts differ only in what they see.
    let without = sell("M::Order").replace("view.extra >= 0", "view.total >= 0");
    finish(
        assemble_model(&format!("{objects}{without}"), model_package(false))
            .expect("the S-M unit assembles"),
    )
    .unwrap_or_else(|refusals| panic!("over M::Order: {refusals:?}"));
}

/// Source objects are unit-scoped: a clause in another unit of the package
/// that names a source type refuses `missing-name`, whichever unit holds the
/// declaration.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_source_type_is_visible_only_in_the_unit_that_declares_it() {
    let header = "profile P = \"filament.model.complete/v1\";\n";
    let objects = format!("{header}{OBJECTS}");
    let clause = format!("{header}invariant X using P on Order at current {{ self.total >= 0 }}\n");
    // The declaring unit's own clause checks.
    let alone = crate::model_operations::try_assemble_units(
        &[(
            "a",
            &format!(
                "{objects}{clause_body}",
                clause_body = "invariant X using P on Order at current { self.total >= 0 }\n"
            ),
        )],
        &[],
    );
    assert!(alone.is_ok(), "{:?}", alone.err());
    // A clause in another unit, whichever order the units sort in.
    for units in [
        [("a", &objects), ("b", &clause)],
        [("a", &clause), ("b", &objects)],
    ] {
        let refusal = crate::model_operations::try_assemble_units(
            &[
                (units[0].0, units[0].1.as_str()),
                (units[1].0, units[1].1.as_str()),
            ],
            &[],
        )
        .expect_err("the other unit does not see the type");
        let [error] = &refusal.errors[..] else {
            panic!("one refusal: {refusal:?}");
        };
        assert!(
            matches!(&error.cause, AssemblyCause::UnresolvedTypeName { name } if name == "Order"),
            "{error:?}"
        );
        assert_eq!(
            (error.cause.code(), error.cause.catalog_code().cause()),
            MISSING
        );
    }
}

/// The profile selections both units of the two-unit tests write.
const UNIT_PROFILES: &str = "profile P = \"filament.model.complete/v1\";\n\
    profile Pp = \"filament.protocol.finite-global/v1\";\n";

/// A protocol `Sell` over `over_type` with the role `Seller` on `role_type`,
/// acting through `operation`.
fn unit_protocol(over_type: &str, role_type: &str, operation: &str) -> String {
    format!(
        "protocol Sell using Pp over (view: {over_type}) on origin {{\n\
           role Seller on {role_type};\n\
           run sequence Main {{\n\
             attempt A by Seller on {operation} contracts [] as (b: Boolean) {{ true }};\n\
           }}\n\
           finish Done as (d: Boolean) {{ true }};\n\
         }}\n"
    )
}

/// A source population is named in the unit that declares it only: another
/// unit's clause naming it refuses `missing-name`.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_source_population_is_visible_only_in_the_unit_that_declares_it() {
    let declaring = format!("{UNIT_PROFILES}{OBJECTS}");
    let other = format!(
        "{UNIT_PROFILES}object Other {{ }}\n\
         invariant X using P on Other at current {{ forall(o in orders : true) }}\n"
    );
    let units = [("a", declaring.as_str()), ("b", other.as_str())];
    let declarations =
        crate::model_operations::try_assemble_units(&units, &[]).expect("the units assemble");
    let refusals = finish(declarations).expect_err("`orders` is not visible in unit b");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    // It is refused as a name nothing declares is.
    assert!(
        matches!(&refusal.cause, CheckCause::MissingName(name) if name == "orders"),
        "{refusal:?}"
    );

    // The declaring unit's own clause over the population checks.
    let own = format!(
        "{declaring}invariant Y using P on Order at current {{ forall(o in orders : true) }}\n"
    );
    let stub = format!("{UNIT_PROFILES}object Stub {{ }}\n");
    let units = [("a", own.as_str()), ("b", stub.as_str())];
    finish(crate::model_operations::try_assemble_units(&units, &[]).expect("assembles"))
        .expect("the declaring unit sees its population");
}

const UNIT_TEMPORAL_PROFILE: &str = "profile t = \"filament.temporal.infinite-trace/v1\";\n";

/// A temporal clause over `over_type` whose body names the population `orders`.
fn unit_temporal(over_type: &str) -> String {
    format!(
        "temporal C using t over (w: {over_type}) clock \"model-steps\" \
         on origin {{ eventually holds(forall(x in orders : true)) }}\n"
    )
}

/// A temporal clause names a source population in the unit that declares it
/// only: another unit's clause naming it refuses `missing-name`, and the
/// declaring unit's own clause checks.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_temporal_clause_sees_only_its_own_units_populations() {
    let declaring = format!("{UNIT_PROFILES}{OBJECTS}");
    let other = format!(
        "{UNIT_PROFILES}{UNIT_TEMPORAL_PROFILE}object Watch {{ }}\n{}",
        unit_temporal("Watch")
    );
    let units = [("a", declaring.as_str()), ("b", other.as_str())];
    let declarations =
        crate::model_operations::try_assemble_units(&units, &[]).expect("the units assemble");
    let refusals = finish(declarations).expect_err("`orders` is not visible in unit b");
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::MissingName(name) if name == "orders"
        )),
        "{refusals:?}"
    );
    // The declaring unit's own temporal clause over the population checks.
    let own = format!(
        "{UNIT_PROFILES}{UNIT_TEMPORAL_PROFILE}{OBJECTS}{}",
        unit_temporal("Order")
    );
    let stub = format!("{UNIT_PROFILES}object Stub {{ }}\n");
    let units = [("a", own.as_str()), ("b", stub.as_str())];
    finish(crate::model_operations::try_assemble_units(&units, &[]).expect("assembles"))
        .expect("the declaring unit sees its population in a temporal clause");
}

/// A role and an `over` of another unit cannot name a unit's source type;
/// a secondary unit's own clause and protocol resolve the type it declares.
#[trace("FR-569-AC-2", "FR-218-AC-39", "TC-1408")]
#[test]
fn a_role_and_an_over_see_only_their_own_units_source_types() {
    let local = "object Local { operation go(): Boolean; }\n";
    let declaring = format!("{UNIT_PROFILES}{OBJECTS}");
    for (variant, protocol) in [
        ("over", unit_protocol("Order", "Local", "Local::go")),
        ("role", unit_protocol("Local", "Order", "Local::go")),
    ] {
        let user = format!("{UNIT_PROFILES}{local}{protocol}");
        // `a` uses the type `b` declares, whichever way they sort.
        let assembled = crate::model_operations::try_assemble_units(
            &[("a", user.as_str()), ("b", declaring.as_str())],
            &[],
        );
        // The unit that does not declare `Order` cannot name it, whether
        // the refusal is the assembler's or the check's.
        match assembled {
            Err(refusal) => {
                assert!(!refusal.errors.is_empty(), "{variant}");
                for error in &refusal.errors {
                    assert!(
                        matches!(&error.cause, AssemblyCause::UnresolvedTypeName { name } if name == "Order"),
                        "{variant}: {error:?}"
                    );
                    assert_eq!(
                        (error.cause.code(), error.cause.catalog_code().cause()),
                        MISSING,
                        "{variant}"
                    );
                }
            }
            Ok(declarations) => {
                let refusals = finish(declarations).expect_err(variant);
                assert!(
                    refusals.iter().all(|refusal| match &refusal.cause {
                        CheckCause::MissingName(name) => name == "Order",
                        CheckCause::ProtocolAnchor(anchor) => {
                            {
                                let text = format!("{anchor:?}");
                                text.contains("MissingName(\"Order\")")
                                    || text.contains(
                                        "MissingRoleType { role: \"Seller\", context: \"Order\"",
                                    )
                            }
                        }
                        _ => false,
                    }),
                    "{variant}: {refusals:?}"
                );
            }
        }
    }

    // The secondary unit `b` declares `Order` and uses it in a clause and a
    // protocol of its own.
    let unit_stub = format!("{UNIT_PROFILES}object Stub {{ }}\n");
    let own = format!(
        "{declaring}invariant Y using P on Order at current {{ self.total >= 0 }}\n{}",
        unit_protocol("Order", "Order", "Order::bump")
    );
    let graph = finish(
        crate::model_operations::try_assemble_units(
            &[("a", unit_stub.as_str()), ("b", own.as_str())],
            &[],
        )
        .expect("the secondary unit assembles"),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert!(graph.state_clause("Y").is_some());
    assert_eq!(graph.protocols().len(), 1);
}

/// Fixture D with the record value type `Flag { ready: Boolean }` beside its
/// own declarations.
fn model_with_flag() -> DomainPackage {
    use fsl_semantics::model::domain_package::{
        DomainPackageRecord, NativeValueType, RecordValueTypeRecord, ValueTypeRef,
    };
    use orders_model::{field, key};
    let mut package = model_package(false);
    package.records.push(DomainPackageRecord::RecordValueType(
        RecordValueTypeRecord {
            key: key("Flag"),
            identity_fields: Vec::new(),
            supertypes: Vec::new(),
        },
    ));
    package.records.push(field(
        "Flag",
        "ready",
        ValueTypeRef::Native(NativeValueType::Boolean),
        filament_exact::Presence::Required,
        0,
    ));
    package
}

/// Fixture D with the payload-free model variant type `Tier`.
fn model_with_tier() -> DomainPackage {
    use fsl_semantics::model::domain_package::{DomainPackageRecord, EnumRecord, VariantRecord};
    use orders_model::key;
    let mut package = model_package(false);
    package.records.extend([
        DomainPackageRecord::Enum(EnumRecord {
            key: key("Tier"),
            name: "Tier".to_owned(),
            members: vec!["Gold".to_owned(), "Standard".to_owned()],
        }),
        DomainPackageRecord::Variant(VariantRecord {
            key: key("Tier/Gold"),
            owner: key("Tier"),
            case: "Gold".to_owned(),
            payload_type: None,
        }),
        DomainPackageRecord::Variant(VariantRecord {
            key: key("Tier/Standard"),
            owner: key("Tier"),
            case: "Standard".to_owned(),
            payload_type: None,
        }),
    ]);
    package
}

/// FR-569 "Declaration checks": a field, parameter or result of a source
/// object type may name a model-owned value type, a reference to a model
/// object type, or a collection of either; each checks, and a clause over the
/// type reads the model-typed field.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_source_member_may_have_a_model_owned_type() {
    let declarations = "\
object Z {
  total: M::Total;
  flag: M::Flag?;
  link: Reference<M::Order>;
  totals: Set<M::Total>[0, 3];
  population zs;
  operation step(): M::Flag;
  operation add(x: M::Total, o: Reference<M::Order>): M::Total;
}

invariant X using P on Z at current { self.total >= 0 }
invariant Y using P on Z at current { forall(z in zs : deref(z).total <= 9) }
";
    let declarations =
        assemble_model(declarations, model_with_flag()).expect("the member types resolve");
    finish(declarations).expect("the clauses over the model-typed fields check");
}

/// A model-owned enum type admitted as a source record field is admitted as
/// a source object field and operation signature too (FR-569-AC-12).
#[trace("FR-569-AC-12", "TC-1940")]
#[test]
fn a_source_object_member_may_have_a_model_owned_variant_type() {
    let source = "\
object O { f: M::Tier; operation echo(p: M::Tier): M::Tier; }\n\
function read(o: Reference<O>): M::Tier pure { deref(o).f }\n\
pre Input using P on O::echo { true }\n\
post Output using P on O::echo { true }\n";
    let declarations = assemble_model(source, model_with_tier())
        .expect("the model-owned variant resolves through M");
    let graph = finish(declarations).expect("the member and operation types check");
    let input = graph
        .state_clause("Input")
        .expect("input signature is checked");
    let field_type = source_field_type(&graph, "O", "f");
    assert_eq!(input.parameters()[1].1, field_type);
    let output = graph
        .state_clause("Output")
        .expect("output signature is checked");
    assert_eq!(output.parameters()[1].1, field_type);
}

/// A model type that does not exist, and a model-owned type a field cannot
/// hold, refuse at the type.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_model_typed_member_that_names_nothing_is_refused_at_the_name() {
    let text = "object Z { total: M::Ghost; }\n";
    let refused = assemble_model(text, model_package(false)).expect_err("M::Ghost names nothing");
    let [error] = &refused.errors[..] else {
        panic!("one refusal: {refused:?}");
    };
    assert_eq!(
        (error.cause.code(), error.cause.catalog_code().cause()),
        MISSING
    );
    let source = model_text(text);
    assert_eq!(&source[error.span.start..error.span.end], "M::Ghost");
}

/// FR-105-AC-18 (the source route's part): `pre Q` and `post R` on
/// `Order::bump` check against one operation whose declaring type is the
/// source-declared `Order`, and a clause on `Sub::bump` resolves to `Order`'s
/// `bump`, which `Sub` inherits.
#[trace("FR-105-AC-18", "TC-1408")]
#[test]
fn clauses_on_a_source_operation_share_the_declaring_type() {
    let graph = check(&format!(
        "{OBJECTS}{CLAUSES}pre Qsub using P on Sub::bump {{ self.total < 9 }}\n"
    ));
    let order = graph.state_clause("I").expect("I is checked").context();
    let operation = |name: &str| {
        graph
            .state_clause(name)
            .and_then(|clause| clause.operation())
            .unwrap_or_else(|| panic!("{name} names an operation"))
    };
    let (q, r, qsub) = (operation("Q"), operation("R"), operation("Qsub"));
    for (name, clause) in [("Q", q), ("R", r), ("Qsub", qsub)] {
        assert_eq!(
            clause.declaring, order,
            "{name}'s operation is declared by the source-declared Order"
        );
    }
    assert_eq!(q.declaration, r.declaration, "Q and R share one operation");
    assert_eq!(
        q.declaration, qsub.declaration,
        "a clause on Sub::bump resolves to Order's bump"
    );
    assert_eq!(q.declaration.name(), "bump");
}

/// FR-569 "Declaration checks": a member may have any type a record field may
/// have, so an unbounded collection and a bounded one both assemble.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn an_unbounded_and_a_bounded_collection_member_both_assemble() {
    let graph = check(
        "\
object Z {
  tags: Set<Int[0, 9]>;
  loose: Sequence<Int[0, 9]>;
  few: Set<Int[0, 9]>[0, 3];
  population zs;
}
invariant X using P on Z at current { forall(z in zs : true) }
",
    );
    let package = graph
        .source_packages()
        .first()
        .expect("the unit's objects form a source package");
    let multiplicity = |name: &str| {
        package
            .records
            .iter()
            .find_map(|record| match record {
                fsl_semantics::model::domain_package::DomainPackageRecord::FieldMember(field)
                    if field.key.node.ends_with(&format!("/Z/{name}")) =>
                {
                    Some((field.multiplicity.lower, field.multiplicity.upper))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("member {name}"))
    };
    assert_eq!(multiplicity("tags"), (0, None));
    assert_eq!(multiplicity("loose"), (0, None));
    assert_eq!(multiplicity("few"), (0, Some(3)));
}

/// Two units of one package, `a` and `b`, each parsed, with `models` the
/// admitted selections of both.
fn assemble_two_units(
    a: &str,
    b: &str,
    models: Vec<fsl_semantics::model::intake::SelectedModel>,
) -> Result<PackageDeclarations, AssemblyRefusal> {
    use fsl_semantics::check::AssemblyLimits;
    let units: Vec<_> = [("a", a), ("b", b)]
        .iter()
        .map(|(identity, text)| {
            let source = format!("{HEADER}{text}\n");
            let parsed = fsl_cst::parse(
                fsl_foundation::SourceIdentity::new("test", *identity),
                "unit.native",
                source.as_bytes(),
                fsl_cst::Limits::default(),
            )
            .expect("S1 reads the unit");
            assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
            let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
            (parsed.source().reference().clone(), unit)
        })
        .collect();
    PackageDeclarations::assemble_units(
        crate::TEST_PACKAGE_NAME.to_owned(),
        units,
        SelectedModels::fixture_entries(models),
        Vec::new(),
        AssemblyLimits::default(),
        &filament_exact::Cancel::new(),
    )
}

/// The admitted selection of `package` as alias `M`.
fn selected_as_m(package: DomainPackage) -> fsl_semantics::model::intake::SelectedModel {
    let selected = SelectedModels::fixture(
        "M",
        fsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    selected[0].clone()
}

/// A second domain package, `test/other`, whose `Total` is a different
/// declaration from fixture D's.
fn other_package() -> DomainPackage {
    use fsl_semantics::model::domain_package::{
        DomainPackageRecord, DomainPackageRef, ScalarTypeRecord,
    };
    DomainPackage::new(
        DomainPackageRef {
            identity: "test/other".to_owned(),
        },
        vec![DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: filament_checked::key::DeclarationKey {
                package: "test/other".to_owned(),
                node: "ix://test/other/Total".to_owned(),
            },
            domain: fsl_semantics::model::domain_package::ScalarValueDomain::Integer {
                lower: 0,
                upper: 99,
            },
        })],
    )
}

/// FR-569-AC-15 retains a model-owned source member as its resolved checked type.
fn retained_member_type(
    graph: &CheckedGraph,
    owner: &str,
    name: &str,
) -> filament_exact::ValueType {
    use fsl_semantics::model::domain_package::{DomainPackageRecord, ValueTypeRef};
    graph
        .source_packages()
        .iter()
        .flat_map(|package| &package.records)
        .find_map(|record| match record {
            DomainPackageRecord::FieldMember(field)
                if field.key.node.ends_with(&format!("/{owner}/{name}")) =>
            {
                match &field.value_type {
                    ValueTypeRef::Resolved(value_type) => Some(value_type.as_ref().clone()),
                    other => panic!("{owner}.{name} is not resolved: {other:?}"),
                }
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("member {owner}.{name}"))
}

/// A model type a source member names resolves through the declaring unit's
/// own `model` aliases: unit `b` selects no `M`, so its `M::Total` names
/// nothing, although unit `a` selects one.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_model_type_resolves_only_through_the_declaring_units_own_aliases() {
    let selecting = "model M = \"test/orders\";\nobject A { t: M::Total; }\n";
    let bare = "object B { t: M::Total; }\n";
    let models = vec![selected_as_m(model_package(false))];
    // `a` alone: the member resolves.
    assemble_two_units(selecting, "object Plain { }\n", models.clone())
        .expect("a's own alias resolves");
    // `b` names `M::Total` without selecting `M`, whichever way they sort.
    for (a, b) in [(selecting, bare), (bare, selecting)] {
        let refused = assemble_two_units(a, b, models.clone())
            .expect_err("the unit that selects no M cannot name M::Total");
        let [error] = &refused.errors[..] else {
            panic!("one refusal: {refused:?}");
        };
        assert_eq!(
            (error.cause.code(), error.cause.catalog_code().cause()),
            MISSING,
            "{error:?}"
        );
    }
}

/// Two units that select different packages as `M` each bind their own.
#[trace("FR-569-AC-2", "TC-1408", "FR-569-AC-15", "TC-1940")]
#[test]
fn two_units_selecting_different_packages_as_m_each_bind_their_own() {
    let a = "model M = \"test/orders\";\nobject A { t: M::Total; }\n";
    let b = "model M = \"test/other\";\nobject B { t: M::Total; }\n";
    let models = vec![
        selected_as_m(model_package(false)),
        selected_as_m(other_package()),
    ];
    for (first, second) in [(a, b), (b, a)] {
        let declarations = assemble_two_units(first, second, models.clone())
            .expect("each unit's M::Total resolves");
        let graph = finish(declarations).expect("the units check");
        for (owner, upper) in [("A", 9_i64), ("B", 99_i64)] {
            let expected = filament_exact::ValueType::Int(
                filament_exact::IntegerInterval::new(
                    filament_exact::Integer::from(0_i64),
                    filament_exact::Integer::from(upper),
                )
                .expect("interval"),
            );
            assert_eq!(retained_member_type(&graph, owner, "t"), expected);
            assert_eq!(source_field_type(&graph, owner, "t"), expected);
        }
    }
}

// FR-569 "Member types": a source object member admits every type a source
// record field admits, with the record field's checks and refusals.

/// The declarations a member-type case needs before its record and object.
const MEMBER_PRELUDE: &str = "\
enum Tier { Gold, Standard }
record Point { x: Integer; }
union Mark { Circle(Integer), Blank }
tuple Pair(Integer, Boolean);
type Small = Int[0, 3];
dimension L;
unit m : L = rational(1, 1);
";

/// The catalog codes of every refusal of `declarations`, from assembly or,
/// when it assembles, from the check; empty when the unit checks.
fn refusal_codes(declarations: &str) -> Vec<(Code, Option<&'static str>)> {
    match assemble(declarations) {
        Err(refusal) => refusal
            .errors
            .iter()
            .map(|error| (error.cause.code(), Some(error.cause.catalog_code().cause())))
            .collect(),
        Ok(assembled) => match finish(assembled) {
            Ok(_) => Vec::new(),
            Err(refusals) => refusals
                .iter()
                .map(|refusal| {
                    (
                        refusal.cause.code(),
                        refusal.cause.catalog_code().map(|code| code.cause()),
                    )
                })
                .collect(),
        },
    }
}

fn source_field_type(
    graph: &CheckedGraph,
    object_name: &str,
    field_name: &str,
) -> filament_exact::ValueType {
    graph
        .scope()
        .types()
        .object_types()
        .find(|object| object.name() == object_name)
        .and_then(|object| {
            object
                .attributes()
                .iter()
                .find(|field| field.name() == field_name)
        })
        .map(|field| field.value_type().clone())
        .unwrap_or_else(|| panic!("checked field {object_name}.{field_name}"))
}

/// A record field of type `ty` checks, and so does an object field of `ty`
/// read by a state clause; with `operation`, so does an operation taking and
/// returning `ty`.
fn assert_member_checks_like_record_field(ty: &str, operation: bool) {
    let record = format!("{MEMBER_PRELUDE}record R {{ f: {ty}; }}\n");
    assert_eq!(
        refusal_codes(&record),
        [],
        "the record field of {ty} checks"
    );
    let operation = if operation {
        format!("  operation g(p: {ty}): {ty};\n")
    } else {
        String::new()
    };
    let object = format!(
        "{MEMBER_PRELUDE}object O {{\n  f: {ty};\n{operation}}}\n\
         invariant I using P on O at current {{ self.f = self.f }}\n"
    );
    assert_eq!(
        refusal_codes(&object),
        [],
        "an object member of {ty} checks as the record field does"
    );
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn an_enum_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Tier", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn an_enum_parameter_and_result_check_as_a_record_field() {
    assert_member_checks_like_record_field("Tier", true);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_record_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Point", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_record_parameter_and_result_check_as_a_record_field() {
    assert_member_checks_like_record_field("Point", true);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_union_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Mark", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_tuple_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Pair", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn an_alias_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Small", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_decimal_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Decimal[0, 100; 2, 2; exact]", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_rational_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Rational[0, 9; 1, 4]", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_float_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Float64", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_quantity_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Quantity<m>", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn an_option_of_a_collection_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Option<Sequence<Integer>[0, 3]>", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_nested_collection_member_checks_as_a_record_field() {
    assert_member_checks_like_record_field("Sequence<Set<Integer>[0, 2]>[0, 2]", false);
}

#[trace("FR-569-AC-12", "TC-1940", "FSpec-FR-610-AC-5", "FSpec-TC-1442")]
#[test]
fn a_union_parameter_and_result_check_as_a_record_field() {
    assert_member_checks_like_record_field("Mark", true);
}

/// Every admitted member type follows the record field's typing and
/// type-mismatch path, including operation signatures where the criterion
/// calls them out explicitly.
#[trace("FR-569-AC-12", "TC-1940")]
#[test]
fn admitted_object_member_types_match_source_record_field_typing() {
    for (ty, operation) in [
        ("Tier", true),
        ("Point", true),
        ("Mark", true),
        ("Pair", false),
        ("Small", false),
        ("Decimal[0, 100; 2, 2; exact]", false),
        ("Rational[0, 9; 1, 4]", false),
        ("Float64", false),
        ("Quantity<m>", false),
        ("Option<Sequence<Integer>[0, 3]>", false),
        ("Sequence<Set<Integer>[0, 2]>[0, 2]", false),
    ] {
        assert_member_checks_like_record_field(ty, operation);
        let typed_uses = format!(
            "{MEMBER_PRELUDE}object O {{ f: {ty}; operation echo(p: {ty}): {ty}; }}\n\
             function read(o: Reference<O>): {ty} pure {{ deref(o).f }}\n\
             pre Input using P on O::echo {{ true }}\n\
             post Output using P on O::echo {{ true }}\n"
        );
        let graph = finish(assemble(&typed_uses).expect("typed uses assemble"))
            .unwrap_or_else(|refusals| panic!("{ty}: {refusals:?}"));
        let field_type = source_field_type(&graph, "O", "f");
        let expected_kind = match ty {
            "Tier" => "Enum",
            "Point" | "Mark" | "Pair" => "Composite",
            "Small" => "Int",
            value if value.starts_with("Decimal[") => "Decimal",
            value if value.starts_with("Rational[") => "Rational",
            "Float64" => "Float",
            value if value.starts_with("Quantity<") => "Quantity",
            value if value.starts_with("Option<") => "Option",
            _ => "Collection",
        };
        let actual_kind = match &field_type {
            filament_exact::ValueType::Enum(_) => "Enum",
            filament_exact::ValueType::Composite(_) => "Composite",
            filament_exact::ValueType::Int(_) => "Int",
            filament_exact::ValueType::Decimal(_) => "Decimal",
            filament_exact::ValueType::Rational(_) => "Rational",
            filament_exact::ValueType::Float(_) => "Float",
            filament_exact::ValueType::Quantity(_) => "Quantity",
            filament_exact::ValueType::Option(_) => "Option",
            filament_exact::ValueType::Collection(_) => "Collection",
            other => panic!("unexpected checked type for {ty}: {other:?}"),
        };
        assert_eq!(actual_kind, expected_kind, "checked O.f retains {ty}");
        assert_eq!(
            graph
                .state_clause("Input")
                .expect("input clause")
                .parameters()[1]
                .1,
            field_type,
            "operation parameter retains {ty}"
        );
        assert_eq!(
            graph
                .state_clause("Output")
                .expect("output clause")
                .parameters()[1]
                .1,
            field_type,
            "operation result retains {ty}"
        );
        let record = format!(
            "{MEMBER_PRELUDE}record R {{ f: {ty}; }}\nfunction bad(r: R): Boolean pure {{ r.f }}\n"
        );
        let object = format!(
            "{MEMBER_PRELUDE}object O {{ f: {ty}; }}\nfunction bad(o: Reference<O>): Boolean pure {{ deref(o).f }}\n"
        );
        let record_refusals = refusal_codes(&record);
        assert!(!record_refusals.is_empty(), "record field {ty} is mistyped");
        assert_eq!(
            refusal_codes(&object),
            record_refusals,
            "object field {ty} has the same mismatch as a record field"
        );
    }
}

/// Operation variance reads the source model retained for conformance, so a
/// rich local type change must fail both parameter and result axes.
#[trace("FR-569-AC-12", "TC-1940")]
#[test]
fn rich_source_object_operation_overrides_keep_their_member_types_for_conformance() {
    let source = "\
record R { x: Integer; }\n\
record S { x: Boolean; }\n\
object Base { operation f(p: R): R; }\n\
object Child extends Base { operation f(p: S): S redefines Base::f; }\n";
    let refused = assemble(source).expect("rich source types assemble");
    let refusals = finish(refused).expect_err("the incompatible override is refused");
    let causes: Vec<_> = refusals
        .iter()
        .filter_map(|refusal| refusal.cause.catalog_code())
        .map(|code| code.cause())
        .collect();
    assert!(causes.contains(&"variance-parameter"), "{causes:?}");
    assert!(causes.contains(&"variance-result"), "{causes:?}");
}

/// Source object signatures retain their exact scalar types while conformance
/// keeps the existing widening and interval-inclusion rules for variance.
#[trace("FR-569-AC-12", "TC-1940")]
#[test]
fn source_object_operation_overrides_allow_scalar_widening_and_narrowing() {
    let bounded = filament_exact::ValueType::Int(
        filament_exact::IntegerInterval::new(
            filament_exact::Integer::from(0_i64),
            filament_exact::Integer::from(9_i64),
        )
        .expect("non-empty integer interval"),
    );
    let scalar_source = "\
object Base { operation f(p: Int[0, 9]): Integer; }\n\
object Child extends Base { operation f(p: Integer): Int[0, 9] redefines Base::f; }\n\
pre BaseInput using P on Base::f { true }\n\
post BaseOutput using P on Base::f { true }\n\
pre ChildInput using P on Child::f { true }\n\
post ChildOutput using P on Child::f { true }\n";
    let graph = finish(assemble(scalar_source).expect("scalar variance assembles"))
        .unwrap_or_else(|refusals| panic!("scalar widening/narrowing: {refusals:?}"));
    assert_eq!(
        graph
            .state_clause("BaseInput")
            .expect("base input")
            .parameters()[1]
            .1,
        bounded,
        "Base parameter retains Int[0, 9]"
    );
    assert_eq!(
        graph
            .state_clause("BaseOutput")
            .expect("base output")
            .parameters()[1]
            .1,
        filament_exact::ValueType::Integer,
        "Base result retains Integer"
    );
    assert_eq!(
        graph
            .state_clause("ChildInput")
            .expect("child input")
            .parameters()[1]
            .1,
        filament_exact::ValueType::Integer,
        "Child parameter retains Integer"
    );
    assert_eq!(
        graph
            .state_clause("ChildOutput")
            .expect("child output")
            .parameters()[1]
            .1,
        bounded,
        "Child result retains Int[0, 9]"
    );

    let bounded_variance_source = "\
object Base { operation f(p: Int[1, 8]): Int[0, 9]; }\n\
object Child extends Base { operation f(p: Int[0, 9]): Int[1, 8] redefines Base::f; }\n";
    finish(assemble(bounded_variance_source).expect("bounded variance assembles"))
        .unwrap_or_else(|refusals| panic!("bounded interval variance: {refusals:?}"));
}

/// A field type a record field refuses is refused for an object member with
/// the same code and cause, and the object unit yields no checked package.
fn assert_member_refused_like_record_field(prelude: &str, ty: &str) {
    let record = refusal_codes(&format!("{prelude}record R {{ f: {ty}; }}\n"));
    assert!(!record.is_empty(), "a record field of {ty} is refused");
    let object = refusal_codes(&format!("{prelude}object O {{ f: {ty}; }}\n"));
    assert_eq!(
        object, record,
        "an object member of {ty} is refused as the record field is"
    );
}

#[trace("FR-569-AC-13", "TC-1940", "FSpec-FR-610-AC-6", "FSpec-TC-1442")]
#[test]
fn a_population_typed_member_is_refused_as_a_record_field_is() {
    assert_member_refused_like_record_field(OBJECTS, "Population<Order>[3]");
}

#[trace("FR-569-AC-13", "TC-1940", "FSpec-FR-610-AC-6", "FSpec-TC-1442")]
#[test]
fn a_member_typed_by_a_function_name_is_refused_as_a_record_field_is() {
    assert_member_refused_like_record_field(
        "function helper using P (n: Integer): Integer pure { n }\n",
        "helper",
    );
}

/// A recursive member name is rejected in the same declaration pass as a
/// recursive record member, before either declaration set is admitted.
#[trace("FR-569-AC-12", "TC-1940")]
#[test]
fn a_recursive_object_member_is_refused_as_a_recursive_record_field_is() {
    let recursive = "type Loop = Loop;\n";
    let record = refusal_codes(&format!("{recursive}record R {{ f: Loop; }}\n"));
    let object = refusal_codes(&format!("{recursive}object O {{ f: Loop; }}\n"));
    assert_eq!(record, [(Code::InvalidPackage, Some("definition-cycle"))]);
    assert_eq!(object, record);
}

/// FR-230-AC-65 (TC-2361): the AC-65 source unit, with `time` for the
/// unit's time member.
fn source_timed(time: &str) -> String {
    format!(
        "profile tm = \"filament.temporal.complete/v1\";\n\
         dimension Time;\nunit ms : Time = rational(1, 1);\n\
         record W {{ c: Clock; }}\n\
         object O {{ w: W; d: Clock; operation step() modifies self.d; }}\n\
         object Other {{ n: Integer; }}\n{time}"
    )
}

/// The temporal tags of the unit's check refusals, in report order.
fn temporal_tags(declarations: &str) -> Vec<&'static str> {
    let refusals = finish(assemble(declarations).expect("the unit assembles"))
        .expect_err("the unit is refused");
    refusals
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::Temporal(cause) => Some(cause.tag()),
            _ => None,
        })
        .collect()
}

/// FR-230-AC-65 (TC-2361): a source object type's name in the `time` slot
/// times the source model; a second source time member and a `tick` one
/// refuse.
#[trace("FR-230-AC-65", "TC-2361")]
#[test]
fn a_source_object_type_name_times_the_source_model() {
    let graph = finish(
        assemble(&source_timed("time O dense unit ms;\n")).expect("the dense unit assembles"),
    )
    .unwrap_or_else(|refusals| panic!("time O dense unit ms checks: {refusals:?}"));
    assert_eq!(graph.time_sources()[0].model, "O");
    let clocks: Vec<_> = graph
        .timed_models()
        .iter()
        .flat_map(|model| model.clocks())
        .map(|clock| clock.name())
        .collect();
    assert!(clocks.contains(&"w.c"), "{clocks:?}");
    assert!(clocks.contains(&"d"), "{clocks:?}");
    let missing = source_timed("time Q dense unit ms;\n");
    let failures = finish(assemble(&missing).expect("unknown time source assembles"))
        .expect_err("unknown source type refuses");
    let failure = failures
        .iter()
        .find_map(|refusal| match &refusal.cause {
            CheckCause::Temporal(cause) if cause.tag() == "missing-name" => Some(cause),
            _ => None,
        })
        .expect("missing-name");
    assert_eq!(failure.code(), Code::MissingDeclaration);
    let span = failure.span();
    let text = format!("{HEADER}{missing}\n");
    assert_eq!(&text[span.start..span.end], "Q");
    assert_eq!(
        temporal_tags(&source_timed(
            "time O dense unit ms;\ntime Other dense unit ms;\n"
        )),
        ["duplicate-time-source"]
    );
    assert_eq!(
        temporal_tags(&source_timed("time O tick O::step period 1 ms unit ms;\n")),
        ["clock-without-dense-time"]
    );
    for time in ["", "time O tick O::step period 1 ms unit ms;\n"] {
        let source = source_timed(time);
        let refusals = finish(assemble(&source).expect("source clock model assembles"))
            .expect_err("a clock requires source dense time");
        let (field, span) = refusals
            .iter()
            .find_map(|refusal| match &refusal.cause {
                CheckCause::Temporal(cause) => match cause.as_ref() {
                    fsl_semantics::check::TemporalCause::ClockWithoutDenseTime { field, span } => {
                        Some((field, span))
                    }
                    _ => None,
                },
                _ => None,
            })
            .expect("clock-without-dense-time");
        assert_eq!(field, "w", "authored order precedes the direct d clock");
        let text = format!("{HEADER}{source}\n");
        assert!(text[span.start..span.end].contains("w: W"));
    }
}

#[trace("FR-230-AC-65", "TC-2361")]
#[test]
fn a_selected_models_dense_time_does_not_time_source_clocks() {
    use fsl_semantics::model::domain_package::DomainPackageRef;
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: "test/orders".to_owned(),
        },
        vec![orders_model::object("Anchor", &[], false)],
    );
    let source = source_timed("time M dense unit ms;\n");
    let (profile, declarations) = source.split_once('\n').expect("source profile header");
    let text = format!("{HEADER}{profile}\nmodel M = \"test/orders\";\n{declarations}\n");
    let failures =
        finish(assemble_model_text(&text, package).expect("selected and source models assemble"))
            .expect_err("selected model time does not cover source clocks");
    let (field, span) = failures
        .iter()
        .find_map(|refusal| match &refusal.cause {
            CheckCause::Temporal(cause) => match cause.as_ref() {
                fsl_semantics::check::TemporalCause::ClockWithoutDenseTime { field, span } => {
                    Some((field, span))
                }
                _ => None,
            },
            _ => None,
        })
        .expect("source clock requires source dense time");
    assert_eq!(field, "w");
    assert!(text[span.start..span.end].contains("w: W"));
}

/// FR-088-AC-14 (TC-2360 step 8): a `Population` keeps no member type, so it
/// prints only its `resolved:` identity form, its maximum included.
#[trace("FR-088-AC-14", "TC-2360")]
#[test]
fn a_population_prints_only_its_identity_form() {
    use filament_exact::ValueType;
    use fsl_semantics::model::domain_package::ValueTypeRef;
    let bounded = ValueTypeRef::Resolved(Box::new(ValueType::Population(Some(3))));
    let open = ValueTypeRef::Resolved(Box::new(ValueType::Population(None)));
    assert_eq!(bounded.to_string(), "resolved:Population[3]");
    assert_eq!(open.to_string(), "resolved:Population");
    use fsl_semantics::model::spelling::{fsl_spelling, SourceSpellings};
    let names = SourceSpellings::default();
    assert_eq!(
        fsl_spelling(&bounded, &names, None),
        "resolved:Population[3]"
    );
    let nested =
        ValueTypeRef::Resolved(Box::new(ValueType::option(ValueType::Population(Some(3)))));
    assert_eq!(
        fsl_spelling(&nested, &names, None),
        "Option<resolved:Population[3]>"
    );
}

/// Authored aliases belong to member slots, while nominal names are diagnostic
/// context shared by the source model. Neither changes the checked value type.
#[trace("FR-088-AC-13", "TC-2360", "TC-260")]
#[test]
fn resolved_source_member_spellings_keep_aliases_without_changing_identity() {
    use fsl_semantics::model::domain_package::{
        DomainPackageRecord, DomainPackageRef, NativeValueType, RecordValueTypeRecord, ValueTypeRef,
    };
    use fsl_semantics::model::spelling::{fsl_spelling, MemberSlot};
    let text = "type Money = Decimal[0, 100; 2, 2; exact];\n\
        record Point { x: Integer; }\n\
        enum Color { red, blue }\n\
        dimension Length; unit m : Length = rational(1, 1);\n\
        object Order { a: Money; b: Point; c: Color; d: Option<Sequence<Money>[0, 3]>;\n\
          e: M::Total; g: Quantity<m>; h: Decimal[0, 100; 2, 2; exact];\n\
          operation f(p: Money): Option<Reference<Order>>; }\n";
    let package = || {
        DomainPackage::new(
            DomainPackageRef {
                identity: "test/orders".to_owned(),
            },
            vec![
                DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
                    key: orders_model::key("Total"),
                    identity_fields: Vec::new(),
                    supertypes: Vec::new(),
                }),
                orders_model::field(
                    "Total",
                    "value",
                    ValueTypeRef::Native(NativeValueType::Integer),
                    filament_exact::Presence::Required,
                    0,
                ),
            ],
        )
    };
    let declarations = assemble_model(text, package()).expect("all spelling forms assemble");
    let second = assemble_model(text, package()).expect("repeat assembly");
    let models = &declarations.model_inputs.as_ref().expect("models").0;
    let source = models
        .iter()
        .find(|model| model.alias.is_empty())
        .expect("source");
    let index = source.view.model_index();
    let spellings = index.source_spellings();
    let records = &source.view.domain_package().records;
    let mut retained = Vec::new();
    for record in records {
        match record {
            DomainPackageRecord::FieldMember(field) => retained.push(&field.value_type),
            DomainPackageRecord::OperationMember(operation) => {
                retained.extend(
                    operation
                        .parameters
                        .iter()
                        .map(|parameter| &parameter.value_type),
                );
                retained.extend(operation.result.iter().map(|result| &result.value_type));
            }
            _ => {}
        }
    }
    for retained in retained {
        let ValueTypeRef::Resolved(value_type) = retained else {
            continue;
        };
        let mut pending = vec![value_type.as_ref()];
        while let Some(value_type) = pending.pop() {
            use filament_exact::ValueType;
            match value_type {
                ValueType::Composite(key) => assert!(spellings.node_name(key).is_some()),
                ValueType::Enum(shape) => {
                    for variant in shape.variants() {
                        assert!(spellings.node_name(variant.owner()).is_some());
                    }
                }
                ValueType::Reference(key) => assert!(spellings.object_name(key).is_some()),
                ValueType::Quantity(unit) => assert!(spellings
                    .unit_name(unit.declared_key().expect("declared m"))
                    .is_some()),
                ValueType::Option(inner) => pending.push(inner),
                ValueType::Collection(collection) => pending.push(collection.element()),
                ValueType::Map(map) => {
                    pending.push(map.key());
                    pending.push(map.values());
                }
                ValueType::Population(_) => panic!("source members contain no Population"),
                _ => {}
            }
        }
    }
    let mut point = None;
    for (name, expected) in [
        ("a", "Money"),
        ("b", "Point"),
        ("c", "Color"),
        ("d", "Option<Sequence<Money>[0, 3]>"),
        ("e", "M::Total"),
        ("g", "Quantity<m>"),
        ("h", "Decimal[0, 100; 2, 2; exact]"),
    ] {
        let field = records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::FieldMember(field)
                    if field.key.node.ends_with(&format!("/Order/{name}")) =>
                {
                    Some(field)
                }
                _ => None,
            })
            .expect("source field");
        let slot = MemberSlot::Field(field.key.clone());
        assert_eq!(
            fsl_spelling(&field.value_type, spellings, Some(&slot)),
            expected
        );
        assert!(field.value_type.to_string().starts_with("resolved:"));
        for debug_form in [
            "Composite(",
            "DecimalType",
            "EnumShape",
            "Reference(",
            "CollectionType",
        ] {
            assert!(!field.value_type.to_string().contains(debug_form));
        }
        let second_models = &second.model_inputs.as_ref().expect("repeat models").0;
        let second_source = second_models
            .iter()
            .find(|model| model.alias.is_empty())
            .expect("repeat source");
        let second_field = second_source
            .view
            .domain_package()
            .records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::FieldMember(candidate) if candidate.key == field.key => {
                    Some(candidate)
                }
                _ => None,
            })
            .expect("repeat field");
        assert_eq!(
            field.value_type.to_string(),
            second_field.value_type.to_string()
        );
        if name == "a" {
            assert_eq!(
                field.value_type.to_string(),
                "resolved:Decimal[0, 100; 2, 2; exact]"
            );
            assert_eq!(
                fsl_spelling(&field.value_type, spellings, None),
                "Decimal[0, 100; 2, 2; exact]"
            );
        }
        if name == "b" {
            point = Some(field);
        }
    }
    let operation = records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation) => Some(operation),
            _ => None,
        })
        .expect("operation");
    assert_eq!(
        fsl_spelling(
            &operation.parameters[0].value_type,
            spellings,
            Some(&MemberSlot::Parameter {
                operation: operation.key.clone(),
                position: 0
            })
        ),
        "Money"
    );
    assert_eq!(
        fsl_spelling(
            &operation.result.as_ref().expect("result").value_type,
            spellings,
            Some(&MemberSlot::Result(operation.key.clone()))
        ),
        "Option<Reference<Order>>"
    );
    let point = point.expect("Point member");
    let original_type = point.value_type.clone();
    let original_identity = point.value_type.to_string();
    let fsl_semantics::model::domain_package::ValueTypeRef::Resolved(value) = &point.value_type
    else {
        panic!("resolved Point")
    };
    let filament_exact::ValueType::Composite(key) = value.as_ref() else {
        panic!("nominal Point")
    };
    let mut renamed = spellings.clone();
    renamed.name_node(key.clone(), "Spot".to_owned());
    assert_eq!(fsl_spelling(&point.value_type, &renamed, None), "Spot");
    assert_eq!(point.value_type, original_type);
    assert_eq!(point.value_type.to_string(), original_identity);
    finish(declarations).expect("diagnostic names leave checking unchanged");
    let refused = assemble_model(&format!("{text}\nobject Rush extends Order {{ operation f(p: Decimal[0, 200; 2, 2; exact]): Option<Reference<Order>> redefines Order::f; }}\n"), package()).expect("override assembles");
    let source = refused
        .model_inputs
        .as_ref()
        .expect("models")
        .0
        .iter()
        .find(|model| model.alias.is_empty())
        .expect("source");
    let operation_keys: Vec<_> = source
        .view
        .domain_package()
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::OperationMember(operation) => Some(operation.key.clone()),
            _ => None,
        })
        .collect();
    let base = operation_keys
        .iter()
        .find(|key| key.node.ends_with("/Order/f"))
        .expect("base");
    let child = operation_keys
        .iter()
        .find(|key| key.node.ends_with("/Rush/f"))
        .expect("child");
    use fsl_semantics::model::conformance::{
        check_operation_redefinition, ConformanceCheckOutcome, ConformanceOutcome,
    };
    let result = check_operation_redefinition(
        source.view.model_index(),
        child,
        base,
        &mut fsl_semantics::model::accounting::Meter::new(ModelNormalizationLimits::UNLIMITED),
    );
    let ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) = result else {
        panic!("distinct Decimal parameter refuses: {result:?}")
    };
    let failure = failures
        .iter()
        .find(|failure| matches!(failure.cause, ModelRefusalCause::VarianceParameter { .. }))
        .expect("variance parameter");
    assert!(failure.detail.contains("Money"), "{}", failure.detail);
    assert!(
        failure.detail.contains("Decimal[0, 200; 2, 2; exact]"),
        "{}",
        failure.detail
    );
    assert!(!failure.detail.contains("resolved:"), "{}", failure.detail);
}

#[trace("FR-569-AC-15", "TC-1940", "FR-230-AC-65", "TC-2361")]
#[test]
fn source_dense_time_covers_direct_nested_and_collection_clock_members() {
    for member in ["w: W;", "w: Sequence<W>[0, 2];", "w: Wrapper;"] {
        let source = format!("profile tm = \"filament.temporal.complete/v1\";\ndimension Time; unit ms : Time = rational(1, 1);\nrecord W {{ c: Clock; }}\nrecord Wrapper {{ w: W; }}\nobject O {{ {member} }}\ntime O dense unit ms;\n");
        let graph = finish(assemble(&source).expect("source clock forms assemble"))
            .unwrap_or_else(|refusals| panic!("{member}: {refusals:?}"));
        assert_eq!(graph.time_sources().len(), 1);
        assert_eq!(graph.time_sources()[0].model, "O");
        assert!(graph.time_sources()[0].model_node.to_string().contains("O"));
    }
}
