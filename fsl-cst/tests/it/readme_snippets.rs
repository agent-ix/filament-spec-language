// SPDX-License-Identifier: AGPL-3.0-or-later
//! Keep the README's FSL examples parseable as the grammar changes.

use fsl_cst::{Limits, ParsedSource};
use fsl_foundation::SourceIdentity;

const README: &str = include_str!("../../../README.md");

fn parse(text: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("test", "readme-snippets"),
        "readme.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("README example is valid UTF-8 within the source ceiling")
}

#[test]
fn readme_protocol_excerpt_parses_and_native_fences_parse() {
    let mut protocol = Vec::new();
    let mut depth = 0isize;
    for line in README
        .lines()
        .skip_while(|line| !line.contains("protocol Orders"))
    {
        depth += line.matches('{').count() as isize;
        depth -= line.matches('}').count() as isize;
        protocol.push(line);
        if depth == 0 {
            break;
        }
    }
    assert!(!protocol.is_empty(), "README contains the Orders protocol");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile P = \"filament.protocol.finite-global/v1\";\nmodel M = \"example/orders\";\n{}\n",
        protocol.join("\n")
    );
    let parsed = parse(&source);
    assert!(
        parsed.is_admissible(),
        "protocol: {:?}",
        parsed.diagnostics()
    );
    assert_eq!(parsed.cst().render(), source.as_bytes());

    let mut in_native_fence = false;
    let mut native = Vec::new();
    let mut fences = Vec::new();
    for line in README.lines() {
        if !in_native_fence && line.trim() == "```ix:native" {
            in_native_fence = true;
            native.clear();
        } else if in_native_fence && line.trim() == "```" {
            fences.push(native.join("\n"));
            in_native_fence = false;
        } else if in_native_fence {
            native.push(line);
        }
    }

    assert!(!fences.is_empty(), "README contains an ix:native example");
    for body in fences {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\n{body}\n"
        );
        let parsed = parse(&source);
        assert!(parsed.is_admissible(), "{body}: {:?}", parsed.diagnostics());
        assert_eq!(parsed.cst().render(), source.as_bytes());
    }
}
