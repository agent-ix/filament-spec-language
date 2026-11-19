// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-961 step 4 and 5: settle the requirement items of a run call over the
//! run's registry, through the one settlement path `analyze` uses.

use filament_exact::Cancel;
use fsl_analyze::analyze::document::{
    resolve_call_items, settle_run, AnalyzeDocumentError, AnalyzeRun, CallItem, Negotiate,
    SettledRun, SubjectSelection,
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
    // FR-960 "Unknown claim": a claim of no item of the call refuses the
    // request before any item is negotiated.
    if let Some(entry) = selection
        .items
        .iter()
        .find(|entry| !claims.contains(&entry.claim))
    {
        return Err(RunCause::UnknownRequirementClaim {
            claim: entry.claim.clone(),
        });
    }
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
    let resolved = resolve_call_items(package, members.function(), &items, &registry);
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
    })
}
