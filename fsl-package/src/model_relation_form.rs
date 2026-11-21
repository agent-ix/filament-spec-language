// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: HP-1 form selection from an already admitted package view.

use filament_contract_model::{
    CheckedNodeId, CheckedNodeOwner, CheckedPackageV2, CheckedStepRelationBody, RequirementItemRow,
};
use fsl_foundation::Span;

use crate::ClaimOccurrence;

/// A selected relation subject in witnessed model declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRelationSubject {
    /// The alias already bound by the step relation's subjects.
    pub alias: String,
    /// Its existing model-root identity.
    pub root: CheckedNodeId,
    /// The selected domain package identity that root owns.
    pub identity: String,
}

/// An HP-1 model form containing only admitted package facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageModelRelationForm {
    claim: ClaimOccurrence,
    occurrence: fsl_foundation::source::provenance::OccurrenceKey,
    name: String,
    span: Span,
    subjects: Vec<ModelRelationSubject>,
    body: CheckedStepRelationBody,
    requirement: RequirementItemRow,
    reads: Vec<ModelRelationRead>,
}

impl PackageModelRelationForm {
    /// Its selected claim occurrence.
    pub fn claim(&self) -> &ClaimOccurrence {
        &self.claim
    }
    /// The structural claim key used by the engine and evaluator.
    pub fn occurrence(&self) -> &fsl_foundation::source::provenance::OccurrenceKey {
        &self.occurrence
    }
    /// The declaration's display name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The recorded declaration region.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Subjects in the admitted declaration-order witness.
    pub fn subjects(&self) -> &[ModelRelationSubject] {
        &self.subjects
    }
    /// The reader's existing ordered step-relation body.
    pub fn body(&self) -> &CheckedStepRelationBody {
        &self.body
    }
    /// Each body model read, keyed by its admitted term and observation.
    pub fn reads(&self) -> &[ModelRelationRead] {
        &self.reads
    }
    /// The model capability's own requirement record.
    pub fn requirement(&self) -> &RequirementItemRow {
        &self.requirement
    }
}

/// Selecting an HP-1 form is separate from FR-390 view creation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ModelRelationFormRefusal {
    /// The selected occurrence names no admitted model relation.
    #[error("the selected occurrence names no model relation")]
    UnresolvedRelation,
    /// A legacy package is globally admitted but lacks HP-1 order evidence.
    #[error("the relation lacks subject declaration order evidence")]
    MissingSubjectDeclarationOrder {
        /// The already admitted package identity.
        package: String,
        /// The relation's node locus.
        node: CheckedNodeId,
    },
    /// The admitted writer facts cannot form one relation descriptor.
    #[error("the admitted model relation is malformed")]
    MalformedNode {
        /// The relation's node locus.
        node: CheckedNodeId,
    },
}

/// One model read's execution and snapshot ownership, without source locations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRelationRead {
    /// The admitted expression node carrying the read.
    pub node: CheckedNodeId,
    /// Execution-binding index.
    pub execution: usize,
    /// Current/post or pre observation at this body use.
    pub observation: fsl_semantics::model::clause_observation::Observation,
}

