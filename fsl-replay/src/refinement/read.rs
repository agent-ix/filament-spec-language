// SPDX-License-Identifier: AGPL-3.0-or-later
//! Borrowed refinement rules shared by production, certification and replay.

use filament_checked::key::DeclarationKey;
use filament_exact::{NodeKey, Value};
use filament_semantic_value::fairness::FairnessConstraint;
use fsl_eval::simulation::model::{
    ModelCheckInput, ModelSystem, PackageModelInput, RowReading, StateObservation,
};
use fsl_eval::simulation::ExpansionStop;
use fsl_package::CheckedPackage;
use fsl_semantics::check::{
    AbstractSide, AbstractStepClass, CheckedRefinement, CheckedRow, FieldRow, HistoryRow,
    ObjectRow, PopulationRow, StepClass, StepRow, StepTarget,
};

pub(crate) mod sealed {
    pub trait Sealed {}
}

impl sealed::Sealed for CheckedPackage {}
impl sealed::Sealed for PackageModelInput<'_> {}
impl sealed::Sealed for CheckedRefinement {}
impl sealed::Sealed for CheckedRow {}
impl sealed::Sealed for PopulationRow {}
impl sealed::Sealed for FieldRow {}
impl sealed::Sealed for ObjectRow {}
impl sealed::Sealed for StepRow {}

/// The already constructed subject input used by both sides of a refinement.
pub trait RefinementWorld: sealed::Sealed {
    /// Borrow the existing compiled or admitted model input without building a world.
    fn refinement_input(&self) -> ModelCheckInput<'_>;
    /// The original source package, only for existing source protocol execution.
    fn source_package(&self) -> Option<&CheckedPackage>;
}

impl RefinementWorld for CheckedPackage {
    fn refinement_input(&self) -> ModelCheckInput<'_> {
        ModelCheckInput::source(self)
    }
    fn source_package(&self) -> Option<&CheckedPackage> {
        Some(self)
    }
}

impl RefinementWorld for PackageModelInput<'_> {
    fn refinement_input(&self) -> ModelCheckInput<'_> {
        self.input()
    }
    fn source_package(&self) -> Option<&CheckedPackage> {
        None
    }
}

/// An authenticated mapping expression, read under its model system's meter.
pub trait MappingExpression: sealed::Sealed {
    /// Evaluate the actual expression against the generated current observation.
    ///
    /// # Errors
    /// An evaluator invariant fault, with no invented value or partial mapping.
    fn read(
        &self,
        system: &ModelSystem<'_>,
        observed: &StateObservation,
        bindings: Vec<Value>,
    ) -> Result<RowReading, ExpansionStop>;
}

impl MappingExpression for CheckedRow {
    fn read(
        &self,
        system: &ModelSystem<'_>,
        observed: &StateObservation,
        bindings: Vec<Value>,
    ) -> Result<RowReading, ExpansionStop> {
        system.read_row(self, observed, bindings)
    }
}

/// One original population association, without manufacturing a source row.
pub trait PopulationMapping: sealed::Sealed {
    /// The abstract population's original declaration key.
    fn abstract_population(&self) -> &DeclarationKey;
    /// The concrete population's original declaration key.
    fn concrete_population(&self) -> &DeclarationKey;
    /// The authentic abstract population's canonical ordering key.
    fn order_key(&self) -> &NodeKey;
}

impl PopulationMapping for PopulationRow {
    fn abstract_population(&self) -> &DeclarationKey {
        &self.abstract_population
    }
    fn concrete_population(&self) -> &DeclarationKey {
        &self.concrete_population
    }
    fn order_key(&self) -> &NodeKey {
        &self.order_key
    }
}

/// An abstract field and its original mapping expression.
pub trait FieldMapping: sealed::Sealed {
    /// The actual abstract field declaration.
    fn field(&self) -> &DeclarationKey;
    /// Its source or admitted package expression.
    fn expression(&self) -> &dyn MappingExpression;
}

