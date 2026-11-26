// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-314 (ADR-018 PC-6, LA-3): the SMT proof certificate of a refinement's
//! per-step simulation obligations, and [`check_refinement_smt_proof`], the
//! core's check of it.
//!
//! The checker reads nothing from the solver but the certificate. It applies
//! FR-314's rules in this order, each to the base part and then to the step
//! parts in row order before the next rule, and reports the first that
//! fails:
//!
//! 1. `ShapeMismatch`: the certificate holds fewer or more step parts than
//!    the refinement has step rows;
//! 2. `QueryMismatch`: a carried query differs, byte for byte, from the one
//!    FR-315 encodes for its part; a row whose query FR-315 refuses has no
//!    expected query, and its part is neither compared nor checked, and a
//!    refused initial query rejects the base part;
//! 3. the proof rules of [`alethe`](super::alethe): `Malformed`,
//!    `NotRefutation` for a proof of no command, `ProofStepInvalid`, then
//!    `NotRefutation` for a last command that does not conclude the empty
//!    clause;
//! 4. `Unverifiable` at the first command whose rule is unchecked, or
//!    `Unencodable` at a refused part that precedes it.
//!
//! Every byte of a carried query and proof and of an expected query is
//! charged to `max_certificate_bytes`, and every command and premise or
//! discharge entry to `max_certificate_edges`, before the work it pays for;
//! the `Cancel` handle is polled at every charge.

use std::cell::Cell;

use ix_trace_rs::implements;
use filament_exact::Cancel;
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::smt::{
    encode_refinement_query, encode_smt_query, RefinementCertificateRequest, RefinementQueryKind,
    SmtEncodingRefusal, SmtItem, SmtQueryKind,
};
use fsl_semantics::check::{Activation, TemporalProfile};

use super::alethe::{read_proof, read_query, Proof, ProofOutcome, Query, Reading};
use super::closure::CertificateRejection;
use super::zone_certificate::CertificateCheckLimits;
use super::{CertificateLocus, CertificateRule, QueryPart};
use crate::certificate::closure::CertificateRequest;
use crate::model_check::item::ModelCheckItem;
use crate::model_check::limits::ModelCheckLimit;
use crate::proof_result::IncompleteCause;

/// The bases an SMT backend may report for a temporal proof (FR-127).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmtProofBasis {
    /// A bounded unrolling closed at or beyond the item's horizon.
    BoundedComplete {
        /// Number of transitions unrolled.
        depth: u64,
    },
    /// A checked induction base and step at this depth.
    Inductive {
        /// Number of transitions used in the base and step.
        depth: u64,
    },
}

impl SmtProofBasis {
    /// The terminal-record basis corresponding to this SMT basis.
    #[must_use]
    pub const fn proof_basis(self) -> crate::proof_result::ProofBasis {
        match self {
            Self::BoundedComplete { depth } => {
                crate::proof_result::ProofBasis::BoundedComplete { depth }
            }
            Self::Inductive { depth } => crate::proof_result::ProofBasis::Inductive { depth },
        }
    }
}

/// What the SMT backend hands FR-127 for a temporal proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmtProofOutcome {
    /// The proof basis reported by the backend.
    pub basis: SmtProofBasis,
    /// The solver certificate, when the backend returned one.
    pub certificate: Option<SmtProofCertificate>,
}

/// The proof format of an SMT proof certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofFormat {
    /// cvc5's Alethe proof format.
    Alethe,
}

/// One refuted query: the SMT-LIB 2 script and the solver's refutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefutedQuery {
    /// The SMT-LIB 2 script, in FR-315's canonical printing.
    pub query: Vec<u8>,
    /// The solver's refutation in the certificate's format.
    pub proof: Vec<u8>,
}

/// A refinement's per-step simulation obligations (FSpec FR-468): the
/// refuted initial query and one refuted step query per step row, in
/// declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementSmtCertificate {
    /// The proofs' format.
    pub format: ProofFormat,
    /// The initial query's part.
    pub base: RefutedQuery,
    /// One part per step row.
    pub steps: Vec<RefutedQuery>,
}

