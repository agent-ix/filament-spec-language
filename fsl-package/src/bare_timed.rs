// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940, FR-120: the bounded dense-time model reader over admitted nodes.

use std::collections::BTreeSet;

use filament_contract_model::{CheckedNodeId, CheckedNodeTag, CheckedOccurrenceRole};
use filament_exact::{Integer, Rational, ValueType};
use fsl_semantics::check::{
    CheckedTimedPredicate, ClauseOperation, ClockConstraint, ClockGuard, ClockId, ClockOp,
    ClockReset, ClockResets, ConvexPart, ModelTimeSource, PackagePredicate, StateAlternative,
    TimeInvariant, TimeSourceKind, TimedModel,
};
use fsl_semantics::model::state::StateModel;
use std::str::FromStr;

use crate::{
    bare_slot::BareSelectionFailure,
    bare_state_graph::BareStatePredicate,
    bare_term::{fault, walk_failure},
    protocol_selection::SlotSelectionFailure,
    protocol_view::{group_member, text},
    reader_protocol_projection::BareWalker,
    BarePackage, CheckedConstraintTarget, CheckedResetTarget, CheckedTimeSourceKind,
    CheckedTimedModel, ClockComparison, ClockPathStep,
};
use ix_trace_rs::implements;

/// Why one admitted dense timed model cannot be reconstructed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BareTimedModelFailure {
    /// The node is not in the directly supported dense timed subset.
    #[error("timed model form at {0:?} is not directly readable")]
    Unread(CheckedNodeId),
    /// A timed operation has more than one matching precondition clause.
    #[error("timed operation at {node:?} has {count} matching preconditions")]
    MultiplePreconditions {
        /// The timed operation anchor whose precondition join was ambiguous.
        node: CheckedNodeId,
        /// Number of matching clauses.
        count: usize,
    },
    /// A timed reset has more than one matching postcondition clause.
    #[error("timed operation at {node:?} has {count} matching postconditions")]
    MultiplePostconditions {
        /// The timed operation anchor whose postcondition join was ambiguous.
        node: CheckedNodeId,
        /// Number of matching clauses.
        count: usize,
    },
}

