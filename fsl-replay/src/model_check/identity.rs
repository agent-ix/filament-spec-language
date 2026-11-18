// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337-AC-5 and FR-338 Inputs: a clause's identity, resolved from the
//! package.
//!
//! A TP-1 clause's key contexts name its identity triple and its `over`
//! node. The triple is the `claim` occurrence of the clause's
//! `temporal`/`temporal_clause` node, and the `over` node is the
//! `value`/`parameter` node that clause node's body references. Neither
//! comes from a caller: [`ClauseIdentity::resolve`] emits the request's
//! package, reads the bytes back through the I2 reader and returns what
//! the I2 lookup ([`resolve_clause_root`]) finds there. A clause the
//! package does not declare, or that the lookup does not find, is
//! `clause-root-unresolved` ([`ClauseRootRefusal`]).

use filament_exact::NodeKey;
use filament_exact::{Cancel, CancelCause};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::{
    AdmittedPackages, CheckedPackage, ClauseRootUnresolved, EmitRefusal, TemporalClockSelection,
    V2ReadLimits, emit_checked_with_temporal_selections_and_cancel, read_import_view,
    resolve_clause_root, resolve_hyper_root, resolve_state_graph_root,
};
use fsl_semantics::check::{
    CheckedHyperClause, CheckedStateGraphClaim, CheckedTemporalClause, TemporalClauseNode,
};
use fsl_semantics::library::LibraryName;

/// The `claim` occurrence of a checked clause `node`: its node id and claim
/// origin (ADR-013 O-07).
pub fn node_occurrence(node: &TemporalClauseNode) -> OccurrenceKey {
    OccurrenceKey::new(node.identity().clone(), node.claim().clone())
}

/// A clause's identity in its package: the checked occurrence and the node of
/// its `over` parameter, `None` for a clause with no `over`.
///
/// Only [`ClauseIdentity::resolve`] makes one: a request cannot name it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseIdentity {
    occurrence: OccurrenceKey,
    over: Option<NodeKey>,
}

impl ClauseIdentity {
    /// The clause's O-07 occurrence: node, role and ordinal.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The `over` parameter's checked node.
    pub fn over(&self) -> Option<NodeKey> {
        self.over.clone()
    }

    /// FR-940: the identity of the package-form `clause` in the admitted
    /// package of the package route, read from its own bytes: the clause's
    /// `temporal_clause_names` row names its node and claim occurrence, and
    /// the I2 lookup resolves that root as [`Self::resolve`] resolves the
    /// source package's read-back.
    ///
    /// # Errors
    ///
    /// [`ClauseRootRefusal::Undeclared`] when no single name row names the
    /// clause; as [`Self::resolve`] when the lookup does not find its root.
    pub fn of_admitted(
        package: &fsl_package::CheckedPackageV2,
        clause: &CheckedTemporalClause,
    ) -> Result<Self, ClauseRootRefusal> {
        let undeclared = || ClauseRootRefusal::Undeclared {
            name: clause.name().to_owned(),
        };
        let mut rows = package
            .temporal_clause_names()
            .unwrap_or_default()
            .iter()
            .filter(|row| &*row.name == clause.name());
        let row = rows.next().ok_or_else(undeclared)?;
        if rows.next().is_some() {
            return Err(undeclared());
        }
        let key = fsl_package::checked_views::temporal_subject(package, &row.declaration)
            .ok()
            .and_then(|view| view.declaration_key())
            .ok_or_else(undeclared)?;
        let triple = OccurrenceKey::new(
            key,
            filament_exact::Origin::new(filament_exact::Role::new("claim"), row.occurrence),
        );
        let root = resolve_clause_root(package, &triple)?;
        Ok(Self {
            occurrence: root.occurrence().clone(),
            over: root.over().cloned(),
        })
    }

