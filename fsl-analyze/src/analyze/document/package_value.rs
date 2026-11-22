// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: value-validity request facts read from admitted requirement rows.

use filament_contract_model::{
    CapabilityKind, DomainKey as WireKey, DomainKind as WireKind, RequirementExtent,
    RequirementItemRow,
};
use filament_exact::{Origin, Role};
use fsl_foundation::bound::{DomainKey, DomainKind};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_package::BarePackage;
use fsl_route::request::RequirementItem;
use fsl_route::Registry;
use fsl_semantics::check::{Capability, PathGuard};
use fsl_semantics::family::ClaimExtent;

pub(crate) fn occurrence(
    package: &BarePackage<'_>,
    row: &RequirementItemRow,
) -> Option<OccurrenceKey> {
    Some(OccurrenceKey::new(
        package.node_key(&row.claim)?,
        Origin::new(Role::new("expression"), row.occurrence.ordinal),
    ))
}

pub(super) fn extent(package: &BarePackage<'_>, row: &RequirementItemRow) -> Option<ClaimExtent> {
    let RequirementExtent::Unbounded(domains) = &row.extent else {
        return Some(ClaimExtent::Bounded);
    };
    let domains = domains
        .iter()
        .map(|(key, kind)| {
            let key = match key {
                WireKey::Node { node, path } => DomainKey::Node {
                    node: package.node_key(node)?,
                    path: path.clone(),
                },
                WireKey::Population {
                    member_type,
                    ordinal,
                } => DomainKey::Population {
                    member_type: package.node_key(member_type)?,
                    ordinal: *ordinal,
                },
            };
            let kind = match kind {
                WireKind::Collection => DomainKind::Collection,
                WireKind::Population => DomainKind::Population,
                WireKind::Integer => DomainKind::Integer,
                WireKind::Recursive => DomainKind::Recursive,
                WireKind::Quantity => DomainKind::Quantity,
                WireKind::Loop => DomainKind::Loop,
                WireKind::InfiniteTrace => DomainKind::InfiniteTrace,
            };
            Some((key, kind))
        })
        .collect::<Option<std::collections::BTreeMap<_, _>>>()?;
    Some(ClaimExtent::from_domains(domains))
}

/// Why an admitted row has no supported request-item read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PackageRequirementRefusal {
    UnsupportedKind(CapabilityKind),
    MissingFact(&'static str),
}

pub(crate) fn requirement_item(
    package: &BarePackage<'_>,
    row: &RequirementItemRow,
    index: RequestIndex,
    registry: &Registry,
    named: Option<&BackendId>,
) -> Result<RequirementItem, PackageRequirementRefusal> {
    if row.kind != CapabilityKind::ValueValidity {
        return Err(PackageRequirementRefusal::UnsupportedKind(row.kind));
    }
    item(package, row, index, registry, named).ok_or(PackageRequirementRefusal::MissingFact(
        "value-validity-record",
    ))
}

fn item(
    package: &BarePackage<'_>,
    row: &RequirementItemRow,
    index: RequestIndex,
    registry: &Registry,
    named: Option<&BackendId>,
) -> Option<RequirementItem> {
    if row.kind != CapabilityKind::ValueValidity {
        return None;
    }
    let record = row.record.as_ref()?;
    let expression = |node, ordinal| {
        Some(OccurrenceKey::new(
            package.node_key(node)?,
            Origin::new(Role::new("expression"), ordinal),
        ))
    };
    let conversion = match &record.conversion {
        Some((node, ordinal)) => Some(expression(node, *ordinal)?),
        None => None,
    };
    let guards = record
        .path_condition
        .iter()
        .map(|guard| {
            Some(PathGuard::new(
                expression(&guard.guard, guard.occurrence)?,
                guard.holds,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(RequirementItem::value_operation(
        index,
        occurrence(package, row)?,
        &extent(package, row)?,
        package.node_key(&record.result_bound)?,
        conversion,
        guards,
        registry.candidates(Capability::ValueValidity, named),
    ))
}
