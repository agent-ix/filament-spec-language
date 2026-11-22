// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit-limit qualification uses actual source, checked bodies and native I2.

use super::*;
use filament_checked::node::{Arithmetic, NodeKind};
use filament_contract_model::{
    CheckedBodyLocationOrigin, CheckedKeyOrdinal, CheckedPackageReadLimits, CheckedPackageV2,
    CheckedPackageV2BudgetedReadResult, NativeJoinBudget,
};
use filament_semantic_value::location::ExpressionLocation;

#[derive(Clone, Copy, Debug)]
enum Family {
    Sum,
    Lets,
}

fn source(family: Family, count: usize) -> String {
    let name = if matches!(family, Family::Sum) {
        "d"
    } else {
        "e"
    };
    let mut text = format!("{HEADER}function {name} using v(x: Integer): Integer pure {{ ");
    match family {
        Family::Sum => {
            text.push_str(&"x + ".repeat(count));
            text.push('x');
        }
        Family::Lets => {
            for level in 1..=count {
                let previous = if level == 1 {
                    "x".to_owned()
                } else {
                    format!("b{}", level - 1)
                };
                write!(&mut text, "let b{level} = {previous} + 1 in ").unwrap();
            }
            write!(&mut text, "b{count}").unwrap();
        }
    }
    text.push_str(" }\n");
    text
}

/// Fixed coefficients cover these two closed source families, not arbitrary FSL.
/// Each checked node has at most two children. Every emitted reference is a
/// flat key (fixed namespace/operation, a u64 ordinal, no nested operand key).
/// An 8 KiB allowance per checked/semantic node or operand edge covers 32
/// references of 256 bytes including field punctuation, type and location rows.
/// Authored identifier/source text, including JSON's at-most-sixfold escaping,
/// gets 128 bytes per source byte; fixed library/envelope records get 1 MiB.
/// The reader's fixed decoding/canonical/validation passes get 128 charges per
/// allowed wire byte; typed storage gets 16 bytes per allowed wire byte. The
/// 64 MiB constant covers the operation catalog and fixed setup in both meters.
/// The cursor visits fixed-width terms/references and delta entries; 1024 per
/// shape item covers setup, pending-vector growth, visits and path growth.
/// These coefficients are independent of count/depth and measured output.
#[derive(Debug)]
struct Ceilings {
    bytes: u64,
    read_work: u64,
    storage: u64,
    locations: u64,
    shape: u64,
}

fn ceilings(
    source_bytes: usize,
    body_nodes: usize,
    semantic_nodes: usize,
    edges: usize,
) -> Ceilings {
    let shape = u64::try_from(body_nodes + semantic_nodes + edges).unwrap();
    let bytes = (1 << 20) + 8192 * shape + 128 * u64::try_from(source_bytes).unwrap();
    Ceilings {
        bytes,
        read_work: (64 << 20) + 128 * bytes + 1024 * shape,
        storage: (64 << 20) + 16 * bytes,
        locations: (1 << 20) + 1024 * shape,
        shape,
    }
}

