// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local evaluator adapter for fsl-package's admitted reader projection.

use filament_checked::node::{CheckedLiteral, NodeKind};
use filament_exact::{Value, ValueType};
use filament_semantic_value::location::ExpressionLocation as Location;

use fsl_foundation::source::provenance::OccurrenceKey;

use super::node_source::{Callee, NodeSource};
use crate::simulation::model::BodyReads;

/// One node handle in the package-owned admitted projection.
#[derive(Clone, Copy)]
pub(crate) struct ReaderNode<'arena> {
    projection: &'arena fsl_package::ReaderProtocolProjection,
    index: usize,
}

impl<'arena> ReaderNode<'arena> {
    pub(crate) fn root(projection: &'arena fsl_package::ReaderProtocolProjection) -> Self {
        Self {
            projection,
            index: projection.root(),
        }
    }
}

impl<'arena> NodeSource<'arena> for ReaderNode<'arena> {
    type Child = usize;
    type Literal = CheckedLiteral;

    fn kind(self) -> &'arena NodeKind<Self::Child, Self::Literal> {
        self.projection
            .kind(self.index)
            .expect("projection node index is valid")
    }

    fn literal(literal: &'arena Self::Literal) -> &'arena Value {
        &literal.0
    }

    fn at(self, child: Self::Child) -> Self {
        Self {
            projection: self.projection,
            index: child,
        }
    }

    fn value_type(self) -> &'arena ValueType {
        self.projection
            .value_type(self.index)
            .expect("projection node index is valid")
    }

    fn location(self) -> Option<&'arena Location> {
        self.projection.location(self.index)
    }

    fn occurrence(self) -> Option<OccurrenceKey> {
        self.projection.occurrence(self.index).cloned()
    }

    fn children(self) -> Vec<Self> {
        self.projection
            .children(self.index)
            .expect("projection node index is valid")
            .iter()
            .copied()
            .map(|index| Self {
                projection: self.projection,
                index,
            })
            .collect()
    }

    fn callee(
        self,
        _graph: Option<&'arena fsl_semantics::check::CheckedGraph>,
        function: usize,
    ) -> Option<Callee<'arena, Self>> {
        let function = self.projection.function(function)?;
        Some(Callee {
            name: function.name,
            body: Self {
                projection: self.projection,
                index: function.body,
            },
            slots: function.slots,
        })
    }
}

impl<'arena> fsl_semantics::check::footprint::AtomNode<'arena> for ReaderNode<'arena> {
    fn form(self) -> fsl_semantics::check::footprint::AtomForm<'arena, Self> {
        fsl_semantics::check::footprint::atom_form(
            NodeSource::kind(self),
            NodeSource::value_type(self),
            |child| NodeSource::at(self, child),
            |child| NodeSource::kind(NodeSource::at(self, child)),
        )
    }

    fn children(self) -> Vec<Self> {
        NodeSource::children(self)
    }
}

impl<'arena> fsl_semantics::check::footprint::ReadNode<'arena> for ReaderNode<'arena> {
    type Child = usize;
    type Literal = CheckedLiteral;

    fn kind(self) -> &'arena NodeKind<Self::Child, Self::Literal> {
        NodeSource::kind(self)
    }

    fn at(self, child: Self::Child) -> Self {
        NodeSource::at(self, child)
    }

    fn value_type(self) -> &'arena ValueType {
        NodeSource::value_type(self)
    }

    fn location(self) -> Option<&'arena Location> {
        NodeSource::location(self)
    }

    fn children(self) -> Vec<Self> {
        NodeSource::children(self)
    }

    fn callee(
        self,
        _graph: Option<&'arena fsl_semantics::check::CheckedGraph>,
        function: usize,
    ) -> Option<Self> {
        NodeSource::callee(self, None, function).map(|callee| callee.body)
    }
}

/// One state clause of the package, its declaration and projected body.
pub struct ProjectedClause {
    /// The clause's declaration row.
    pub declaration: fsl_package::BareStateClauseDeclaration,
    /// The clause's body as evaluator nodes.
    pub clause: fsl_package::BareStateClause,
}

fn clause_body<'p>(
    projected: &'p ProjectedClause,
    operation: Option<&fsl_semantics::model::operation::OperationDeclaration>,
) -> Result<
    fsl_semantics::check::footprint::ClauseBody<'p, ReaderNode<'p>>,
    fsl_semantics::check::footprint::FootprintRefusal,
