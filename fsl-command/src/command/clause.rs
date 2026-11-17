// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-312: turn one closed native-run clause selection into a spine request.

use std::collections::BTreeMap;

use fsl_replay::spine::{
    ClauseRequestParts, ClauseRunRequest, ClauseRunSource, DependencyInput, SpineLimits,
    SuppliedLibrary, SuppliedObservation,
};

use super::{wire, FormalSource, Intake, Result, RunCause};

fn invalid(member: impl Into<String>, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.into(),
        reason: reason.into(),
    }
}

fn observation(
    intake: &mut Intake<'_>,
    key: &str,
    file: &str,
) -> Result<SuppliedObservation> {
    let remaining = intake.remaining;
    let bytes = intake.file(file, remaining)?;
    let mut observation = SuppliedObservation::from_document(key, bytes)
        .map_err(|error| invalid(error.member, error.reason))?;
    observation.path = Some(file.to_owned());
    Ok(observation)
}

pub(super) fn read(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    clause: &wire::ClauseRun,
    source: FormalSource,
) -> Result<ClauseRunRequest> {
    if request
        .models
        .iter()
        .any(|model| !super::is_domain_package(model))
    {
        return Err(invalid("models", "expected semantic-ir/2.0.0"));
    }
    let documents = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let packages = fsl_semantics::model::intake::package_input(
        documents
            .iter()
            .map(|document| document.source().text().as_bytes()),
    )
    .map_err(|refusal| invalid("models", refusal.detail))?;
    let libraries = request
        .libraries
        .iter()
        .enumerate()
        .map(|(index, library)| {
            Ok((
                library,
                intake.source(&library.source, &format!("libraries[{index}].source"))?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let dependencies =
        DependencyInput::new(libraries.iter().map(|(library, read)| SuppliedLibrary {
            identity: library.identity.clone(),
            source: read.source().identity().clone(),
            path: read.source().path().to_owned(),
            bytes: read.source().text().as_bytes().to_vec(),
        }))
        .map_err(|error| invalid("libraries", error.to_string()))?;
    let observation_limits =
        fsl_replay::spine::ObservationLimitsWire::resolve(clause.observation_limits.as_ref())
            .map_err(|error| invalid(error.member, error.reason))?;
    let mut by_file = BTreeMap::<String, SuppliedObservation>::new();
    let mut observations = BTreeMap::new();
    for (key, file) in &clause.observations {
        let observed = if let Some(existing) = by_file.get(file) {
            existing.clone()
        } else {
            let observed = observation(intake, key, file)?;
            by_file.insert(file.clone(), observed.clone());
            observed
        };
        observations.insert(key.clone(), observed);
    }
    let ledger = clause
        .selection
        .ledger_path()
        .map(|path| {
            let bytes = intake.file(path, intake.remaining)?;
            fsl_replay::spine::read_ledger(&bytes)
                .map_err(|error| invalid(error.member, error.reason))
        })
        .transpose()?;
    // FR-526: the evidence ledger file a prior run wrote, read once here.
    let evidence_ledger = clause
        .selection
        .correction()
        .and_then(|(_, path, _)| path)
        .map(|path| {
            let bytes = intake.file(path, intake.remaining)?;
            fsl_replay::spine::read_evidence_ledger(&bytes)
                .map_err(|error| invalid("evidence_ledger", error.to_string()))
        })
        .transpose()?;
    let mut selection = clause
        .selection
        .into_parts()
        .map_err(|error| invalid(error.member, error.reason))?;
    if let fsl_replay::spine::ClauseSelectionPartsKind::Temporal {
        trace: fsl_replay::spine::TemporalTraceParts::Timestamped(timestamped),
        ..
    } = &mut selection
    {
        timestamped.late.evidence_ledger = evidence_ledger;
    }
    let source_identity = source.source().identity().clone();
    let source = ClauseRunSource::Program {
        identity: source.source().identity().clone(),
        path: source.source().path().to_owned(),
        bytes: source.source().text().as_bytes().to_vec(),
    };
    fsl_replay::spine::read_clause_run_request(ClauseRequestParts {
        package_name: super::package_name(&source_identity),
        source,
        packages,
        dependencies,
        observations,
        selection,
        limits: SpineLimits::default(),
        observation_limits,
        accounting: super::call_accounting(&clause.accounting),
        clocks: request.clocks.clone(),
        ledger,
    })
    .map_err(|error| invalid(error.member, error.reason))
}