/// A temporal model check's bounded-complete or inductive proof (FR-314).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SmtProofCertificate {
    /// One refutation of the bounded unrolling query.
    BoundedComplete {
        /// The proof format.
        format: ProofFormat,
        /// The bounded unrolling query and its refutation.
        unrolling: RefutedQuery,
    },
    /// Refutations of the induction base and step queries.
    Inductive {
        /// The proof format.
        format: ProofFormat,
        /// The induction base query and its refutation.
        base: RefutedQuery,
        /// The induction step query and its refutation.
        step: RefutedQuery,
    },
}

/// What an SMT proof check that rejects nothing returns (FR-314).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SmtProofCheck {
    /// Every proof refutes its query with checked rules alone.
    Verified,
    /// No rule rejects, and the command numbered `step` of `part`'s proof
    /// is the first whose rule is unchecked.
    Unverifiable {
        /// The part.
        part: QueryPart,
        /// The command's number in the part's proof.
        step: u64,
    },
    /// No rule rejects, and FR-315 refuses to encode the query of `part`.
    Unencodable {
        /// The part.
        part: QueryPart,
    },
    /// A limit was reached or the check was cancelled.
    Stopped(IncompleteCause),
    /// FR-315 faulted rebuilding a query, or the checker broke an
    /// invariant of its own.
    Fault(InternalFault),
}

/// Why the check ended before its verdict.
enum Halt {
    Rejected(CertificateRejection),
    Check(SmtProofCheck),
}

impl From<CertificateRejection> for Halt {
    fn from(rejection: CertificateRejection) -> Self {
        Self::Rejected(rejection)
    }
}

fn reject(rule: CertificateRule, at: CertificateLocus) -> Halt {
    Halt::Rejected(CertificateRejection { rule, at })
}

fn fault(invariant: &'static str) -> Halt {
    Halt::Check(SmtProofCheck::Fault(InternalFault::new(
        "smt-proof-check",
        invariant,
    )))
}

/// The two certificate budgets and the cancellation handle.
struct Budget<'c> {
    limits: CertificateCheckLimits,
    bytes: Cell<u64>,
    edges: Cell<u64>,
    cancel: &'c Cancel,
}

impl Budget<'_> {
    fn poll(&self) -> Result<(), Halt> {
        if self.cancel.poll() {
            let Some(source) = self.cancel.cause() else {
                return Err(fault("a cancelled handle has a cause"));
            };
            return Err(Halt::Check(SmtProofCheck::Stopped(
                IncompleteCause::Cancelled { source },
            )));
        }
        Ok(())
    }

    fn charge(
        &self,
        counter: &Cell<u64>,
        limit: ModelCheckLimit,
        value: u64,
        count: u64,
    ) -> Result<(), Halt> {
        self.poll()?;
        let total = counter.get().saturating_add(count);
        if total > value {
            return Err(Halt::Check(SmtProofCheck::Stopped(
                IncompleteCause::LimitReached {
                    limit,
                    value,
                    required: None,
                },
            )));
        }
        counter.set(total);
        Ok(())
    }

    fn bytes(&self, count: usize) -> Result<(), Halt> {
        self.charge(
            &self.bytes,
            ModelCheckLimit::CertificateMaxBytes,
            self.limits.max_certificate_bytes,
            u64::try_from(count).unwrap_or(u64::MAX),
        )
    }

    fn edges(&self, count: u64) -> Result<(), Halt> {
        self.charge(
            &self.edges,
            ModelCheckLimit::CertificateMaxEdges,
            self.limits.max_certificate_edges,
            count,
        )
    }
}

/// One part of the certificate, with what the checker learned of it.
struct Part<'c> {
    part: QueryPart,
    carried: &'c RefutedQuery,
    /// The expected query, read; `None` for a part FR-315 refuses.
    query: Option<Query>,
}

