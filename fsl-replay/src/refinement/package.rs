// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model-refinement execution over original admitted package rows.

use filament_checked::key::DeclarationKey;
use filament_contract_model::{CheckedNodeId, CheckedTerm, RefinementKey, RefinementStepTarget};
use filament_exact::{NodeKey, Value};
use filament_semantic_value::fairness::FairnessConstraint;
use fsl_eval::simulation::ExpansionStop;
use fsl_eval::simulation::model::{ModelSystem, PackageModelInput, RowReading, StateObservation};
use fsl_package::{
    BarePackage, BareWorld, ReaderProtocolProjection, RefinementRowRefusal, RefinementView,
};
use fsl_semantics::check::{AbstractSide, AbstractStepClass, HistoryRow, StepClass};

use super::read::{
    self, FieldMapping, MappingExpression, MappingTarget, ObjectMapping, PopulationMapping,
    RefinementRead, StepMapping,
};

/// Why no package refinement execution definition could be published.
#[derive(Debug, thiserror::Error)]
pub enum PackageRefinementRefusal {
    /// The admitted row cannot be executed by the existing package expression machine.
    #[error(transparent)]
    Row(#[from] RefinementRowRefusal),
    /// The admitted declaration requires a named unsupported execution fact.
    #[error("package refinement requires its {0} form")]
    Unsupported(&'static str),
}

impl PackageRefinementRefusal {
    /// The exact unsupported fact, when the failure is a subset boundary.
    pub fn pending_fact(&self) -> Option<&'static str> {
        match self {
            Self::Unsupported(fact) => Some(fact),
            Self::Row(RefinementRowRefusal::View(refusal)) => Some(refusal.fact),
            Self::Row(RefinementRowRefusal::Projection(_)) => None,
        }
    }
}

struct Expression<'a> {
    world: &'a BareWorld,
    projection: ReaderProtocolProjection,
}
impl read::sealed::Sealed for Expression<'_> {}
impl MappingExpression for Expression<'_> {
    fn read(
        &self,
        system: &ModelSystem<'_>,
        observed: &StateObservation,
        bindings: Vec<Value>,
    ) -> Result<RowReading, ExpansionStop> {
        system.read_package_refinement_row(self.world, &self.projection, observed, bindings)
    }
}

struct Population {
    abstract_population: DeclarationKey,
    concrete_population: DeclarationKey,
    order_key: NodeKey,
}
impl read::sealed::Sealed for Population {}
impl PopulationMapping for Population {
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

struct Field<'a> {
    key: DeclarationKey,
    expression: Expression<'a>,
}
impl read::sealed::Sealed for Field<'_> {}
impl FieldMapping for Field<'_> {
    fn field(&self) -> &DeclarationKey {
        &self.key
    }
    fn expression(&self) -> &dyn MappingExpression {
        &self.expression
    }
}

struct Object<'a> {
    abstract_type: DeclarationKey,
    concrete_type: DeclarationKey,
    fields: Vec<Field<'a>>,
    hidden: Vec<DeclarationKey>,
}
impl read::sealed::Sealed for Object<'_> {}
impl ObjectMapping for Object<'_> {
    fn abstract_type(&self) -> &DeclarationKey {
        &self.abstract_type
    }
    fn concrete_type(&self) -> &DeclarationKey {
        &self.concrete_type
    }
    fn fields(&self) -> Box<dyn Iterator<Item = &dyn FieldMapping> + '_> {
        Box::new(self.fields.iter().map(|field| field as &dyn FieldMapping))
    }
    fn hidden(&self) -> &[DeclarationKey] {
        &self.hidden
    }
}

struct OperationTarget<'a> {
    key: DeclarationKey,
    target_type: DeclarationKey,
    receiver: Expression<'a>,
    arguments: Vec<Expression<'a>>,
    binds_result: bool,
}