impl FieldMapping for FieldRow {
    fn field(&self) -> &DeclarationKey {
        &self.field
    }
    fn expression(&self) -> &dyn MappingExpression {
        &self.expression
    }
}

/// A borrowed object association and its visible and hidden fields.
pub trait ObjectMapping: sealed::Sealed {
    /// The original abstract object declaration.
    fn abstract_type(&self) -> &DeclarationKey;
    /// The original concrete object declaration.
    fn concrete_type(&self) -> &DeclarationKey;
    /// Each original visible field mapping in its stored order.
    fn fields(&self) -> Box<dyn Iterator<Item = &dyn FieldMapping> + '_>;
    /// The abstract fields left open by the original declaration.
    fn hidden(&self) -> &[DeclarationKey];
}

impl ObjectMapping for ObjectRow {
    fn abstract_type(&self) -> &DeclarationKey {
        &self.abstract_type
    }
    fn concrete_type(&self) -> &DeclarationKey {
        &self.concrete_type
    }
    fn fields(&self) -> Box<dyn Iterator<Item = &dyn FieldMapping> + '_> {
        Box::new(self.fields.iter().map(|row| row as &dyn FieldMapping))
    }
    fn hidden(&self) -> &[DeclarationKey] {
        &self.hidden
    }
}

/// The original target of one mapping step, with borrowed expressions.
pub enum MappingTarget<'a> {
    /// A concrete operation maps to this abstract operation application.
    Abstract {
        /// The actual abstract operation declaration.
        operation: &'a DeclarationKey,
        /// The receiver expression evaluated before the step.
        receiver: &'a dyn MappingExpression,
        /// Parameter expressions; an absent expression is an authored open slot.
        arguments: Vec<Option<&'a dyn MappingExpression>>,
        /// Whether the original application binds the operation result.
        binds_result: bool,
    },
    /// An original protocol node target, retained for existing source execution.
    Node {
        /// The original abstract protocol spelling.
        protocol: &'a str,
        /// The original protocol node spelling.
        node: &'a str,
        /// Original parameter expressions.
        arguments: Vec<Option<&'a dyn MappingExpression>>,
    },
    /// An authored stuttering step.
    Stutter,
    /// An authored unrestricted abstract step.
    Any,
}

/// One borrowed concrete class and its original abstract target.
pub trait StepMapping: sealed::Sealed {
    /// The exact declared concrete class.
    fn class(&self) -> &StepClass;
    /// The actual target, without converting package data into checked source.
    fn target(&self) -> MappingTarget<'_>;
}

impl StepMapping for StepRow {
    fn class(&self) -> &StepClass {
        &self.class
    }
    fn target(&self) -> MappingTarget<'_> {
        match &self.target {
            StepTarget::Abstract {
                operation,
                receiver,
                arguments,
                binds_result,
            } => MappingTarget::Abstract {
                operation,
                receiver,
                arguments: arguments
                    .iter()
                    .map(|row| row.as_ref().map(|row| row as &dyn MappingExpression))
                    .collect(),
                binds_result: *binds_result,
            },
            StepTarget::Node {
                protocol,
                node,
                arguments,
            } => MappingTarget::Node {
                protocol,
                node,
                arguments: arguments
                    .iter()
                    .map(|row| row.as_ref().map(|row| row as &dyn MappingExpression))
                    .collect(),
            },
            StepTarget::Stutter => MappingTarget::Stutter,
            StepTarget::Any => MappingTarget::Any,
        }
    }
}

