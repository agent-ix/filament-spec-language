// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710: S1 parses a model unit, the language's own route to a domain
//! package (TC-1942, TC-1943).

use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n";

fn parse(text: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("test", "model-unit"),
        "shop.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("source metadata admitted")
}

/// FR-710-AC-1's model unit.
const SHOP: &str = "domain \"example/shop\";
object Order identity (id) {
  id: Int[0, 999];
  total: Int[0, 100];
  operation pay(): Boolean modifies self.total;
}
population orders: Order;
";

/// FR-710-AC-3: one declaration of every form of the model-unit grammar.
const EVERY_FORM: &str = "domain \"example/all\";
type Small = Int[0, 3];
record Point { x: Integer; }
tuple Pair(Integer, Boolean);
enum Tier { Gold, Standard }
union Mark { Circle(Integer), Blank }
map Index = Map<Text[0, 8; nfc], Integer>[0, 4];
dimension L;
unit m : L = 1;
object Node identity (id) {
  id: Int[0, 9];
  note: Text[0, 8; nfc]?;
  tags: Integer [0, *] ordered unique;
  relationship next: Node [0, 1] inverse prev [0, 1];
  operation bump(): Boolean modifies self.id {
    pre Room { self.id < 9 };
    post Grew { self.id > pre(self.id) };
  }
  invariant Bounded { self.id >= 0 };
}
event Placed { at: Integer; occurs at; }
machine Gate { open: Boolean; operation toggle(); states (Open, Shut); transition Open -> Shut on toggle emits Placed; }
process Flow { n: Integer; operation go(); step start: begin consumes Placed; }
store Nodes persists Node { operation save(n: Reference<Node>); }
namespace Terms { members Node, Gate; term node = \"a node\"; }
population nodes: Node;
open symmetric population peers: Node;
interface Pump { rate: Integer; operation start(); }
object Sys { level: Integer; }
part pump: Sys [1, 1] in Sys;
port out_port: Pump [1, 1] out on pump;
port in_port: Pump [1, 1] in on pump;
connection Pipe pump.out_port -> pump.in_port;
allocate Alloc from pump to pump;
";

#[trace("FR-710-AC-1", "TC-1942")]
#[test]
#[ignore = "QSL-1980: S1 parses a model unit; not implemented yet"]
fn the_smallest_model_unit_parses_and_renders_back() {
    let text = format!("{HEADER}{SHOP}");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.cst().render(), text.as_bytes());
}

#[trace("FR-710-AC-2", "TC-1942")]
#[test]
#[ignore = "QSL-1980: S1 parses a model unit's relationships; not implemented yet"]
fn a_model_unit_relationship_parses() {
    let text = format!(
        "{HEADER}domain \"example/graph\";\n\
         object Node {{ relationship next: Node [0, 1]; }}\n\
         population nodes: Node;\n"
    );
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
#[ignore = "QSL-1980: S1 parses every model-unit declaration form; not implemented yet"]
fn a_model_unit_with_every_declaration_form_parses() {
    let text = format!("{HEADER}{EVERY_FORM}");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.cst().render(), text.as_bytes());
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
#[ignore = "QSL-1980: a model unit's `domain` with no string refuses at `;`; not implemented yet"]
fn a_domain_with_no_string_refuses_at_the_semicolon() {
    let text = format!("{HEADER}domain ;\n");
    let parsed = parse(&text);
    let [diagnostic] = parsed.diagnostics() else {
        panic!("one diagnostic: {:?}", parsed.diagnostics());
    };
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    let at = text.find(';').map(|first| first + 1).unwrap();
    let at = at + text[at..].find(';').unwrap();
    assert_eq!(
        diagnostic.region.as_ref().map(|region| region.start()),
        Some(at as u64)
    );
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn a_domain_after_a_profile_refuses_at_the_word() {
    let text = format!("{HEADER}profile P = \"filament.model.complete/v1\";\n{SHOP}");
    let parsed = parse(&text);
    let first = parsed.diagnostics().first().expect("a diagnostic");
    assert_eq!(
        (first.code, first.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    assert_eq!(
        first.region.as_ref().map(|region| region.start()),
        Some(text.find("domain").unwrap() as u64)
    );
}
