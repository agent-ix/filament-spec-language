// SPDX-License-Identifier: AGPL-3.0-or-later
//! Clock components retained from source type forms before data projection.

use std::collections::{BTreeMap, BTreeSet};

use filament_exact::{CollectionKind, NodeKey, ValueType};
use fsl_forms::{BuiltinType, DeclarationForm, TypeForm, TypeFormHead};

use crate::check::lowering::{
    clock_type_identity, CollectionShape, PlaceStep, SourceClockLayout, SourceClockPlace,
};
use crate::check::type_form::TypeNames;
use crate::model::domain_package::{DomainPackage, DomainPackageRecord, ValueTypeRef};

/// Only clock layouts reachable from a retained source object's fields are
/// materialized; an unused chain of record types does not expand every prefix.
pub(in crate::check::assemble) fn needed_types(
    forms: &[DeclarationForm],
    names: &impl TypeNames,
    package: &DomainPackage,
) -> (BTreeSet<NodeKey>, BTreeSet<NodeKey>) {
    let declarations: BTreeMap<NodeKey, &DeclarationForm> = forms
        .iter()
        .filter_map(|form| {
            if matches!(form, DeclarationForm::Alias(_)) {
                return None;
            }
            let name = super::written(form)?.declares?;
            let [ValueType::Composite(key)] = names.named_types(name) else {
                return None;
            };
            Some((key.clone(), form))
        })
        .collect();
    let mut pending = Vec::new();
    for record in &package.records {
        if let DomainPackageRecord::FieldMember(field) = record {
            if let ValueTypeRef::Resolved(value_type) = &field.value_type {
                composite_keys(value_type, &mut pending);
            }
        }
    }
    let roots = pending.iter().cloned().collect();
    let mut needed = BTreeSet::new();
    while let Some(key) = pending.pop() {
        if !needed.insert(key.clone()) {
            continue;
        }
        let Some(declaration) = declarations.get(&key) else {
            continue;
        };
        let Some(written) = super::written(declaration) else {
            continue;
        };
        let mut types = written.signature;
        while let Some(form) = types.pop() {
            types.extend(&form.arguments);
            if let TypeFormHead::Name(name) = &form.head {
                for value_type in names.named_types(&name.name) {
                    composite_keys(value_type, &mut pending);
                }
            }
        }
    }
    (roots, needed)
}

fn composite_keys(mut value_type: &ValueType, keys: &mut Vec<NodeKey>) {
    loop {
        match value_type {
            ValueType::Option(inner) => value_type = inner,
            ValueType::Collection(collection) => value_type = collection.element(),
            ValueType::Map(map) => value_type = map.values(),
            ValueType::Composite(key) => {
                keys.push(key.clone());
                return;
            }
            _ => return,
        }
    }
}

struct Pending<'a> {
    form: &'a TypeForm,
    place: SourceClockPlace,
    visiting: Vec<&'a str>,
}

