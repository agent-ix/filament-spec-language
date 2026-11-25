// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared projected-hyperproperty fixture helpers.

/// `μ` matching the arguments of two `step` steps and nothing else: a `step`
/// is matched against a `step` of equal input, and against any step that is
/// not one.
pub(crate) const AGREE: &str =
    "(if present(a.step.Vault::step.i) and present(b.step.Vault::step.i) \
     then value(a.step.Vault::step.i) = value(b.step.Vault::step.i) else true)";

/// The vault's two idle initial states `(h, l, busy)`.
pub(crate) const IDLE: [(i64, i64, i64); 2] = [(0, 0, 0), (1, 0, 0)];

/// The secret-1 run caught mid-operation, `busy` already set: its first step
/// is a `mix`.
pub(crate) const MID_OPERATION: [(i64, i64, i64); 2] = [(0, 0, 0), (1, 0, 1)];

/// Section 13's clause with `align skip { V::Vault::mix }` and the `match`
/// conjuncts `conjuncts`.
pub(crate) fn aligned(conjuncts: &[&str]) -> String {
    format!(
        "hyper NonInterference using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V align skip {{ V::Vault::mix }} \
         match {{ {} }} {{ always holds(v.l @ a = v.l @ b) }}",
        [&[AGREE], conjuncts].concat().join(" and ")
    )
}
