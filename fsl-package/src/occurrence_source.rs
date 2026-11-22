// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: source attribution of an explicitly selected admitted occurrence.
//!
//! Child paths identify semantic visits; only the source map supplies byte
//! regions. This reader never obtains source text or selects an occurrence
//! merely because its structural node matches.

use std::cmp::Ordering;

use filament_contract_model::{
    CheckedBodyLocationOrigin, CheckedBodyLocationReadError, CheckedKeyOrdinal, CheckedNodeId,
    CheckedOccurrence, CheckedPackageV2, NativeJoinBudget, NativeJoinLocus,
};
use filament_exact::{NodeKey, Origin, Role};
use fsl_foundation::source::provenance::{OccurrenceKey, SourceRegion};

use crate::native_row::{self, NativeRowError};

/// One exact admitted occurrence and its selected source region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OccurrenceSource {
    /// Structural occurrence identity, including its role and ordinal.
    pub occurrence: OccurrenceKey,
    /// The source map's byte region, without source text.
    pub region: SourceRegion,
}

impl OccurrenceSource {
    /// Compare source positions only when both regions name the same source.
    /// Equal starts are broken by end, then the explicit occurrence origin.
    /// Different source identities carry no declaration-order relationship.
    pub fn same_source_order(&self, other: &Self) -> Option<Ordering> {
        (self.region.source() == other.region.source()).then(|| {
            self.region
                .start()
                .cmp(&other.region.start())
                .then_with(|| self.region.end().cmp(&other.region.end()))
                .then_with(|| self.occurrence.origin().cmp(other.occurrence.origin()))
        })
    }
}

/// The complete identity of a captured function-body visit.
pub struct BodySourceCoordinate<'a> {
    /// The owning local function, never a function from a different package.
    pub function: &'a CheckedNodeId,
    /// The application reached by this visit.
    pub application: &'a CheckedNodeId,
    /// The application's explicit role and ordinal.
    pub occurrence: &'a CheckedOccurrence,
    /// The captured term coordinate within that occurrence.
    pub term_ordinal: u64,
    /// The reconstructed semantic child path, not a byte span.
    pub path: &'a [CheckedKeyOrdinal],
}

/// Why an admitted occurrence has no unique selected source attribution.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum OccurrenceSourceRefusal {
    /// The selected node is absent from the owning admitted package.
    #[error("unknown source node")]
    UnknownNode,
    /// The explicit role and ordinal are not one of the node's occurrences.
    #[error("unknown source occurrence")]
    UnknownOccurrence,
    /// The source map contains no region for the selected occurrence.
    #[error("source region is unavailable")]
    MissingRegion,
    /// More than one region requires an explicit region selection.
    #[error("source region selection is ambiguous")]
    AmbiguousRegion,
    /// The supplied region is not one of the occurrence's admitted regions.
    #[error("selected region does not belong to the occurrence")]
    UnknownRegion,
    /// The existing bounded body-location reader refused the visit.
    #[error(transparent)]
    Body(#[from] CheckedBodyLocationReadError),
}

/// Read the exact source region of an explicit occurrence in its owning
/// admitted package. Multiple regions require `selected`; none is guessed.
pub(crate) fn occurrence_source(
    package: &CheckedPackageV2,
    application: &CheckedNodeId,
    occurrence: &CheckedOccurrence,
    selected: Option<&SourceRegion>,
) -> Result<OccurrenceSource, OccurrenceSourceRefusal> {
    let key = occurrence_key(package, application, occurrence)?;
    source_for_key(
        package,
        application,
        occurrence,
        key,
        selected,
        &mut Unmetered,
    )
}

/// A metered attribution preserves the native denial separately from a
/// semantic attribution refusal, without changing legacy callers' enum.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum MeteredOccurrenceSourceRefusal {
    #[error(transparent)]
    Source(#[from] OccurrenceSourceRefusal),
    #[error(transparent)]
    Native(#[from] NativeRowError),
}

struct Unmetered;

trait SourceWork {
    type Error: From<OccurrenceSourceRefusal>;
    fn visit(&mut self) -> Result<(), Self::Error>;
    fn items(&mut self, count: usize) -> Result<(), Self::Error>;
    fn text(&mut self, value: &str) -> Result<(), Self::Error>;
}

impl SourceWork for Unmetered {
    type Error = OccurrenceSourceRefusal;
    fn visit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn items(&mut self, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }
    fn text(&mut self, _: &str) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct Metered<'a> {
    budget: &'a mut NativeJoinBudget,
    locus: NativeJoinLocus,
}

impl SourceWork for Metered<'_> {
    type Error = MeteredOccurrenceSourceRefusal;
    fn visit(&mut self) -> Result<(), Self::Error> {
        Ok(native_row::work(self.budget, self.locus, 1)?)
    }
    fn items(&mut self, count: usize) -> Result<(), Self::Error> {
        Ok(native_row::items(self.budget, self.locus, count)?)
    }
    fn text(&mut self, value: &str) -> Result<(), Self::Error> {
        Ok(native_row::text(self.budget, self.locus, value)?)
    }
}

