// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-003 "Canonical style" (TC-2783, TC-2784): the formatter prints the
//! canonical line layout and spacing and keeps every significant token.
use fsl_cst::{Limits, ParsedSource, TokenClass};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

fn header(profile: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n\
         profile inf = \"{profile}\";\nmodel M = \"acme/model\";\nmodel V = \"acme/vault\";\n"
    )
}

fn parse(text: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:formatter-style".into(),
        },
        "formatter-style.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn significant(parsed: &ParsedSource) -> Vec<Vec<u8>> {
    parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == TokenClass::Token)
        .map(|token| token.spelling().to_vec())
        .collect()
}

fn formatted(text: &str) -> String {
    let parsed = parse(text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let out = fsl_cst::format::format(&parsed).expect("formats");
    assert_eq!(significant(&parsed), significant(&parse(&out)), "{out}");
    out
}

fn stripped(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// FR-003-AC-12: calls, binders, bounds, intervals and type arguments print
/// in the canonical spacing.
#[trace("TC-2783", "FR-003-AC-12")]
#[test]
#[ignore = "QSL-1974: the canonical call, binder, bracket and type-argument spacing is not implemented yet"]
fn calls_binders_bounds_and_type_arguments_print_canonically() {
    let text = format!(
        "{}function f using v (x : Option < Int [0,9] >, c : Sequence<Integer>[0,4], a : Integer, b : Integer) : Boolean pure \
         {{ present (x) and forall (r in c : r > 0) and a<b and convert < Integer > (a) = b }}\n",
        header("filament.value.complete/v1")
    );
    let out = formatted(&text);
    for expected in [
        "f using v(x: Option<Int[0, 9]>, c: Sequence<Integer>[0, 4], a: Integer, b: Integer): Boolean pure",
        "present(x)",
        "forall(r in c: r > 0)",
        "a < b",
        "convert<Integer>(a)",
    ] {
        assert!(out.contains(expected), "{expected}: {out}");
    }
    assert_eq!(stripped(&out), stripped(&text));
    let temporal = format!(
        "{}temporal T using inf over (o: M::O) on origin {{ always [0,30] holds (o.ok) }}\n",
        header("filament.temporal.complete/v1")
    );
    let out = formatted(&temporal);
    assert!(out.contains("over (o: M::O)"), "{out}");
    assert!(out.contains("always[0, 30] holds(o.ok)"), "{out}");
}

/// FR-003-AC-11: a protocol statement after a closing brace starts its own
/// line at every depth.
#[trace("TC-2783", "FR-003-AC-11")]
#[test]
#[ignore = "QSL-1974: starting a statement after a nested closing brace on its own line is not implemented yet"]
fn a_statement_after_a_closing_brace_starts_its_own_line() {
    let text = format!(
        "{}protocol P using v over (o: M::O) on origin {{ role a on M::A; role b on M::B; \
         run sequence Main {{}} parallel Both {{ branch x sequence s {{}} branch y sequence t {{}} }} join all [x, y]; \
         finish Closed as (r: Boolean) {{ true }}; }}\n",
        header("filament.value.complete/v1")
    );
    let out = formatted(&text);
    for line in out.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix('}') {
            let rest = rest.trim_start();
            assert!(
                rest.is_empty() || rest.starts_with([';', ',', ')', ']', '{']),
                "a word follows a closing brace: {line}"
            );
        }
    }
    for statement in ["parallel Both", "finish Closed"] {
        assert!(
            out.lines()
                .any(|line| line.trim_start().starts_with(statement)),
            "{statement}: {out}"
        );
    }
}

/// FR-003-AC-13: a hyper clause's `match` block closes as `} {`.
#[trace("TC-2784", "FR-003-AC-13")]
#[test]
#[ignore = "QSL-1974: keeping `} {` together after a match block is not implemented yet"]
fn a_hyper_match_block_closes_without_a_split_brace() {
    let text = format!(
        "{}hyper NI using inf over behaviours (v: V::Vault) forall trace a of V forall trace b of V \
         match {{ a.step.op = b.step.op }} {{ always holds(v.l @ a = v.l @ b) }}\n",
        header("filament.temporal.infinite-trace/v1")
    );
    let out = formatted(&text);
    assert!(out.lines().any(|line| line.trim() == "} {"), "{out}");
    assert!(out.contains("v.l @ a = v.l @ b"), "{out}");
}
