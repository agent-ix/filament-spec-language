// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 shared equality admission on producer and bytes-only paths.

use super::*;
use crate::equality_file::EqualityDecisionFailure;
use crate::reader_protocol_projection::{admit_equality_decisions, ProjectionSource};
use crate::EvaluationWitnessSupplier;
use crate::{BarePackage, BareSupply, NativeJoinFailure, ReadEqualityRows};
use filament_contract_model::{
    AdmittedEqualityReadErrorCode as Code, NativeJoinBudget, NativeJoinChargeKind,
};

fn emission() -> Emission {
    converted_equality::emitted(
        "function same using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }",
    )
}

#[trace("TC-1331", "FR-531-AC-28")]
#[test]
fn producer_and_bytes_equality_share_exact_owner_charges_and_denials() {
    let emission = emission();
    let Read::Verified { admitted, .. } = read_back(&emission) else {
        panic!("native reader admits")
    };
    let [row] = emission.equality_producer_identity_rows() else {
        panic!("one equality")
    };
    let witness = emission
        .equality_witness(&admitted, &mut NativeJoinBudget::new(1_000_000, 1_000_000))
        .expect("budgeted witness");
    let bytes = witness
        .to_bytes(&admitted)
        .expect("authentic equality bytes");
    let read = ReadEqualityRows::read(&bytes, &admitted).expect("bytes read");
    let mut materialize = NativeJoinBudget::new(1_000_000, 1_000_000);
    emission
        .equality_rows(&mut materialize)
        .expect("actual producer materialization");
    let prefix = materialize.consumed();
    let mut measure = NativeJoinBudget::new(1_000_000, 1_000_000);
    let expected = emission
        .admitted_equality(
            &admitted,
            row.checked_node(),
            row.occurrence(),
            &mut measure,
        )
        .expect("producer decision");
    assert_eq!(
        expected.schedule(),
        filament_semantic_value::declaration::EqualitySchedule::Text
    );
    assert_eq!(
        expected.operator(),
        filament_semantic_value::declaration::EqualityOperator::Equal
    );
    let total = measure.consumed();
    let owner = measure
        .last_success_delta()
        .expect("the owner reader records its own delta");
    assert_eq!(total.work, prefix.work + owner.work);
    assert_eq!(total.storage, prefix.storage + owner.storage);
    let mut exact = NativeJoinBudget::new(owner.work, owner.storage);
    assert_eq!(
        read.decision(&admitted, row.checked_node(), &mut exact)
            .unwrap_or_else(|failure| match failure {
                EqualityDecisionFailure::Witness(error) =>
                    panic!("exact bytes witness refused: {error:?}"),
                EqualityDecisionFailure::Exhausted(error) =>
                    panic!("exact bytes budget refused: {error:?}"),
            }),
        expected
    );
    assert_eq!(exact.consumed(), owner);
    for (kind, code) in [
        (NativeJoinChargeKind::Work, Code::WorkLimit),
        (NativeJoinChargeKind::Storage, Code::StorageLimit),
    ] {
        let (work, storage, owner_work, owner_storage) = match kind {
            NativeJoinChargeKind::Work => {
                (total.work - 1, total.storage, owner.work - 1, owner.storage)
            }
            NativeJoinChargeKind::Storage => {
                (total.work, total.storage - 1, owner.work, owner.storage - 1)
            }
        };
        let mut producer = NativeJoinBudget::new(work, storage);
        let failure = emission
            .admitted_equality(
                &admitted,
                row.checked_node(),
                row.occurrence(),
                &mut producer,
            )
            .expect_err("one short refuses the native reader after the admitted adapter prefix");
        let NativeJoinFailure::Equality(native) = failure.native() else {
            panic!("row materialization fits; the actual owner read refuses")
        };
        assert_eq!(native.code, code);
        let mut bytes = NativeJoinBudget::new(owner_work, owner_storage);
        let Err(EqualityDecisionFailure::Exhausted(native)) =
            read.decision(&admitted, row.checked_node(), &mut bytes)
        else {
            panic!("one short is the original native exhaustion")
        };
        assert_eq!(native.code, code);
        assert_eq!(
            producer.consumed().work,
            prefix.work + bytes.consumed().work
        );
        assert_eq!(
            producer.consumed().storage,
            prefix.storage + bytes.consumed().storage
        );
        assert_eq!(producer.first_unpaid(), bytes.first_unpaid());
    }
    let mut shared = NativeJoinBudget::new(total.work * 2 - 1, total.storage * 2);
    emission
        .admitted_equality(&admitted, row.checked_node(), row.occurrence(), &mut shared)
        .expect("first read");
    let first = shared.consumed();
    let error = emission
        .admitted_equality(&admitted, row.checked_node(), row.occurrence(), &mut shared)
        .expect_err("second read spends the same allowance");
    let NativeJoinFailure::Equality(native) = error.native() else {
        panic!("second row prefix fits before the owner denial")
    };
    assert_eq!(native.code, Code::WorkLimit);
    assert!(shared.consumed().work >= first.work);
    assert_eq!(
        shared.first_unpaid().expect("unpaid work").kind,
        NativeJoinChargeKind::Work
    );
}

