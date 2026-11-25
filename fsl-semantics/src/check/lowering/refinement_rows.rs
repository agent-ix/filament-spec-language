// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-375: lowering a checked refinement's authored rows to the node
//! keys and terms its V2 body names.
//!
//! Each row expression is lowered as a state clause's body is (FR-341): one
//! `parameter` node per binder, `self` first at level 0, then a step's
//! operation parameters, each read through a `reference`. A row's nodes are
//! located in its expression; every declaration a row names that no other
//! declaration located takes one `generated` occurrence at the refinement.

use filament_checked::key::DeclarationKey;
use filament_exact::NodeKey;
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use ix_trace_rs::implements;

use super::state::AnchorInput;
use super::{Binder, Binders, Lowering, fault};
use crate::check::claims::BinderSite;
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{BindingValue, BodyTerm, LeafTerm, MemberTerm};
use crate::check::refinement::{
    CheckedRefinement, CheckedRow, KeyedCatalogClass, KeyedFairnessRow, KeyedHistoryRow,
    KeyedObjectRow, KeyedPopulationRow, KeyedRefinementRows, KeyedStepRow, KeyedStepTarget,
    StepClass, StepTarget,
};
use crate::check::refusal::{CheckRefusal, KeyFault};
use filament_exact::{Value, ValueType};
use filament_semantic_value::fairness::FairnessConstraint;

