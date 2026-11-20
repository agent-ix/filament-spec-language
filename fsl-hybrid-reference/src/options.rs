// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1019 "Options": the three method settings and the domain of each.

use std::collections::BTreeMap;

use fsl_replay::OptionRefusal;

/// The published options and their defaults, in name order.
pub const OPTION_DEFAULTS: [(&str, &str); 3] = [
    ("precision_bits", "64"),
    ("refine_depth", "32"),
    ("step", "1/16"),
];

/// The least and greatest `precision_bits`.
const PRECISION_BITS: (u32, u32) = (1, 4096);

/// The greatest `refine_depth`.
const REFINE_DEPTH_MAX: u32 = 1024;

/// Whether `text` is a decimal integer in `lower..=upper`: ASCII digits
/// only, no sign.
fn integer_in(text: &str, lower: u32, upper: u32) -> bool {
    if !digits(text) {
        return false;
    }
    let significant = text.trim_start_matches('0');
    let value = if significant.is_empty() {
        Some(0)
    } else {
        significant.parse::<u32>().ok()
    };
    value.is_some_and(|value| (lower..=upper).contains(&value))
}

/// Whether `text` is a non-empty run of ASCII digits with a non-zero digit.
fn positive_digits(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && text.bytes().any(|byte| byte != b'0')
}

/// Whether `text` is a run of ASCII digits, possibly all zeros.
fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Whether `text` is a positive exact rational written `n/d` or as a decimal
/// literal `i.f`.
fn positive_rational(text: &str) -> bool {
    if let Some((numerator, denominator)) = text.split_once('/') {
        return positive_digits(numerator) && positive_digits(denominator);
    }
    match text.split_once('.') {
        Some((whole, fraction)) => {
            digits(whole)
                && digits(fraction)
                && (whole.bytes().chain(fraction.bytes())).any(|byte| byte != b'0')
        }
        None => positive_digits(text),
    }
}

/// Refuse the first published option, in name order, whose value lies outside
/// its domain; a name the provider does not publish is FR-967's, not read.
///
/// # Errors
///
/// [`OptionRefusal`] naming that option and value.
pub fn check(options: &BTreeMap<String, String>) -> Result<(), OptionRefusal> {
    for (option, value) in options {
        let admitted = match option.as_str() {
            "precision_bits" => integer_in(value, PRECISION_BITS.0, PRECISION_BITS.1),
            "refine_depth" => integer_in(value, 0, REFINE_DEPTH_MAX),
            "step" => positive_rational(value),
            _ => true,
        };
        if !admitted {
            return Err(OptionRefusal {
                option: option.clone(),
                value: value.clone(),
            });
        }
    }
    Ok(())
}
