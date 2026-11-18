// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: the one sealed read view the analysis engines read a checked
//! package through.
//!
//! [`AnalysisView`] is the table of FR-531 "What the entry derives from the
//! bytes" as a trait. It is implemented for [`AdmittedPackageRead`] (the strict
//! reader's admitted read-back of an in-process package's own emission) and
//! for [`BarePackage`] (a package admitted from bytes), and for no other
//! type: in particular not for [`crate::CheckedPackage`], so nothing read from
//! bytes becomes a checked package (FR-087 R-10). Both implementations read
//! the admitted [`CheckedPackageV2`], never a source-side value, so they
//! return equal values for a package and the bytes it was written to.
//!
//! Each engine family ports onto this trait by adding the accessor for the
//! fact row it reads; an accessor is a provided method over [`package`]
//! so an added fact needs no change to either implementation.
//!
//! [`package`]: AnalysisView::package

use std::collections::VecDeque;

use filament_contract_model::{CheckedNodeId, CheckedPackageV2, TemporalClauseName};
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::domain_package::DomainPackageRef;

use crate::checked_views::{self, TemporalSubjectView, ViewRefusal};
use crate::{AdmittedPackageRead, BarePackage};

/// The packages a view's package depends on, each once, nearest first, in
/// lock-selection order. The root package is not listed.
pub type DependencyClosure<'p> = Vec<&'p CheckedPackageV2>;

mod sealed {
    /// Private to `fsl_package`: no other type implements the view.
    pub trait Sealed {}
}

/// What an analysis engine reads of a checked package (FR-940).
///
/// The trait is sealed: `AdmittedPackageRead` and `BarePackage` are its only
/// implementors.
///
/// ```compile_fail,E0277
/// struct Local;
/// impl fsl_package::AnalysisView for Local {
///     fn package(&self) -> &filament_contract_model::CheckedPackageV2 {
///         unreachable!()
///     }
///     fn identity(&self) -> &fsl_semantics::library::LibraryName {
///         unreachable!()
///     }
/// }
/// ```
///
/// A checked package is not a view.
///
/// ```compile_fail,E0277
/// fn takes(_: &dyn fsl_package::AnalysisView) {}
/// fn pass(package: &fsl_package::CheckedPackage) {
///     takes(package);
/// }
/// ```
pub trait AnalysisView: sealed::Sealed {
    /// The admitted package every other accessor reads.
    fn package(&self) -> &CheckedPackageV2;

    /// The package's identity: its admitted `name()`, which the strict
    /// reader keeps nonempty (FR-940 `identity`).
    fn identity(&self) -> &LibraryName;

    /// The model identities the package selects, ascending (FR-533).
    fn model_selections(&self) -> Vec<DomainPackageRef> {
        self.package()
            .selected_model_packages()
            .map(|identity| DomainPackageRef {
                identity: identity.to_owned(),
            })
            .collect()
    }

    /// The dependency closure, read from the direct dependencies the package
    /// retains from its read evidence (FR-531).
    fn dependencies(&self) -> DependencyClosure<'_> {
        closure(self.package())
    }

    /// The package's FR-449 activation map: each protocol clause with the
    /// temporal clause its `requirements` name, in the package's row order.
    fn activation_relation(&self) -> Vec<(CheckedNodeId, CheckedNodeId)> {
        checked_views::activation_relation(self.package())
    }

    /// The temporal subject of the clause node `clause` (FR-515): its
    /// identity, `over`, activation, captures, formula operations, operand
    /// edges and `holds` atoms.
    ///
    /// # Errors
    ///
    /// [`ViewRefusal`] when the node is no readable temporal clause.
    fn temporal_subject(
        &self,
        clause: &CheckedNodeId,
    ) -> Result<TemporalSubjectView<'_>, ViewRefusal> {
        checked_views::temporal_subject(self.package(), clause)
    }

    /// The declared name of each temporal clause occurrence: the package's
    /// `temporal_clause_names` rows, in row order (FR-531 "clause name";
    /// FR-850 claim lookup). Empty when the package carries none.
    fn clause_names(&self) -> &[TemporalClauseName] {
        self.package().temporal_clause_names().unwrap_or_default()
    }

    /// The claim nodes whose name the package does not carry: each node
    /// with a `claim` occurrence that is neither a temporal clause with a
    /// `temporal_clause_names` row, nor a named declaration, nor a `pre` or
    /// `post` state clause (which no request names, FR-625). An `invariant`
    /// state clause, a refinement and a state-graph claim are named in
    /// source but carry no name row in the package (FR-850 claim lookup).
    fn unnamed_claims(&self) -> Vec<&CheckedNodeId> {
        use filament_contract_model::{CheckedNodePathSegment, CheckedOccurrenceRole};
        let named: Vec<&CheckedNodeId> =
            self.clause_names().iter().map(|row| &row.declaration).collect();
        self.package()
            .resolved_graph()
            .nodes
            .iter()
            .filter(|node| {
                node.occurrences
                    .iter()
                    .any(|occurrence| occurrence.role == CheckedOccurrenceRole::Claim)
                    && node.declaration.is_none()
                    && !named.contains(&&node.node_id)
            })
            .filter(|node| {
                let contract = node.semantic_form.as_ref() == "state_clause"
                    && !matches!(
                        node.node_id.path.get(2),
                        Some(CheckedNodePathSegment::Name(kind)) if kind.as_ref() == "invariant"
                    );
                !contract
            })
            .map(|node| &node.node_id)
            .collect()
    }

    /// Each named declaration's `declaration.qualified_name`, its segments
    /// joined by `::`, with its node, in the package's node order (FSpec
    /// FR-322; FR-850 claim lookup).
    fn declared_names(&self) -> Vec<(String, &CheckedNodeId)> {
        self.package()
            .resolved_graph()
            .nodes
            .iter()
            .filter_map(|node| {
                let declaration = node.declaration.as_ref()?;
                let name = declaration
                    .qualified_name
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<&str>>()
                    .join("::");
                Some((name, &node.node_id))
            })
            .collect()
    }
}

fn closure(package: &CheckedPackageV2) -> DependencyClosure<'_> {
    let mut closure: DependencyClosure<'_> = Vec::new();
    let mut queue: VecDeque<&CheckedPackageV2> = package.direct_dependencies().collect();
    while let Some(next) = queue.pop_front() {
        if closure.iter().any(|held| std::ptr::eq(*held, next)) {
            continue;
        }
        closure.push(next);
        queue.extend(next.direct_dependencies());
    }
    closure
}

impl sealed::Sealed for AdmittedPackageRead<'_> {}
impl sealed::Sealed for BarePackage<'_> {}

impl AnalysisView for AdmittedPackageRead<'_> {
    fn package(&self) -> &CheckedPackageV2 {
        self.admitted()
    }

    fn identity(&self) -> &LibraryName {
        self.admitted_identity()
    }
}

impl AnalysisView for BarePackage<'_> {
    fn package(&self) -> &CheckedPackageV2 {
        BarePackage::package(self)
    }

    fn identity(&self) -> &LibraryName {
        self.admitted_identity()
    }
}
