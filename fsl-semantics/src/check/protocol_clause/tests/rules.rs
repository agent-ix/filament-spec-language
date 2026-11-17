//! FR-218 (TC-2020, TC-2021, TC-2022): a role on a whole model, a choice
//! guard over the protocol input, and the variant's type, each asserted with
//! the refusal region every reporting surface shows.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use ix_trace_rs::trace;

use super::assemble;
use crate::check::CheckRefusal;

fn catalog(refusal: &CheckRefusal) -> (&'static str, Option<&'static str>) {
    (refusal.cause.code().as_str(), refusal.cause.cause())
}

/// The S3 refusals of `declarations`, each with its catalog pair and the
/// source text of its refusal region (`PackageDeclarations::regions`).
fn located_refusals(declarations: &str) -> Vec<(CheckRefusal, String)> {
    let (text, assembled) = assemble(declarations);
    let regions = assembled.regions();
    let refusals: Vec<CheckRefusal> =
        match assembled.check_with_protocol_results(CheckMode::Linked, CheckingLimits::default()) {
            Ok(graph) => graph
                .protocol_refusals()
                .iter()
                .flat_map(|result| result.refusals.iter().cloned())
                .collect(),
            Err(refusals) => refusals,
        };
    refusals
        .into_iter()
        .map(|refusal| {
            let region = regions
                .refusal_region(&refusal)
                .unwrap_or_else(|| panic!("{refusal:?} names a region"));
            let start = usize::try_from(region.start()).expect("a start");
            let end = usize::try_from(region.end()).expect("an end");
            let at = text[start..end].to_owned();
            (refusal, at)
        })
        .collect()
}

