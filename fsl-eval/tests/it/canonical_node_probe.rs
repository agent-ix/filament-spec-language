// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-400-AC-12: the canonical `filament-checked` vocabulary is available, while
//! the public evaluator accepts only FSL's admitted checked expression.

use filament_checked::{NodeKind, NodeRead};
use filament_exact::{Value, ValueType};
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use ix_trace_rs::trace;

fn source(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn doc_blocks(text: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut open: Option<(String, String)> = None;
    for line in text.lines() {
        let Some(doc) = line.strip_prefix("//!") else {
            continue;
        };
        let doc = doc.trim();
        match (&mut open, doc.strip_prefix("```")) {
            (None, Some(info)) => open = Some((info.to_owned(), String::new())),
            (Some(_), Some(_)) => blocks.extend(open.take()),
            (Some((_, body)), None) => {
                body.push_str(doc);
                body.push('\n');
            }
            (None, None) => {}
        }
    }
    blocks
}

#[trace("FR-400-AC-12")]
#[test]
fn canonical_descriptor_and_rejected_evaluator_inputs_are_pinned() {
    let value_type = ValueType::Boolean;
    let location = Location::root(Origin::Expression);
    let operation: NodeKind<usize, &Value> = NodeKind::Local(0);
    let descriptor = NodeRead {
        operation,
        value_type: &value_type,
        location: &location,
    };
    let _ = descriptor;

    let docs = source("src/lib.rs");
    let blocks = doc_blocks(&docs);
    assert!(blocks.iter().any(|(info, body)| {
        info == "no_run"
            && body.contains("use filament_checked::{NodeKind, NodeRead};")
            && body.contains("let _descriptor = NodeRead")
    }));
    assert!(blocks.iter().any(|(info, body)| {
        info == "compile_fail,E0308" && body.contains("package.evaluate(&descriptor")
    }));
    assert!(blocks.iter().any(|(info, body)| {
        info == "compile_fail,E0308" && body.contains("package.evaluate(&operation")
    }));
}
