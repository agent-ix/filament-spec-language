// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218-AC-17 (TC-2978): each protocol decision refusal names its
//! offending item by its typed cause (the declared decision name and slot,
//! or the undeclared owner) at the item's span, and the FR-865 text line
//! shows that name and the span's path, line and column.

use std::path::{Path, PathBuf};

use filament_exact::Cancel;
use fsl_command::command::{self, CheckInputs};
use fsl_inspect::{render_outcome, RenderForm};
use fsl_replay::RenderLimits;
use ix_trace_rs::trace;
use serde_json::Value;

/// The spine-model unit with the protocol profile and `run` as the body of
/// protocol `Decide` over `input: M::Gadget` with role `R`.
fn unit(run: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile p = \"filament.protocol.finite-global/v1\";\n\
         model M = \"acme/orders\";\n\
         protocol Decide using p over (input: M::Gadget) on origin {{\n\
         role R on M::Gadget;\n\
         run {run}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

fn model() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/spine-model.semantic-ir.json")
}

/// The check of `text`: its document, its text form and its path.
fn refuse(text: &str) -> (Value, String, String) {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("decide.fsl");
    std::fs::write(&file, text).unwrap();
    let inputs = CheckInputs {
        models: vec![("acme/orders".to_owned(), model())],
        ..CheckInputs::default()
    };
    let outcome = command::check_outcome(&file, &inputs);
    let document: Value = serde_json::from_slice(&outcome.document().to_bytes().unwrap()).unwrap();
    assert_eq!(document["category"], "refusal", "{document}");
    let rendered = render_outcome(
        outcome.document(),
        RenderForm::Text,
        &fsl_command::render_sources(outcome.sources()),
        RenderLimits::default(),
        &Cancel::new(),
    )
    .expect("the refusal renders");
    (
        document,
        rendered.value().as_str().to_owned(),
        file.to_string_lossy().into_owned(),
    )
}

/// The one-based line of the first occurrence of `needle` in `text`.
fn line_of(text: &str, needle: &str) -> usize {
    let at = text.find(needle).expect("the item is in the source");
    text[..at].matches('\n').count() + 1
}

/// Asserts the first diagnostic is `code`/`cause` at exactly `item`, that its
/// message names each of `names`, and that its text line shows the path and
/// the item's line.
fn assert_named(text: &str, item: &str, code: &str, cause: &str, names: &[&str]) {
    let (document, rendered, path) = refuse(text);
    let diagnostic = &document["diagnostics"][0];
    assert_eq!(diagnostic["code"], code, "{document}");
    assert_eq!(diagnostic["cause"], cause, "{document}");
    let start = diagnostic["locus"]["start"]
        .as_u64()
        .expect("a region start") as usize;
    let end = diagnostic["locus"]["end"].as_u64().expect("a region end") as usize;
    assert_eq!(&text[start..end], item, "{diagnostic}");
    let message = diagnostic["message"].as_str().expect("a message");
    for name in names {
        assert!(
            message.to_lowercase().contains(&name.to_lowercase()),
            "{message} names {name}"
        );
    }
    let line = format!("{path}:{}:", line_of(text, item));
    let shown = rendered
        .lines()
        .find(|shown| shown.starts_with(&line))
        .unwrap_or_else(|| panic!("a text line at {line}: {rendered}"));
    for name in names {
        assert!(
            shown.to_lowercase().contains(&name.to_lowercase()),
            "{shown} names {name}"
        );
    }
}

/// FR-218-AC-17: the `over` read, the unlisted fact read and the undeclared
/// owner each refuse at the item, named by declared name and slot, and the
/// text form shows that name and location.
#[test]
#[trace("TC-2978", "FR-218-AC-17")]
fn each_protocol_refusal_names_its_subject() {
    let over = unit(
        "sequence Main { choice Pick by R visible () { \
         case big when { input.code > 0 } sequence Big { } \
         case small when { not (input.code > 0) } sequence Small { } } }",
    );
    assert_named(
        &over,
        "input.code",
        "ill_typed",
        "operator-ineligible",
        &["Pick", "guard"],
    );

    let unlisted = unit(
        "sequence Main { event Seen by R as (seen: Boolean) { true }; \
         choice Pick by R visible () { \
         case yes when { seen } sequence Yes { } \
         case no when { not seen } sequence No { } } }",
    );
    assert_named(
        &unlisted,
        "seen",
        "wrong_snapshot",
        "wrong-observation",
        &["Pick", "seen"],
    );

    let owner = unit(
        "sequence Main { choice Pick by Nobody visible () { \
         case yes when { true } sequence Yes { } \
         case no when { false } sequence No { } } }",
    );
    assert_named(
        &owner,
        "Nobody",
        "missing_declaration",
        "missing-name",
        &["Nobody"],
    );
}
