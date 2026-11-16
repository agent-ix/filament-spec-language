// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retain the names discarded when source member types are resolved.

use std::collections::{BTreeMap, BTreeSet};

use filament_exact::ValueType;
use fsl_forms::{BuiltinType, ObjectMemberForm, TypeForm, TypeFormHead};

use super::{
    source_member_declaration_key, ModelSpellings, Names, RawSourceRef, SelectedModel, Unit,
};
use crate::check::type_form::{resolve_form, TypeNames};
use crate::model::domain_package::DomainPackageRecord;
use crate::model::intake::type_identity_segment;
use crate::model::spelling::{AliasNode, AliasTree, MemberSlot, SourceSpellings, TypeView};

pub(super) fn build(
    unit: &Unit,
    all_names: &Names,
    spellings: &ModelSpellings,
    owners: &BTreeMap<String, RawSourceRef>,
    source: &SelectedModel,
    models: &[SelectedModel],
) -> SourceSpellings {
    let owner_key = |name: &str| {
        source
            .view
            .domain_package()
            .records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::ObjectType(object)
                    if type_identity_segment(&object.key.package, &object.key.node)
                        == Some(name) =>
                {
                    Some(&object.key)
                }
                _ => None,
            })
    };
    let source_unit = unit
        .objects
        .iter()
        .find(|object| owner_key(&object.name.name).is_some())
        .and_then(|object| owners.get(&object.name.name));
    let names = all_names.in_unit(spellings, source_unit);
    let mut result = SourceSpellings::default();
    for name in unit
        .composites
        .iter()
        .map(|record| &record.name.name)
        .chain(unit.enums.iter().map(|enumeration| &enumeration.name.name))
    {
        for value_type in names.named_types(name) {
            result.name_type(value_type, name);
        }
    }
    for object in &unit.objects {
        if owner_key(&object.name.name).is_some() {
            if let Some(key) = names.object_type_named(&object.name.name) {
                result.name_object(key, object.name.name.clone());
            }
        }
    }
    for unit in &unit.units {
        if let Some(id) = names.unit_named(&unit.name.name) {
            if let Some(key) = id.declared_key() {
                result.name_unit(key.clone(), unit.name.name.clone());
            }
        }
    }
    let mut model_aliases = BTreeSet::new();
    for model in models.iter().filter(|model| !model.alias.is_empty()) {
        if spellings.selection(source_unit, &model.alias)
            != Some(&model.view.domain_package().model_selection)
        {
            continue;
        }
        for record in &model.view.domain_package().records {
            let key = record.key();
            let Some(member) = type_identity_segment(&key.package, &key.node) else {
                continue;
            };
            let name = format!("{}::{member}", model.alias);
            match record {
                DomainPackageRecord::RecordValueType(_)
                | DomainPackageRecord::TupleValueType(_)
                | DomainPackageRecord::Enum(_)
                | DomainPackageRecord::ObjectType(_) => {
                    for value_type in names.named_types(&name) {
                        result.name_type(value_type, &name);
                    }
                    if let Some(key) = names.object_type_named(&name) {
                        result.name_object(key, name);
                    }
                }
                DomainPackageRecord::AliasType(_) => {
                    model_aliases.insert(name);
                }
                _ => {}
            }
        }
        for unit in &model.view.domain_package().units {
            let name = format!("{}::{}", model.alias, unit.name);
            if let Some(id) = names.unit_named(&name) {
                if let Some(key) = id.declared_key() {
                    result.name_unit(key.clone(), name);
                }
            }
        }
    }
    let aliases: BTreeMap<&str, &TypeForm> = unit
        .aliases
        .iter()
        .map(|alias| (alias.name.name.as_str(), &alias.target))
        .collect();
    for object in &unit.objects {
        let Some(owner) = owner_key(&object.name.name) else {
            continue;
        };
        for member in &object.members {
            match member {
                ObjectMemberForm::Field(field) => {
                    if let Some(tree) =
                        member_tree(&field.type_form, &names, &aliases, &model_aliases)
                    {
                        result.aliases(
                            MemberSlot::Field(source_member_declaration_key(
                                owner,
                                &field.name.name,
                            )),
                            tree,
                        );
                    }
                }
                ObjectMemberForm::Operation(operation) => {
                    let key = source_member_declaration_key(owner, &operation.name.name);
                    for (position, parameter) in operation.parameters.iter().enumerate() {
                        if let Some(tree) =
                            member_tree(&parameter.value_type, &names, &aliases, &model_aliases)
                        {
                            result.aliases(
                                MemberSlot::Parameter {
                                    operation: key.clone(),
                                    position,
                                },
                                tree,
                            );
                        }
                    }
                    if let Some(form) = &operation.result {
                        if let Some(tree) = member_tree(form, &names, &aliases, &model_aliases) {
                            result.aliases(MemberSlot::Result(key), tree);
                        }
                    }
                }
                ObjectMemberForm::Population(_) => {}
            }
        }
    }
    result
}

fn expanded<'a>(
    mut form: &'a TypeForm,
    aliases: &BTreeMap<&str, &'a TypeForm>,
    model_aliases: &BTreeSet<String>,
) -> (&'a TypeForm, Option<String>) {
    let mut name = None;
    for _ in 0..=aliases.len() {
        let TypeFormHead::Name(written) = &form.head else {
            break;
        };
        if let Some(target) = aliases.get(written.name.as_str()) {
            if name.is_none() {
                name = Some(written.name.clone());
            }
            form = target;
        } else {
            if model_aliases.contains(&written.name) && name.is_none() {
                name = Some(written.name.clone());
            }
            break;
        }
    }
    (form, name)
}

fn member_tree<'a>(
    form: &'a TypeForm,
    names: &impl TypeNames,
    aliases: &BTreeMap<&str, &'a TypeForm>,
    model_aliases: &BTreeSet<String>,
) -> Option<AliasTree> {
    let whole = resolve_form(names, form).ok()?;
    let (value_type, form) = match &whole {
        ValueType::Collection(collection) => {
            let (expanded, _) = expanded(form, aliases, model_aliases);
            (collection.element(), expanded.arguments.first())
        }
        ValueType::Reference(_) => return None,
        _ => (&whole, Some(form)),
    };
    let mut tree = AliasTree {
        nodes: vec![AliasNode::default()],
    };
    let mut pending = vec![(0, TypeView::Value(value_type), form)];
    while let Some((index, view, form)) = pending.pop() {
        let (form, alias) = form.map_or((None, None), |form| {
            let (form, alias) = expanded(form, aliases, model_aliases);
            (Some(form), alias)
        });
        if let Some(node) = tree.nodes.get_mut(index) {
            node.alias = alias;
        }
        for (position, child) in view.children().into_iter().enumerate() {
            let child_form = form.and_then(|form| match (&form.head, view) {
                (TypeFormHead::Builtin(BuiltinType::Option), _)
                | (TypeFormHead::Collection(_), _) => form.arguments.get(position),
                (
                    TypeFormHead::Builtin(BuiltinType::Quantity),
                    TypeView::Value(ValueType::CarriedQuantity(_)),
                ) => form.arguments.get(1),
                _ => None,
            });
            let child_index = tree.nodes.len();
            tree.nodes.push(AliasNode::default());
            if let Some(node) = tree.nodes.get_mut(index) {
                node.children.push(child_index);
            }
            pending.push((child_index, child, child_form));
        }
    }
    Some(tree)
}
