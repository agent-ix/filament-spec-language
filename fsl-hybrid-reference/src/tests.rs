// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1019 "Options": the value domains `check_options` refuses.

use std::collections::BTreeMap;

use fsl_replay::HybridProvider;
use ix_trace_rs::trace;

use super::{ReferenceProvider, OPTION_DEFAULTS};

fn options(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn refused(pairs: &[(&str, &str)]) -> Option<(String, String)> {
    ReferenceProvider
        .check_options(&options(pairs))
        .err()
        .map(|refusal| (refusal.option, refusal.value))
}

/// FR-1019-AC-7: the published defaults and the in-domain values pass.
#[trace("TC-2869", "FR-1019-AC-7")]
#[test]
fn the_defaults_and_in_domain_values_are_admitted() {
    assert_eq!(refused(&[]), None);
    assert_eq!(refused(&OPTION_DEFAULTS), None);
    for pairs in [
        &[("precision_bits", "1")][..],
        &[("precision_bits", "4096")],
        &[("precision_bits", "8"), ("step", "1/4")],
        &[("refine_depth", "0")],
        &[("refine_depth", "1024")],
        &[("step", "0.25")],
        &[("step", "3")],
        &[("step", "007/0010")],
    ] {
        assert_eq!(refused(pairs), None, "{pairs:?}");
    }
}

/// FR-1019-AC-7: a value outside its domain is refused, naming the option and
/// value as written; the least refused option is named.
#[trace("TC-2869", "FR-1019-AC-7")]
#[test]
fn a_value_outside_its_domain_is_refused_by_name() {
    for (option, value) in [
        ("step", "0"),
        ("step", "0/5"),
        ("step", "1/0"),
        ("step", "-1"),
        ("step", "0.0"),
        ("step", "1."),
        ("step", ".5"),
        ("step", "1/2/3"),
        ("step", ""),
        ("precision_bits", "0"),
        ("precision_bits", "4097"),
        ("precision_bits", "+8"),
        ("precision_bits", "99999999999999999999"),
        ("refine_depth", "1025"),
        ("refine_depth", "-1"),
        ("refine_depth", "1.5"),
    ] {
        assert_eq!(
            refused(&[(option, value)]),
            Some((option.to_owned(), value.to_owned())),
            "{option}={value:?}"
        );
    }
    assert_eq!(
        refused(&[("step", "0"), ("precision_bits", "0")]),
        Some(("precision_bits".to_owned(), "0".to_owned()))
    );
}

/// An unpublished name is FR-967's refusal, not `check_options`'.
#[trace("TC-2869", "FR-1019-AC-7")]
#[test]
fn an_unpublished_name_is_left_to_the_registry() {
    assert_eq!(refused(&[("colour", "blue")]), None);
}
