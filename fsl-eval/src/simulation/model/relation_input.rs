// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: source-free execution facts shared by HP-1 checking and replay.

use std::collections::BTreeMap;

use filament_exact::Meter;
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckedModelRelation, OperationKey};
use fsl_semantics::model::object_environment::ObjectEnvironment;

/// The admitted identity and ordered execution facts of an HP-1 model claim.
///
/// This value retains no source expression, checked body or source arena.
#[derive(Clone, Debug)]
pub struct ModelRelationDescriptor {
    occurrence: OccurrenceKey,
    name: String,
    executions: Vec<(String, OperationKey)>,
    execution_aliases: Vec<String>,
    aliases: Vec<String>,
    identities: BTreeMap<String, String>,
}

impl ModelRelationDescriptor {
    /// Read the source claim's assembled identity and selected alias identities.
    ///
    /// # Errors
    ///
    /// Refuses a relation not owned by the package or an unselected alias.
    pub fn of_source(
        package: &CheckedPackage,
        relation: &CheckedModelRelation,
    ) -> Result<Self, InternalFault> {
        let fault = || InternalFault::new("S6a", "relation-descriptor-owned-by-package");
        let selected = package
            .graph()
            .step_relations()
            .iter()
            .find(|candidate| match relation.occurrence() {
                Some(key) => candidate.occurrence() == Some(key),
                None => candidate.name() == relation.name(),
            })
            .ok_or_else(fault)?;
        if selected.executions() != relation.executions()
            || selected.execution_aliases() != relation.execution_aliases()
            || selected.aliases() != relation.aliases()
            || selected.body() != relation.body()
        {
            return Err(fault());
        }
        let identities = selected
            .aliases()
            .iter()
            .map(|alias| {
                package
                    .graph()
                    .model_package_identity(alias)
                    .map(|identity| (alias.clone(), identity.to_owned()))
                    .ok_or_else(fault)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            occurrence: selected.occurrence().cloned().ok_or_else(fault)?,
            name: selected.name().to_owned(),
            executions: selected.executions().to_vec(),
            execution_aliases: selected.execution_aliases().to_vec(),
            aliases: selected.aliases().to_vec(),
            identities,
        })
    }

    /// The claim occurrence that keys tuple evaluation.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The declaration's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Execution variables and their operations, in binding order.
    pub fn executions(&self) -> &[(String, OperationKey)] {
        &self.executions
    }

    /// One alias for each execution, in binding order.
    pub fn execution_aliases(&self) -> &[String] {
        &self.execution_aliases
    }

    /// Distinct subjects, in model declaration order.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    /// The selected domain identity of one declared subject alias.
    pub fn model_package_identity(&self, alias: &str) -> Option<&str> {
        self.identities.get(alias).map(String::as_str)
    }

    pub(crate) fn source_relation<'a>(
        &self,
        package: &'a CheckedPackage,
    ) -> Result<&'a CheckedModelRelation, InternalFault> {
        package
            .graph()
            .step_relations()
            .iter()
            .find(|relation| relation.occurrence() == Some(&self.occurrence))
            .ok_or_else(|| InternalFault::new("S6a", "relation-evaluator-owns-occurrence"))
    }
}

/// One execution-ordered labelled tuple and its pre/post observations.
pub struct ModelRelationTuple<'a> {
    /// The operation, receiver and arguments of each execution.
    pub transitions: &'a [&'a ModelTransition],
    /// Each execution's actual typed result, absent only for a resultless operation.
    pub results: &'a [Option<&'a str>],
    /// The observations before each labelled transition.
    pub pre: &'a [&'a ObjectEnvironment],
    /// The observations after each labelled transition.
    pub post: &'a [&'a ObjectEnvironment],
}

pub(crate) fn evaluate_source(
    package: &CheckedPackage,
    relation: &ModelRelationDescriptor,
    tuple: ModelRelationTuple<'_>,
    meter: &mut Meter,
) -> Result<crate::value::ModelRelationEvaluation, InternalFault> {
    crate::value::evaluate_model_relation_transitions(
        package,
        relation.source_relation(package)?,
        tuple.transitions,
        tuple.results,
        tuple.pre,
        tuple.post,
        meter,
    )
}