/// Check `certificate` for the refinement item `request` (FR-314
/// "Refinement certificates").
///
/// # Errors
///
/// [`CertificateRejection`] naming the first rule that fails and its part,
/// or the proof command at which it fails.
#[implements("FR-314-AC-4", "FR-314-AC-17")]
pub fn check_refinement_smt_proof(
    request: &RefinementCertificateRequest<'_>,
    certificate: &RefinementSmtCertificate,
    limits: &CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<SmtProofCheck, CertificateRejection> {
    let budget = Budget {
        limits: *limits,
        bytes: Cell::new(0),
        edges: Cell::new(0),
        cancel,
    };
    match check(request, certificate, &budget) {
        Ok(check) => Ok(check),
        Err(Halt::Check(check)) => Ok(check),
        Err(Halt::Rejected(rejection)) => Err(rejection),
    }
}

/// Check a temporal model-check certificate against FR-315's rebuilt query.
/// The request is re-used from FR-338 so the subject, item and encoding
/// limits have one canonical representation.
#[implements("FR-314-AC-1")]
pub fn check_smt_proof(
    request: &CertificateRequest<'_>,
    basis: &SmtProofBasis,
    certificate: &SmtProofCertificate,
    limits: &CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<SmtProofCheck, CertificateRejection> {
    let budget = Budget {
        limits: *limits,
        bytes: Cell::new(0),
        edges: Cell::new(0),
        cancel,
    };
    match check_temporal(request, basis, certificate, &budget) {
        Ok(check) => Ok(check),
        Err(Halt::Check(check)) => Ok(check),
        Err(Halt::Rejected(rejection)) => Err(rejection),
    }
}

/// Validate the bounded-complete basis before any certificate work. FR-127
/// uses this for outcomes without a certificate as well, so an inadequate
/// horizon cannot become an uncertified proof.
pub(crate) fn validate_smt_proof_basis(
    request: &CertificateRequest<'_>,
    basis: &SmtProofBasis,
) -> Result<(), CertificateRejection> {
    let SmtProofBasis::BoundedComplete { depth } = basis else {
        return Ok(());
    };
    let ModelCheckItem::Clause { clause, .. } = request.item else {
        return Err(CertificateRejection {
            rule: CertificateRule::ShapeMismatch,
            at: CertificateLocus::Query {
                part: QueryPart::Unrolling,
            },
        });
    };
    let horizon = fsl_eval::automaton::bounded_shape(clause).map(|(_, upper, _)| upper);
    if clause.profile() != TemporalProfile::EventPositionFalseExtension
        || !matches!(clause.activation(), Activation::Origin)
        || horizon.is_none_or(|horizon| *depth < horizon)
    {
        return Err(CertificateRejection {
            rule: CertificateRule::ShapeMismatch,
            at: CertificateLocus::Query {
                part: QueryPart::Unrolling,
            },
        });
    }
    Ok(())
}

fn check_temporal(
    request: &CertificateRequest<'_>,
    basis: &SmtProofBasis,
    certificate: &SmtProofCertificate,
    budget: &Budget<'_>,
) -> Result<SmtProofCheck, Halt> {
    let (format, carried): (ProofFormat, Vec<(QueryPart, SmtQueryKind, &RefutedQuery)>) =
        match (basis, certificate) {
            (
                SmtProofBasis::BoundedComplete { depth },
                SmtProofCertificate::BoundedComplete { format, unrolling },
            ) => {
                validate_smt_proof_basis(request, basis).map_err(Halt::Rejected)?;
                (
                    *format,
                    vec![(
                        QueryPart::Unrolling,
                        SmtQueryKind::Unrolling { depth: *depth },
                        unrolling,
                    )],
                )
            }
            (
                SmtProofBasis::Inductive { depth },
                SmtProofCertificate::Inductive { format, base, step },
            ) => (
                *format,
                vec![
                    (
                        QueryPart::Base,
                        SmtQueryKind::InductionBase { depth: *depth },
                        base,
                    ),
                    (
                        QueryPart::Step { row: None },
                        SmtQueryKind::InductionStep { depth: *depth },
                        step,
                    ),
                ],
            ),
            (SmtProofBasis::BoundedComplete { .. }, _) => {
                return Err(reject(
                    CertificateRule::ShapeMismatch,
                    CertificateLocus::Query {
                        part: QueryPart::Unrolling,
                    },
                ));
            }
            (SmtProofBasis::Inductive { .. }, _) => {
                return Err(reject(
                    CertificateRule::ShapeMismatch,
                    CertificateLocus::Query {
                        part: QueryPart::Base,
                    },
                ));
            }
        };
    let ProofFormat::Alethe = format;
    let item = match request.item {
        ModelCheckItem::Clause { clause, .. } => SmtItem::Clause(clause),
        ModelCheckItem::DeadlockFreedom => SmtItem::DeadlockFreedom,
        _ => {
            return Err(reject(
                CertificateRule::ShapeMismatch,
                CertificateLocus::Query { part: carried[0].0 },
            ));
        }
    };
    let mut parts = Vec::with_capacity(carried.len());
    for (part, kind, carried) in carried {
        let expected = match encode_smt_query(&request.subject, item, kind, &request.smt_encoding) {
            Ok(expected) => expected,
            Err(SmtEncodingRefusal::Refused(_)) => {
                return Err(reject(
                    CertificateRule::QueryMismatch,
                    CertificateLocus::Query { part },
                ));
            }
            Err(SmtEncodingRefusal::Fault(fault)) => {
                return Err(Halt::Check(SmtProofCheck::Fault(fault)));
            }
        };
        budget.bytes(expected.len())?;
        budget.bytes(carried.query.len())?;
        if expected != carried.query {
            return Err(reject(
                CertificateRule::QueryMismatch,
                CertificateLocus::Query { part },
            ));
        }
        let query = read_query(&expected).ok_or_else(|| fault("a printed query reads back"))?;
        parts.push(Part {
            part,
            carried,
            query: Some(query),
        });
    }
    check_proofs(&parts, budget)
}

fn check(
    request: &RefinementCertificateRequest<'_>,
    certificate: &RefinementSmtCertificate,
    budget: &Budget<'_>,
) -> Result<SmtProofCheck, Halt> {
    let ProofFormat::Alethe = certificate.format;
    // Rule 1: the shape.
    let rows = step_rows(request)?;
    let present = u64::try_from(certificate.steps.len()).unwrap_or(u64::MAX);
    if present != rows {
        return Err(reject(
            CertificateRule::ShapeMismatch,
            CertificateLocus::Query {
                part: QueryPart::Step {
                    row: Some(present.min(rows)),
                },
            },
        ));
    }
    // Rule 2: each carried query is the expected one.
    let kinds = std::iter::once((QueryPart::Base, RefinementQueryKind::Initial)).chain(
        (0..rows).map(|row| {
            (
                QueryPart::Step { row: Some(row) },
                RefinementQueryKind::Step { row },
            )
        }),
    );
    let carried = std::iter::once(&certificate.base).chain(&certificate.steps);
    let mut parts = Vec::with_capacity(certificate.steps.len() + 1);
    for ((part, kind), carried) in kinds.zip(carried) {
        let query = match encode_refinement_query(request, kind) {
            Ok(expected) => {
                budget.bytes(expected.len())?;
                budget.bytes(carried.query.len())?;
                if carried.query != expected {
                    return Err(reject(
                        CertificateRule::QueryMismatch,
                        CertificateLocus::Query { part },
                    ));
                }
                Some(read_query(&expected).ok_or_else(|| fault("a printed query reads back"))?)
            }
            // A refused row has no expected query (FR-315 "Refinement
            // queries"); a refused initial query rejects the base part
            // `QueryMismatch`, as FR-314 rejects any part FR-315 prints no
            // query for.
            Err(SmtEncodingRefusal::Refused(_)) if part == QueryPart::Base => {
                return Err(reject(
                    CertificateRule::QueryMismatch,
                    CertificateLocus::Query { part },
                ));
            }
            Err(SmtEncodingRefusal::Refused(_)) => None,
            Err(SmtEncodingRefusal::Fault(fault)) => {
                return Err(Halt::Check(SmtProofCheck::Fault(fault)));
            }
        };
        parts.push(Part {
            part,
            carried,
            query,
        });
    }
    check_proofs(&parts, budget)
}

fn check_proofs(parts: &[Part<'_>], budget: &Budget<'_>) -> Result<SmtProofCheck, Halt> {
    // Rule 3: the proofs, rule by rule across the parts.
    let mut proofs: Vec<(QueryPart, &Query, Proof)> = Vec::new();
    for part in parts {
        let Some(query) = &part.query else { continue };
        budget.bytes(part.carried.proof.len())?;
        let mut stop = None;
        let read = read_proof(
            &part.carried.proof,
            query,
            &mut |count| match budget.edges(count) {
                Ok(()) => true,
                Err(halt) => {
                    stop = Some(halt);
                    false
                }
            },
        );
        let read = match read {
            Reading::Read(proof) => Some(proof),
            Reading::Malformed => None,
            Reading::Stopped => {
                return Err(stop.unwrap_or_else(|| fault("a stopped proof read names its stop")));
            }
        };
        let Some(proof) = read else {
            return Err(reject(
                CertificateRule::Malformed,
                CertificateLocus::Query { part: part.part },
            ));
        };
        proofs.push((part.part, query, proof));
    }
    if let Some((part, _, _)) = proofs.iter().find(|(_, _, proof)| proof.len() == 0) {
        return Err(reject(
            CertificateRule::NotRefutation,
            CertificateLocus::Query { part: *part },
        ));
    }
    for (part, query, proof) in &proofs {
        let mut stop = None;
        let invalid = proof.invalid_step(query, &mut |count| match budget.edges(count) {
            Ok(()) => true,
            Err(halt) => {
                stop = Some(halt);
                false
            }
        });
        let invalid = match invalid {
            Ok(invalid) => invalid,
            Err(()) => {
                return Err(
                    stop.unwrap_or_else(|| fault("a stopped polynomial check names its stop"))
                );
            }
        };
        if let Some(index) = invalid {
            return Err(reject(
                CertificateRule::ProofStepInvalid,
                CertificateLocus::ProofStep { part: *part, index },
            ));
        }
    }
    for (part, _, proof) in &proofs {
        if let Some(index) = proof.not_refutation() {
            return Err(reject(
                CertificateRule::NotRefutation,
                CertificateLocus::ProofStep { part: *part, index },
            ));
        }
    }
    // Rule 4: the first unchecked command or refused part.
    let mut checked = proofs.iter();
    for part in parts {
        if part.query.is_none() {
            return Ok(SmtProofCheck::Unencodable { part: part.part });
        }
        let Some((_, query, proof)) = checked.next() else {
            return Err(fault("every encodable part has a proof"));
        };
        let mut stop = None;
        let outcome = proof.outcome(query, &mut |count| match budget.edges(count) {
            Ok(()) => true,
            Err(halt) => {
                stop = Some(halt);
                false
            }
        });
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(()) => {
                return Err(
                    stop.unwrap_or_else(|| fault("a stopped polynomial check names its stop"))
                );
            }
        };
        if let ProofOutcome::Unverifiable { step } = outcome {
            return Ok(SmtProofCheck::Unverifiable {
                part: part.part,
                step,
            });
        }
    }
    Ok(SmtProofCheck::Verified)
}

/// The number of step rows of the refinement `request` names.
fn step_rows(request: &RefinementCertificateRequest<'_>) -> Result<u64, Halt> {
    crate::refinement_query::refinement_step_rows(request.package, &request.refinement)
        .ok_or_else(|| fault("the request names a refinement of its package"))
}