fn qualify(family: Family, count: usize, require_default_denial: bool) {
    let text = source(family, count);
    // Source-derived finite S1/S3 ceilings are fixed before checking, too.
    let input = u64::try_from(text.len()).unwrap();
    let graph = deep_declarations(&text)
        .check(
            CheckMode::Linked,
            CheckingLimits::new(input * 64 + 1_000_000)
                .with_input_bytes(input * 64 + 1_000_000)
                .with_work_budget(input * 4096 + 64_000_000),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|errors| panic!("{family:?}: checking failed ({} refusals)", errors.len()));
    let package = CheckedPackage::link(graph);
    let body = package.graph().function_state(0).unwrap().body.body();
    let mut pending = vec![body.root()];
    let mut terms = Vec::with_capacity(count);
    let mut edges = 0;
    let mut bindings = 0;
    let mut locals = 0;
    while let Some(node) = pending.pop() {
        let children = node.children();
        edges += children.len();
        match node.kind() {
            NodeKind::Arithmetic(Arithmetic::Add, ..) => terms.push(node),
            NodeKind::Let {
                slot,
                value,
                body: inner,
                ..
            } => {
                bindings += 1;
                assert_eq!(*slot, bindings, "each binding owns its next local slot");
                let value = node.at(*value);
                let NodeKind::Arithmetic(Arithmetic::Add, left, right) = value.kind() else {
                    panic!("let value must increment")
                };
                assert!(
                    matches!(node.at(*left).kind(), NodeKind::Local(previous) if *previous + 1 == *slot)
                );
                assert!(
                    matches!(node.at(*right).kind(), NodeKind::Literal(literal) if matches!(&literal.0, filament_exact::Value::Integer(value) if value == &filament_exact::Integer::from(1_i64)))
                );
                if bindings == count {
                    assert!(
                        matches!(node.at(*inner).kind(), NodeKind::Local(last) if last == slot),
                        "final body reads the last binder"
                    );
                }
            }
            NodeKind::Local(_) => locals += 1,
            NodeKind::Literal(_) => {}
            _ => panic!("{family:?}: unexpected checked node kind"),
        }
        pending.extend(children.into_iter().rev());
    }
    assert_eq!(
        terms.len(),
        count,
        "one checked addition per requested step"
    );
    assert_eq!(
        locals,
        count + 1,
        "all operands and final binder read remain"
    );
    assert_eq!(
        bindings,
        if matches!(family, Family::Lets) {
            count
        } else {
            0
        }
    );
    let limits = ceilings(
        text.len(),
        body.len(),
        package.graph().semantic_graph().nodes().count(),
        edges,
    );
    eprintln!("PREDECLARED family={family:?} count={count} source_bytes={} body_nodes={} edges={edges} shape={} bytes={} read_work={} storage={} location_work={}", text.len(), body.len(), limits.shape, limits.bytes, limits.read_work, limits.storage, limits.locations);
    let emission = crate::emit::emit_checked(&package, crate::EmitLimits::new(limits.bytes))
        .unwrap_or_else(|failure| {
            panic!("{family:?}: emission failed with code {:?}", failure.code())
        });
    let actual_bytes = emission.package().bytes().len();
    assert!(u64::try_from(actual_bytes).unwrap() <= limits.bytes);
    eprintln!("EMITTED family={family:?} bytes={actual_bytes}");
    let mut budget = NativeJoinBudget::new(limits.read_work, limits.storage)
        .with_read_storage_limit(limits.storage);
    let read_limits = CheckedPackageReadLimits {
        bytes: limits.bytes,
        nodes: limits.shape * 32 + 4096,
        edges: limits.shape * 64 + 4096,
        occurrences: limits.shape * 64 + 4096,
        diagnostics: limits.shape + 4096,
        work: limits.read_work,
    };
    let admitted = match CheckedPackageV2::read_with_budget(
        emission.package().bytes(),
        read_limits,
        emission.evidence(),
        &mut budget,
    ) {
        CheckedPackageV2BudgetedReadResult::Admitted(package) => package,
        CheckedPackageV2BudgetedReadResult::Refused(refusal) => panic!(
            "{family:?}: strict read refused code={:?} cause={:?}",
            refusal.code, refusal.cause
        ),
        CheckedPackageV2BudgetedReadResult::Incomplete(failure) => {
            panic!("{family:?}: strict read incomplete {failure:?}")
        }
        CheckedPackageV2BudgetedReadResult::BudgetIncomplete(failure) => {
            panic!("{family:?}: strict read budget {failure:?}")
        }
        CheckedPackageV2BudgetedReadResult::Exhausted(failure) => {
            panic!("{family:?}: strict read exhausted {failure:?}")
        }
        CheckedPackageV2BudgetedReadResult::CatalogUnavailable(_) => {
            panic!("{family:?}: catalog unavailable")
        }
    };
    let charged = budget.consumed();
    let read_storage = budget.read_storage_consumed();
    assert!(charged.work <= limits.read_work && charged.storage <= limits.storage);
    assert!(read_storage <= limits.storage);
    let rows = &admitted.checked_term_data().unwrap().body_locations;
    assert_eq!(rows.len(), count);
    let entries: usize = rows.iter().map(|row| 1 + row.location.down.len()).sum();
    assert_eq!(
        entries,
        match family {
            Family::Sum => 2 * count - 1,
            Family::Lets => 3 * count - 1,
        }
    );
    let CheckedBodyLocationOrigin::FunctionBody {
        function,
        name,
        index,
    } = &rows[0].location.origin;
    assert_eq!(
        name.as_ref(),
        if matches!(family, Family::Sum) {
            "d"
        } else {
            "e"
        }
    );
    assert_eq!(*index, 0);
    let mut cursor = admitted.body_locations(function, limits.locations).unwrap();
    let mut expected = Vec::with_capacity(count);
    let mut previous: Option<&ExpressionLocation> = None;
    for (visit, term) in terms.iter().enumerate() {
        let location = term.location();
        // Prove the checker's full path by induction over shared persistent
        // ancestors, without expanding all checked paths. Cursor equality below
        // compares the entire current path, not merely its last component.
        match family {
            Family::Sum => {
                if let Some(previous) = previous {
                    assert!(location.parent().unwrap().shares_chain_with(previous));
                    assert_eq!(location.last_index(), Some(0));
                    expected.push(CheckedKeyOrdinal::Safe(0));
                } else {
                    assert_eq!(location.depth(), 0);
                }
            }
            Family::Lets => {
                let parent = location.parent().unwrap();
                assert_eq!(location.last_index(), Some(0));
                if let Some(previous) = previous {
                    assert_eq!(parent.last_index(), Some(1));
                    assert!(parent
                        .parent()
                        .unwrap()
                        .shares_chain_with(&previous.parent().unwrap()));
                    expected.pop();
                    expected.push(CheckedKeyOrdinal::Safe(1));
                } else {
                    assert_eq!(parent.depth(), 0);
                }
                expected.push(CheckedKeyOrdinal::Safe(0));
            }
        }
        previous = Some(location);
        let view = cursor
            .next_location()
            .unwrap()
            .expect("one lent path per addition");
        assert!(
            view.path == expected,
            "{family:?}: whole checker path differs at visit {visit}"
        );
        assert_eq!(view.row.term_ordinal, 0);
        let (up, down): (u64, &[CheckedKeyOrdinal]) = match (family, visit) {
            (Family::Sum, 0) => (0, &[]),
            (Family::Sum, _) => (0, &[CheckedKeyOrdinal::Safe(0)]),
            (Family::Lets, 0) => (0, &[CheckedKeyOrdinal::Safe(0)]),
            (Family::Lets, _) => (1, &[CheckedKeyOrdinal::Safe(1), CheckedKeyOrdinal::Safe(0)]),
        };
        assert_eq!(view.row.location.up, up);
        assert_eq!(view.row.location.down, down);
    }
    assert!(cursor.next_location().unwrap().is_none());
    let location_work = cursor.work_consumed();
    assert!(location_work <= limits.locations);
    eprintln!("MEASURED family={family:?} count={count} bytes={actual_bytes} rows={} relative_entries={entries} strict_work={} strict_read_storage={read_storage} logical_storage={} location_work={location_work}", rows.len(), charged.work, charged.storage);
    drop(cursor);
    drop(admitted);
    drop(emission);
    if require_default_denial {
        const DEFAULT: u64 = 16_777_216;
        let failure = match crate::emit::emit_checked(&package, crate::EmitLimits::new(DEFAULT)) {
            Err(failure) => failure,
            Ok(_) => panic!("{family:?}: published default unexpectedly emitted"),
        };
        let EmitRefusal::OutputLimit(limit) = failure else {
            panic!("{family:?}: expected OutputLimit")
        };
        assert_eq!(limit.setting(), fsl_foundation::Setting::EmitOutputBytes);
        assert_eq!(limit.configured_bound(), DEFAULT);
        assert_eq!(limit.actual(), u128::from(DEFAULT) + 1);
        eprintln!(
            "DEFAULT_DENIAL family={family:?} configured={DEFAULT} first_unpaid={}",
            limit.actual()
        );
    }
}

