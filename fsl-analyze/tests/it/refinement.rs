// SPDX-License-Identifier: AGPL-3.0-or-later
//! refinement integration tests.

mod litmus;
mod load_buffering;
mod operation_contract;
#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;
mod quotient;
mod race_freedom;
mod rc11_oracle;
#[path = "../support/rc11.rs"]
mod rc11_support;
mod reduction;
mod reduction_differential;
mod reduction_manifest;
mod reduction_proptest;
mod reduction_world;
#[path = "../../../tests/support/refinement_aux.rs"]
mod refinement_aux;
mod refinement_auxiliary;
mod refinement_budget;
mod refinement_certificate;
#[path = "../../../tests/support/refinement_cas.rs"]
mod refinement_fixture;
mod refinement_liveness;
#[path = "../../../tests/support/refinement_smt_proofs.rs"]
mod refinement_proofs;
mod refinement_protocol;
mod refinement_reads;
mod refinement_replay;
mod refinement_request;
mod refinement_safety;
mod related_by;
mod relation_certificate;
mod relation_claims;
mod relation_vs_smt;
mod release_acquire;
#[path = "../../../tests/support/ring_world.rs"]
mod ring_world;
mod smt_query;
mod source_model_boundaries;
mod tied_item_resolution;
mod weak_memory;