impl Lowering<'_> {
    /// `refinement`'s authored rows, keyed. Its class catalog, when it has
    /// one, is already keyed: a protocol or memory step's node is its
    /// catalog entry's. `location` is the refinement declaration's own.
    pub(crate) fn refinement_rows(
        &mut self,
        refinement: &CheckedRefinement,
        location: &Location,
    ) -> Result<KeyedRefinementRows, CheckRefusal> {
        let catalog = refinement.class_catalog_keys();
        let mut rows = KeyedRefinementRows::default();
        for row in refinement.populations() {
            rows.populations.push(KeyedPopulationRow {
                abstract_population: self.located(location, |this| {
                    this.population_reference_node(&row.abstract_population, location)
                })?,
                concrete_population: self.located(location, |this| {
                    this.population_reference_node(&row.concrete_population, location)
                })?,
            });
        }
        for row in refinement.objects() {
            let abstract_type = self.located(location, |this| {
                this.model_node(&row.abstract_type, location)
            })?;
            let concrete_type = self.located(location, |this| {
                this.model_node(&row.concrete_type, location)
            })?;
            let mut fields = Vec::with_capacity(row.fields.len());
            for field in &row.fields {
                let (_, name) = self.frame_field(&field.field, location)?;
                fields.push((name, self.row_term(&field.expression)?));
            }
            rows.objects.push(KeyedObjectRow {
                abstract_type,
                concrete_type,
                fields,
            });
        }
        for row in refinement.history() {
            let concrete_type =
                self.located(location, |this| this.model_node(&row.object_type, location))?;
            let type_node =
                self.located(location, |this| this.type_node(&row.declared, location))?;
            let initial = self.history_initial(&row.initial, &row.declared, location)?;
            let mut updates = Vec::with_capacity(row.updates.len());
            for update in &row.updates {
                updates.push((
                    self.operation_anchor_of(&update.operation, location)?,
                    self.row_term(&update.expression)?,
                ));
            }
            rows.histories.push(KeyedHistoryRow {
                concrete_type,
                name: row.field.clone(),
                type_node,
                initial,
                updates,
            });
        }
        for (source, row) in refinement.steps().iter().enumerate() {
            let source_node = match (&row.class, catalog) {
                (_, Some(catalog)) => catalog_node(catalog.concrete(), row.spelling(), location)?,
                (StepClass::Operation(operation), None) => {
                    Some(self.operation_anchor_of(operation, location)?)
                }
                (_, None) => return Err(rows_fault(location, "a-protocol-row-has-a-catalog")),
            };
            let target = match &row.target {
                StepTarget::Abstract {
                    operation,
                    target_type,
                    receiver,
                    arguments,
                    ..
                } => KeyedStepTarget::Operation {
                    operation: self.operation_anchor_of(operation, location)?,
                    target_type: self
                        .located(location, |this| this.model_node(target_type, location))?,
                    receiver: self.row_term(receiver)?,
                    arguments: self.row_arguments(arguments)?,
                },
                StepTarget::Node {
                    protocol,
                    node,
                    arguments,
                } => {
                    let class = format!("{protocol}::{node}");
                    let Some(catalog) = catalog else {
                        return Err(rows_fault(location, "a-protocol-target-has-a-catalog"));
                    };
                    KeyedStepTarget::ProtocolStep {
                        node: catalog_node(catalog.abstract_side(), &class, location)?,
                        class,
                        arguments: self.row_arguments(arguments)?,
                    }
                }
                StepTarget::Stutter => KeyedStepTarget::Stutter,
                StepTarget::Any => KeyedStepTarget::Any,
            };
            rows.steps.push(KeyedStepRow {
                source_class: row.spelling().to_owned(),
                source_node,
                target,
                source,
            });
        }
        for (side, fairness) in [
            (
                &mut rows.assume,
                refinement.concrete_operation_fairness().collect::<Vec<_>>(),
            ),
            (
                &mut rows.ensure,
                refinement.abstract_operation_fairness().collect::<Vec<_>>(),
            ),
        ] {
            for constraint in fairness {
                side.push(self.fairness_row(constraint, location)?);
            }
        }
        Ok(rows)
    }

    /// FR-144: the `Boolean` type node the refinement's requirement record
    /// is result-bounded at, with a `generated` occurrence at the
    /// refinement's declaration `location` when no other declaration locates
    /// it.
    pub(crate) fn refinement_result_type(
        &mut self,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        self.located(location, |this| {
            this.type_node(&filament_exact::ValueType::Boolean, location)
        })
    }

    /// `key`, built by `node`, with a `generated` occurrence at `location`
    /// when no declaration has located it.
    fn located(
        &mut self,
        location: &Location,
        node: impl FnOnce(&mut Self) -> Result<NodeKey, CheckRefusal>,
    ) -> Result<NodeKey, CheckRefusal> {
        let key = node(self)?;
        if !self.occurrences.has(key.clone()) {
            self.record(key.clone(), OccurrenceRole::Generated, location.clone());
        }
        Ok(key)
    }

    /// The lowered initial term of a history field, typed at its declared
    /// type and located at the refinement. An initial that is not a present
    /// option is an inline `literal` term; a present option is an
    /// `option_value` node around its recursively lowered payload.
    #[implements("FR-901-AC-1")]
    fn history_initial(
        &mut self,
        value: &Value,
        declared: &ValueType,
        location: &Location,
    ) -> Result<LeafTerm, CheckRefusal> {
        let mut current_value = value;
        let mut current_type = declared;
        let mut option_types = Vec::new();
        while let (Value::Option(option), ValueType::Option(payload_type)) =
            (current_value, current_type)
        {
            let Some(payload) = option.payload() else {
                break;
            };
            option_types.push(current_type);
            current_value = payload;
            current_type = payload_type;
        }
        // A `reference` initial is an option-typed history's present value,
        // naming its `option_value` node; every other initial is the row's
        // own `literal` term (FSpec FR-375 `histories`). Inside a present
        // option an enum member is inline too, its node being an
        // `enum_value`; any other payload is a node the `option_value` names.
        let mut term = if option_types.is_empty() || matches!(current_value, Value::Enum(_)) {
            let literal = Self::literal_value(location, current_value, None)?;
            let ty = self
                .type_ref(current_type, location)?
                .admitted()
                .cloned()
                .ok_or_else(|| fault(location, KeyFault::UnresolvedDraft))?;
            LeafTerm::literal(ty, literal)
        } else {
            self.literal_at(location, current_value, current_type, None)?
        };
        for option_type in option_types.into_iter().rev() {
            let semantic_type = self.type_ref(option_type, location)?;
            term = self.value_node_at(
                location,
                "option_value",
                semantic_type,
                BodyTerm::aggregate(vec![MemberTerm::binding("value", BindingValue::Leaf(term))]),
            )?;
        }
        Ok(term)
    }

    /// The lowered term of one row expression, over one `parameter` node per
    /// binder of its context.
    fn row_term(&mut self, row: &CheckedRow) -> Result<LeafTerm, CheckRefusal> {
        let location = Location::root(Origin::TypeDeclaration {
            name: row.site.clone(),
        });
        let mut scope = Vec::with_capacity(row.parameters.len());
        for (level, (name, value_type)) in row.parameters.iter().enumerate() {
            let type_key = self.binder_type(value_type, None, &location)?;
            let key = self.parameter(name, level, type_key.clone(), &location)?;
            self.record(type_key.clone(), OccurrenceRole::Type, location.clone());
            self.record_binder(
                BinderSite {
                    binder: location.clone(),
                    slot: level,
                },
                key.clone(),
            );
            scope.push(Binder {
                slot: level,
                parameter: key.into(),
                semantic_type: type_key.into(),
            });
        }
        let mut binders = Binders::new(&row.slot_names, &scope);
        self.expression(row.body(), &mut binders)
    }

    /// A step target's arguments, `_` as `None`.
    fn row_arguments(
        &mut self,
        arguments: &[Option<CheckedRow>],
    ) -> Result<Vec<Option<LeafTerm>>, CheckRefusal> {
        arguments
            .iter()
            .map(|argument| argument.as_ref().map(|row| self.row_term(row)).transpose())
            .collect()
    }

    /// The `operation_anchor` node of the model operation `operation`.
    fn operation_anchor_of(
        &mut self,
        operation: &DeclarationKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let (_, model) = self.model_owner(operation, location)?;
        let (declaring, name) = model
            .operation_anchor_declaration(operation)
            .ok_or_else(|| fault(location, KeyFault::UnknownDeclaration(operation.clone())))?;
        self.anchor_by_name(declaring, &name, location)
    }

    /// The `operation_anchor` node of the operation `name` that `declaring`
    /// declares.
    fn anchor_by_name(
        &mut self,
        declaring: filament_exact::EffectiveId,
        name: &str,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let scope = self.scope;
        let operation = scope
            .operations()
            .declared_by(declaring.clone())
            .iter()
            .find(|operation| operation.name() == name)
            .ok_or_else(|| rows_fault(location, "a-row-operation-is-declared"))?;
        let anchor = AnchorInput {
            declaring,
            operation,
            location,
            reference: location,
        };
        let binding = self.operation_anchor(&anchor, None, location)?;
        // An `operation_anchor`'s every occurrence is an `anchor` (FR-322).
        if !self.occurrences.has(binding.anchor.clone()) {
            self.record(
                binding.anchor.clone(),
                OccurrenceRole::Anchor,
                location.clone(),
            );
        }
        Ok(binding.anchor)
    }

    fn fairness_row(
        &mut self,
        constraint: &FairnessConstraint,
        location: &Location,
    ) -> Result<KeyedFairnessRow, CheckRefusal> {
        let operation = constraint.operation();
        Ok(KeyedFairnessRow {
            operation: self.anchor_by_name(
                operation.declaring_type(),
                operation.name(),
                location,
            )?,
            kind: constraint.kind(),
            granularity: constraint.granularity(),
        })
    }
}

