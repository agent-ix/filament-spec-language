// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-526 "On FR-641's run request": chain a finite-window run result to its
//! direct predecessor through the evidence ledger file a prior run wrote, and
//! refuse a relation that changes nothing (FR-966 "A refused correction
//! through a run").
//!
//! The ledger holds, for each temporal clause, its latest result: the
//! result's `result_identity`, `input_identity` and `lineage`, and the
//! identities of the results it superseded, so a relation naming an earlier
//! result of the clause is told apart from one naming no result at all.

use serde::{Deserialize, Serialize};

use super::{ClauseRunReport, FiniteWindowReport};

/// The format tag of the evidence ledger file a run writes (FR-526).
pub const EVIDENCE_LEDGER_FORMAT: &str = "filament.finite-window-evidence-ledger/1";

/// The relation of a run's result to an earlier one (FR-526).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum RunRelation {
    /// The first result of a chain (the default).
    #[default]
    Original,
    /// The result replaces the predecessor with this `result_identity`.
    Superseding(String),
    /// The result retracts the predecessor with this `result_identity`.
    Invalidating(String),
}

impl RunRelation {
    /// The relation's kind label.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Superseding(_) => "superseding",
            Self::Invalidating(_) => "invalidating",
        }
    }

    /// The predecessor's `result_identity`, for a correction.
    pub fn predecessor(&self) -> Option<&str> {
        match self {
            Self::Original => None,
            Self::Superseding(previous) | Self::Invalidating(previous) => Some(previous),
        }
    }

    /// The run result's `relation` member: `"original"`, or
    /// `{"superseding": ID}` / `{"invalidating": ID}`.
    pub fn wire(&self) -> serde_json::Value {
        match self.predecessor() {
            None => serde_json::json!(self.kind()),
            Some(previous) => serde_json::json!({ self.kind(): previous }),
        }
    }
}

/// The evidence ledger file a run writes and a correction reads.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceLedgerFile {
    format: String,
    results: Vec<LedgerResult>,
}

/// One clause's latest result in the ledger.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerResult {
    clause: String,
    result_identity: String,
    input_identity: String,
    lineage: u64,
    superseded: Vec<String>,
}

/// A malformed evidence ledger file.
#[derive(Debug, thiserror::Error)]
#[error("a closed {EVIDENCE_LEDGER_FORMAT} document: {reason}")]
pub struct EvidenceLedgerReadError {
    /// Why the file is not one.
    pub reason: String,
}

/// Read the evidence ledger file a prior run wrote.
///
/// # Errors
///
/// [`EvidenceLedgerReadError`] for bytes that are not a closed ledger
/// document of [`EVIDENCE_LEDGER_FORMAT`].
pub fn read_evidence_ledger(bytes: &[u8]) -> Result<EvidenceLedgerFile, EvidenceLedgerReadError> {
    let ledger: EvidenceLedgerFile =
        serde_json::from_slice(bytes).map_err(|error| EvidenceLedgerReadError {
            reason: error.to_string(),
        })?;
    if ledger.format != EVIDENCE_LEDGER_FORMAT {
        return Err(EvidenceLedgerReadError {
            reason: format!("format {:?}", ledger.format),
        });
    }
    Ok(ledger)
}

/// FR-526: a finite-window result's place in its correction chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrectionRecord {
    /// The result's identity: its input identity under its relation.
    pub result_identity: String,
    /// The identity of everything the evaluation read (FR-522).
    pub input_identity: String,
    /// The relation to an earlier result.
    pub relation: RunRelation,
    /// 0 for an `Original` result, the predecessor's lineage plus 1
    /// otherwise.
    pub lineage: u64,
    /// The updated ledger, in its file form.
    pub ledger: serde_json::Value,
}

/// A relation the run refuses (FR-526, FR-966): stage `evaluate`,
/// `invalid_runtime_input`/`invalid-value`, its `cause` and `member`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the correction is refused at member {member}: {cause}")]
pub struct CorrectionRefusal {
    /// The FR-516 cause.
    pub cause: &'static str,
    /// The request member at fault: `relation` or `admit_late`.
    pub member: &'static str,
    /// For `admitted-not-late`, the name no late observation carries.
    pub observation: Option<String>,
}

impl CorrectionRefusal {
    fn of(cause: &'static str, member: &'static str) -> Self {
        Self {
            cause,
            member,
            observation: None,
        }
    }
}

/// Settle the relation of the finite-window result in `report` of the
/// clause `clause` against `ledger`, and record the result's identities,
/// lineage and updated ledger on it. A report with no finite-window result
/// (the run refused before it evaluated) is left as it is. `admit_late` is
/// whether the request named late observations to admit.
///
/// # Errors
///
/// [`CorrectionRefusal`]: `admission-without-repair` for `admit_late` on an
/// `Original` or `Invalidating` relation, `admitted-not-late` for a name no
/// late observation carries, `no-ledger`, `predecessor-unknown`,
/// `predecessor-not-latest` and `unchanged-input` for the relation, and
/// `lineage` for a lineage past `u64`.
pub fn settle_correction(
    report: &mut ClauseRunReport,
    clause: &str,
    relation: RunRelation,
    ledger: Option<EvidenceLedgerFile>,
    admit_late: bool,
) -> Result<(), CorrectionRefusal> {
    let Some(window) = report.finite_window.as_mut() else {
        return Ok(());
    };
    let record = correction(window, clause, relation, ledger, admit_late)?;
    window.correction = Some(record);
    Ok(())
}