> {
    use fsl_semantics::check::footprint::FootprintRefusal;
    use fsl_semantics::model::clause_observation::Observation;
    let declaration = &projected.declaration;
    let mut types = vec![ValueType::Reference(declaration.context.clone())];
    let mut binds_result = false;
    if let Some(operation) = operation {
        if declaration.observation == Observation::Post {
            if let Some(result) = operation.result() {
                binds_result = true;
                types.push(result.clone());
            }
        }
        types.extend(
            operation
                .parameters()
                .iter()
                .map(|(_, value_type)| value_type.clone()),
        );
    }
    // The footprint reads a parameter by its slot, so the package's own
    // parameters must take their slots in declaration order, one for each
    // the operation declares.
    let declared = projected.clause.parameters();
    if declared.len() != types.len()
        || declared
            .iter()
            .enumerate()
            .any(|(position, (_, slot))| position != *slot)
    {
        return Err(FootprintRefusal::Unresolved {
            what: "state clause parameters in slot order",
        });
    }
    let parameters: Vec<(String, ValueType)> = declared
        .iter()
        .map(|(name, _)| name.clone())
        .zip(types)
        .collect();
    let observation = declaration.observation;
    let root = ReaderNode::root(projected.clause.projection());
    Ok(fsl_semantics::check::footprint::ClauseBody {
        name: &declaration.name,
        identity: declaration.identity.clone(),
        context: declaration.context.clone(),
        operation: declaration
            .operation
            .as_ref()
            .map(|owner| (owner.declaring.clone(), owner.operation.clone())),
        binds_result,
        pre_reads: fsl_semantics::check::footprint::pre_observed_locations(
            root,
            observation,
            parameters.len(),
        ),
        parameters,
        observation,
        root,
    })
}

/// FR-155 "Derivation" over the package route: the symbolic footprint of
/// `operation` from the package's own projected state clauses, by the
/// derivation the source route runs over its checked clauses.
///
/// # Errors
///
/// [`fsl_semantics::check::footprint::FootprintRefusal`] as the derivation
/// refuses.
pub(crate) fn package_operation_footprint(
    types: &filament_semantic_value::declaration::TypeEnvironment,
    model: &fsl_semantics::model::state::StateModel,
    clauses: &[ProjectedClause],
    reference_literals: &std::collections::BTreeMap<
        (filament_exact::EffectiveId, Vec<u8>),
        Vec<filament_checked::key::DeclarationKey>,
    >,
    operation: fsl_semantics::check::footprint::PackageOperation<'_>,
    select: &dyn Fn(filament_exact::NodeKey) -> bool,
) -> Result<
    fsl_semantics::check::footprint::SymbolicFootprint,
    fsl_semantics::check::footprint::FootprintRefusal,
> {
    let bodies: Vec<_> = clauses
        .iter()
        .map(|clause| clause_body(clause, Some(operation.operation)))
        .collect::<Result<_, _>>()?;
    fsl_semantics::check::footprint::derive_selected_footprint_over(
        types,
        model,
        &bodies,
        reference_literals,
        operation,
        select,
    )
}

/// FR-156 "Visibility" over the package route: the locations the state
/// constraint `clause` reads.
///
/// # Errors
///
/// As [`package_operation_footprint`].
pub(crate) fn package_constraint_reads(
    types: &filament_semantic_value::declaration::TypeEnvironment,
    model: &fsl_semantics::model::state::StateModel,
    clause: &ProjectedClause,
) -> Result<BodyReads, fsl_semantics::check::footprint::FootprintRefusal> {
    let body = clause_body(clause, None)?;
    let reads = fsl_semantics::check::footprint::derive_constraint_reads_over(types, model, &body)?;
    Ok(BodyReads {
        reads,
        literals: fsl_semantics::check::footprint::reference_literals_of(body.root),
    })
}

/// FR-156 "Atom footprints" over the package route: the locations a
/// projected term reads, `receivers` being the slots that read as the receiver, and
/// the reference literals it names.
///
/// # Errors
///
/// As [`package_operation_footprint`].
pub(crate) fn package_term_reads(
    projection: &fsl_package::ReaderProtocolProjection,
    types: &filament_semantic_value::declaration::TypeEnvironment,
    model: &fsl_semantics::model::state::StateModel,
    receivers: &[usize],
) -> Result<BodyReads, fsl_semantics::check::footprint::FootprintRefusal> {
    let root = ReaderNode::root(projection);
    Ok(BodyReads {
        reads: fsl_semantics::check::footprint::derive_atom_reads_over(
            types, model, receivers, root,
        )?,
        literals: fsl_semantics::check::footprint::reference_literals_of(root),
    })
}

/// FR-125-AC-18, FR-337 over the package route: the locations the `when`
/// guard of an `on each` activation reads, `receivers` being the slots of its
/// `over` and trigger binders.
///
/// # Errors
///
/// As [`package_operation_footprint`].
pub(crate) fn package_guard_reads(
    projection: &fsl_package::ReaderProtocolProjection,
    types: &filament_semantic_value::declaration::TypeEnvironment,
    model: &fsl_semantics::model::state::StateModel,
    receivers: &[usize],
) -> Result<BodyReads, fsl_semantics::check::footprint::FootprintRefusal> {
    let root = ReaderNode::root(projection);
    Ok(BodyReads {
        reads: fsl_semantics::check::footprint::derive_guard_reads_over(
            types,
            model,
            &[],
            receivers,
            root,
        )?,
        literals: fsl_semantics::check::footprint::reference_literals_of(root),
    })
}
