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
    parameters: Vec<(String, filament_exact::ValueType)>,
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
            parameters: selected.body_parameters().to_vec(),
        })
    }

    /// Cross-check source execution facts against the run's one admitted view.
    ///
    /// # Errors
    ///
    /// The emission's order evidence or execution bindings differ from S3.
    pub fn of_source_form(
        package: &CheckedPackage,
        relation: &CheckedModelRelation,
        form: &fsl_package::PackageModelRelationForm,
    ) -> Result<Self, InternalFault> {
        let selected = Self::of_source(package, relation)?;
        let aliases: Vec<_> = form
            .subjects()
            .iter()
            .map(|subject| subject.alias.clone())
            .collect();
        let identities: BTreeMap<_, _> = form
            .subjects()
            .iter()
            .map(|subject| (subject.alias.clone(), subject.identity.clone()))
            .collect();
        let variables: Vec<_> = form
            .body()
            .executions
            .iter()
            .map(|execution| execution.variable.as_ref())
            .collect();
        if form.occurrence() != selected.occurrence()
            || aliases != selected.aliases
            || identities != selected.identities
            || !variables
                .into_iter()
                .eq(selected.executions.iter().map(|(name, _)| name.as_str()))
            || !form
                .body()
                .executions
                .iter()
                .map(|execution| execution.alias.as_ref())
                .eq(selected.execution_aliases.iter().map(String::as_str))
        {
            return Err(InternalFault::new(
                "S6a",
                "relation-source-form-matches-emission",
            ));
        }
        Ok(selected)
    }

    /// Read the execution facts of an admitted package form.
    ///
    /// # Errors
    ///
    /// A body that cannot be projected or a claim with no structural key.
    pub fn of_package(
        package: &fsl_package::BarePackage<'_>,
        form: &fsl_package::PackageModelRelationForm,
    ) -> Result<Self, fsl_package::BareSelectionFailure> {
        let projected = package.project_model_relation(form)?;
        Ok(Self {
            occurrence: form.occurrence().clone(),
            name: form.name().to_owned(),
            executions: projected.executions().to_vec(),
            execution_aliases: form
                .body()
                .executions
                .iter()
                .map(|execution| execution.alias.to_string())
                .collect(),
            aliases: form
                .subjects()
                .iter()
                .map(|subject| subject.alias.clone())
                .collect(),
            identities: form
                .subjects()
                .iter()
                .map(|subject| (subject.alias.clone(), subject.identity.clone()))
                .collect(),
            parameters: projected.parameters().to_vec(),
        })
    }

    /// Receiver, result and argument types in execution-slot order.
    pub fn parameters(&self) -> &[(String, filament_exact::ValueType)] {
        &self.parameters
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