fn on_small_stack(count: usize, default_denial: bool) {
    // Each thread drops its source, graph, wire and admitted package before the
    // next family begins. There are never two complete deep artifacts resident.
    for family in [Family::Sum, Family::Lets] {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(move || qualify(family, count, default_denial))
            .unwrap()
            .join()
            .unwrap_or_else(|_| panic!("{family:?}: qualification failed on 512 KiB stack"));
    }
}

#[trace("FR-531-AC-41", "TC-3133", "FSpec-FR-322-AC-510")]
#[test]
fn small_explicit_limit_families_check_actual_paths_and_relative_rows() {
    on_small_stack(8, false);
}

#[trace("FR-531-AC-41", "TC-3133")]
#[test]
fn row_preflight_names_the_first_unpaid_byte_and_legacy_whole_path_refuses() {
    let (_, package) = super::super::package_from_text(
        "function d using v(x: Integer): Integer pure { x + x + x }\n",
    );
    let failure = crate::emit::emit_checked(&package, crate::EmitLimits::new(1))
        .expect_err("a row cannot fit in one byte");
    let EmitRefusal::OutputLimit(limit) = failure else {
        panic!("expected OutputLimit")
    };
    assert_eq!(limit.configured_bound(), 1);
    assert_eq!(
        limit.actual(),
        2,
        "preflight stops at the first unpaid byte"
    );
    let emitted = crate::emit::emit_checked(&package, crate::EmitLimits::new(1 << 20)).unwrap();
    let mut wire: Value = serde_json::from_slice(emitted.package().bytes()).unwrap();
    let location = wire["checked_term_data"]["body_locations"][0]["location"]
        .as_object_mut()
        .unwrap();
    location.remove("up");
    location.remove("down");
    location.insert("path".to_owned(), json!([]));
    let bytes = super::super::wire_bytes(&wire).unwrap();
    let result = CheckedPackageV2::read_with_budget(
        &bytes,
        CheckedPackageReadLimits::bounded(),
        emitted.evidence(),
        &mut NativeJoinBudget::new(100_000_000, 100_000_000).with_read_storage_limit(100_000_000),
    );
    let CheckedPackageV2BudgetedReadResult::Refused(refusal) = result else {
        panic!("legacy whole path must refuse")
    };
    assert_eq!(
        refusal.code,
        filament_contract_model::CheckedPackageRefusalCode::MalformedWire
    );
    assert_eq!(
        refusal.loci[0].path.as_ref().unwrap().as_str(),
        "/checked_term_data/body_locations/0/location"
    );
}

/// The single explicit large qualification invocation; never select by module.
#[trace("FR-531-AC-41", "TC-3133", "FSpec-FR-322-AC-510")]
#[trace("TC-738", "FR-264-AC-8")]
#[test]
fn two_100000_step_families_measure_complete_wire_and_native_work() {
    on_small_stack(100_000, true);
}
