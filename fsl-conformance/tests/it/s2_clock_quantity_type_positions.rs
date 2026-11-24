// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-393 S2 type forms through the public forms API.

use fsl_forms::{
    build_unit, BuiltinType, DeclarationForm, ObjectMemberForm, ParsedUnit, TypeForm, TypeFormHead,
};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

const DECLARATIONS: &str = "function f(c: Clock): Clock pure { c }\n\
function g(q: Quantity<ms>): Quantity<ms> pure { q }\n\
predicate p(c: Clock, q: Quantity<ms>): Boolean { true }\n\
object O { operation oc(c: Clock): Clock; operation oq(q: Quantity<ms>): Quantity<ms>; }\n\
record R { c: Clock; q: Quantity<ms>; }\n\
tuple T(Clock, Quantity<ms>);\n\
union U { C(Clock), Q(Quantity<ms>) }\n\
type AliasClock = Clock;\n\
type AliasQuantity = Quantity<ms>;\n\
record C {\n\
  option_clock: Option<Clock>; option_quantity: Option<Quantity<ms>>;\n\
  set_clock: Set<Clock>; set_quantity: Set<Quantity<ms>>;\n\
  sequence_clock: Sequence<Clock>; sequence_quantity: Sequence<Quantity<ms>>;\n\
  bag_clock: Bag<Clock>; bag_quantity: Bag<Quantity<ms>>;\n\
  ordered_clock: OrderedSet<Clock>; ordered_quantity: OrderedSet<Quantity<ms>>;\n\
  map_clock_quantity: Map<Clock, Quantity<ms>>;\n\
  map_quantity_clock: Map<Quantity<ms>, Clock>;\n\
}\n";

fn parsed(text: &str) -> fsl_forms::ParsedUnit {
    let input = fsl_forms::cst::parse(
        SourceIdentity::new("agent-ix", "test:qsl-2094-fr393-ac8"),
        "unit.native",
        text.as_bytes(),
        fsl_forms::cst::Limits::default(),
    )
    .expect("S1 accepts the type-position fixture");
    assert!(input.is_admissible(), "{:?}", input.diagnostics());
    build_unit(&input).expect("S2 builds the fixture")
}

fn by_name<'a>(unit: &'a fsl_forms::ParsedUnit, name: &str) -> &'a DeclarationForm {
    unit.forms()
        .iter()
        .find(|form| form.declared_name() == Some(name))
        .unwrap_or_else(|| panic!("declaration {name}"))
        .form()
}

fn span_text<'a>(source: &'a str, span: Span) -> &'a str {
    &source[span.start..span.end]
}

fn assert_clock(source: &str, ty: &TypeForm) {
    assert_eq!(ty.head, TypeFormHead::Builtin(BuiltinType::Clock));
    assert!(ty.arguments.is_empty());
    assert_eq!(span_text(source, ty.span), "Clock");
}

fn assert_quantity(source: &str, ty: &TypeForm) {
    assert_eq!(ty.head, TypeFormHead::Builtin(BuiltinType::Quantity));
    assert_eq!(ty.arguments.len(), 1);
    assert_eq!(span_text(source, ty.span), "Quantity<ms>");
    assert_eq!(span_text(source, ty.arguments[0].span), "ms");
}

fn observed_types(unit: &ParsedUnit) -> Vec<TypeForm> {
    let mut types = Vec::new();
    for name in ["f", "g", "p"] {
        let DeclarationForm::Function(function) = by_name(unit, name) else {
            panic!("function or predicate {name}");
        };
        types.extend(function.parameters.iter().map(|(_, ty)| ty.clone()));
        types.push(function.result.clone());
    }
    let DeclarationForm::Object(object) = by_name(unit, "O") else {
        panic!("object form");
    };
    for member in &object.members {
        if let ObjectMemberForm::Operation(operation) = member {
            types.extend(
                operation
                    .parameters
                    .iter()
                    .map(|parameter| parameter.value_type.clone()),
            );
            types.extend(operation.result.iter().cloned());
        }
    }
    let DeclarationForm::Record(record) = by_name(unit, "R") else {
        panic!("record form");
    };
    types.extend(record.fields.iter().map(|field| field.type_form.clone()));
    let DeclarationForm::Tuple(tuple) = by_name(unit, "T") else {
        panic!("tuple form");
    };
    types.extend(tuple.elements.iter().cloned());
    let DeclarationForm::Union(union) = by_name(unit, "U") else {
        panic!("union form");
    };
    types.extend(
        union
            .members
            .iter()
            .flat_map(|member| member.payload.iter().cloned()),
    );
    for name in ["AliasClock", "AliasQuantity"] {
        let DeclarationForm::Alias(alias) = by_name(unit, name) else {
            panic!("alias form {name}");
        };
        types.push(alias.target.clone());
    }
    let DeclarationForm::Record(collections) = by_name(unit, "C") else {
        panic!("collection record");
    };
    types.extend(
        collections
            .fields
            .iter()
            .map(|field| field.type_form.clone()),
    );
    types
}