/// The one rule read interface used by mapping, step decisions and replay.
pub trait RefinementRead: sealed::Sealed {
    /// The original declared identifier.
    fn name(&self) -> &str;
    /// The abstract source alias or authenticated package model identity.
    fn abstract_model(&self) -> &str;
    /// The concrete source alias or authenticated package model identity.
    fn concrete_model(&self) -> &str;
    /// Canonical concrete and abstract model identities, when the rows name them.
    fn subject_models(&self) -> Option<(&str, &str)>;
    /// The original abstract subject kind.
    fn abstract_side(&self) -> &AbstractSide;
    /// The original selected concrete protocol, if any.
    fn concrete_protocol(&self) -> Option<&NodeKey>;
    /// Original population mappings in their stored order.
    fn populations(&self) -> Box<dyn Iterator<Item = &dyn PopulationMapping> + '_>;
    /// Original object mappings in their stored order.
    fn objects(&self) -> Box<dyn Iterator<Item = &dyn ObjectMapping> + '_>;
    /// Original step mappings in their stored order.
    fn steps(&self) -> Box<dyn Iterator<Item = &dyn StepMapping> + '_>;
    /// Actual generated step-simulation occurrences in their stored order.
    fn step_simulation_occurrences(&self) -> Box<dyn Iterator<Item = (&str, u64)> + '_>;
    /// Original history rows; unsupported package histories are refused at admission.
    fn history(&self) -> &[HistoryRow];
    /// The original abstract internal classes.
    fn abstract_internal(&self) -> &[AbstractStepClass];
    /// Original concrete operation fairness constraints.
    fn concrete_operation_fairness(&self) -> Box<dyn Iterator<Item = &FairnessConstraint> + '_>;
    /// Original abstract operation fairness constraints.
    fn abstract_operation_fairness(&self) -> Box<dyn Iterator<Item = &FairnessConstraint> + '_>;
}

impl RefinementRead for CheckedRefinement {
    fn name(&self) -> &str {
        self.name()
    }
    fn abstract_model(&self) -> &str {
        self.abstract_model()
    }
    fn concrete_model(&self) -> &str {
        self.concrete_model()
    }
    fn subject_models(&self) -> Option<(&str, &str)> {
        self.populations()
            .first()
            .map(|row| {
                (
                    &*row.concrete_population.package,
                    &*row.abstract_population.package,
                )
            })
            .or_else(|| {
                self.objects()
                    .first()
                    .map(|row| (&*row.concrete_type.package, &*row.abstract_type.package))
            })
    }
    fn abstract_side(&self) -> &AbstractSide {
        self.abstract_side()
    }
    fn concrete_protocol(&self) -> Option<&NodeKey> {
        self.concrete_protocol()
    }
    fn populations(&self) -> Box<dyn Iterator<Item = &dyn PopulationMapping> + '_> {
        Box::new(
            self.populations()
                .iter()
                .map(|row| row as &dyn PopulationMapping),
        )
    }
    fn objects(&self) -> Box<dyn Iterator<Item = &dyn ObjectMapping> + '_> {
        Box::new(self.objects().iter().map(|row| row as &dyn ObjectMapping))
    }
    fn steps(&self) -> Box<dyn Iterator<Item = &dyn StepMapping> + '_> {
        Box::new(self.steps().iter().map(|row| row as &dyn StepMapping))
    }
    fn step_simulation_occurrences(&self) -> Box<dyn Iterator<Item = (&str, u64)> + '_> {
        Box::new(self.step_simulations().iter().map(|entry| {
            let origin = entry.occurrence().origin();
            (origin.role().as_str(), origin.ordinal())
        }))
    }
    fn history(&self) -> &[HistoryRow] {
        self.history()
    }
    fn abstract_internal(&self) -> &[AbstractStepClass] {
        self.abstract_internal()
    }
    fn concrete_operation_fairness(&self) -> Box<dyn Iterator<Item = &FairnessConstraint> + '_> {
        Box::new(self.concrete_operation_fairness())
    }
    fn abstract_operation_fairness(&self) -> Box<dyn Iterator<Item = &FairnessConstraint> + '_> {
        Box::new(self.abstract_operation_fairness())
    }
}