#[trace("TC-1331", "FR-531-AC-28")]
#[test]
fn bare_admission_retains_equality_and_reports_its_denial_after_projection() {
    let emission = emission();
    let Read::Verified { admitted, .. } = read_back(&emission) else {
        panic!("native reader admits")
    };
    let identities = emission
        .identity_rows(&admitted, &mut NativeJoinBudget::new(1_000_000, 1_000_000))
        .expect("original identities");
    let witness = emission
        .equality_witness(&admitted, &mut NativeJoinBudget::new(1_000_000, 1_000_000))
        .expect("budgeted witness");
    let bytes = witness.to_bytes(&admitted).expect("owner bytes");
    let equality = ReadEqualityRows::read(&bytes, &admitted).expect("read owner bytes");
    let supply = BareSupply {
        identities: &identities,
        equality: Some(&equality),
    };
    let mut measure = NativeJoinBudget::new(1_000_000, 1_000_000);
    let bare = BarePackage::new(&admitted)
        .with_supply(supply, &mut measure)
        .expect("full admission");
    let full = measure.consumed();
    let [row] = equality.applications() else {
        panic!("one original row")
    };
    let expected = bare
        .equality_decisions()
        .get(&row.application)
        .expect("retained equality")
        .as_ref()
        .expect("decision")
        .clone();
    assert_eq!(
        expected.schedule(),
        filament_semantic_value::declaration::EqualitySchedule::Text
    );
    assert!(
        equality.source_units().is_empty(),
        "no source-unit join in this fixture"
    );
    // Measure the same identity and per-node projection prefix that the bare
    // entry charges, ending immediately after the native equality read.
    let mut equality_measure = NativeJoinBudget::new(1_000_000, 1_000_000);
    let selected = identities
        .selected(&admitted, &mut equality_measure)
        .expect("the authentic identity rows join on the equality measurement budget");
    project_every_node(&admitted, &selected, &mut equality_measure);
    assert_eq!(
        equality_measure.first_unpaid(),
        None,
        "the entire projection prefix is paid"
    );
    let prefix = equality_measure.consumed();
    let decisions = admit_equality_decisions(
        &admitted,
        ProjectionSource::Bytes {
            equality: Some(&equality),
        },
        &mut equality_measure,
    )
    .expect("the equality phase admits on the same finite budget");
    let need = equality_measure.consumed();
    assert!(need.work > prefix.work && need.storage > prefix.storage);
    assert!(need.work < full.work && need.storage < full.storage);
    assert_eq!(
        decisions
            .get(&row.application)
            .expect("the measured phase retains equality")
            .as_ref()
            .expect("the native equality decision"),
        &expected
    );
    bare.world().expect("world uses admitted projections");
    assert_eq!(measure.consumed(), full);
    let mut exact = NativeJoinBudget::new(full.work, full.storage);
    let exact_bare = BarePackage::new(&admitted)
        .with_supply(supply, &mut exact)
        .expect("exact complete pass");
    assert_eq!(
        exact_bare
            .equality_decisions()
            .get(&row.application)
            .expect("retained")
            .as_ref()
            .expect("decision"),
        &expected
    );
    assert_eq!(exact.consumed(), full);
    for (work, storage, kind) in [
        (full.work - 1, full.storage, NativeJoinChargeKind::Work),
        (full.work, full.storage - 1, NativeJoinChargeKind::Storage),
    ] {
        let mut budget = NativeJoinBudget::new(work, storage);
        let error = BarePackage::new(&admitted)
            .with_supply(supply, &mut budget)
            .expect_err("one short of the complete admission pass");
        let NativeJoinFailure::Rows(native) = error.native() else {
            panic!("source enum index owns the final denial: {error:?}")
        };
        let (code, setting, bound, consumed) = match kind {
            NativeJoinChargeKind::Work => (
                crate::NativeRowErrorCode::WorkLimit,
                fsl_foundation::Setting::I2WorkUnits,
                work,
                budget.consumed().work,
            ),
            NativeJoinChargeKind::Storage => (
                crate::NativeRowErrorCode::StorageLimit,
                fsl_foundation::Setting::I2NativeJoinStorage,
                storage,
                budget.consumed().storage,
            ),
        };
        assert_eq!(native.code(), code);
        assert_eq!(native.configured_bound(), Some(bound));
        assert_eq!(native.denied(), budget.first_unpaid());
        assert_eq!(native.locus(), budget.first_unpaid_locus());
        let locus = native.locus().expect("original source enum index locus");
        assert_eq!(
            locus.source,
            filament_contract_model::NativeJoinLocusSource::AdapterRow(0)
        );
        assert_eq!(locus.form, "source_enum_catalog_index");
        let unpaid = budget.first_unpaid().expect("original unpaid charge");
        assert_eq!(unpaid.kind, kind);
        assert_eq!(unpaid.amount, 1);
        assert_eq!(budget.consumed().work, work);
        assert_eq!(budget.consumed().storage, full.storage - 1);
        let diagnostic = error.limit().expect("original limit diagnostic");
        assert_eq!(diagnostic.setting(), setting);
        assert_eq!(diagnostic.configured_bound(), bound);
        assert_eq!(
            diagnostic.actual(),
            u128::from(consumed) + unpaid.amount
        );
    }
    for (work, storage, kind, code) in [
        (
            need.work - 1,
            full.storage,
            NativeJoinChargeKind::Work,
            Code::WorkLimit,
        ),
        (
            full.work,
            need.storage - 1,
            NativeJoinChargeKind::Storage,
            Code::StorageLimit,
        ),
    ] {
        let mut budget = NativeJoinBudget::new(work, storage);
        let error = BarePackage::new(&admitted)
            .with_supply(supply, &mut budget)
            .expect_err("one short in equality");
        let NativeJoinFailure::Equality(native) = error.native() else {
            panic!(
                "actual equality cause after identity/type prefix: native={:?}, limit={:?}, \
                 ceilings=({work}, {storage}), consumed={:?}, first_unpaid={:?}, \
                 first_unpaid_locus={:?}, full={full:?}, prefix={prefix:?}, need={need:?}",
                error.native(),
                error.limit(),
                budget.consumed(),
                budget.first_unpaid(),
                budget.first_unpaid_locus(),
            )
        };
        assert_eq!(native.code, code, "native={native:?}, budget={budget:?}");
        let diagnostic = error
            .limit()
            .unwrap_or_else(|| panic!("bare entry retains the stage diagnostic: {error:?}"));
        let (bound, consumed) = if kind == NativeJoinChargeKind::Work {
            (work, budget.consumed().work)
        } else {
            (storage, budget.consumed().storage)
        };
        assert_eq!(diagnostic.configured_bound(), bound, "{error:?}");
        assert_eq!(
            diagnostic.actual(),
            u128::from(consumed) + budget.first_unpaid().expect("unpaid").amount,
            "native={native:?}, budget={budget:?}"
        );
        assert_eq!(
            native.consumed_work,
            budget.consumed().work,
            "native={native:?}, budget={budget:?}"
        );
        assert!(
            budget.consumed().work > 0 && budget.consumed().storage > 0,
            "native={native:?}, budget={budget:?}"
        );
        assert_eq!(
            budget.first_unpaid().expect("first unpaid").kind,
            kind,
            "native={native:?}, budget={budget:?}"
        );
    }
}

