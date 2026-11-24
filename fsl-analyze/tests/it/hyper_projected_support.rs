// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared projected-hyperproperty fixture helpers.

const AGREE: &str = "(if present(a.step.Vault::step.i) and present(b.step.Vault::step.i) \
     then value(a.step.Vault::step.i) = value(b.step.Vault::step.i) else true)";

pub(crate) const IDLE: [(i64, i64, i64); 2] = [(0, 0, 0), (1, 0, 0)];
pub(crate) const MID_OPERATION: [(i64, i64, i64); 2] = [(0, 0, 0), (1, 0, 1)];

pub(crate) fn aligned(conjuncts: &[&str]) -> String {
    format!(
        "hyper NonInterference using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V align skip {{ V::Vault::mix }} \
         match {{ {} }} {{ always holds(v.l @ a = v.l @ b) }}",
        [&[AGREE], conjuncts].concat().join(" and ")
    )
}