impl BarePackage<'_> {
    /// Project an admitted timed predicate and its ordered parameter nodes
    /// into the reader evaluator's shared expression representation.
    #[implements("FR-940-AC-39")]
    pub fn project_timed_predicate(
        &self,
        predicate: &PackagePredicate,
    ) -> Result<BareStatePredicate, BareSelectionFailure> {
        let package = self.package();
        let (_, identities) = self.witness()?;
        let walker = BareWalker::new(
            package,
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
            self.dependency_views(),
        )
        .map_err(SlotSelectionFailure::Fault)?;
        if predicate.slots() < predicate.binders().len() {
            return Err(fault("timed-predicate-has-all-binder-slots"));
        }
        let owner = match predicate.state_clause_owner() {
            Some((clause, index)) => {
                filament_semantic_value::location::ExpressionOwner::StateClause {
                    clause: clause.to_owned(),
                    index,
                }
            }
            None => filament_semantic_value::location::ExpressionOwner::TypeDeclaration {
                name: predicate
                    .type_declaration_owner()
                    .unwrap_or("timed model predicate")
                    .to_owned(),
            },
        };
        let projection = walker
            .project_term(predicate.node(), predicate.binders(), owner)
            .map_err(walk_failure)?;
        let parameters = predicate
            .binders()
            .iter()
            .map(|binder| {
                projection
                    .local_slot(binder)
                    .map(|slot| (String::new(), slot))
                    .ok_or_else(|| fault("timed-predicate-binder-has-a-local-slot"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if projection.frame_slots() < predicate.slots() {
            return Err(fault("timed-predicate-projection-has-all-slots"));
        }
        Ok(BareStatePredicate::new(parameters, projection))
    }

    /// Read one admitted timed predicate into the shared semantic timed
    /// predicate representation.
    #[implements("FR-940-AC-39")]
    pub fn checked_timed_predicate(
        &self,
        expression: filament_exact::NodeKey,
        binders: Vec<filament_exact::NodeKey>,
        parameter_types: Vec<ValueType>,
        state_clause_owner: Option<(String, usize)>,
        type_declaration_owner: Option<String>,
    ) -> Result<CheckedTimedPredicate, BareSelectionFailure> {
        if binders.len() != parameter_types.len() {
            return Err(fault("timed-predicate-types-match-binders"));
        }
        let mut provisional =
            PackagePredicate::new(expression.clone(), binders.len()).with_binders(binders.clone());
        if let Some((clause, index)) = &state_clause_owner {
            provisional = provisional.with_state_clause_owner(clause.clone(), *index);
        }
        if let Some(owner) = &type_declaration_owner {
            provisional = provisional.with_type_declaration_owner(owner.clone());
        }
        let projected = self.project_timed_predicate(&provisional)?;
        let mut parameters = Vec::with_capacity(binders.len());
        for (binder, value_type) in binders.iter().zip(parameter_types) {
            let node = self
                .node_by_key(binder)?
                .ok_or_else(|| fault("timed-predicate-binder-is-admitted"))?;
            let name = group_member(&node.body, "name")
                .and_then(text)
                .ok_or_else(|| fault("timed-predicate-binder-has-a-name"))?
                .to_owned();
            if projected.projection().local_slot(binder).is_none() {
                return Err(fault("timed-predicate-binder-has-a-local-slot"));
            }
            parameters.push((name, value_type));
        }
        let reads_self = projected
            .parameters()
            .first()
            .is_some_and(|(_, slot)| projected.projection().reads_local(*slot));
        let mut package = PackagePredicate::new(expression, projected.projection().frame_slots())
            .with_binders(binders);
        if let Some((clause, index)) = state_clause_owner {
            package = package.with_state_clause_owner(clause, index);
        }
        if let Some(owner) = type_declaration_owner {
            package = package.with_type_declaration_owner(owner);
        }
        Ok(CheckedTimedPredicate::from_package(
            parameters, package, reads_self,
        ))
    }

    /// Read every timed node into dense delay tables. Only a source-owned
    /// base Time dimension, direct non-affine unit and required direct clock
    /// fields are supported. The error identifies an unread timed node; no
    /// partially reconstructed tables are returned.
    #[implements("FR-940-AC-39")]
    pub fn direct_dense_tables(
        &self,
        state: &StateModel,
    ) -> Result<(Vec<TimedModel>, Vec<ModelTimeSource>), BareTimedModelFailure> {
        let timed = self
            .package()
            .resolved_graph()
            .nodes
            .iter()
            .filter(|node| {
                CheckedNodeTag::from_wire(&node.node_tag) == Some(CheckedNodeTag::TimedModel)
            })
            .collect::<Vec<_>>();
        let Some(first) = timed.first() else {
            return Ok((Vec::new(), Vec::new()));
        };
        let failure = || BareTimedModelFailure::Unread(first.node_id.clone());
        let roots = CheckedTimedModel::from_admitted(self.package()).map_err(|_| failure())?;
        let declarations = self.clock_declarations().map_err(|_| failure())?;
        let state_clauses = self.state_clause_declarations().map_err(|_| failure())?;
        let operation_table = state
            .operation_table(&std::collections::BTreeMap::new())
            .map_err(|_| failure())?;
        let mut consumed = BTreeSet::new();
        let mut selections = BTreeSet::new();
        let mut models = Vec::new();
        let mut sources = Vec::new();
        for root in roots {
            let bad = || BareTimedModelFailure::Unread(root.metadata.node_id.clone());
            if root.source.kind != CheckedTimeSourceKind::Dense || !root.urgencies.is_empty() {
                return Err(bad());
            }
            let model_node = self.node_key(&root.model).ok_or_else(bad)?;
            let model = state.direct_dense_model(&model_node).ok_or_else(bad)?;
            if !selections.insert(model.model().to_owned()) {
                return Err(bad());
            }
            let unit_node = self.node_key(&root.source.unit).ok_or_else(bad)?;
            let time_dimension = self.node_key(&root.time_dimension).ok_or_else(bad)?;
            let (unit, nominal_nodes) = self
                .units
                .direct_time_nodes(self.package(), &unit_node, &time_dimension)
                .ok_or_else(bad)?;
            let mut regions = self
                .package()
                .source_map()
                .iter()
                .filter(|entry| {
                    entry.node_id == root.source.metadata.node_id
                        && entry.role == CheckedOccurrenceRole::Expression
                })
                .flat_map(|entry| &entry.regions);
            let region = regions.next().ok_or_else(bad)?;
            if regions.next().is_some() {
                return Err(bad());
            }
            let region = crate::checked_v2::source_region(region).map_err(|_| bad())?;
            let Some(filament_contract_model::CheckedNodeOwner::Source {
                authority,
                identity,
            }) = root.metadata.owner.as_ref()
            else {
                return Err(bad());
            };
            if region.source().authority() != authority.as_ref()
                || region.source().identity() != identity.as_ref()
            {
                return Err(bad());
            }
            let source = ModelTimeSource {
                model: model.model().to_owned(),
                model_node: model_node.clone(),
                span: fsl_foundation::Span {
                    start: usize::try_from(region.start()).map_err(|_| bad())?,
                    end: usize::try_from(region.end()).map_err(|_| bad())?,
                },
                declared_in: Some(region.source().clone()),
                tick_operation_key: None,
                tick_operation_span: None,
                tick_operation_anchor: None,
                source: TimeSourceKind::Dense,
                unit,
                unit_node,
                time_dimension,
                nominal_nodes,
            };
            let mut places = Vec::new();
            let mut place_clocks = std::collections::BTreeMap::new();
            let mut joined = BTreeSet::new();
            for clock in model.clocks() {
                let template = declarations
                    .source()
                    .clock_template(clock)
                    .map_err(|_| bad())?;
                let origin = declarations
                    .source()
                    .clock_origin(clock)
                    .map_err(|_| bad())?;
                let mut matching = Vec::new();
                for place in &root.clock_places {
                    if place.model != root.model
                        || !matches!(place.path.as_slice(), [ClockPathStep::Field(_)])
                    {
                        return Err(BareTimedModelFailure::Unread(
                            place.metadata.node_id.clone(),
                        ));
                    }
                    if declarations.clock_template(place).map_err(|_| bad())? == template
                        && declarations.clock_origin(place).map_err(|_| bad())? == origin
                    {
                        matching.push(place);
                    }
                }
                let [place] = matching.as_slice() else {
                    return Err(bad());
                };
                if !joined.insert(place.metadata.node_id.clone()) {
                    return Err(bad());
                }
                places.push(self.node_key(&place.metadata.node_id).ok_or_else(bad)?);
                place_clocks.insert(
                    place.metadata.node_id.clone(),
                    ClockId::from_index(places.len() - 1),
                );
            }
            if joined.len() != root.clock_places.len() {
                return Err(bad());
            }
            let constraint = |id: &CheckedNodeId| {
                let checked = root
                    .constraints
                    .iter()
                    .find(|constraint| &constraint.metadata.node_id == id)
                    .ok_or_else(bad)?;
                let CheckedConstraintTarget::Place { place, slots, .. } = &checked.target else {
                    return Err(bad());
                };
                if !slots.is_empty() {
                    return Err(bad());
                }
                let clock = *place_clocks.get(place).ok_or_else(bad)?;
                let operation = match checked.comparison {
                    ClockComparison::Lt => ClockOp::Less,
                    ClockComparison::Le => ClockOp::LessOrEqual,
                    ClockComparison::Eq => ClockOp::Equal,
                    ClockComparison::Ge => ClockOp::GreaterOrEqual,
                    ClockComparison::Gt => ClockOp::Greater,
                };
                let bound = package_rational(&checked.bound).ok_or_else(bad)?;
                Ok(ClockConstraint::from_package_parts(
                    clock,
                    Vec::new(),
                    Vec::new(),
                    None,
                    operation,
                    bound,
                    self.node_key(&checked.metadata.node_id).ok_or_else(bad)?,
                    None,
                ))
            };
            let operation = |anchor: &CheckedNodeId,
                             observation: fsl_semantics::model::clause_observation::Observation|
             -> Result<
                (
                    ClauseOperation,
                    usize,
                    String,
                    filament_exact::EffectiveId,
                    filament_exact::NodeKey,
                ),
                BareTimedModelFailure,
            > {
                let frame = self.protocol_operation(anchor).map_err(|_| bad())?;
                let matching = state_clauses
                    .iter()
                    .enumerate()
                    .filter(|(_, clause)| {
                        clause.operation.as_ref().is_some_and(|operation| {
                            operation.declaring == frame.declaring
                                && operation.operation == frame.operation
                        }) && clause.observation == observation
                    })
                    .collect::<Vec<_>>();
                let [(index, clause)] = matching.as_slice() else {
                    if matching.len() > 1 {
                        return Err(match observation {
                            fsl_semantics::model::clause_observation::Observation::Pre => {
                                BareTimedModelFailure::MultiplePreconditions {
                                    node: anchor.clone(),
                                    count: matching.len(),
                                }
                            }
                            fsl_semantics::model::clause_observation::Observation::Post => {
                                BareTimedModelFailure::MultiplePostconditions {
                                    node: anchor.clone(),
                                    count: matching.len(),
                                }
                            }
                            fsl_semantics::model::clause_observation::Observation::Current => {
                                BareTimedModelFailure::Unread(root.metadata.node_id.clone())
                            }
                        });
                    }
                    return Err(bad());
                };
                let declaration = operation_table
                    .declared_by(frame.declaring.clone())
                    .iter()
                    .find(|declaration| declaration.name() == frame.operation)
                    .cloned()
                    .ok_or_else(bad)?;
                Ok((
                    ClauseOperation {
                        declaring: frame.declaring,
                        declaration,
                    },
                    *index,
                    clause.name.clone(),
                    clause.context.clone(),
                    clause.identity.clone(),
                ))
            };
            let mut guards = Vec::new();
            for guard in &root.guards {
                if guard.point_anchor.is_some() {
                    return Err(bad());
                }
                let (operation, index, clause, context, clause_node) = operation(
                    &guard.operation,
                    fsl_semantics::model::clause_observation::Observation::Pre,
                )?;
                let mut parts = Vec::with_capacity(guard.parts.len());
                for part in &guard.parts {
                    if !part.binders.is_empty() || part.quantifier.is_some() {
                        return Err(bad());
                    }
                    let predicate = self.node_key(&part.predicate).ok_or_else(bad)?;
                    let state_clause = self
                        .project_state_clause(&clause_node, &clause)
                        .map_err(|_| bad())?;
                    let predicate_owner = (clause.clone(), state_clause.owner_index());
                    let data = self
                        .checked_timed_predicate(
                            predicate,
                            state_clause.binders().to_vec(),
                            std::iter::once(ValueType::Reference(context.clone()))
                                .chain(
                                    operation
                                        .declaration
                                        .parameters()
                                        .iter()
                                        .map(|(_, value_type)| value_type.clone()),
                                )
                                .collect(),
                            Some(predicate_owner),
                            None,
                        )
                        .map_err(|_| bad())?;
                    let clocks = part
                        .constraints
                        .iter()
                        .map(constraint)
                        .collect::<Result<Vec<_>, _>>()?;
                    let part = ConvexPart::from_parts(
                        Some(data),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        false,
                        clocks,
                        Vec::new(),
                    )
                    .ok_or_else(bad)?;
                    parts.push(part);
                }
                guards.push(
                    ClockGuard::from_package_parts(
                        clause,
                        operation,
                        parts,
                        index,
                        context,
                        self.node_key(&guard.metadata.node_id).ok_or_else(bad)?,
                        None,
                    )
                    .ok_or_else(bad)?,
                );
            }
            let mut resets = Vec::new();
            for reset in &root.resets {
                if reset.point_anchor.is_some() {
                    return Err(bad());
                }
                let (operation, index, clause, context, _clause_node) = operation(
                    &reset.operation,
                    fsl_semantics::model::clause_observation::Observation::Post,
                )?;
                let CheckedResetTarget::Place { place, slots, .. } = &reset.target else {
                    return Err(bad());
                };
                if !slots.is_empty() {
                    return Err(bad());
                }
                let clock = *place_clocks.get(place).ok_or_else(bad)?;
                let value = package_rational(&reset.value).ok_or_else(bad)?;
                let reset_origin = self.node_key(&reset.metadata.node_id).ok_or_else(bad)?;
                let reset = ClockReset::from_package_parts(
                    clock,
                    value,
                    reset_origin.clone(),
                    None,
                    None,
                    Vec::new(),
                );
                resets.push(
                    ClockResets::from_package_parts(
                        clause,
                        operation,
                        vec![reset],
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        index,
                        context,
                        reset_origin,
                        None,
                    )
                    .ok_or_else(bad)?,
                );
            }
            let mut invariants = Vec::new();
            let invariant_context = state
                .direct_dense_model_context(&model_node)
                .ok_or_else(bad)?;
            let self_declaration = state_clauses
                .iter()
                .find(|clause| clause.context == invariant_context)
                .ok_or_else(bad)?;
            let self_clause = self
                .project_state_clause(&self_declaration.identity, &self_declaration.name)
                .map_err(|_| bad())?;
            let self_binder = self_clause.binders().first().cloned().ok_or_else(bad)?;
            for invariant in &root.invariants {
                if !invariant.binders.is_empty() {
                    return Err(bad());
                }
                let expression = self.node_key(&invariant.predicate).ok_or_else(bad)?;
                let binders = vec![self_binder.clone()];
                let data = self
                    .checked_timed_predicate(
                        expression,
                        binders,
                        vec![ValueType::Reference(invariant_context.clone())],
                        None,
                        None,
                    )
                    .map_err(|_| bad())?;
                let context = match data.parameters().first() {
                    Some((_, ValueType::Reference(context))) => context.clone(),
                    _ => return Err(bad()),
                };
                let bounds = invariant
                    .bounds
                    .iter()
                    .map(constraint)
                    .collect::<Result<Vec<_>, _>>()?;
                let alternative = StateAlternative::from_parts(Vec::new(), data);
                invariants.push(
                    TimeInvariant::from_package_parts(
                        context,
                        vec![alternative],
                        bounds,
                        Vec::new(),
                        self.node_key(&invariant.metadata.node_id).ok_or_else(bad)?,
                    )
                    .ok_or_else(bad)?,
                );
            }
            let consumed_model_nodes = std::iter::once(root.metadata.node_id.clone())
                .chain(std::iter::once(root.source.metadata.node_id.clone()))
                .chain(
                    root.resets
                        .iter()
                        .map(|reset| reset.metadata.node_id.clone()),
                )
                .chain(
                    root.guards
                        .iter()
                        .map(|guard| guard.metadata.node_id.clone()),
                )
                .chain(
                    root.invariants
                        .iter()
                        .map(|invariant| invariant.metadata.node_id.clone()),
                )
                .chain(
                    root.constraints
                        .iter()
                        .map(|constraint| constraint.metadata.node_id.clone()),
                )
                .chain(joined);
            for id in consumed_model_nodes {
                if !consumed.insert(id) {
                    return Err(bad());
                }
            }
            let model = model.with_package_places(places).ok_or_else(bad)?;
            models.push(
                model
                    .with_package_clauses(guards, resets, invariants)
                    .ok_or_else(bad)?,
            );
            sources.push(source);
        }
        if let Some(node) = timed.iter().find(|node| !consumed.contains(&node.node_id)) {
            return Err(BareTimedModelFailure::Unread(node.node_id.clone()));
        }
        Ok((models, sources))
    }
}

fn package_rational(value: &str) -> Option<Rational> {
    let (numerator, denominator) = value.split_once('/')?;
    Rational::new(
        Integer::from_str(numerator).ok()?,
        Integer::from_str(denominator).ok()?,
    )
    .ok()
}