#[trace("TC-1331", "FR-531-AC-28")]
#[test]
fn source_evaluation_view_preserves_the_equality_denial_and_exact_stage_limit() {
    use filament_semantic_value::catalog::CatalogCoded;
    let package = CheckedPackage::link(converted_equality::checked_graph(
        "function same using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }",
    ));
    let emission = emit(&package);
    let Read::Verified { admitted, .. } = read_back(&emission) else {
        panic!("owner admits")
    };
    let mut measure = NativeJoinBudget::new(1_000_000, 1_000_000);
    // The pass view creation owns: the supplier's join, the reader's
    // validation of what it supplied, then the equality admission.
    let rebound = crate::SourceWitnessSupplier::new(&emission)
        .supply(&admitted, &mut measure)
        .expect("the source supplier joins its rows")
        .identities
        .expect("the source supplier holds identities")
        .validate_and_rebind(&admitted, &mut measure)
        .expect("the reader validates the supplied identities");
    project_every_node(&admitted, &rebound, &mut measure);
    admit_equality_decisions(
        &admitted,
        ProjectionSource::Source {
            emission: &emission,
        },
        &mut measure,
    )
    .expect("same complete native pass");
    let need = measure.consumed();
    let domains = BTreeMap::new();
    let limits = |work, storage| {
        V2ReadLimits::fixture()
            .with_work(work)
            .with_storage(storage)
    };
    let read = crate::read_evaluation_emission(
        &package,
        &emission,
        library(),
        &domains,
        limits(need.work, need.storage),
        crate::EmitLimits::fixture(),
        crate::SourceWitnessSupplier::new(&emission),
    )
    .expect("public source reader admits");
    read.evaluation_view()
        .expect("the exact native allowance admits the public view");
    for (work, storage, kind, code, setting) in [
        (
            need.work - 1,
            need.storage,
            NativeJoinChargeKind::Work,
            Code::WorkLimit,
            "i2.work_units",
        ),
        (
            need.work,
            need.storage - 1,
            NativeJoinChargeKind::Storage,
            Code::StorageLimit,
            "i2.native_join_storage",
        ),
    ] {
        // The same original row-production prefix that evaluation_view owns.
        let mut expected_budget = NativeJoinBudget::new(work, storage);
        let rebound = crate::SourceWitnessSupplier::new(&emission)
            .supply(&admitted, &mut expected_budget)
            .expect("prefix admits")
            .identities
            .expect("the source supplier holds identities")
            .validate_and_rebind(&admitted, &mut expected_budget)
            .expect("the reader validates the supplied identities");
        project_every_node(&admitted, &rebound, &mut expected_budget);
        let error = admit_equality_decisions(
            &admitted,
            ProjectionSource::Source {
                emission: &emission,
            },
            &mut expected_budget,
        )
        .expect_err("equality is the denied tail");
        let NativeJoinFailure::Equality(expected) = &error else {
            panic!("owner equality denial")
        };
        assert_eq!(expected.code, code);
        let unpaid = expected_budget
            .first_unpaid()
            .expect("original first unpaid charge");
        assert_eq!(unpaid.kind, kind);
        let bound = if kind == NativeJoinChargeKind::Work {
            work
        } else {
            storage
        };
        let consumed = if kind == NativeJoinChargeKind::Work {
            expected_budget.consumed().work
        } else {
            expected_budget.consumed().storage
        };
        let short = crate::read_evaluation_emission(
            &package,
            &emission,
            library(),
            &domains,
            limits(work, storage),
            crate::EmitLimits::fixture(),
            crate::SourceWitnessSupplier::new(&emission),
        )
        .expect("I2 read still admits");
        let Err(crate::ViewFailure::Exhausted(denial)) = short.evaluation_view() else {
            panic!("public view must expose exhaustion")
        };
        assert_eq!(denial.native, error.clone());
        assert_eq!(denial.limit.configured_bound(), bound);
        assert_eq!(denial.limit.setting().name(), setting);
        assert_eq!(denial.limit.actual(), u128::from(consumed) + unpaid.amount);
        assert_eq!(denial.catalog_code().code(), "stage_limit_exceeded");
    }
}

/// The per-node projection that view creation charges after validation.
fn project_every_node(
    admitted: &filament_contract_model::CheckedPackageV2,
    identities: &filament_contract_model::SelectedExactIdentities,
    budget: &mut NativeJoinBudget,
) {
    for node in &admitted.resolved_graph().nodes {
        let _ = admitted.project_exact_type(&node.node_id, identities, budget);
    }
}
