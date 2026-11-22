// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-961 step 4 and 5: settle the requirement items of a run call over the
//! run's registry, through the one settlement path `analyze` uses.

use filament_exact::Cancel;
use fsl_analyze::analyze::document::{
    resolve_call_items_from_view, settle_run, AnalyzeDocumentError, AnalyzeRun, CallItem,
    Negotiate, SettledRun, SubjectSelection,
};
use fsl_analyze::analyze::{AnalyzeLimits, Engines, SourceProvision};
use fsl_semantics::model::observation::DocumentProvision;

use super::requirements::Requirements;
use super::{Result, RunCause};

/// What a run settles its call's requirement items with: the caller's
/// negotiator, the run's registry and the settlement's cancel handle.
pub(super) struct Settle<'a> {
    /// The caller's negotiator (FSpec FR-711).
    pub negotiate: &'a dyn Negotiate,
    /// `Engines::fsl()` with the caller's providers registered beside it.
    pub engines: &'a Engines,
    /// The settlement's cancel handle.
    pub cancel: &'a Cancel,
}

/// Settle the items of the call of `function` that `checked` was compiled
/// for, in FR-395's package order, under `selection`'s named backends.
///
/// # Errors
///
/// [`RunCause::UnknownRequirementClaim`] for an `items` entry whose claim
/// names no item of the call, before any item is negotiated; the analyze
/// settlement's refusal or fault otherwise.
pub(super) fn settle_call(
    checked: &fsl_replay::spine::CheckedUnit,
    members: &fsl_replay::CallRequirements,
    selection: &Requirements,
    packages: &fsl_semantics::model::intake::PackageInput,
    settle: &Settle<'_>,
) -> Result<SettledRun> {
    let claims = members
        .members()
        .iter()
        .map(|member| fsl_replay::claim_locus_wire(member.occurrence()))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| RunCause::AnalyzeFault)?;
    let resolved = resolve_held_claims(checked, packages, selection)?;
    // FR-960 "Unknown claim": a claim of no item of the call refuses the
    // request before any item is negotiated, naming the claim as supplied.
    if let Some(position) = resolved
        .items
        .iter()
        .position(|entry| !claims.contains(&entry.claim))
    {
        return Err(RunCause::UnknownRequirementClaim {
            claim: selection.items[position].claim.clone(),
        });
    }
    let selection = resolved.as_ref();
    let items: Vec<CallItem> = members
        .members()
        .iter()
        .zip(&claims)
        .map(|(member, claim)| {
            let named = selection.named_for(claim);
            CallItem {
                claim: member.occurrence().clone(),
                request_index: member.request_index(),
                backend: named.backend.clone(),
                options: named.options.clone(),
            }
        })
        .collect();
    let package = checked.package();
    let mut registry = fsl_route::Registry::new();
    for descriptor in settle.engines.registrations() {
        registry
            .register(descriptor.clone())
            .map_err(|_| RunCause::AnalyzeFault)?;
    }
    let read = fsl_package::read_evaluation_package(
        package,
        fsl_semantics::library::LibraryName::new(super::package_name(checked.source().identity()))
            .map_err(|_| RunCause::AnalyzeFault)?,
        packages,
        fsl_package::V2ReadLimits::bounded(u64::MAX),
        AnalyzeLimits::unbounded().emit,
    )
    .map_err(|_| RunCause::AnalyzeFault)?;
    let resolved =
        resolve_call_items_from_view(package, &read, members.function(), &items, &registry)
            .map_err(|_| RunCause::AnalyzeFault)?;
    let mut provision = SourceProvision::default();
    for source in std::iter::once(checked.source()).chain(checked.libraries()) {
        provision.insert(source.reference(), source.text().as_bytes().to_vec());
    }
    let documents = DocumentProvision::new();
    let subject = SubjectSelection::default_over(&documents);
    let run = AnalyzeRun {
        package,
        binding: subject.binding(packages, &documents),
        provision,
        items: resolved,
        engines: settle.engines,
        limits: AnalyzeLimits::unbounded(),
    };
    settle_run(run, settle.negotiate, settle.cancel).map_err(|error| match error {
        AnalyzeDocumentError::Refused(refusal) => RunCause::AnalyzeRefused(Box::new(refusal)),
        AnalyzeDocumentError::Fault => RunCause::AnalyzeFault,
        AnalyzeDocumentError::Identity(error) => super::identity_cause(error),
    })
}

/// The path segments of `claim`'s node that hold a `node` member, which is an
/// index segment of FSL FR-621's document-held form, with their position.
fn index_segments(claim: &serde_json::Value) -> Vec<(usize, &serde_json::Value)> {
    claim["node"]["path"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(position, segment)| Some((position, segment.get("node")?)))
        .collect()
}

/// FR-960 "Inputs": an `items` claim is FSpec FR-331's structural spelling or
/// the document-held form a run outcome reports (FR-963, FSL FR-621 "Node ids
/// in an item"), each operand of the node's path an index segment
/// `{"node": i}` into the graph of the request's package. Resolve every claim
/// holding an index segment to its structural key through that graph
/// (`WireGraph::structural_value`, the one resolver) so it compares with the
/// call's own claims; a structural claim is kept as written.
///
/// # Errors
///
/// A package the strict reader refuses or that does not emit refuses as the
/// analyze route does; a malformed or out-of-range index segment refuses
/// `invalid-request` at that segment's `node` member; two entries naming one
/// claim in either form refuse at the later entry.
fn resolve_held_claims<'a>(
    checked: &fsl_replay::spine::CheckedUnit,
    packages: &fsl_semantics::model::intake::PackageInput,
    selection: &'a Requirements,
) -> Result<std::borrow::Cow<'a, Requirements>> {
    if selection
        .items
        .iter()
        .all(|entry| index_segments(&entry.claim).is_empty())
    {
        return Ok(std::borrow::Cow::Borrowed(selection));
    }
    let graph = fsl_analyze::analyze::DocumentGraphs::of_package(
        checked.package(),
        packages,
        AnalyzeLimits::unbounded().emit,
    )
    .map_err(|refusal| RunCause::AnalyzeRefused(Box::new(refusal)))?;
    let mut resolved = selection.clone();
    for (position, entry) in resolved.items.iter_mut().enumerate() {
        if index_segments(&entry.claim).is_empty() {
            continue;
        }
        let at = format!("/call/requirements/items/{position}/claim/node");
        let node = graph
            .structural_value(&entry.claim["node"])
            .map_err(|message| {
                let bad = index_segments(&entry.claim).into_iter().find(|(_, index)| {
                    index
                        .as_u64()
                        .and_then(|index| usize::try_from(index).ok())
                        .is_none_or(|index| index >= graph.keys().len())
                });
                RunCause::ClauseRequest {
                    member: bad.map_or_else(
                        || at.clone(),
                        |(segment, _)| format!("{at}/path/{segment}/node"),
                    ),
                    reason: message,
                }
            })?;
        entry.claim["node"] = node;
    }
    for (position, entry) in resolved.items.iter().enumerate() {
        if resolved.items[..position]
            .iter()
            .any(|seen| seen.claim == entry.claim)
        {
            return Err(RunCause::ClauseRequest {
                member: format!("/call/requirements/items/{position}"),
                reason: "an entry repeats the claim of an earlier entry".to_owned(),
            });
        }
    }
    Ok(std::borrow::Cow::Owned(resolved))
}
