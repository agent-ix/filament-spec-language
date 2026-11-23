// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2059 properties 2 and 3 (parse half): the CST is lossless over every byte
//! string the reader admits, and no byte string panics parsing.
//!
//! These live in `fsl-cst` because `fsl-cst` depends on `fsl-foundation`; the
//! reader properties are in `fsl-foundation/tests/source_bytes.rs`.
use fsl_bench::prop;
use fsl_cst::{Limits, ParsedSource};
use fsl_foundation::SourceIdentity;
use proptest::prelude::*;

const HOST: &str = "language \"ix:native\" edition \"1-draft\";\r\n// note\n/* block */ profile v = \"filament.value.complete/v1\";\nfunction f using v(x: Integer): Integer pure { x + 1 }\n";

fn byte_strings() -> BoxedStrategy<Vec<u8>> {
    let forms: Vec<Vec<u8>> = vec![
        vec![0xEF, 0xBB, 0xBF],
        vec![0],
        vec![b'\r'],
        vec![0xFF],
        vec![0xC0, 0x80],
        vec![b'#'],
        vec![b'"'],
        vec![b'/', b'*'],
        "\u{7f}\u{2028}é".as_bytes().to_vec(),
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
        proptest::sample::select(forms),
        0..=HOST.len(),
        proptest::bool::ANY,
    )
        .prop_map(|(form, at, at_start)| {
            let at = if at_start { 0 } else { at };
            let mut bytes = HOST.as_bytes().to_vec();
            // Splice at a char boundary of the ASCII host.
            bytes.splice(at..at, form);
            bytes
        });
    prop_oneof![arbitrary, utf8, spliced].boxed()
}

fn parse(bytes: &[u8]) -> Result<ParsedSource, ()> {
    fsl_cst::parse(
        SourceIdentity::new("agent-ix", "source-bytes"),
        "generated.native",
        bytes,
        Limits::default(),
    )
    .map_err(|_| ())
}

/// The bytes of every token leaf in order, each leaf contributed by `leaf`.
fn concatenate(parsed: &ParsedSource, leaf: impl Fn(&[u8]) -> Vec<u8>) -> Vec<u8> {
    parsed
        .cst()
        .tokens()
        .iter()
        .flat_map(|token| leaf(token.spelling()))
        .collect()
}

/// Provenance: QSL-2059 P2. Every leaf (token, whitespace, comment, invalid
/// leaf) concatenates back to the input, and the leaf spans tile the source.
#[test]
fn cst_leaves_concatenate_to_the_source() {
    prop::run(
        "cst_leaves_concatenate_to_the_source",
        prop::config(256),
        &byte_strings(),
        |bytes| {
            let Ok(parsed) = parse(&bytes) else {
                return Ok(());
            };
            let joined = concatenate(&parsed, |spelling| spelling.to_vec());
            prop_assert_eq!(&joined, &bytes, "leaves differ from the source");
            prop_assert_eq!(parsed.cst().render(), bytes.clone());
            let mut next = 0usize;
            for token in parsed.cst().tokens() {
                let span = token.span();
                prop_assert_eq!(span.start, next, "gap or overlap before {:?}", span);
                next = span.end;
            }
            prop_assert_eq!(next, bytes.len());
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2059 P3 (parse half). Parsing ends in a parse result or a
/// located diagnostic and never panics.
#[test]
fn arbitrary_bytes_never_panic_in_the_parser() {
    prop::run(
        "arbitrary_bytes_never_panic_in_the_parser",
        prop::config(256),
        &byte_strings(),
        |bytes| {
            let outcome = std::panic::catch_unwind(|| match parse(&bytes) {
                Ok(parsed) => parsed.is_admissible() || !parsed.diagnostics().is_empty(),
                Err(()) => true,
            });
            prop_assert_eq!(outcome.ok(), Some(true), "{:02x?}", bytes);
            Ok(())
        },
    )
    .unwrap();
}

/// A leaf builder that drops the last byte of any leaf longer than one byte,
/// standing in for a CST that loses a byte.
fn drops_a_byte(spelling: &[u8]) -> Vec<u8> {
    let keep = spelling.len().saturating_sub(usize::from(spelling.len() > 1));
    spelling[..keep].to_vec()
}

/// Provenance: QSL-2059 negative. The concatenation property fails, and shrinks
/// to a short input, when the leaf builder drops one byte.
#[test]
fn dropped_byte_is_caught() {
    let mut config = prop::config(256);
    config.failure_persistence = None;
    let outcome = prop::run(
        "dropped_byte_is_caught",
        config,
        &byte_strings(),
        |bytes| {
            let Ok(parsed) = parse(&bytes) else {
                return Ok(());
            };
            prop_assert_eq!(concatenate(&parsed, drops_a_byte), bytes);
            Ok(())
        },
    );
    match outcome {
        Err(proptest::test_runner::TestError::Fail(_, minimal)) => {
            assert!(minimal.len() <= 8, "did not shrink: {minimal:02x?}");
        }
        other => panic!("the dropped byte went unnoticed: {other:?}"),
    }
}

/// Provenance: QSL-2059 negative. A bare CR is read (FR-001) and then refused by
/// the parser: the parse is not admissible and carries a located diagnostic.
#[test]
fn bare_cr_is_refused_by_the_parser() {
    let parsed = parse(b"language \"ix:native\" edition \"1-draft\";\rprofile v = \"x\";\n")
        .expect("the reader keeps a bare CR");
    assert!(!parsed.is_admissible(), "a bare CR parsed as admissible source");
    assert!(!parsed.diagnostics().is_empty());
}
