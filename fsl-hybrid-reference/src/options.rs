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

/// The method settings of one request, each option read with its published
/// default when absent.
#[derive(Clone, Debug)]
pub struct Settings {
    /// `precision_bits`: every enclosure bound is a multiple of `2^-bits`.
    pub bits: u32,
    /// `step`: the flowpipe's initial time step.
    pub step: filament_exact::Rational,
    /// `refine_depth`: how often a step is halved or a box bisected.
    pub refine_depth: u32,
}

fn integer(text: &str) -> filament_exact::Integer {
    text.parse()
        .unwrap_or_else(|_| filament_exact::Integer::zero())
}

fn rational_of(text: &str) -> filament_exact::Rational {
    use filament_exact::{Integer, Rational};
    let (numerator, denominator) = if let Some((n, d)) = text.split_once('/') {
        (integer(n), integer(d))
    } else if let Some((whole, fraction)) = text.split_once('.') {
        let scale = Integer::power_of_ten(fraction.len() as u64);
        let n = integer(&format!("{whole}{fraction}"));
        (n, scale)
    } else {
        (integer(text), Integer::one())
    };
    Rational::new(numerator, denominator).unwrap_or_else(|_| Rational::from_integer(Integer::one()))
}

impl Settings {
    /// The settings of `options`, which `check` admitted.
    pub fn read(options: &BTreeMap<String, String>) -> Self {
        let value = |name: &str| {
            options.get(name).map_or_else(
                || {
                    OPTION_DEFAULTS
                        .iter()
                        .find(|(option, _)| *option == name)
                        .map_or("", |(_, default)| *default)
                },
                String::as_str,
            )
        };
        let small = |name: &str| value(name).trim_start_matches('0').parse().unwrap_or(0);
        Self {
            bits: small("precision_bits"),
            step: rational_of(value("step")),
            refine_depth: small("refine_depth"),
        }
    }
}