fn correction(
    window: &FiniteWindowReport,
    clause: &str,
    relation: RunRelation,
    ledger: Option<EvidenceLedgerFile>,
    admit_late: bool,
) -> Result<CorrectionRecord, CorrectionRefusal> {
    if admit_late && !matches!(relation, RunRelation::Superseding(_)) {
        return Err(CorrectionRefusal::of(
            "admission-without-repair",
            "admit_late",
        ));
    }
    if let Some(name) = window.unmatched_admissions.first() {
        return Err(CorrectionRefusal {
            observation: Some(name.clone()),
            ..CorrectionRefusal::of("admitted-not-late", "admit_late")
        });
    }
    let input = window.identity.clone();
    let supplied = ledger.is_some();
    let mut ledger = ledger.unwrap_or_else(|| EvidenceLedgerFile {
        format: EVIDENCE_LEDGER_FORMAT.to_owned(),
        results: Vec::new(),
    });
    let held = ledger
        .results
        .iter()
        .position(|entry| entry.clause == clause);
    let (lineage, superseded) = match relation.predecessor() {
        None => (0, Vec::new()),
        Some(previous) => {
            if !supplied {
                return Err(CorrectionRefusal::of("no-ledger", "relation"));
            }
            let entry = held
                .and_then(|index| ledger.results.get(index))
                .ok_or_else(|| CorrectionRefusal::of("predecessor-unknown", "relation"))?;
            if entry.result_identity != previous {
                let cause = if entry.superseded.iter().any(|earlier| earlier == previous) {
                    "predecessor-not-latest"
                } else {
                    "predecessor-unknown"
                };
                return Err(CorrectionRefusal::of(cause, "relation"));
            }
            if entry.input_identity == input {
                return Err(CorrectionRefusal::of("unchanged-input", "relation"));
            }
            let lineage = entry
                .lineage
                .checked_add(1)
                .ok_or_else(|| CorrectionRefusal::of("lineage", "relation"))?;
            let mut superseded = entry.superseded.clone();
            superseded.push(entry.result_identity.clone());
            (lineage, superseded)
        }
    };
    let result_identity =
        fsl_eval::temporal::correction_identity(&input, relation.kind(), relation.predecessor())
            .map_err(|_| CorrectionRefusal::of("result-identity", "relation"))?
            .0;
    let entry = LedgerResult {
        clause: clause.to_owned(),
        result_identity: result_identity.clone(),
        input_identity: input.clone(),
        lineage,
        superseded,
    };
    match held.and_then(|index| ledger.results.get_mut(index)) {
        Some(held) => *held = entry,
        None => ledger.results.push(entry),
    }
    ledger
        .results
        .sort_by(|left, right| left.clause.cmp(&right.clause));
    let ledger = serde_json::to_value(&ledger)
        .map_err(|_| CorrectionRefusal::of("ledger-encodes", "relation"))?;
    Ok(CorrectionRecord {
        result_identity,
        input_identity: input,
        relation,
        lineage,
        ledger,
    })
}

/// [`settle_correction`] for the run of `selection`: a timestamped trace's
/// relation, evidence ledger and admitted late observations (FR-641's
/// temporal selection members). A run over any other trace carries none
/// and is left as it is.
///
/// # Errors
///
/// As [`settle_correction`].
pub fn settle_run_correction(
    report: &mut ClauseRunReport,
    selection: &super::ClauseRunSelection,
) -> Result<(), CorrectionRefusal> {
    let record = run_correction(report, selection)?;
    if let (Some(window), Some(record)) = (report.finite_window.as_mut(), record) {
        window.correction = Some(record);
    }
    Ok(())
}

/// The correction record of the finite-window result in `report`, run for
/// `selection`: the one the report already carries, or the one its
/// selection's relation and ledger give; `None` for a run with no
/// finite-window result or over another trace.
///
/// # Errors
///
/// As [`settle_correction`].
pub(crate) fn run_correction(
    report: &ClauseRunReport,
    selection: &super::ClauseRunSelection,
) -> Result<Option<CorrectionRecord>, CorrectionRefusal> {
    let Some(window) = report.finite_window.as_ref() else {
        return Ok(None);
    };
    if let Some(record) = &window.correction {
        return Ok(Some(record.clone()));
    }
    let (super::ClauseRunSelection::Temporal { name, trace, .. }
    | super::ClauseRunSelection::SampledTemporal { name, trace, .. }) = selection
    else {
        return Ok(None);
    };
    let super::TraceInput::Timestamped { window: input, .. } = trace else {
        return Ok(None);
    };
    let late = &input.late;
    correction(
        window,
        name,
        late.relation.clone(),
        late.evidence_ledger.clone(),
        !late.admit_late.is_empty(),
    )
    .map(Some)
}
