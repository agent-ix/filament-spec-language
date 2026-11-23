// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2059: reading arbitrary bytes as source either refuses them with a typed
//! cause or keeps them byte for byte.
//!
//! Properties 2 and the parse half of 3 live in `fsl-cst/tests/it/source_bytes.rs`
//! (`fsl-cst` depends on `fsl-foundation`, so the CST cannot be built from here).
use fsl_foundation::source::{Source, SourceIdentity, SourceReadCause};
use fsl_proptest_policy as prop;
use proptest::prelude::*;

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
const LIMIT: usize = 1 << 20;

fn read(bytes: &[u8]) -> Result<Source, SourceReadCause> {
    Source::read_typed(
        SourceIdentity::new("agent-ix", "source-bytes"),
        "generated.native",
        bytes,
        LIMIT,
    )
    .map_err(|refusal| refusal.cause)
}

/// The refusal the standard requires, computed from the bytes alone, in the
/// FR-001 order (invalid UTF-8, then a leading BOM, then NUL).
fn expected_cause(bytes: &[u8]) -> Option<SourceReadCause> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Some(SourceReadCause::InvalidUtf8);
    };
    if bytes.starts_with(&BOM) {
        return Some(SourceReadCause::Bom);
    }
    if text.contains('\0') {
        return Some(SourceReadCause::Nul);
    }
    None
}

/// Valid FSL-looking source used as the host for spliced refused forms.
const HOST: &str = "language \"ix:native\" edition \"1-draft\";\r\n// note\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(x: Integer): Integer pure { x + 1 }\n";

/// Arbitrary bytes, UTF-8-biased text, and valid source with refused forms spliced in.
pub fn byte_strings() -> BoxedStrategy<Vec<u8>> {
    let refused_forms: Vec<Vec<u8>> = vec![
        BOM.to_vec(),
        vec![0],
        vec![b'\r'],
        vec![0xFF],
        vec![0xC0, 0x80],
        vec![0xED, 0xA0, 0x80],
        vec![0xF4, 0x90, 0x80, 0x80],
        vec![0xE2, 0x82],
    ];
    let arbitrary = proptest::collection::vec(any::<u8>(), 0..64);
    let utf8 = proptest::collection::vec(
        prop_oneof![
            4 => proptest::char::range(' ', '~'),
            1 => Just('\n'),
            1 => Just('\r'),
            1 => proptest::char::range('\u{80}', '\u{10FFFF}'),
        ],
        0..48,
    )
    .prop_map(|chars| chars.into_iter().collect::<String>().into_bytes());
    let spliced = (
        proptest::sample::select(refused_forms),
        0..=HOST.len(),
        proptest::bool::ANY,
    )
        .prop_map(|(form, at, at_start)| {
            let at = if at_start { 0 } else { at };
            let mut bytes = HOST.as_bytes().to_vec();
            bytes.splice(at..at, form);
            bytes
        });
    prop_oneof![arbitrary, utf8, spliced].boxed()
}

/// Provenance: QSL-2059 P1. Either a typed refusal for exactly the refused
/// forms, or a source whose bytes equal the input.
#[test]
fn read_either_refuses_or_keeps_the_exact_bytes() {
    prop::run(
        "read_either_refuses_or_keeps_the_exact_bytes",
        prop::config(256),
        &byte_strings(),
        |bytes| {
            match (read(&bytes), expected_cause(&bytes)) {
                (Ok(source), None) => prop_assert_eq!(source.text().as_bytes(), &bytes[..]),
                (Err(actual), Some(expected)) => prop_assert_eq!(actual, expected),
                (Ok(_), Some(expected)) => {
                    return Err(TestCaseError::fail(format!(
                        "read a source that must be refused as {expected:?}: {bytes:02x?}"
                    )))
                }
                (Err(actual), None) => {
                    return Err(TestCaseError::fail(format!(
                        "refused valid source as {actual:?}: {bytes:02x?}"
                    )))
                }
            }
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2059 P3 (read half). No byte string panics the reader.
#[test]
fn arbitrary_bytes_never_panic() {
    prop::run(
        "arbitrary_bytes_never_panic",
        prop::config(256),
        &byte_strings(),
        |bytes| {
            let outcome = std::panic::catch_unwind(|| read(&bytes).is_ok());
            prop_assert!(outcome.is_ok(), "reader panicked on {bytes:02x?}");
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2059 P4. UTF-8 text without BOM or NUL is always read, and
/// its bytes, including bare CR and CRLF, are unchanged.
#[test]
fn valid_utf8_source_is_always_read() {
    let text = proptest::collection::vec(
        prop_oneof![
            4 => proptest::char::range(' ', '~'),
            2 => Just('\n'),
            2 => Just('\r'),
            1 => Just('\t'),
            2 => proptest::char::range('\u{80}', '\u{10FFFF}'),
        ],
        0..64,
    )
    .prop_map(|chars| {
        chars
            .into_iter()
            .filter(|c| *c != '\0' && *c != '\u{FEFF}')
            .collect::<String>()
    });
    prop::run(
        "valid_utf8_source_is_always_read",
        prop::config(256),
        &text,
        |text| {
            let source = read(text.as_bytes())
                .map_err(|cause| TestCaseError::fail(format!("{cause:?} for {text:?}")))?;
            prop_assert_eq!(source.text(), text.as_str());
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2059 negative. Each refused form is refused with its cause.
#[test]
fn refused_forms_are_refused() {
    assert_eq!(
        read(&[0xEF, 0xBB, 0xBF, b'a']).unwrap_err(),
        SourceReadCause::Bom
    );
    assert_eq!(read(b"a\0b").unwrap_err(), SourceReadCause::Nul);
    assert_eq!(
        read(&[0xFF, b'a']).unwrap_err(),
        SourceReadCause::InvalidUtf8
    );
    assert_eq!(
        read(&[0xC0, 0x80]).unwrap_err(),
        SourceReadCause::InvalidUtf8
    );
}

/// Provenance: QSL-2059. FR-001 closes the reader refusals to invalid UTF-8, BOM and
/// NUL; a bare CR is kept byte for byte here and refused by the parser instead
/// (`bare_cr_is_refused_by_the_parser` in `fsl-cst/tests/it/source_bytes.rs`).
#[test]
fn bare_cr_is_read_unchanged() {
    let source = read(b"a\rb").expect("FR-001 does not refuse a bare CR at read time");
    assert_eq!(source.text().as_bytes(), b"a\rb");
}