enum Target<'a> {
    Operation(Box<OperationTarget<'a>>),
    Stutter,
    Any,
}
struct Step<'a> {
    class: StepClass,
    target: Target<'a>,
}
impl read::sealed::Sealed for Step<'_> {}
impl StepMapping for Step<'_> {
    fn class(&self) -> &StepClass {
        &self.class
    }
    fn target(&self) -> MappingTarget<'_> {
        match &self.target {
            Target::Operation(operation) => MappingTarget::Abstract {
                operation: &operation.key,
                target_type: Some(&operation.target_type),
                receiver: &operation.receiver,
                arguments: operation
                    .arguments
                    .iter()
                    .map(|argument| Some(argument as &dyn MappingExpression))
                    .collect(),
                binds_result: operation.binds_result,
            },
            Target::Stutter => MappingTarget::Stutter,
            Target::Any => MappingTarget::Any,
        }
    }
}

/// The authentic admitted rows read by the same refinement rules as source.
/// Construction accepts an admitted view and the existing model input only.
pub struct PackageRefinement<'a> {
    name: &'a str,
    concrete_model: String,
    abstract_model: String,
    populations: Vec<Population>,
    objects: Vec<Object<'a>>,
    steps: Vec<Step<'a>>,
    occurrences: &'a [filament_contract_model::CheckedOccurrence],
}

fn local(key: &RefinementKey) -> Result<&CheckedNodeId, PackageRefinementRefusal> {
    match key {
        RefinementKey::Local(node) => Ok(node),
        RefinementKey::External(_) => Err(PackageRefinementRefusal::Unsupported(
            "external abstract owner",
        )),
    }
}

fn object_key(
    package: &BarePackage<'_>,
    node: &CheckedNodeId,
) -> Result<DeclarationKey, PackageRefinementRefusal> {
    let row = package
        .identities()
        .and_then(|identities| {
            identities
                .declaration_rows()
                .find(|row| row.structural_node == *node)
        })
        .ok_or(PackageRefinementRefusal::Unsupported(
            "selected object identity",
        ))?;
    if !matches!(
        &row.exact_identity,
        filament_contract_model::SelectedExactIdentity::Object(_)
    ) {
        return Err(PackageRefinementRefusal::Unsupported(
            "selected object identity",
        ));
    }
    Ok(DeclarationKey {
        package: row.selected_document.to_string(),
        node: row.original_identity.to_string(),
    })
}

fn operation<'a>(
    package: &BarePackage<'_>,
    input: &'a PackageModelInput<'_>,
    node: &CheckedNodeId,
) -> Result<&'a fsl_semantics::model::domain_package::OperationMemberRecord, PackageRefinementRefusal>
{
    let unsupported = || PackageRefinementRefusal::Unsupported("selected operation declaration");
    let CheckedTerm::OperationAnchorBody(body) =
        package.package().term(node).ok_or_else(unsupported)?
    else {
        return Err(unsupported());
    };
    let CheckedTerm::Binding { value: context, .. } = &body.members[0] else {
        return Err(unsupported());
    };
    let CheckedTerm::Reference { target: context } = context.as_ref() else {
        return Err(unsupported());
    };
    let owner = object_key(package, context)?;
    let CheckedTerm::Binding { value, .. } = &body.members[1] else {
        return Err(unsupported());
    };
    let value = match value.as_ref() {
        CheckedTerm::Reference { target } => {
            package.package().term(target).ok_or_else(unsupported)?
        }
        value => value,
    };
    let Some(literal) = value.literal_read() else {
        return Err(unsupported());
    };
    let filament_contract_model::CheckedLiteralValue::Text(name) = literal.value else {
        return Err(unsupported());
    };
    input
        .refinement_model()
        .operation_declaration(&owner, name)
        .ok_or_else(unsupported)
}

