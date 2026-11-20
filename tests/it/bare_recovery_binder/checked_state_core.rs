// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compound checked expressions evaluated after the source-side handoff has gone away.

use super::*;

#[test]
#[trace("FR-531-AC-3", "FR-520-AC-1")]
fn bytes_only_recovery_combines_modulo_multiplicity_sum_and_branch_selection() {
    let recipe = recipe_with(Some(
        "let n = size(recoveredFull.receipts) in if activatedFull then n mod 3 = 0 else sum<M::Total>(r in recoveredFull.receipts: r) = targetFull and n mod 3 = 1",
    ));
    for (receipts, target, activated, expected) in [
        (&[3, 3, 3, 3][..], 12, false, true),
        (&[4, 4, 4][..], 12, false, false),
        (&[12][..], 12, false, true),
        (&[12][..], 11, false, false),
        (&[4, 4, 4][..], 0, true, true),
        (&[3, 3, 3, 3][..], 0, true, false),
    ] {
        assert_eq!(
            holds(recipe.recover("Full", receipts, target, activated)),
            expected,
            "receipts={receipts:?}, target={target}, activated={activated}"
        );
    }
}

#[test]
#[trace("FR-531-AC-3", "FR-520-AC-1")]
fn bytes_only_recovery_calls_the_emitted_function_with_the_query_result() {
    let recipe = recipe_with_declarations(Some(
        "residue(size(recoveredFull.receipts)) = 2 and sum<M::Total>(r in recoveredFull.receipts: r) = targetFull and not activatedFull",
    ), "function residue using v(n: Integer): Integer pure { (n + 4) mod 5 }");
    for (receipts, target, activated, expected) in [
        (&[2, 2, 5][..], 9, false, true),
        (&[4, 5][..], 9, false, false),
        (&[2, 2, 5][..], 8, false, false),
        (&[2, 2, 5][..], 9, true, false),
    ] {
        assert_eq!(
            holds(recipe.recover("Full", receipts, target, activated)),
            expected,
            "receipts={receipts:?}, target={target}, activated={activated}"
        );
    }
}
