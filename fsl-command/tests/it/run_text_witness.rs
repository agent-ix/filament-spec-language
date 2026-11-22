// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FSL command run seam retains checked source correspondence for text
//! outcome rendering while leaving the outcome JSON unchanged.

use filament_exact::Cancel;
use fsl_command::command;
use fsl_inspect::{render_outcome, RenderForm, RenderLimits};
use fsl_semantic_value::catalog::Category;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const MODEL: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"example/shop\";\n\
    object Order identity (id) {\n\
      id: Int[0, 9999];\n\
      charged: Int[0, 9999];\n\
      total: Int[0, 9999];\n\
    }\n\
    population orders: Order;\n";

const PROGRAM: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    model Shop = \"example/shop\";\n\
    invariant PaidInFull using v on Shop::Order at current {\n\
      self.charged = self.total\n\
    }\n";

const PAID_SHORT: &str = r#"{
  "format": "filament.state.snapshot/v1",
  "identity": {"authority": "agent-ix", "identity": "test:paid-short:current"},
  "model": {"identity": "example/shop"},
  "observation": "current",
  "anchor": {"kind": "handler", "name": "validate"},
  "populations": [{
    "population": "ix://example/shop/orders",
    "complete": true,
    "objects": [{
      "key": "o1",
      "type": "ix://example/shop/Order",
      "fields": {
        "id": {"integer": "1"},
        "charged": {"integer": "4000"},
        "total": {"integer": "4200"}
      }
    }]
  }]
}"#;

fn source(file: &str) -> Value {
    json!({
        "file": file,
        "authority": "agent-ix",
        "identity": format!("test:paid-short:{file}"),
        "document": file,
        "formal_revision": 1
    })
}

fn paid_short_request(directory: &Path) -> std::path::PathBuf {
    std::fs::create_dir_all(directory.join("data")).expect("data directory creates");
    std::fs::write(directory.join("model.fsl"), MODEL).expect("model source writes");
    std::fs::write(directory.join("program.fsl"), PROGRAM).expect("program source writes");
    std::fs::write(directory.join("data/paid-short.json"), PAID_SHORT)
        .expect("paid-short observation writes");
    let request = json!({
        "format": "native-run/1",
        "request": {
            "models": [{
                "format": "native-model-unit/1",
                "source": source("model.fsl")
            }],
            "program": {"source": source("program.fsl")},
            "clause": {
                "selection": {
                    "clause": {
                        "name": "PaidInFull",
                        "observation": "current",
                        "self": {
                            "population": "ix://example/shop/orders",
                            "key": "o1"
                        },
                        "anchor": {"kind": "handler", "name": "validate"}
                    }
                },
                "observations": {"current": "data/paid-short.json"}
            }
        }
    });
    let path = directory.join("run.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&request).expect("request serializes"),
    )
    .expect("request writes");
    path
}

/// FSL command integration coverage: run a paid-short-shaped invariant and
/// render the returned outcome through the same source bridge used by callers.
/// Provenance: QSL-2871.
#[test]
#[trace("FR-865-AC-2")]
fn fsl_command_run_text_includes_paid_short_witness_values() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let request = paid_short_request(directory.path());
    let result = command::run(&request, &crate::negotiation::Fr290)
        .expect("the paid-short clause completes as a violation");
    let document = result.outcome.as_ref().expect("clause run has an outcome");
    assert_eq!(result.exit_code, Category::Violation.exit_code());

    let rendered = render_outcome(
        document,
        RenderForm::Text,
        &fsl_command::render_sources(result.sources()),
        RenderLimits::default(),
        &Cancel::new(),
    )
    .expect("text outcome renders with the command's retained sources");
    let expected_header = format!(
        "run: violation (exit {}) at S6a\n",
        Category::Violation.exit_code()
    );
    assert!(rendered.value().as_str().starts_with(&expected_header));
    assert!(
        rendered
            .value()
            .as_str()
            .contains("  witness: self.charged = 4000, self.total = 4200"),
        "{}",
        rendered.value().as_str()
    );

    let json = render_outcome(
        document,
        RenderForm::Json,
        &fsl_command::render_sources(result.sources()),
        RenderLimits::default(),
        &Cancel::new(),
    )
    .expect("JSON outcome renders with the same retained source map");
    assert_eq!(
        json.value().bytes(),
        document.to_bytes().expect("outcome document serializes")
    );
}