impl<'a> PackageRefinement<'a> {
    /// Read original mapping rows, retaining actual model keys and byte expressions.
    ///
    /// # Errors
    /// Names unsupported row forms, unavailable original identities or projection failures.
    pub fn read(
        view: RefinementView<'a>,
        package: &BarePackage<'_>,
        input: &'a PackageModelInput<'_>,
    ) -> Result<Self, PackageRefinementRefusal> {
        let unsupported = |fact| PackageRefinementRefusal::Unsupported(fact);
        let region = view
            .declaration_region()
            .ok_or_else(|| unsupported("refinement declaration source"))?;
        let world = input.refinement_world();
        let mut populations = Vec::new();
        for row in view.populations() {
            let abstract_node = local(&row.r#abstract)?;
            let abstract_key = package
                .node_key(abstract_node)
                .ok_or_else(|| unsupported("abstract population identity"))?;
            let concrete_key = package
                .node_key(&row.concrete)
                .ok_or_else(|| unsupported("concrete population identity"))?;
            let (abstract_population, _) = package
                .population_types(&abstract_key)
                .ok_or_else(|| unsupported("abstract population identity"))?;
            let (concrete_population, _) = package
                .population_types(&concrete_key)
                .ok_or_else(|| unsupported("concrete population identity"))?;
            populations.push(Population {
                abstract_population: abstract_population.clone(),
                concrete_population: concrete_population.clone(),
                order_key: abstract_key,
            });
        }
        let mut objects = Vec::new();
        for row in view.objects() {
            let abstract_node = local(&row.abstract_type)?;
            let abstract_type = object_key(package, abstract_node)?;
            let concrete_type = object_key(package, &row.concrete_type)?;
            let declared = package
                .package()
                .model_object_fields(abstract_node)
                .map_err(|_| unsupported("abstract field declarations"))?;
            let identity_key = |field: &filament_contract_model::CheckedModelField|
             -> Result<DeclarationKey, PackageRefinementRefusal> {
                // FSpec's model field key is its declaring object key with
                // the field name appended (FR-106). The admitted model field
                // table supplies both facts; exact-field rows in
                // `identities.json` describe record value types instead.
                let owner = object_key(package, field.declaring_node_id())?;
                Ok(DeclarationKey {
                    package: owner.package,
                    node: format!("{}/{}", owner.node, field.name()),
                })
            };
            let declared_keys = declared
                .fields()
                .iter()
                .map(identity_key)
                .collect::<Result<Vec<_>, _>>()?;
            let mut fields = Vec::new();
            for field in &row.fields {
                let admitted = declared
                    .field(field.name.as_ref())
                    .ok_or_else(|| unsupported("abstract field identity"))?;
                let key = identity_key(admitted)?;
                fields.push(Field {
                    key,
                    expression: Expression {
                        world,
                        projection: package.project_refinement_row(
                            &field.expression,
                            region,
                            world,
                        )?,
                    },
                });
            }
            let hidden = declared_keys
                .into_iter()
                .filter(|key| !fields.iter().any(|field| field.key == *key))
                .collect();
            objects.push(Object {
                abstract_type,
                concrete_type,
                fields,
                hidden,
            });
        }
        let mut steps = Vec::new();
        for (ordinal, row) in view.steps().iter().enumerate() {
            let source_node = row
                .source_node
                .as_ref()
                .ok_or_else(|| unsupported("protocol step"))?;
            let concrete = operation(package, input, source_node)?;
            let step_region = view
                .step_region(ordinal)
                .ok_or_else(|| unsupported("authored step source"))?;
            let target = match &row.target {
                RefinementStepTarget::Operation {
                    operation: target,
                    target_type,
                    receiver,
                    arguments,
                } => {
                    let abstract_operation = operation(package, input, local(target)?)?;
                    let target_type = object_key(package, local(target_type)?)?;
                    if concrete.parameters.len() != abstract_operation.parameters.len()
                        || arguments.len() != abstract_operation.parameters.len()
                    {
                        return Err(unsupported("step argument arity"));
                    }
                    let arguments = arguments
                        .iter()
                        .map(|argument| {
                            let filament_contract_model::RefinementStepArgument::Term(term) =
                                argument
                            else {
                                return Err(unsupported("open step arguments"));
                            };
                            Ok(Expression {
                                world,
                                projection: package.project_refinement_step_argument(
                                    term,
                                    receiver,
                                    step_region,
                                    world,
                                )?,
                            })
                        })
                        .collect::<Result<Vec<_>, PackageRefinementRefusal>>()?;
                    Target::Operation(Box::new(OperationTarget {
                        key: abstract_operation.key.clone(),
                        target_type,
                        receiver: Expression {
                            world,
                            projection: package.project_refinement_row(
                                receiver,
                                step_region,
                                world,
                            )?,
                        },
                        arguments,
                        binds_result: concrete.result.is_some()
                            && abstract_operation.result.is_some(),
                    }))
                }
                RefinementStepTarget::Stutter => Target::Stutter,
                RefinementStepTarget::Any => Target::Any,
                RefinementStepTarget::ProtocolStep { .. } => {
                    return Err(unsupported("protocol step"));
                }
            };
            steps.push(Step {
                class: StepClass::Operation(concrete.key.clone()),
                target,
            });
        }
        let (concrete_model, abstract_model) = populations
            .first()
            .map(|row| {
                (
                    row.concrete_population.package.clone(),
                    row.abstract_population.package.clone(),
                )
            })
            .or_else(|| {
                objects.first().map(|row| {
                    (
                        row.concrete_type.package.clone(),
                        row.abstract_type.package.clone(),
                    )
                })
            })
            .ok_or_else(|| unsupported("empty mapping subjects"))?;
        let occurrences = view
            .package()
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| node.node_id == *view.node())
            .map(|node| node.occurrences.as_slice())
            .ok_or_else(|| unsupported("refinement occurrences"))?;
        Ok(Self {
            name: view.name(),
            concrete_model,
            abstract_model,
            populations,
            objects,
            steps,
            occurrences,
        })
    }
}