    /// The identity of `clause` in `package`, read from the package's
    /// emitted `filament.checked-package/v2` bytes through the I2 lookup.
    /// `domain_packages` is FR-056's package input the read checks the
    /// package's model selections against. The emission does not run S3, so
    /// `package` was checked with the same `temporal_clock_selections`
    /// (`PackageDeclarations::with_clock_units`): a `duration` window in a
    /// unit other than the clock unit is converted there (FR-337
    /// "Aggregate formula emission"), and a package checked without them
    /// refuses at emission with `InvalidTemporalClockValue`.
    ///
    /// # Errors
    ///
    /// [`ClauseRootRefusal`] when `package` does not declare `clause`, does
    /// not emit or read back, or the lookup does not find the clause's root;
    /// or `cancel` stopped the emission.
    pub fn resolve(
        package: &CheckedPackage,
        domain_packages: &fsl_semantics::model::intake::PackageInput,
        clause: &CheckedTemporalClause,
        temporal_clock_selections: &[TemporalClockSelection],
        cancel: &Cancel,
        emit: fsl_package::EmitLimits,
    ) -> Result<Self, ClauseRootRefusal> {
        let node = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause() == clause)
            .ok_or_else(|| ClauseRootRefusal::Undeclared {
                name: clause.name().to_owned(),
            })?;
        let triple = node_occurrence(node);
        // FR-1000's pending reference-literal node lowers to a typed literal
        // leaf which the current I2 expression vocabulary cannot admit yet.
        // This identity and `over` are already in the immutable S3-checked
        // package, so resolve them directly when this clause contains that
        // node instead of round-tripping a representation I2 cannot express.
        if temporal_clause_has_reference_literal(node) {
            return Ok(Self {
                occurrence: triple,
                over: Some(node.over()),
            });
        }
        let (admitted, library) = read_back(
            package,
            domain_packages,
            temporal_clock_selections,
            cancel,
            emit,
        )?;
        let read = admitted
            .package(&library)
            .ok_or_else(|| ClauseRootRefusal::Package("the read recorded no package".to_owned()))?;
        let root = resolve_clause_root(read, &triple)?;
        Ok(Self {
            occurrence: root.occurrence().clone(),
            over: root.over().cloned(),
        })
    }
}

/// `package` emitted under `cancel` and read back through the I2 reader,
/// with the library name the read recorded it under.
fn read_back(
    package: &CheckedPackage,
    domain_packages: &fsl_semantics::model::intake::PackageInput,
    temporal_clock_selections: &[TemporalClockSelection],
    cancel: &Cancel,
    emit: fsl_package::EmitLimits,
) -> Result<(AdmittedPackages, LibraryName), ClauseRootRefusal> {
    let emission = emit_checked_with_temporal_selections_and_cancel(
        package,
        temporal_clock_selections,
        cancel,
        emit,
    )
    .map_err(|refusal| match refusal {
        EmitRefusal::Cancelled { cause } => ClauseRootRefusal::Cancelled(cause),
        other => ClauseRootRefusal::Package(other.to_string()),
    })?;
    let library = LibraryName::new(emission.package().name())
        .map_err(|refusal| ClauseRootRefusal::Package(format!("{refusal:?}")))?;
    let mut admitted = AdmittedPackages::default();
    read_import_view(
        package,
        &emission,
        library.clone(),
        domain_packages,
        &mut admitted,
        // The clause-root lookup reads nodes and performs no native identity
        // join, so it asks for no native storage: a join attempted here is
        // refused at once rather than allowed an invented allowance.
        V2ReadLimits::bounded(0),
        emit,
    )
    .map_err(|refusal| ClauseRootRefusal::Package(format!("{refusal:?}")))?;
    Ok((admitted, library))
}

/// A state-graph claim's identity in its package: the checked occurrence and
/// the node of its `over` parameter, `None` for a claim with no `over`
/// (FR-410 "Behavior").
///
/// Only [`StateGraphIdentity::resolve`] makes one: a request cannot name it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphIdentity {
    occurrence: OccurrenceKey,
    over: Option<NodeKey>,
}

impl StateGraphIdentity {
    /// The claim's O-07 occurrence: node, role and ordinal.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The `over` parameter's checked node.
    pub fn over(&self) -> Option<NodeKey> {
        self.over.clone()
    }

    /// The identity of `claim` in `package`, read from the package's emitted
    /// `filament.checked-package/v2` bytes through the I2 state-graph lookup.
    ///
    /// # Errors
    ///
    /// [`ClauseRootRefusal`] when `claim` carries no occurrence, the package
    /// does not emit or read back, or the lookup does not find the claim's
    /// triple; or `cancel` stopped the emission.
    pub fn resolve(
        package: &CheckedPackage,
        domain_packages: &fsl_semantics::model::intake::PackageInput,
        claim: &CheckedStateGraphClaim,
        temporal_clock_selections: &[TemporalClockSelection],
        cancel: &Cancel,
        emit: fsl_package::EmitLimits,
    ) -> Result<Self, ClauseRootRefusal> {
        let triple = claim
            .occurrence()
            .ok_or_else(|| ClauseRootRefusal::Undeclared {
                name: claim.name().to_owned(),
            })?;
        let (admitted, library) = read_back(
            package,
            domain_packages,
            temporal_clock_selections,
            cancel,
            emit,
        )?;
        let read = admitted
            .package(&library)
            .ok_or_else(|| ClauseRootRefusal::Package("the read recorded no package".to_owned()))?;
        let root = resolve_state_graph_root(read, triple)
            .map_err(|fault| ClauseRootRefusal::Package(format!("{fault:?}")))??;
        Ok(Self {
            occurrence: root.occurrence().clone(),
            over: root.over().cloned(),
        })
    }
}

