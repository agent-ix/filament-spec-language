// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared structural traversal with each synthesis context's finite-domain policy.

use filament_exact::{NodeKey, ValueType};
use filament_semantic_value::declaration::{CompositeShape, TypeEnvironment};
use std::collections::BTreeSet;

/// The value set a checking rule is asking to enumerate.
#[derive(Clone, Copy)]
pub(crate) enum DomainContext {
    /// Standalone value synthesis preserves its existing domain restrictions.
    ValueSynthesis,
    /// References range over the current model state. Other bounded families
    /// may be unsupported by planning without being semantically unbounded.
    PlanParameter,
}

impl DomainContext {
    fn unbounded_leaf(self, ty: &ValueType) -> Option<&'static str> {
        match ty {
            ValueType::Integer => Some("Integer"),
            ValueType::String => Some("String"),
            ValueType::Bytes => Some("Bytes"),
            ValueType::JsonObject => Some("JsonObject"),
            ValueType::Rational(_) => matches!(self, Self::ValueSynthesis).then_some("Rational"),
            ValueType::Quantity(_)
            | ValueType::CarriedQuantity(_)
            | ValueType::BoundedQuantity(_) => {
                matches!(self, Self::ValueSynthesis).then_some("Quantity")
            }
            ValueType::Reference(_) | ValueType::ClockReference | ValueType::Population(None) => {
                matches!(self, Self::ValueSynthesis).then_some("unbounded population")
            }
            ValueType::Boolean
            | ValueType::Int(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Text(_)
            | ValueType::Uuid
            | ValueType::Timestamp
            | ValueType::Enum(_)
            | ValueType::Population(Some(_))
            | ValueType::Option(_)
            | ValueType::Collection(_)
            | ValueType::Map(_)
            | ValueType::Composite(_) => None,
        }
    }

    fn union_position(self, name: &str, index: usize) -> String {
        match self {
            Self::ValueSynthesis => format!("{name}[{index}]"),
            Self::PlanParameter => format!("{name}.{index}"),
        }
    }
}

/// Find the first offending component in declaration order. The structural
/// walk, first-back-edge rule and ordering are shared; leaf classification
/// and historical diagnostic path spelling are explicitly context-specific.
pub(crate) fn first_unbounded_component(
    types: &TypeEnvironment,
    value_type: &ValueType,
    context: DomainContext,
) -> Option<String> {
    enum Work<'a> {
        Visit(&'a ValueType, String),
        Leave(NodeKey),
    }
    let root = match context {
        DomainContext::ValueSynthesis => "domain",
        DomainContext::PlanParameter => "",
    };
    let mut pending = vec![Work::Visit(value_type, root.to_owned())];
    let mut visiting = BTreeSet::new();
    while let Some(work) = pending.pop() {
        let (ty, path) = match work {
            Work::Visit(ty, path) => (ty, path),
            Work::Leave(key) => {
                visiting.remove(&key);
                continue;
            }
        };
        let member = |name: &str| {
            if path.is_empty() {
                name.to_owned()
            } else {
                format!("{path}.{name}")
            }
        };
        if let Some(leaf) = context.unbounded_leaf(ty) {
            return Some(member(leaf));
        }
        match ty {
            ValueType::Option(inner) => pending.push(Work::Visit(inner, member("some"))),
            ValueType::Collection(collection) => {
                if collection.bound().is_none() {
                    return Some(member("unbounded collection"));
                }
                pending.push(Work::Visit(collection.element(), member("element")));
            }
            ValueType::Map(map) => {
                if map.bound().is_none() {
                    return Some(member("unbounded map"));
                }
                pending.push(Work::Visit(map.values(), member("value")));
                pending.push(Work::Visit(map.key(), member("key")));
            }
            ValueType::Composite(key) => {
                if !visiting.insert(key.clone()) {
                    return Some(member("recursive composite"));
                }
                let Some(declaration) = types.composite(key.clone()) else {
                    return Some(member("unknown composite"));
                };
                pending.push(Work::Leave(key.clone()));
                let children: Vec<_> = match declaration.shape() {
                    CompositeShape::Record(fields) => fields
                        .iter()
                        .map(|field| (member(field.name()), field.value_type()))
                        .collect(),
                    CompositeShape::Tuple(items) => items
                        .iter()
                        .enumerate()
                        .map(|(index, ty)| (member(&index.to_string()), ty))
                        .collect(),
                    CompositeShape::Union(cases) => cases
                        .iter()
                        .flat_map(|case| {
                            case.positions().iter().enumerate().map(move |(index, ty)| {
                                (
                                    member(&context.union_position(case.identifier(), index)),
                                    ty,
                                )
                            })
                        })
                        .collect(),
                };
                pending.extend(
                    children
                        .into_iter()
                        .rev()
                        .map(|(path, ty)| Work::Visit(ty, path)),
                );
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{DomainContext, first_unbounded_component};
    use filament_exact::{
        EffectiveId, Integer, IntegerInterval, OriginalDeclarationKey, RationalDomain, ValueType,
    };
    use filament_semantic_value::declaration::TypeEnvironment;
    use ix_trace_rs::trace;

    #[test]
    #[trace("FR-1013", "FR-754-AC-2", "FSpec-FR-806-AC-1")]
    fn planning_and_value_synthesis_keep_their_distinct_leaf_policies() {
        let types = TypeEnvironment::default();
        let rational = ValueType::Rational(
            RationalDomain::new(
                IntegerInterval::new(Integer::from(0_i64), Integer::from(2_i64)).unwrap(),
                IntegerInterval::new(Integer::from(1_i64), Integer::from(2_i64)).unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(
            first_unbounded_component(&types, &rational, DomainContext::PlanParameter),
            None
        );
        assert_eq!(
            first_unbounded_component(&types, &rational, DomainContext::ValueSynthesis),
            Some("domain.Rational".to_owned())
        );
        let reference = ValueType::option(ValueType::Reference(EffectiveId::new(
            None,
            OriginalDeclarationKey::new("test", "ix://test/Object"),
        )));
        assert_eq!(
            first_unbounded_component(&types, &reference, DomainContext::PlanParameter),
            None
        );
        assert_eq!(
            first_unbounded_component(&types, &reference, DomainContext::ValueSynthesis),
            Some("domain.some.unbounded population".to_owned())
        );
        let integer = ValueType::option(ValueType::Integer);
        assert_eq!(
            first_unbounded_component(&types, &integer, DomainContext::PlanParameter),
            Some("some.Integer".to_owned())
        );
    }
}
