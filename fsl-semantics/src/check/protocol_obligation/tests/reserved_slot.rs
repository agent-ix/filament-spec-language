//! FR-488 "Reserved slot names": the checker refuses a source identifier in
//! FSpec FR-402's reserved slot-parameter form where it would name a
//! `value`/`parameter` node, at the identifier, and admits near misses
//! (TC-3082).

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::diagnostic::Code;
use ix_trace_rs::trace;

use super::{assemble, BASE};
use crate::check::{CheckCause, CheckRefusal};

/// One reserved-name refusal: the identifier, its tag and the text of the
/// region the refusal names.
#[derive(Debug, Eq, PartialEq)]
struct Found {
    name: String,
    tag: String,
    region: Option<String>,
}

/// Every refusal of `declarations`, with the reserved-name ones read out
/// beside the text of their region, and whether the unit checked.
fn reserved(declarations: &str) -> (Vec<Found>, Vec<CheckRefusal>) {
    let (text, assembled) = assemble(declarations);
    let regions = assembled.regions();
    let refusals = match assembled.check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &crate::check::test_support::ZeroRewardEvaluator,
    ) {
        Err(refusals) => refusals,
        Ok(_) => Vec::new(),
    };
    let found = refusals
        .iter()
        .filter_map(|refusal| {
            let CheckCause::ReservedSlotName(cause) = &refusal.cause else {
                return None;
            };
            assert_eq!(refusal.cause.code(), Code::AmbiguousDeclaration);
            assert_eq!(refusal.cause.cause(), Some("ambiguous-name"));
            let region = regions.refusal_region(refusal).map(|region| {
                let start = usize::try_from(region.start()).expect("an offset");
                let end = usize::try_from(region.end()).expect("an offset");
                text[start..end].to_owned()
            });
            Some(Found {
                name: cause.name.clone(),
                tag: cause.tag.clone(),
                region,
            })
        })
        .collect();
    (found, refusals)
}

fn function(parameter: &str) -> String {
    format!("function f({parameter}: Integer): Integer pure {{ {parameter} }}")
}

fn found(name: &str, tag: &str, region: &str) -> Found {
    Found {
        name: name.to_owned(),
        tag: tag.to_owned(),
        region: Some(region.to_owned()),
    }
}

/// FR-488-AC-29: a function parameter in the reserved form refuses at the
/// parameter's identifier, naming the identifier and its tag, whether the
/// escape is canonical or not.
#[trace("TC-3082", "FR-488-AC-29", "FR-402-AC-12")]
#[test]
fn a_function_parameter_in_the_reserved_slot_form_is_refused_at_the_identifier() {
    for (name, tag) in [("op_sa", "op"), ("op_s_x41", "op"), ("rcv_sa", "rcv")] {
        let (reserved, _) = reserved(&function(name));
        assert_eq!(reserved, [found(name, tag, name)], "{name}");
    }
    let (reserved, _) = reserved(&function("arg_sa_sb_sc_sd"));
    assert_eq!(
        reserved,
        [found("arg_sa_sb_sc_sd", "arg", "arg_sa_sb_sc_sd")]
    );
}

/// FR-488-AC-29: a quantifier binder in the reserved form refuses, naming
/// the identifier and the tag `slot`, at the form that declares it.
#[trace("TC-3082", "FR-488-AC-29")]
#[test]
fn a_quantifier_binder_in_the_reserved_slot_form_is_refused() {
    let source = "function g(xs: Sequence<Integer>): Boolean pure \
                  { forall(slot_sx_sy in xs: slot_sx_sy < 5) }";
    let (reserved, _) = reserved(source);
    let [found] = reserved.as_slice() else {
        panic!("one refusal, got {reserved:?}");
    };
    assert_eq!(
        (found.name.as_str(), found.tag.as_str()),
        ("slot_sx_sy", "slot")
    );
    let region = found.region.as_deref().expect("a region");
    assert!(region.contains("slot_sx_sy"), "{region}");
}

/// FR-488-AC-29: each protocol binder, `over` input, activation parameter,
/// capture, trigger, retry and recovery binder in the reserved form refuses
/// at its own identifier.
#[trace("TC-3082", "FR-488-AC-29")]
#[test]
fn protocol_binders_in_the_reserved_slot_form_are_refused_at_the_identifier() {
    for (old, new, tag) in [
        ("view", "op_sv", "op"),
        ("opened", "op_so", "op"),
        ("order", "rcv_sx", "rcv"),
        ("forward", "op_sf", "op"),
        ("target", "op_st", "op"),
        ("trigger", "op_sg", "op"),
        ("activated", "op_sa", "op"),
        ("earlier", "op_se", "op"),
        ("recovered", "op_sr", "op"),
    ] {
        let source = BASE.replace(old, new);
        let (reserved, refusals) = reserved(&source);
        assert!(
            reserved.contains(&found(new, tag, new)),
            "{old} -> {new}: {reserved:?} in {refusals:?}"
        );
    }
}

/// FR-488-AC-29, FR-402-AC-12: a name outside the form is an ordinary
/// identifier and checks.
#[trace("TC-3082", "FR-488-AC-29", "FR-402-AC-12")]
#[test]
fn a_near_miss_of_the_reserved_slot_form_is_admitted() {
    for name in [
        "slot_sv", "op_sa_sb", "rcv_s", "op_sa_x4", "op_s_xC3", "op_s_xc3", "slotted",
    ] {
        let (reserved, refusals) = reserved(&function(name));
        assert!(reserved.is_empty(), "{name}: {reserved:?}");
        assert!(refusals.is_empty(), "{name}: {refusals:?}");
    }
}
