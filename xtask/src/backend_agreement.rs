// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-448: compare negotiated backend records and independently re-check refutations.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use filament_exact::Integer;
use fsl_analyze::analyze::document::ResolvedClaim;
use fsl_foundation::BackendId;
use fsl_replay::spine::CheckedUnit;
use fsl_route::Registry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::Outcome;

#[path = "backend_agreement/driver.rs"]
mod driver;

/// A candidate's negotiated disposition, retained independently of its record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Negotiated {
    /// The candidate may execute the item.
    Supported,
    /// A finite bound must be supplied before the candidate can execute.
    RequiresBound,
    /// The candidate cannot execute this item.
    Unsupported,
    /// Negotiation rejected the request.
    InvalidRequest,
}

/// The independent replay check of a backend's reported refutation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum Recheck {
    /// Native evaluation reproduced the witness's violation.
    ReproducedWithEvaluatedWitness,
    /// The hybrid witness checker accepted the supplied interval witness again.
    ReproducedWithValidatedWitness,
    /// Evaluation completed without reproducing the reported refutation.
    Inconclusive,
    /// Replay could not decide under the effective limits or supported input.
    Refused {
        /// The original replay refusal's catalog code.
        code: String,
        /// The original typed refusal's cause.
        cause: String,
    },
    /// Replay encountered an internal fault.
    Fault {
        /// The invariant or fault the replay reported.
        detail: String,
    },
}

/// One inclusive integer bound in a bounded-mode proof's declared domain.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainBound {
    /// The proof parameter's exact wire identity.
    pub parameter: Value,
    /// The inclusive lower bound in canonical decimal spelling.
    pub lower: String,
    /// The inclusive upper bound in canonical decimal spelling.
    pub upper: String,
}

/// One candidate's disposition and its losslessly retained terminal item.
#[derive(Clone, Debug, Serialize)]
pub struct CandidateRecord {
    /// The registered backend identity.
    pub backend: String,
    /// The disposition returned by the orchestrating driver.
    pub disposition: Negotiated,
    /// The FR-286 item, including its category, proof basis and typed cause.
    pub record: Option<Value>,
    /// A bounded-mode proof's finite domain; absent for unbounded proofs.
    pub declared_domain: Option<Vec<DomainBound>>,
    /// The gate's independent replay check, present for every refuted record.
    pub replay: Option<Recheck>,
    /// Why the candidate did not contribute a definite comparison.
    pub not_compared: Option<String>,
}

/// The gate settlement of a corpus item or a vector that produced no item.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Settlement {
    /// At least two comparable definite records agreed.
    Agree,
    /// Definite records or a backend and its independent replay conflicted.
    Conflict,
    /// Fewer than two comparable definite records were available.
    NotCompared,
    /// The source did not compile, so no requirement item was produced.
    NotCompiled,
    /// An internal fault, I/O failure or failed backend record occurred.
    ToolFailure,
}

/// One report entry, ordered by vector identity and original request index.
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    /// The corpus's vector identity.
    pub vector: String,
    /// The original requirement item's index; absent when compilation failed.
    pub request_index: Option<usize>,
    /// The corpus capability that owns the vector.
    pub capability: String,
    /// The item's agreement settlement.
    pub settlement: Settlement,
    /// Every candidate, in FR-075 candidate order.
    pub candidates: Vec<CandidateRecord>,
    /// The compile refusal, I/O failure or internal fault, when one occurred.
    pub failure: Option<Value>,
}

/// A compiled corpus item supplied to the driver and replay boundaries.
pub struct Item<'a> {
    /// The checked source package whose request is being compared.
    pub checked: &'a CheckedUnit,
    /// The resolved claim with its full extent, bounds and original index.
    pub claim: &'a ResolvedClaim,
    /// The source/model/document request the driver will read.
    pub request: &'a Value,
    /// Every run and replay reads this same resolved limit set.
    pub limits: &'a fsl_replay::CallerLimits,
    /// The caller's exact limit operands for the driver invocation.
    pub overrides: &'a [String],
}