/// Read an occurrence on a composition whose admitted declaration and
/// structural key the caller already retained. No graph or body walk repeats.
pub(crate) fn composition_occurrence_source(
    package: &CheckedPackageV2,
    application: &CheckedNodeId,
    occurrence: &CheckedOccurrence,
    node: &NodeKey,
    budget: &mut NativeJoinBudget,
) -> Result<OccurrenceSource, MeteredOccurrenceSourceRefusal> {
    let locus = native_row::start(budget, "component_composition")?;
    let mut work = Metered { budget, locus };
    let role = crate::checked_v2::role_spelling(&occurrence.role);
    work.text(role)?;
    let key = OccurrenceKey::new(
        node.clone(),
        Origin::new(Role::new(role), occurrence.ordinal),
    );
    source_for_key(package, application, occurrence, key, None, &mut work)
}

fn source_for_key<W: SourceWork>(
    package: &CheckedPackageV2,
    application: &CheckedNodeId,
    occurrence: &CheckedOccurrence,
    key: OccurrenceKey,
    selected: Option<&SourceRegion>,
    work: &mut W,
) -> Result<OccurrenceSource, W::Error> {
    let mut found = None;
    for entry in package.source_map() {
        work.visit()?;
        if entry.node_id == *application
            && entry.role == occurrence.role
            && entry.ordinal == occurrence.ordinal
        {
            found = Some(entry);
            break;
        }
    }
    let entry = found.ok_or(OccurrenceSourceRefusal::MissingRegion)?;
    work.items(entry.regions.len())?;
    let mut regions = Vec::with_capacity(entry.regions.len());
    for region in &entry.regions {
        work.visit()?;
        work.text(&region.source.authority)?;
        work.text(&region.source.identity)?;
        regions.push(
            crate::checked_v2::source_region(region)
                .map_err(|_| OccurrenceSourceRefusal::MissingRegion)?,
        );
    }
    let region = match (selected, regions.as_slice()) {
        (Some(selected), _) => {
            let mut found = None;
            for region in &regions {
                work.visit()?;
                if region == selected {
                    found = Some(region);
                    break;
                }
            }
            found.ok_or(OccurrenceSourceRefusal::UnknownRegion)?
        }
        (None, [only]) => only,
        (None, []) => return Err(OccurrenceSourceRefusal::MissingRegion.into()),
        (None, _) => return Err(OccurrenceSourceRefusal::AmbiguousRegion.into()),
    };
    work.text(region.source().authority())?;
    work.text(region.source().identity())?;
    Ok(OccurrenceSource {
        occurrence: key,
        region: region.clone(),
    })
}

/// Validate the explicit occurrence even when a consumer needs its identity
/// alone, such as a captured function-body quantifier's witness.
pub(crate) fn occurrence_key(
    package: &CheckedPackageV2,
    application: &CheckedNodeId,
    occurrence: &CheckedOccurrence,
) -> Result<OccurrenceKey, OccurrenceSourceRefusal> {
    let node = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.node_id == *application)
        .ok_or(OccurrenceSourceRefusal::UnknownNode)?;
    if application.package.as_ref() != package.name() {
        return Err(OccurrenceSourceRefusal::UnknownNode);
    }
    if !node.occurrences.contains(occurrence) {
        return Err(OccurrenceSourceRefusal::UnknownOccurrence);
    }
    let structural = package
        .structural_key(application)
        .ok_or(OccurrenceSourceRefusal::UnknownNode)?;
    Ok(OccurrenceKey::new(
        fsl_semantics::check::wire_id::node_key(structural)
            .map_err(|_| OccurrenceSourceRefusal::UnknownNode)?,
        Origin::new(
            Role::new(crate::checked_v2::role_spelling(&occurrence.role)),
            occurrence.ordinal,
        ),
    ))
}

