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
fn calls_binders_bounds_and_type_arguments_print_canonically() {
    let text = format!(
        "{}function f using v (x : Option < Int [0,9] >, c : Sequence<Integer>[0,4], a : Integer, b : Integer) : Boolean pure \
         {{ present (x) and forall (r in c : r > 0) and a<b and convert < Integer > (a) = b and not (true and false) }}\n",
        header("filament.value.complete/v1")
    );
    let out = formatted(&text);
    for expected in [
        "f using v(x: Option<Int[0, 9]>, c: Sequence<Integer>[0, 4], a: Integer, b: Integer): Boolean pure",
        "present(x)",
        "forall(r in c: r > 0)",
        "a < b",
        "convert<Integer>(a)",
        "not (true and false)",
    ] {
        assert!(out.contains(expected), "{expected}: {out}");
    }
    assert_eq!(stripped(&out), stripped(&text));
    let temporal = format!(
        "{}temporal T using inf over (o: M::O) on origin when (o.ok) {{ always [0,30] holds (o.ok) }}\n",
        header("filament.temporal.complete/v1")
    );
    let out = formatted(&temporal);
    assert!(out.contains("over (o: M::O)"), "{out}");
    assert!(out.contains("when (o.ok)"), "{out}");
    assert!(out.contains("always[0, 30] holds(o.ok)"), "{out}");
}

/// FR-003-AC-11: a protocol statement after a closing brace starts its own
/// line at every depth.
#[trace("TC-2783", "FR-003-AC-11")]
#[test]
fn a_statement_after_a_closing_brace_starts_its_own_line() {
    let text = format!(
        "{}protocol P using v over (o: M::O) on origin {{ role a on M::A; role b on M::B; \
         channel Messages from a to b carries M::Message ordering fifo by (m: M::Message) {{ m.key }} delivery [0,1] capacity 2 overflow block; \
         run sequence Main {{ send Sent via Messages as (sent: M::Message) {{ true }}; \
         parallel Both {{ branch x sequence s {{}} branch y sequence t {{}} }} join all [x, y]; \
         await Refund after Main::Sent using v clock \"ticks\" within [0,1] match receive Reply via Messages of Main::Sent as (reply: M::Message) {{ true }}; then sequence Then {{ }} timeout sequence Timeout {{ }} }} \
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
    for statement in ["parallel Both", "await Refund", "finish Closed"] {
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

/// FR-003-AC-15: the token after a type-argument `>` is spaced by its own
/// rule, a `;` inside brackets keeps its line, and a closing `)`, `]` or `>`
/// is separated from a following word or `{`.
#[trace("TC-2783", "FR-003-AC-15")]
#[test]
fn closing_tokens_and_bracketed_semicolons_print_canonically() {
    let header = header("filament.value.complete/v1");
    for (source, expected) in [
        (
            "function f using v(): Boolean pure { let o: Option < Int[0,9] > = none in present(o) }",
            "let o: Option<Int[0, 9]> = none in present(o)",
        ),
        (
            "function g using v(x: Option<Int[0,9]>): Option<Int[0,9]>pure { x }",
            "g using v(x: Option<Int[0, 9]>): Option<Int[0, 9]> pure {",
        ),
        (
            "function d using v(x: Decimal[0,9;0,2;exact]): Boolean pure { true }",
            "x: Decimal[0, 9; 0, 2; exact]",
        ),
    ] {
        let out = formatted(&format!("{header}{source}\n"));
        assert!(out.contains(expected), "{expected}: {out}");
    }
    let protocol = format!(
        "{header}protocol P using v over (o: M::O) on origin {{ role a on M::A; \
         run sequence Main {{}} finish Closed as (r: Boolean){{ true }}; }}\n"
    );
    let out = formatted(&protocol);
    assert!(out.contains("finish Closed as (r: Boolean) {"), "{out}");
}