/// The node of the catalog entry spelled `class` on one side: `None` for a
/// memory class, which names no declaration.
fn catalog_node(
    side: &[KeyedCatalogClass],
    class: &str,
    location: &Location,
) -> Result<Option<NodeKey>, CheckRefusal> {
    match side.iter().find(|entry| entry.class() == class) {
        Some(KeyedCatalogClass::Declared { node, .. }) => Ok(Some(node.clone())),
        Some(KeyedCatalogClass::Memory(_)) => Ok(None),
        None => Err(rows_fault(location, "a-row-class-is-catalogued")),
    }
}

fn rows_fault(location: &Location, invariant: &'static str) -> CheckRefusal {
    fault(
        location,
        KeyFault::StageFault(filament_semantic_value::catalog::InternalFault::new(
            "check.refinement-rows",
            invariant,
        )),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use filament_exact::{Integer, IntegerInterval, Meter, OptionValue};
    use serde_json::json;

    use super::*;
    use crate::check::family::fixtures::{empty_scope, fixture_source};
    use crate::check::family::OccurrenceMap;
    use crate::check::lowering::generated_location;
    use crate::check::lowering::tests::view::preimage;
    use crate::check::node_key::SourceOwner;

    /// A present option holding an absent one, of type
    /// `Option<Option<Int[0, 1]>>`, is an `option_value` node whose member
    /// refers to a `none` `literal` node of the inner option type: only a
    /// row's outermost initial is inline. No source spelling writes this
    /// value, so the test lowers it directly.
    #[test]
    fn an_absent_payload_inside_a_present_option_is_a_node_the_option_value_names() {
        let scope = empty_scope();
        let owner = SourceOwner::from(&fixture_source());
        let mut occurrences = OccurrenceMap::default();
        let mut meter = Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
        let mut lowering = Lowering::new_with_source_owners(
            &scope,
            &owner,
            &std::collections::BTreeMap::new(),
            &[],
            &[],
            scope.types().units().clone(),
            None,
            0,
            &mut occurrences,
            &mut meter,
            "fixture",
        );
        let location = generated_location();
        let int = ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(1_i64)).expect("an interval"),
        );
        let inner = ValueType::Option(Arc::new(int.clone()));
        let declared = ValueType::Option(Arc::new(inner.clone()));
        let initial = OptionValue::present(inner.clone(), OptionValue::none(int))
            .expect("a present option of an absent one");

        let term = lowering
            .history_initial(&initial, &declared, &location)
            .expect("the initial lowers");

        let declared_node = lowering
            .type_node(&declared, &location)
            .expect("the declared type keys");
        let inner_node = lowering
            .type_node(&inner, &location)
            .expect("the inner type keys");
        let graph = lowering.finish(&location).expect("lowering finishes").graph;
        let node = |term: &LeafTerm| {
            let LeafTerm::Reference { target } = term else {
                panic!("a reference to a node, not {term:?}");
            };
            let key = target.admitted().expect("an admitted key").clone();
            preimage(graph.node(key).expect("the referenced node"))
        };
        let outer = node(&term);
        assert_eq!(outer["semantic_form"], "option_value", "{outer}");
        assert_eq!(
            outer["semantic_type"]["digest"],
            json!(declared_node.to_string())
        );
        let member = &outer["body"]["members"][0];
        assert_eq!(member["name"], "value", "{outer}");
        assert_eq!(member["value"]["term"], "reference", "{outer}");
        let payload = graph
            .nodes()
            .find(|candidate| {
                member["value"]["target"]["digest"] == json!(candidate.key().to_string())
            })
            .map(preimage)
            .expect("the member names a node of this graph");
        assert_eq!(payload["semantic_form"], "literal", "{payload}");
        assert_eq!(
            payload["semantic_type"]["digest"],
            json!(inner_node.to_string())
        );
        assert_eq!(payload["body"]["value_kind"], "none", "{payload}");
        assert_eq!(payload["body"]["value"], serde_json::Value::Null);
    }
}