/// The orchestrating provider seam: negotiation and execution stay with the driver.
pub trait Provider {
    /// The registry snapshot used by FR-075 candidate discovery.
    fn registry(&self) -> &Registry;
    /// Verify every selected provider and its tool before any backend executes.
    fn preflight(&self, backends: &[BackendId]) -> Result<()>;
    /// Negotiate and, only when supported, run one explicitly named candidate.
    fn execute(&self, item: &Item<'_>, backend: &BackendId) -> Result<CandidateRecord>;
    /// Re-check a reported refutation independently of the provider's settlement.
    fn replay(&self, item: &Item<'_>, record: &CandidateRecord) -> Recheck;
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Definite {
    Proved,
    Refuted,
}

#[ix_trace_rs::string_edge]
fn definite(candidate: &mut CandidateRecord) -> Option<Definite> {
    if candidate.disposition != Negotiated::Supported {
        candidate.not_compared = Some(format!("negotiation: {:?}", candidate.disposition));
        return None;
    }
    let record = candidate.record.as_ref()?;
    let terminal = record.get("record")?;
    match (
        terminal.get("result")?.as_str()?,
        record.get("category")?.as_str()?,
    ) {
        ("proved", "success") => Some(Definite::Proved),
        ("refuted", "violation") => match candidate.replay {
            Some(
                Recheck::ReproducedWithEvaluatedWitness | Recheck::ReproducedWithValidatedWitness,
            ) => Some(Definite::Refuted),
            _ => {
                candidate.not_compared = Some("refutation not reproduced by the gate".to_owned());
                None
            }
        },
        (_, category) => {
            candidate.not_compared = Some(format!("not definite: {category}"));
            None
        }
    }
}

fn within_domain(proof: &CandidateRecord, refutation: &CandidateRecord) -> bool {
    let Some(domain) = &proof.declared_domain else {
        return true;
    };
    let Some(values) = refutation
        .record
        .as_ref()
        .and_then(|record| record.pointer("/counterexample/values"))
        .and_then(Value::as_array)
    else {
        return false;
    };
    domain.iter().all(|bound| {
        let integer = values
            .iter()
            .find(|value| value.get("parameter") == Some(&bound.parameter))
            .and_then(|value| value.pointer("/value/value"))
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<Integer>().ok());
        match (
            integer,
            bound.lower.parse::<Integer>(),
            bound.upper.parse::<Integer>(),
        ) {
            (Some(value), Ok(lower), Ok(upper)) => lower <= value && value <= upper,
            _ => false,
        }
    })
}

/// Apply ADR-013 O-16 without choosing a preferred backend or repairing a verdict.
pub fn compare(candidates: &mut [CandidateRecord]) -> Settlement {
    let mut conflict = candidates
        .iter()
        .any(|candidate| matches!(candidate.replay, Some(Recheck::Inconclusive)));
    let mut agreed = false;
    let definite = candidates.iter_mut().map(definite).collect::<Vec<_>>();
    for (left_index, left) in candidates.iter().enumerate() {
        let Some(left_value) = definite[left_index] else {
            continue;
        };
        for (right_index, right) in candidates.iter().enumerate().skip(left_index + 1) {
            let Some(right_value) = definite[right_index] else {
                continue;
            };
            let comparable = match (left_value, right_value) {
                (Definite::Proved, Definite::Refuted) => within_domain(left, right),
                (Definite::Refuted, Definite::Proved) => within_domain(right, left),
                _ => true,
            };
            if comparable {
                conflict |= left_value != right_value;
                agreed |= left_value == right_value;
            }
        }
    }
    if conflict {
        return Settlement::Conflict;
    }
    let failed = candidates.iter().any(|candidate| {
        (candidate.disposition == Negotiated::Supported && candidate.record.is_none())
            || matches!(candidate.replay, Some(Recheck::Fault { .. }))
            || candidate.record.as_ref().is_some_and(|record| {
                record.pointer("/record/result") == Some(&json!("failed"))
                    || record.get("category").and_then(Value::as_str) == Some("internal-failure")
            })
    });
    if failed {
        Settlement::ToolFailure
    } else if agreed {
        Settlement::Agree
    } else {
        for (index, candidate) in candidates.iter_mut().enumerate() {
            if definite[index].is_some() && candidate.not_compared.is_none() {
                candidate.not_compared = Some("no comparable definite peer; a bounded proof requires a witness inside its declared domain".to_owned());
            }
        }
        Settlement::NotCompared
    }
}

/// Render a deterministic report and the FR-448 exit code.
pub fn report(mut entries: Vec<Entry>, text: bool) -> Result<Outcome> {
    entries.sort_by(|left, right| {
        (&left.vector, left.request_index).cmp(&(&right.vector, right.request_index))
    });
    let mut counts = BTreeMap::from([
        ("agree", 0_usize),
        ("conflict", 0),
        ("not-compared", 0),
        ("not-compiled", 0),
        ("tool-failure", 0),
    ]);
    for entry in &entries {
        let key = match entry.settlement {
            Settlement::Agree => "agree",
            Settlement::Conflict => "conflict",
            Settlement::NotCompared => "not-compared",
            Settlement::NotCompiled => "not-compiled",
            Settlement::ToolFailure => "tool-failure",
        };
        *counts.entry(key).or_default() += 1;
    }
    let code = u8::from(entries.iter().any(|entry| {
        matches!(
            entry.settlement,
            Settlement::Conflict | Settlement::ToolFailure
        )
    }));
    let stdout = if text {
        entries
            .iter()
            .map(|entry| {
                format!(
                    "{} {:?}: {:?}\n",
                    entry.vector, entry.request_index, entry.settlement
                )
            })
            .collect()
    } else {
        serde_json::to_string(&json!({"entries": entries, "counts": counts}))
            .map_err(|error| Error::QualificationRefusal(error.to_string()))?
            + "\n"
    };
    Ok(Outcome { stdout, code })
}

/// Run the standalone gate using the explicit `FSL` driver executable.
pub fn run(operands: &[OsString]) -> Result<Outcome> {
    let executable = std::env::var_os("FSL").ok_or_else(|| {
        Error::QualificationRefusal(
            "backend-agreement: FSL must name the orchestrating driver's executable".to_owned(),
        )
    })?;
    let provider = driver::Driver::new(Path::new(&executable))?;
    run_with_provider(operands, &provider)
}

/// Run the real corpus compiler and report with a caller-owned provider boundary.
pub fn run_with_provider(operands: &[OsString], provider: &dyn Provider) -> Result<Outcome> {
    driver::execute(operands, provider)
}