#[trace("TC-1078", "FR-393-AC-8")]
#[test]
fn clock_and_quantity_forms_keep_type_position_and_span_under_each_facet() {
    let headers = [
        "language \"ix:native\" edition \"1-draft\";\n",
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n",
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.temporal.timed/v1\";\n",
    ];
    let width = headers.iter().map(|header| header.len()).max().unwrap();
    let variants: Vec<_> = headers
        .iter()
        .map(|header| {
            let mut padded = header.trim_end().to_owned();
            padded.push_str(&" ".repeat(width - header.trim_end().len()));
            padded.push('\n');
            let text = format!("{padded}{DECLARATIONS}");
            let unit = parsed(&text);
            (text, unit)
        })
        .collect();
    let (_, plain) = &variants[0];
    let expected_forms = observed_types(plain);
    for (text, selected) in &variants {
        assert_eq!(selected.forms().len(), plain.forms().len());

        for (name, expected) in [("f", "clock"), ("g", "quantity")] {
            let DeclarationForm::Function(function) = by_name(&selected, name) else {
                panic!("function {name}");
            };
            if expected == "clock" {
                assert_clock(text, &function.parameters[0].1);
                assert_clock(text, &function.result);
            } else {
                assert_quantity(text, &function.parameters[0].1);
                assert_quantity(text, &function.result);
            }
        }

        let DeclarationForm::Function(predicate) = by_name(&selected, "p") else {
            panic!("predicate form");
        };
        assert_eq!(predicate.kind(), fsl_forms::DeclarationKind::Predicate);
        assert_clock(text, &predicate.parameters[0].1);
        assert_quantity(text, &predicate.parameters[1].1);

        let DeclarationForm::Object(object) = by_name(&selected, "O") else {
            panic!("object form");
        };
        let operations: Vec<_> = object
            .members
            .iter()
            .filter_map(|member| match member {
                ObjectMemberForm::Operation(operation) => Some(operation),
                _ => None,
            })
            .collect();
        assert_eq!(operations.len(), 2);
        assert_clock(text, &operations[0].parameters[0].value_type);
        assert_clock(text, operations[0].result.as_ref().expect("Clock result"));
        assert_quantity(text, &operations[1].parameters[0].value_type);
        assert_quantity(
            text,
            operations[1].result.as_ref().expect("Quantity result"),
        );

        let DeclarationForm::Record(record) = by_name(&selected, "R") else {
            panic!("record form");
        };
        assert_clock(text, &record.fields[0].type_form);
        assert_quantity(text, &record.fields[1].type_form);

        let DeclarationForm::Tuple(tuple) = by_name(&selected, "T") else {
            panic!("tuple form");
        };
        assert_clock(text, &tuple.elements[0]);
        assert_quantity(text, &tuple.elements[1]);

        let DeclarationForm::Union(union) = by_name(&selected, "U") else {
            panic!("union form");
        };
        assert_clock(text, &union.members[0].payload[0]);
        assert_quantity(text, &union.members[1].payload[0]);

        let DeclarationForm::Alias(alias_clock) = by_name(&selected, "AliasClock") else {
            panic!("Clock alias");
        };
        assert_clock(text, &alias_clock.target);
        let DeclarationForm::Alias(alias_quantity) = by_name(&selected, "AliasQuantity") else {
            panic!("Quantity alias");
        };
        assert_quantity(text, &alias_quantity.target);

        let DeclarationForm::Record(collections) = by_name(&selected, "C") else {
            panic!("collection record");
        };
        for (index, field) in collections.fields.iter().enumerate() {
            let expected_clock = matches!(index, 0 | 2 | 4 | 6 | 8);
            let expected_head = match index {
                0 | 1 => TypeFormHead::Builtin(BuiltinType::Option),
                2 | 3 => TypeFormHead::Collection(filament_exact::CollectionKind::Set),
                4 | 5 => TypeFormHead::Collection(filament_exact::CollectionKind::Sequence),
                6 | 7 => TypeFormHead::Collection(filament_exact::CollectionKind::Bag),
                8 | 9 => TypeFormHead::Collection(filament_exact::CollectionKind::OrderedSet),
                10 | 11 => TypeFormHead::Builtin(BuiltinType::Map),
                _ => panic!("unexpected collection field index {index}"),
            };
            assert_eq!(field.type_form.head, expected_head);
            if index == 10 {
                assert_eq!(
                    field.type_form.head,
                    TypeFormHead::Builtin(BuiltinType::Map)
                );
                assert_clock(text, &field.type_form.arguments[0]);
                assert_quantity(text, &field.type_form.arguments[1]);
                continue;
            }
            if index == 11 {
                assert_eq!(
                    field.type_form.head,
                    TypeFormHead::Builtin(BuiltinType::Map)
                );
                assert_quantity(text, &field.type_form.arguments[0]);
                assert_clock(text, &field.type_form.arguments[1]);
                continue;
            }
            assert_eq!(field.type_form.arguments.len(), 1);
            if expected_clock {
                assert_clock(text, &field.type_form.arguments[0]);
            } else {
                assert_quantity(text, &field.type_form.arguments[0]);
            }
        }
        assert_eq!(
            observed_types(selected),
            expected_forms,
            "type forms and source spans are facet-independent"
        );
    }
}