/// An HP-5 clause's identity in its package, resolved through FR-488's I2
/// hyper-root lookup. A replay request cannot supply this identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperIdentity {
    occurrence: OccurrenceKey,
}

impl HyperIdentity {
    /// The checked `claim` occurrence of the hyper clause.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// Resolve the assembled hyper clause through the emitted package's I2
    /// reader and FR-488's hyper-root lookup.
    ///
    /// # Errors
    ///
    /// [`ClauseRootRefusal`] when the package does not assemble and read back
    /// the clause's claim root, or when `cancel` stops emission.
    pub fn resolve(
        package: &CheckedPackage,
        domain_packages: &fsl_semantics::model::intake::PackageInput,
        clause: &CheckedHyperClause,
        temporal_clock_selections: &[TemporalClockSelection],
        cancel: &Cancel,
        emit: fsl_package::EmitLimits,
    ) -> Result<Self, ClauseRootRefusal> {
        let assembled = package
            .graph()
            .hyper_clauses()
            .iter()
            .find(|candidate| {
                candidate.name() == clause.name()
                    && candidate.assembled_occurrence() == clause.assembled_occurrence()
            })
            .ok_or_else(|| ClauseRootRefusal::Undeclared {
                name: clause.name().to_owned(),
            })?;
        let occurrence = assembled.assembled_occurrence().cloned().ok_or_else(|| {
            ClauseRootRefusal::Undeclared {
                name: clause.name().to_owned(),
            }
        })?;
        // As for temporal clauses above, keep a hyper clause's pending
        // reference literal in its checked representation. The checked
        // assembled occurrence is package-owned and does not come from the
        // replay request.
        if hyper_clause_has_reference_literal(clause) {
            return Ok(Self { occurrence });
        }
        let (admitted, library) = read_back(
            package,
            domain_packages,
            temporal_clock_selections,
            cancel,
            emit,
        )?;
        let read = admitted
            .package(&library)
            .ok_or_else(|| ClauseRootRefusal::Package("the read recorded no package".to_owned()))?;
        let root = resolve_hyper_root(read, &occurrence)
            .map_err(|fault| ClauseRootRefusal::Package(format!("{fault:?}")))?;
        if root.occurrence() != &occurrence {
            return Err(ClauseRootRefusal::Package(
                "the I2 hyper root changed the checked occurrence".to_owned(),
            ));
        }
        Ok(Self { occurrence })
    }
}

fn node_has_reference_literal(root: fsl_semantics::check::CheckedNode<'_>) -> bool {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if matches!(
            node.kind(),
            filament_checked::NodeKind::ReferenceLiteral { .. }
        ) {
            return true;
        }
        pending.extend(node.children());
    }
    false
}

fn temporal_clause_has_reference_literal(node: &fsl_semantics::check::TemporalClauseNode) -> bool {
    let clause = node.clause();
    let formula = clause.formula();
    let atom_has_literal = (0..formula.nodes().len()).any(|index| {
        let Some(atom) = formula.node_id(index) else {
            return false;
        };
        node.condition(atom)
            .is_some_and(|(root, _)| node_has_reference_literal(root))
            || node.aggregate_conditions(atom).is_some_and(|conditions| {
                [conditions.value, conditions.filter, conditions.predicate]
                    .into_iter()
                    .flatten()
                    .chain([conditions.threshold])
                    .any(|(root, _)| node_has_reference_literal(root))
            })
    });
    atom_has_literal
        || node
            .guard()
            .is_some_and(|(root, _)| node_has_reference_literal(root))
        || (0..clause.captures().len()).any(|index| {
            node.capture(index)
                .is_some_and(|(root, _)| node_has_reference_literal(root))
        })
}

fn hyper_clause_has_reference_literal(clause: &CheckedHyperClause) -> bool {
    (0..clause.body().nodes().len()).any(|index| {
        clause
            .body()
            .node_id(index)
            .and_then(|atom| clause.condition(atom))
            .is_some_and(|(root, _)| node_has_reference_literal(root))
    })
}

/// `clause-root-unresolved` (FR-337 "Behavior", FR-338): the request's
/// package gives the clause no root, so the request is at fault and no key
/// context exists.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClauseRootRefusal {
    /// The package declares no clause equal to the request's.
    #[error("clause-root-unresolved: the package declares no clause {name}")]
    Undeclared {
        /// The request's clause name.
        name: String,
    },
    /// The I2 lookup does not find the clause's triple in the emitted
    /// package.
    #[error("clause-root-unresolved: {0}")]
    Unresolved(#[from] ClauseRootUnresolved),
    /// The package did not emit or did not read back, so it has no root to
    /// look up.
    #[error("clause-root-unresolved: the package did not emit or read: {0}")]
    Package(String),
    /// The handle stopped the emission.
    #[error("the emission was cancelled: {0:?}")]
    Cancelled(CancelCause),
}