pub(in crate::check::assemble) fn source_places(
    forms: &[DeclarationForm],
    names: &impl TypeNames,
    inherited: &BTreeMap<NodeKey, SourceClockLayout>,
    needed: &BTreeSet<NodeKey>,
) -> BTreeMap<NodeKey, SourceClockLayout> {
    let declarations: BTreeMap<&str, &DeclarationForm> = forms
        .iter()
        .filter_map(|form| {
            let name = match form {
                DeclarationForm::Record(record) => &record.name.name,
                DeclarationForm::Tuple(tuple) => &tuple.name.name,
                DeclarationForm::Union(union) => &union.name.name,
                DeclarationForm::Alias(alias) => &alias.name.name,
                _ => return None,
            };
            Some((name.as_str(), form))
        })
        .collect();
    let mut result = inherited.clone();
    let render = |form: &TypeForm| type_identity(form, names, inherited, &declarations);
    for (name, declaration) in &declarations {
        let [ValueType::Composite(key)] = names.named_types(name) else {
            continue;
        };
        if !needed.contains(key) {
            continue;
        }
        let root = SourceClockPlace {
            path: Vec::new(),
            optional: false,
            bit: false,
            case: false,
            members: Vec::new(),
            slots: BTreeMap::new(),
        };
        let mut pending = Vec::new();
        members(declaration, root, vec![*name], &mut pending, &render);
        let mut places = Vec::new();
        while let Some(Pending {
            form,
            mut place,
            mut visiting,
        }) = pending.pop()
        {
            match &form.head {
                TypeFormHead::Builtin(BuiltinType::Clock) => places.push(place),
                TypeFormHead::Builtin(BuiltinType::Option) => {
                    if let [inner] = form.arguments.as_slice() {
                        place.bit = !place.optional;
                        place.optional = true;
                        pending.push(Pending {
                            form: inner,
                            place,
                            visiting,
                        });
                    }
                }
                TypeFormHead::Collection(kind) => {
                    if let [inner] = form.arguments.as_slice() {
                        let (min, max) = match form.bounds.as_slice() {
                            [] => (0, None),
                            [min, max] => match (min.parse(), max.parse()) {
                                (Ok(min), Ok(max)) => (min, Some(max)),
                                _ => continue,
                            },
                            _ => continue,
                        };
                        place.path.push(PlaceStep::Slot(CollectionShape {
                            ordered: kind.is_ordered(),
                            unique: kind.is_unique(),
                            min,
                            max,
                            map: false,
                        }));
                        place.bit = false;
                        place.case = false;
                        let identity = render(inner);
                        place
                            .slots
                            .insert(place.path.len().saturating_sub(1), identity.clone());
                        place.members.push((identity, None));
                        pending.push(Pending {
                            form: inner,
                            place,
                            visiting,
                        });
                    }
                }
                TypeFormHead::Name(name) if !visiting.contains(&name.name.as_str()) => {
                    if let Some(declaration) = declarations.get(name.name.as_str()) {
                        visiting.push(&name.name);
                        members(declaration, place, visiting, &mut pending, &render);
                    } else if let [ValueType::Composite(key)] = names.named_types(&name.name) {
                        if let Some(layout) = inherited.get(key) {
                            for component in &layout.places {
                                let mut below = place.clone();
                                let offset = below.path.len();
                                below.path.extend(component.path.iter().cloned());
                                below.bit = !below.optional && component.bit;
                                below.case = !below.optional && component.case;
                                below.optional |= component.optional;
                                below.members.extend(component.members.iter().cloned());
                                below.slots.extend(
                                    component.slots.iter().map(|(index, identity)| {
                                        (index + offset, identity.clone())
                                    }),
                                );
                                places.push(below);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if !places.is_empty() {
            places.sort_by(|a, b| a.path.cmp(&b.path));
            result.insert(
                key.clone(),
                SourceClockLayout {
                    identity: key.to_string(),
                    places,
                },
            );
        }
    }
    result
}

fn members<'a>(
    declaration: &'a DeclarationForm,
    at: SourceClockPlace,
    visiting: Vec<&'a str>,
    pending: &mut Vec<Pending<'a>>,
    render: &impl Fn(&TypeForm) -> String,
) {
    match declaration {
        DeclarationForm::Record(record) => {
            for field in &record.fields {
                let mut place = at.clone();
                place.path.push(PlaceStep::Field(field.name.clone()));
                place.bit = field.optional && !place.optional;
                place.case = false;
                place.optional |= field.optional;
                let declared = render(&field.type_form);
                place.members.push((
                    if field.optional {
                        format!("Option<{declared}>")
                    } else {
                        declared
                    },
                    Some(format!("{}/{}", record.name.name, field.name)),
                ));
                pending.push(Pending {
                    form: &field.type_form,
                    place,
                    visiting: visiting.clone(),
                });
            }
        }
        DeclarationForm::Tuple(tuple) => {
            for (position, form) in tuple.elements.iter().enumerate() {
                let mut place = at.clone();
                place.path.push(PlaceStep::Field(position.to_string()));
                place.bit = false;
                place.case = false;
                place.members.push((render(form), None));
                pending.push(Pending {
                    form,
                    place,
                    visiting: visiting.clone(),
                });
            }
        }
        DeclarationForm::Union(union) => {
            for member in &union.members {
                for form in &member.payload {
                    let mut place = at.clone();
                    place.path.push(PlaceStep::Union(member.name.name.clone()));
                    place.case = !place.optional && place.path.len() == 1;
                    place.bit = false;
                    place.optional = true;
                    place.members.push((render(form), None));
                    pending.push(Pending {
                        form,
                        place,
                        visiting: visiting.clone(),
                    });
                }
            }
        }
        DeclarationForm::Alias(alias) => pending.push(Pending {
            form: &alias.target,
            place: at,
            visiting,
        }),
        _ => {}
    }
}

fn type_identity(
    form: &TypeForm,
    names: &impl TypeNames,
    inherited: &BTreeMap<NodeKey, SourceClockLayout>,
    declarations: &BTreeMap<&str, &DeclarationForm>,
) -> String {
    enum Part<'a> {
        Form(&'a TypeForm),
        Text(&'static str),
    }
    let mut pending = vec![Part::Form(form)];
    let mut output = String::new();
    while let Some(part) = pending.pop() {
        let form = match part {
            Part::Form(form) => form,
            Part::Text(text) => {
                output.push_str(text);
                continue;
            }
        };
        if let Ok(value_type) = crate::check::type_form::resolve_form(names, form) {
            if let Some(identity) = clock_type_identity(&value_type, |key| {
                inherited.get(key).map(|layout| layout.identity.clone())
            }) {
                output.push_str(&identity);
                continue;
            }
        }
        match &form.head {
            TypeFormHead::Builtin(BuiltinType::Clock) => {
                output.push_str("ix://filament/native/Clock")
            }
            TypeFormHead::Name(name) => match declarations.get(name.name.as_str()) {
                Some(DeclarationForm::Alias(alias)) => pending.push(Part::Form(&alias.target)),
                _ => output.push_str(&spelling(form)),
            },
            TypeFormHead::Builtin(BuiltinType::Option) => {
                output.push_str("Option<");
                pending.push(Part::Text(">"));
                if let Some(inner) = form.arguments.first() {
                    pending.push(Part::Form(inner));
                }
            }
            TypeFormHead::Collection(kind) => {
                output.push_str(match kind {
                    CollectionKind::Sequence => "Sequence<",
                    CollectionKind::Set => "Set<",
                    CollectionKind::Bag => "Bag<",
                    CollectionKind::OrderedSet => "OrderedSet<",
                });
                pending.push(Part::Text(">"));
                if let Some(inner) = form.arguments.first() {
                    pending.push(Part::Form(inner));
                }
            }
            _ => output.push_str(&spelling(form)),
        }
    }
    output
}

fn spelling(form: &TypeForm) -> String {
    enum Part<'a> {
        Form(&'a TypeForm),
        Text(&'a str),
        Bound(&'a TypeForm),
    }
    let mut pending = vec![Part::Form(form)];
    let mut rendered = String::new();
    while let Some(part) = pending.pop() {
        let form = match part {
            Part::Text(text) => {
                rendered.push_str(text);
                continue;
            }
            Part::Bound(form) => {
                if !form.bounds.is_empty() {
                    rendered.push('[');
                    for (position, bound) in form.bounds.iter().enumerate() {
                        if position != 0 {
                            let semicolon = match form.head {
                                TypeFormHead::Builtin(
                                    BuiltinType::Rational | BuiltinType::Decimal,
                                ) => position == 2 || position == 4,
                                TypeFormHead::Builtin(BuiltinType::Text | BuiltinType::Int) => {
                                    position == 2
                                }
                                _ => false,
                            };
                            rendered.push_str(if semicolon { "; " } else { ", " });
                        }
                        rendered.push_str(bound);
                    }
                    rendered.push(']');
                }
                continue;
            }
            Part::Form(form) => form,
        };
        let name = match &form.head {
            TypeFormHead::Builtin(builtin) => match builtin {
                BuiltinType::Boolean => "Boolean",
                BuiltinType::Integer => "Integer",
                BuiltinType::Int => "Int",
                BuiltinType::Rational => "Rational",
                BuiltinType::Decimal => "Decimal",
                BuiltinType::Float32 => "Float32",
                BuiltinType::Float64 => "Float64",
                BuiltinType::Text => "Text",
                BuiltinType::Option => "Option",
                BuiltinType::Reference => "Reference",
                BuiltinType::Quantity => "Quantity",
                BuiltinType::Clock => "ix://filament/native/Clock",
            },
            TypeFormHead::Collection(kind) => match kind {
                CollectionKind::Sequence => "Sequence",
                CollectionKind::Set => "Set",
                CollectionKind::Bag => "Bag",
                CollectionKind::OrderedSet => "OrderedSet",
            },
            TypeFormHead::Population => "Population",
            TypeFormHead::Name(name) => &name.name,
            TypeFormHead::UnitExpression(terms) => {
                for (position, term) in terms.iter().enumerate() {
                    if position != 0 {
                        rendered.push_str(" * ");
                    }
                    rendered.push_str(&term.name.name);
                    if let Some((power, _)) = &term.exponent {
                        rendered.push('^');
                        rendered.push_str(&power.to_string());
                    }
                }
                continue;
            }
        };
        rendered.push_str(name);
        pending.push(Part::Bound(form));
        if !form.arguments.is_empty() {
            rendered.push('<');
            pending.push(Part::Text(">"));
            for (position, argument) in form.arguments.iter().enumerate().rev() {
                pending.push(Part::Form(argument));
                if position != 0 {
                    pending.push(Part::Text(", "));
                }
            }
        }
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::PackageDeclarations;
    use crate::model::accounting::ModelNormalizationLimits;
    use crate::model::domain_package::{
        DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
        ObjectTypeRecord, RecordValueTypeRecord, ValueTypeRef,
    };
    use crate::model::intake::SelectedModels;
    use filament_checked::key::DeclarationKey;
    use ix_trace_rs::trace;

    #[trace("FR-569-AC-15", "TC-1940", "FR-088-AC-13", "TC-2360")]
    #[test]
    fn resolved_source_record_members_keep_the_record_fields_clock_places() {
        let source = "language \"ix:native\" edition \"1-draft\";\n\
            profile P = \"filament.temporal.complete/v1\";\n\
            model M = \"test/clocks\";\n\
            dimension Time; unit ms : Time = rational(1, 1); time M dense unit ms;\n\
            record W { c: Clock; }\n\
            record Wrapper { w: W; }\n\
            object Order { w: W; nested: Wrapper; collection: Sequence<W>[0, 2]; model_w: M::Timer; }\n";
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("test", "source-clock-record"),
            "source.fsl",
            source.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("parse");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = fsl_forms::build_unit(&parsed).expect("build source forms");
        let package = DomainPackage::new(
            DomainPackageRef {
                identity: "test/clocks".to_owned(),
            },
            vec![
                DomainPackageRecord::ObjectType(ObjectTypeRecord {
                    key: DeclarationKey {
                        package: "test/clocks".to_owned(),
                        node: "ix://test/clocks/Anchor".to_owned(),
                    },
                    interface_features: None,
                    abstract_type: false,
                    supertypes: Vec::new(),
                }),
                DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
                    key: DeclarationKey {
                        package: "test/clocks".to_owned(),
                        node: "ix://test/clocks/Timer".to_owned(),
                    },
                    identity_fields: Vec::new(),
                    supertypes: Vec::new(),
                }),
                DomainPackageRecord::FieldMember(FieldMemberRecord {
                    key: DeclarationKey {
                        package: "test/clocks".to_owned(),
                        node: "ix://test/clocks/Timer/c".to_owned(),
                    },
                    owner: DeclarationKey {
                        package: "test/clocks".to_owned(),
                        node: "ix://test/clocks/Timer".to_owned(),
                    },
                    value_type: ValueTypeRef::Clock,
                    multiplicity: Multiplicity {
                        lower: 1,
                        upper: Some(1),
                        ordered: false,
                        unique: false,
                    },
                    presence: filament_exact::Presence::Required,
                    subsets: Vec::new(),
                    redefines: None,
                    position: 0,
                }),
            ],
        );
        let selected = SelectedModels::fixture(
            "M",
            fsl_foundation::Span { start: 0, end: 1 },
            package,
            ModelNormalizationLimits::UNLIMITED,
        )
        .expect("admitted model");
        let declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "test/source-clock-record".to_owned(),
            unit,
            selected,
            Vec::new(),
        )
        .expect("assemble the record's clock-bearing source member");
        let source_model = declarations
            .models
            .iter()
            .find(|model| model.alias() == Some(""))
            .expect("source model");
        let paths: Vec<_> = source_model
            .clock_places()
            .iter()
            .map(|place| place.path.clone())
            .collect();
        assert!(
            paths.contains(&vec![
                PlaceStep::Field("w".to_owned()),
                PlaceStep::Field("c".to_owned())
            ]),
            "{paths:?}"
        );
        assert!(
            paths.contains(&vec![
                PlaceStep::Field("model_w".to_owned()),
                PlaceStep::Field("c".to_owned())
            ]),
            "{paths:?}"
        );
        assert!(
            paths.contains(&vec![
                PlaceStep::Field("nested".to_owned()),
                PlaceStep::Field("w".to_owned()),
                PlaceStep::Field("c".to_owned())
            ]),
            "{paths:?}"
        );
        assert!(
            paths.contains(&vec![
                PlaceStep::Field("collection".to_owned()),
                PlaceStep::Slot(CollectionShape {
                    ordered: true,
                    unique: false,
                    min: 0,
                    max: Some(2),
                    map: false
                }),
                PlaceStep::Field("c".to_owned())
            ]),
            "{paths:?}"
        );
        let w = source_model
            .clock_places()
            .iter()
            .find(|place| {
                place.path
                    == vec![
                        PlaceStep::Field("w".to_owned()),
                        PlaceStep::Field("c".to_owned()),
                    ]
            })
            .expect("w.c");
        assert!(!w.optional);
        assert_eq!(
            w.members.last().expect("clock member row").0,
            "ix://filament/native/Clock"
        );
        let rows = w.rows();
        let expected_w_identity = "test/source-clock-record:[\"W\",]:0";
        assert_eq!(rows[0].member_type, expected_w_identity);
        assert_eq!(rows[0].member_type, rows[1].declaring_type);
        assert!(!rows[0].member_type.contains("resolved:"));
        assert!(!rows[0].member_type.contains("Composite("));
        let nested = source_model
            .clock_places()
            .iter()
            .find(|place| {
                place.path
                    == vec![
                        PlaceStep::Field("nested".to_owned()),
                        PlaceStep::Field("w".to_owned()),
                        PlaceStep::Field("c".to_owned()),
                    ]
            })
            .expect("nested.w.c")
            .rows();
        let record_field_row = nested
            .iter()
            .find(|row| row.member_identity == "Wrapper/w")
            .expect("record field Wrapper.w typed W");
        assert_eq!(record_field_row.member_type, expected_w_identity);
        assert_eq!(rows[0].member_type, record_field_row.member_type);
        assert_eq!(rows[0].member_type, nested[1].member_type);
        assert_eq!(nested[1].member_type, nested[2].declaring_type);
        let collection = source_model.clock_places().iter().find(|place| {
            matches!(place.path.first(), Some(PlaceStep::Field(name)) if name == "collection")
        }).expect("collection clock").rows();
        assert_eq!(collection[1].member_type, rows[0].member_type);
        assert_eq!(collection[1].member_type, collection[2].declaring_type);
    }
}