pub(crate) fn model_relation_form(
    package: &CheckedPackageV2,
    claim: &ClaimOccurrence,
) -> Result<PackageModelRelationForm, ModelRelationFormRefusal> {
    if claim.occurrence.role != filament_contract_model::CheckedOccurrenceRole::Claim {
        return Err(ModelRelationFormRefusal::UnresolvedRelation);
    }
    let node = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.node_id == claim.declaration)
        .ok_or(ModelRelationFormRefusal::UnresolvedRelation)?;
    let malformed = || ModelRelationFormRefusal::MalformedNode {
        node: node.node_id.clone(),
    };
    let body = node
        .step_relation_body()
        .ok_or(ModelRelationFormRefusal::UnresolvedRelation)?;
    let order = node
        .step_relation_subject_declaration_order()
        .ok_or_else(
            || ModelRelationFormRefusal::MissingSubjectDeclarationOrder {
                package: package.name().to_owned(),
                node: node.node_id.clone(),
            },
        )?;
    let subjects = order
        .map(|(alias, root)| {
            let selected = package
                .resolved_graph()
                .nodes
                .iter()
                .find(|node| &node.node_id == root)
                .ok_or_else(malformed)?;
            let Some(CheckedNodeOwner::ModelRoot { identity }) = &selected.owner else {
                return Err(malformed());
            };
            Ok(ModelRelationSubject {
                alias: alias.to_owned(),
                root: root.clone(),
                identity: identity.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let row = package
        .claim_names()
        .unwrap_or_default()
        .iter()
        .find(|row| row.declaration == claim.declaration && row.occurrence == claim.occurrence)
        .ok_or_else(malformed)?;
    let requirement = package
        .requirement_items()
        .iter()
        .find(|row| {
            row.claim == claim.declaration
                && row.occurrence == claim.occurrence
                && row.kind == filament_contract_model::CapabilityKind::TemporalSatisfaction
        })
        .cloned()
        .ok_or_else(malformed)?;
    let reads = model_reads(package, &body).ok_or_else(malformed)?;
    let key = crate::checked_v2::wire_graph(package)
        .map_err(|_| malformed())?
        .key(&node.node_id)
        .map_err(|_| malformed())?;
    let occurrence = fsl_foundation::source::provenance::OccurrenceKey::new(
        key,
        filament_exact::Origin::new(filament_exact::Role::new("claim"), claim.occurrence.ordinal),
    );
    Ok(PackageModelRelationForm {
        claim: claim.clone(),
        occurrence,
        name: row.name.to_string(),
        span: Span {
            start: usize::try_from(row.declaration_region.start).unwrap_or(usize::MAX),
            end: usize::try_from(row.declaration_region.end).unwrap_or(usize::MAX),
        },
        subjects,
        body,
        requirement,
        reads,
    })
}

/// A relation's admitted reader projection and the execution slots it binds.
pub struct BareModelRelation {
    parameters: Vec<(String, filament_exact::ValueType)>,
    executions: Vec<(String, fsl_semantics::check::OperationKey)>,
    projection: crate::ReaderProtocolProjection,
    components:
        std::collections::BTreeMap<filament_semantic_value::location::ExpressionLocation, usize>,
}

impl BareModelRelation {
    /// Receiver, result and arguments in execution order.
    pub fn parameters(&self) -> &[(String, filament_exact::ValueType)] {
        &self.parameters
    }
    /// Each execution's admitted operation identity.
    pub fn executions(&self) -> &[(String, fsl_semantics::check::OperationKey)] {
        &self.executions
    }
    /// The one existing package expression projection.
    pub fn projection(&self) -> &crate::ReaderProtocolProjection {
        &self.projection
    }
    /// The unique execution owning each model-read location.
    pub fn components(
        &self,
    ) -> &std::collections::BTreeMap<filament_semantic_value::location::ExpressionLocation, usize>
    {
        &self.components
    }
}

impl crate::BarePackage<'_> {
    /// Project an admitted HP-1 body without constructing a checked source body.
    ///
    /// # Errors
    ///
    /// A missing supply witness, malformed model binding or unsupported term.
    #[ix_trace_rs::string_edge]
    pub fn project_model_relation(
        &self,
        form: &PackageModelRelationForm,
    ) -> Result<BareModelRelation, crate::BareSelectionFailure> {
        use crate::protocol_selection::SlotSelectionFailure;
        use crate::protocol_view::{group_member, text};
        use filament_contract_model::CheckedTerm;
        use filament_exact::{Identifier, ValueType};
        let key = self
            .node_key(&form.claim.declaration)
            .ok_or_else(|| crate::bare_term::fault("relation-has-node-key"))?;
        let malformed =
            || crate::BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(key.clone()));
        let nodes = &self.package().resolved_graph().nodes;
        let node = |id: &CheckedNodeId| {
            nodes
                .iter()
                .find(|node| &node.node_id == id)
                .ok_or_else(malformed)
        };
        let mut parameters = Vec::new();
        let mut binders = Vec::new();
        let mut executions = Vec::new();
        for execution in &form.body.executions {
            let mut declaring = None;
            for parameter in &execution.parameters {
                let selected = node(parameter)?;
                let name = group_member(&selected.body, "name")
                    .and_then(text)
                    .ok_or_else(malformed)?;
                let value_type = self
                    .exact_type_projections()
                    .get(parameter)
                    .and_then(|projection| projection.as_ref().ok())
                    .map(|projection| projection.value_type().clone())
                    .ok_or_else(malformed)?;
                if name == "self" {
                    let ValueType::Reference(context) = &value_type else {
                        return Err(malformed());
                    };
                    declaring = Some(filament_checked::key::DeclarationKey {
                        package: context.original().package().to_owned(),
                        node: context.original().node().to_owned(),
                    });
                }
                parameters.push((format!("{}.{}", execution.variable, name), value_type));
                binders.push(self.node_key(parameter).ok_or_else(malformed)?);
            }
            let anchor = node(&execution.operation)?;
            let CheckedTerm::OperationAnchorBody(body) = &anchor.body else {
                return Err(malformed());
            };
            let CheckedTerm::Binding { value, .. } = &body.members[1] else {
                return Err(malformed());
            };
            let operation = text(value).ok_or_else(malformed)?;
            executions.push((
                execution.variable.to_string(),
                fsl_semantics::check::OperationKey {
                    declaring: declaring.ok_or_else(malformed)?,
                    operation: Identifier::new(operation.to_owned()).map_err(|_| malformed())?,
                },
            ));
        }
        let (_, identities) = self.witness()?;
        let walker = crate::reader_protocol_projection::BareWalker::new(
            self.package(),
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
            self.dependency_views(),
        )
        .map_err(SlotSelectionFailure::Fault)?;
        let body = self.node_key(&form.body.body).ok_or_else(malformed)?;
        let projection = walker
            .project_term(
                &body,
                &binders,
                filament_semantic_value::location::ExpressionOwner::TypeDeclaration {
                    name: format!("relation {} body", form.name),
                },
            )
            .map_err(crate::bare_term::walk_failure)?;
        let components = projection
            .model_relation_components(form.reads())
            .ok_or_else(malformed)?;
        Ok(BareModelRelation {
            parameters,
            executions,
            projection,
            components,
        })
    }
}

#[ix_trace_rs::string_edge]
fn model_reads(
    package: &CheckedPackageV2,
    body: &CheckedStepRelationBody,
) -> Option<Vec<ModelRelationRead>> {
    use filament_contract_model::{CheckedOperationMember, CheckedTerm};
    use fsl_semantics::model::clause_observation::Observation;
    use std::collections::{BTreeMap, BTreeSet};
    let nodes: BTreeMap<_, _> = package
        .resolved_graph()
        .nodes
        .iter()
        .map(|node| (&node.node_id, node))
        .collect();
    let parameters: BTreeMap<_, _> = body
        .executions
        .iter()
        .enumerate()
        .flat_map(|(index, execution)| {
            execution
                .parameters
                .iter()
                .map(move |parameter| (parameter, index))
        })
        .collect();
    let mut pending = vec![(&body.body, false)];
    let mut seen = BTreeSet::new();
    let mut reads = Vec::new();
    while let Some((key, under_pre)) = pending.pop() {
        if !seen.insert((key, under_pre)) {
            continue;
        }
        let node = nodes.get(key)?;
        let CheckedTerm::Application {
            operation,
            arguments,
            ..
        } = &node.body
        else {
            continue;
        };
        let operation = operation.read().ok()?;
        let model_read = matches!(
            operation.member,
            Some(CheckedOperationMember::Field {
                read_form: Some(_),
                ..
            }) | Some(CheckedOperationMember::RelationshipEnd { .. })
        );
        if model_read {
            let mut roots = arguments.first()?.reference_targets();
            let mut met = BTreeSet::new();
            let mut owners = BTreeSet::new();
            while let Some(root) = roots.pop() {
                if !met.insert(root) {
                    continue;
                }
                if let Some(owner) = parameters.get(root) {
                    owners.insert(*owner);
                } else {
                    roots.extend(nodes.get(root)?.body.reference_targets());
                }
            }
            if owners.len() != 1 {
                return None;
            }
            reads.push(ModelRelationRead {
                node: (*key).clone(),
                execution: *owners.first()?,
                observation: if under_pre {
                    Observation::Pre
                } else {
                    Observation::Post
                },
            });
        }
        let below = under_pre || operation.identity == "filament.op.state.pre";
        pending.extend(
            arguments
                .iter()
                .flat_map(CheckedTerm::reference_targets)
                .rev()
                .map(|child| (child, below)),
        );
    }
    Some(reads)
}
