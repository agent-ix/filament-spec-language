// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-962: FR-489's claim-form tie rule, applied while an unnamed item's
//! candidates are computed, before the item reaches the negotiator.
//!
//! The linked set ties by design on `operation-contract`: EN-1 and
//! `contract-checker` both advertise it. The rule reads only the claim's form
//! and whether a model subject is named, so the narrowed set is a function of
//! the item and the registry. Any other multi-candidate set stays whole, and
//! negotiation settles it `ambiguous-backend`.

use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::BackendId;
use fsl_package::CheckedPackage;
use fsl_replay::contract_check::{select_contract, ContractForm};
use fsl_route::CandidateOutcome;
use fsl_semantics::check::Capability;

use crate::{contract_check, model_check};

/// `candidates` of the item of `kind` at `claim`, narrowed to the one
/// provider FR-489's rule selects when a row decides the item: the item
/// names no backend, its kind is `operation-contract`, and its matching set
/// is exactly EN-1 and `contract-checker`. EN-1 then decides an `invariant`
/// state clause of a request naming a model subject; `contract-checker`
/// decides one with no model subject, an operation's `pre` or `post`
/// contract and a frame obligation. EN-1 is first excluded from the set of
/// any item that is not such an invariant (FR-962,
/// [`fsl_route::tie::exclude_en1`]). Every other set is returned whole.
pub fn resolve_tie(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
    kind: Capability,
    named: Option<&BackendId>,
    model_subject: bool,
    candidates: CandidateOutcome,
) -> CandidateOutcome {
    if named.is_some() || kind != Capability::OperationContract {
        return candidates;
    }
    let invariant_over_model = model_subject
        && matches!(
            select_contract(package, claim).map(|contract| contract.form()),
            Ok(ContractForm::Invariant)
        );
    let candidates = fsl_route::tie::exclude_en1(candidates, kind, named, invariant_over_model);
    let CandidateOutcome::Candidates(set) = &candidates else {
        return candidates;
    };
    let mut members: Vec<&str> = set
        .candidates()
        .iter()
        .map(|candidate| candidate.id().as_str())
        .collect();
    members.sort_unstable();
    let mut tied = [
        contract_check::manifest::BACKEND_ID,
        model_check::manifest::BACKEND_ID,
    ];
    tied.sort_unstable();
    if members != tied {
        return candidates;
    }
    let selected = match select_contract(package, claim).map(|contract| contract.form()) {
        Ok(ContractForm::Invariant) if model_subject => model_check::manifest::BACKEND_ID,
        Ok(ContractForm::Invariant | ContractForm::Clause | ContractForm::Frame) => {
            contract_check::manifest::BACKEND_ID
        }
        _ => return candidates,
    };
    match set.narrowed_to(&BackendId::new(selected)) {
        Some(narrowed) => CandidateOutcome::Candidates(narrowed),
        None => candidates,
    }
}
