// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generated values are tested against the actual checked expression type.
//! The generated operators are total on these admitted parameter assignments;
//! no runtime-only operation is exempted from successful completion.

use super::typed_value_case::{body_type, call, compile, family, inside, sampled_source};
use fsl_bench::prop::{self, value};
use proptest::prelude::{any, prop_assert, TestCaseError};
use std::cell::RefCell;
use std::collections::BTreeSet;

/// Provenance: QSL-2056. The membership oracle reads real S3 node types.
#[test]
fn p1_completed_values_lie_inside_the_checked_type() {
    let covered = RefCell::new(BTreeSet::new());
    prop::run(
        "p1_completed_values_lie_inside_the_checked_type",
        prop::config(256),
        &(value::expressions(), any::<u8>()),
        |(input, assignment)| {
            let source = sampled_source(&input, assignment);
            let package = compile(&source)
                .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
            let checked = body_type(&package, "f");
            let result = call(&package, "probe", Vec::new())
                .map_err(|error| TestCaseError::fail(format!("{error}\n{source}")))?;
            prop_assert!(
                inside(&result, &checked),
                "source={source}\nassignment seed={assignment}\nchecked={checked:?}\nvalue={result:?}"
            );
            covered.borrow_mut().insert(family(&input.result));
            Ok(())
        },
    )
    .unwrap();
    for expected in ["integer", "rational", "decimal", "sequence", "set", "bag"] {
        assert!(
            covered.borrow().contains(expected),
            "no actual {expected} result was evaluated"
        );
    }
}

/// Provenance: QSL-2056. No refusal, undefined result, or internal failure is
/// allowed for the generator's total expressions and in-domain arguments.
#[test]
fn p2_no_completed_call_returns_runtime_invariant() {
    prop::run(
        "p2_no_completed_call_returns_runtime_invariant",
        prop::config(256),
        &(value::expressions(), any::<u8>()),
        |(input, assignment)| {
            let source = sampled_source(&input, assignment);
            let package = compile(&source)
                .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
            call(&package, "probe", Vec::new())
                .map_err(|error| TestCaseError::fail(format!("{error}\n{source}")))?;
            Ok(())
        },
    )
    .unwrap();
}