/// Validate a complete body visit through the existing lending cursor before
/// resolving its occurrence. Returns cumulative cursor work so callers can
/// debit their shared allowance without resetting it between selections.
pub(crate) fn body_source(
    package: &CheckedPackageV2,
    coordinate: BodySourceCoordinate<'_>,
    selected: Option<&SourceRegion>,
    work_limit: u64,
) -> Result<(OccurrenceSource, u64), OccurrenceSourceRefusal> {
    let mut cursor = package.body_locations(coordinate.function, work_limit)?;
    let mut found = false;
    while let Some(view) = cursor.next_location()? {
        let row = view.row;
        let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &row.location.origin;
        if function == coordinate.function
            && row.application == *coordinate.application
            && row.occurrence == *coordinate.occurrence
            && row.term_ordinal == coordinate.term_ordinal
            && view.path == coordinate.path
        {
            found = true;
        }
    }
    if !found {
        return Err(CheckedBodyLocationReadError::MissingRow.into());
    }
    let source = occurrence_source(
        package,
        coordinate.application,
        coordinate.occurrence,
        selected,
    )?;
    Ok((source, cursor.work_consumed()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-531-AC-39", "FSpec-FR-322-AC-510")]
    #[test]
    fn repeated_body_occurrences_keep_their_own_regions_and_paths() {
        let (text, checked) = crate::emit::tests::package_from_text(
            "function f using v(x: Integer): Integer pure { (x + 1) * (x + 1) }\n",
        );
        let emission =
            crate::emit::emit_checked(&checked, crate::checked::EmitLimits::new(1 << 20)).unwrap();
        let package = match filament_contract_model::read_checked_package(
            emission.package().bytes(),
            crate::checked_v2::V2ReadLimits::fixture().for_ir(),
            emission.evidence(),
        ) {
            filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(package) => package,
            other => panic!("authentic repeated body must read: {other:?}"),
        };
        let rows = &package.checked_term_data().unwrap().body_locations;
        let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &rows[0].location.origin;
        let mut cursor = package.body_locations(function, 100_000).unwrap();
        let mut visits = Vec::new();
        while let Some(view) = cursor.next_location().unwrap() {
            visits.push((view.row.clone(), view.path.to_vec()));
        }
        let consumed = cursor.work_consumed();
        let source_at = |row: &filament_contract_model::CheckedBodySemanticLocation,
                         path: &[CheckedKeyOrdinal],
                         limit| {
            body_source(
                &package,
                BodySourceCoordinate {
                    function,
                    application: &row.application,
                    occurrence: &row.occurrence,
                    term_ordinal: row.term_ordinal,
                    path,
                },
                None,
                limit,
            )
        };
        let repeated: Vec<_> = visits
            .iter()
            .filter(|(row, _)| {
                visits
                    .iter()
                    .filter(|(other, _)| other.application == row.application)
                    .count()
                    == 2
            })
            .collect();
        assert_eq!(
            repeated.len(),
            2,
            "both visits to the coalesced addition are captured"
        );
        let first = source_at(&repeated[0].0, &repeated[0].1, consumed).unwrap();
        let second = source_at(&repeated[1].0, &repeated[1].1, consumed).unwrap();
        assert_ne!(first.0.occurrence, second.0.occurrence);
        assert_ne!(first.0.region, second.0.region);
        for located in [&first.0, &second.0] {
            let start = usize::try_from(located.region.start()).unwrap();
            let end = usize::try_from(located.region.end()).unwrap();
            assert!(text.get(start..end).unwrap().contains("x + 1"));
        }
        assert_eq!(
            occurrence_source(
                &package,
                &repeated[0].0.application,
                &repeated[0].0.occurrence,
                Some(&second.0.region)
            ),
            Err(OccurrenceSourceRefusal::UnknownRegion)
        );
        let mut unknown = repeated[0].0.occurrence.clone();
        unknown.ordinal = u64::MAX;
        assert_eq!(
            occurrence_source(&package, &repeated[0].0.application, &unknown, None),
            Err(OccurrenceSourceRefusal::UnknownOccurrence)
        );
        assert_eq!(first.0.same_source_order(&second.0), Some(Ordering::Less));
        assert_eq!(first.1, consumed);
        assert!(matches!(
            source_at(&repeated[0].0, &repeated[1].1, consumed),
            Err(OccurrenceSourceRefusal::Body(
                CheckedBodyLocationReadError::MissingRow
            ))
        ));
        assert!(matches!(
            source_at(&repeated[0].0, &repeated[0].1, consumed - 1),
            Err(OccurrenceSourceRefusal::Body(
                CheckedBodyLocationReadError::WorkExhausted { .. }
            ))
        ));
    }
}