impl read::sealed::Sealed for PackageRefinement<'_> {}
impl RefinementRead for PackageRefinement<'_> {
    fn name(&self) -> &str {
        self.name
    }
    fn concrete_model(&self) -> &str {
        &self.concrete_model
    }
    fn abstract_model(&self) -> &str {
        &self.abstract_model
    }
    fn subject_models(&self) -> Option<(&str, &str)> {
        Some((&self.concrete_model, &self.abstract_model))
    }
    fn abstract_side(&self) -> &AbstractSide {
        &AbstractSide::Model
    }
    fn concrete_protocol(&self) -> Option<&NodeKey> {
        None
    }
    fn populations(&self) -> Box<dyn Iterator<Item = &dyn PopulationMapping> + '_> {
        Box::new(
            self.populations
                .iter()
                .map(|row| row as &dyn PopulationMapping),
        )
    }
    fn objects(&self) -> Box<dyn Iterator<Item = &dyn ObjectMapping> + '_> {
        Box::new(self.objects.iter().map(|row| row as &dyn ObjectMapping))
    }
    fn steps(&self) -> Box<dyn Iterator<Item = &dyn StepMapping> + '_> {
        Box::new(self.steps.iter().map(|row| row as &dyn StepMapping))
    }
    fn step_simulation_occurrences(&self) -> Box<dyn Iterator<Item = (&str, u64)> + '_> {
        Box::new(self.occurrences.iter().filter_map(|occurrence| {
            (occurrence.role == filament_contract_model::CheckedOccurrenceRole::Generated)
                .then_some(("generated", occurrence.ordinal))
        }))
    }
    fn history(&self) -> &[HistoryRow] {
        &[]
    }
    fn abstract_internal(&self) -> &[AbstractStepClass] {
        &[]
    }
    fn concrete_operation_fairness(&self) -> Box<dyn Iterator<Item = &FairnessConstraint> + '_> {
        Box::new(std::iter::empty())
    }
    fn abstract_operation_fairness(&self) -> Box<dyn Iterator<Item = &FairnessConstraint> + '_> {
        Box::new(std::iter::empty())
    }
}
