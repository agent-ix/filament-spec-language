// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local evaluator adapter for fsl-package's admitted reader projection.

use filament_checked::node::{CheckedLiteral, NodeKind};
use filament_exact::{Value, ValueType};
use filament_semantic_value::location::ExpressionLocation as Location;

use fsl_foundation::source::provenance::OccurrenceKey;

use super::node_source::{Callee, NodeSource};

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

    fn is_ordering_part(self) -> bool {
        false
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

/// FR-156 "Atom footprints" over the package route: the symbolic locations
/// the predicate `projection` reads, as the source route derives them from
/// its checked body (FR-940 "Package form of a state-graph claim").
///
/// # Errors
///
/// [`fsl_semantics::check::footprint::FootprintRefusal`] as the derivation
/// refuses a form with no footprint rule.
pub(crate) fn projection_atom_reads(
    projection: &fsl_package::ReaderProtocolProjection,
    types: &filament_semantic_value::declaration::TypeEnvironment,
    model: &fsl_semantics::model::state::StateModel,
) -> Result<
    std::collections::BTreeSet<fsl_semantics::check::footprint::SymbolicLocation>,
    fsl_semantics::check::footprint::FootprintRefusal,
> {
    fsl_semantics::check::footprint::derive_atom_reads_over(
        types,
        model,
        ReaderNode::root(projection),
    )
}