/// A protocol over `M::Actor` with the `captures`, the `roles` and the
/// `run` control.
fn protocol(captures: &str, roles: &str, run: &str) -> String {
    format!(
        "protocol Flow using pp over (input: M::Actor) on origin {{\n\
         {captures}\n\
         {roles}\n\
         run {run}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}

const TRIED: &str = "attempt Tried by R on M::Actor::op contracts [] as (tried: Boolean) { true };";

/// FR-218-AC-41: `role W on M` checks beside a role on a model type, and the
/// replicated form over the alias refuses with the alias as its region.
#[trace("TC-2020", "FR-218-AC-41")]
#[test]
fn a_role_on_a_whole_model_checks_and_its_replicated_form_refuses_at_the_alias() {
    let roles = "role W on M;\nrole R on M::Actor;";
    let run = format!("sequence Main {{ {TRIED} }}");
    super::checks(&protocol("", roles, &run));

    let replicated = protocol(
        "",
        &format!("{roles}\nrole r each M from pop max 2 lifetime workflow;"),
        &run,
    );
    let found = located_refusals(&replicated);
    let at_alias: Vec<&String> = found
        .iter()
        .filter(|(refusal, _)| {
            catalog(refusal) == ("invalid_model_binding", Some("malformed-declaration"))
        })
        .map(|(_, at)| at)
        .collect();
    assert_eq!(at_alias, ["M"], "{found:?}");
    assert!(
        found.iter().all(|(_, at)| at != "pop"),
        "no refusal for the population: {found:?}"
    );
}

/// FR-218-AC-42: a guard over a capture of the input checks with no listing;
/// a guard over `input.ready` refuses at that read, and no region covers the
/// guard block, the choice or the protocol.
#[trace("TC-2021", "FR-218-AC-42", "FSpec-TC-1521", "FSpec-FR-053-AC-5")]
#[test]
fn a_guard_reads_the_input_through_a_capture_and_a_direct_read_refuses_at_the_read() {
    let pick = |yes: &str, no: &str| {
        format!(
            "sequence Main {{ {TRIED} choice Pick by R visible () {{ \
             case yes when {{ {yes} }} sequence Yes {{ }} \
             case no when {{ {no} }} sequence No {{ }} }} }}"
        )
    };
    super::checks(&protocol(
        "capture ready: Boolean = input.ready;",
        "role R on M::Actor;",
        &pick("ready", "not ready"),
    ));

    let direct = protocol(
        "",
        "role R on M::Actor;",
        &pick("input.ready", "not input.ready"),
    );
    let found = located_refusals(&direct);
    assert_eq!(found.len(), 2, "{found:?}");
    for (refusal, at) in &found {
        assert_eq!(
            catalog(refusal),
            ("ill_typed", Some("operator-ineligible")),
            "{refusal:?}"
        );
        assert_eq!(at, "input.ready", "the region is the read: {refusal:?}");
    }
}

/// FR-218-AC-43: a variant of an integer type checks; a Boolean variant
/// refuses `ill_typed`/`type-mismatch` at the variant.
#[trace("TC-2022", "FR-218-AC-43", "FSpec-TC-1520", "FSpec-FR-171-AC-6")]
#[ignore = "QSL-2222: the well-founded variant type rule is not implemented yet"]
#[test]
fn a_variant_without_a_well_founded_order_refuses_at_the_variant() {
    let repeat = |variant: &str| {
        protocol(
            "",
            "role R on M::Actor;",
            &format!(
                "sequence Main {{ repeat Loop by R visible () \
                 invariant {{ true }} variant {{ {variant} }} \
                 while {{ false }} sequence Body {{ {TRIED} }} }}"
            ),
        )
    };
    super::checks(&repeat("3"));

    let found = located_refusals(&repeat("true"));
    let mismatches: Vec<&String> = found
        .iter()
        .filter(|(refusal, _)| catalog(refusal) == ("ill_typed", Some("type-mismatch")))
        .map(|(_, at)| at)
        .collect();
    assert_eq!(mismatches, ["true"], "{found:?}");
}

/// FR-884-AC-2 (TC-2508): an unproved-exhaustive `choice` refuses at its
/// header and two roles of one name refuse at each name, never at the
/// protocol declaration.
#[trace("TC-2508", "FR-884-AC-2")]
#[test]
fn a_nonexhaustive_choice_and_a_duplicate_role_refuse_at_their_subject() {
    let run = format!(
        "sequence Main {{ {TRIED} choice Pick by R visible () {{ \
         case yes when {{ true }} sequence Yes {{ }} \
         case no when {{ true }} sequence No {{ }} }} }}"
    );
    let found = located_refusals(&protocol("", "role R on M::Actor;", &run));
    let at: Vec<&String> = found
        .iter()
        .filter(|(refusal, _)| {
            catalog(refusal) == ("undefined_expression", Some("unproved-exhaustiveness"))
        })
        .map(|(_, at)| at)
        .collect();
    assert_eq!(at, ["choice Pick by R"], "{found:?}");

    let simple = format!("sequence Main {{ {TRIED} }}");
    let twice = located_refusals(&protocol(
        "",
        "role R on M::Actor;\nrole R on M::Actor;",
        &simple,
    ));
    let at: Vec<&String> = twice
        .iter()
        .filter(|(refusal, _)| {
            catalog(refusal) == ("ambiguous_declaration", Some("ambiguous-name"))
        })
        .map(|(_, at)| at)
        .collect();
    assert_eq!(at, ["R", "R"], "{twice:?}");
}

/// FR-884-AC-4 (TC-2509): two protocols of one name in one unit refuse at
/// each name; two different names check.
#[trace("TC-2509", "FR-884-AC-4")]
#[test]
fn two_protocols_of_one_name_refuse_at_each_name() {
    let run = format!("sequence Main {{ {TRIED} }}");
    let one = protocol("", "role R on M::Actor;", &run);
    let found = located_refusals(&format!("{one}\n{one}"));
    let at: Vec<&String> = found
        .iter()
        .filter(|(refusal, _)| {
            catalog(refusal) == ("ambiguous_declaration", Some("ambiguous-name"))
        })
        .map(|(_, at)| at)
        .collect();
    assert_eq!(at, ["Flow", "Flow"], "{found:?}");
    super::checks(&format!(
        "{one}\n{}",
        one.replace("protocol Flow", "protocol Drain")
    ));
}
