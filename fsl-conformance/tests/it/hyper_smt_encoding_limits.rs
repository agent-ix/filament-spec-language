// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315-AC-40's hyper-query size boundary and identity behavior.

use filament_exact::Meter;
use fsl_analyze::model_check::{ModelCheckRequest, ModelCheckRequestIdentity};
use fsl_eval::smt::{Measure, Refusal, SmtEncodingRefusal};
use fsl_replay::{
    encode_hyper_query, HyperProductIdentity, HyperQueryRequest, SmtEncodingLimits, SmtQueryKind,
};
use ix_trace_rs::trace;

use crate::vault_world::{non_interference, World, UNLIMITED};

#[test]
fn hyper_encoding_size_refusal_keeps_the_default_product_identity() {
    let world = World::new(&["v"], false, &non_interference("", ""));
    let subjects = [world.subject()];
    let instance = vec![("v".to_owned(), "v".to_owned())];

    let mut default_meter = Meter::new(UNLIMITED);
    let default_identity = HyperProductIdentity::of(&HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance: &instance,
        meter: &mut default_meter,
        encoding: SmtEncodingLimits::default(),
    })
    .expect("the default request has a product identity");

    let mut limited_meter = Meter::new(UNLIMITED);
    let request = HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance: &instance,
        meter: &mut limited_meter,
        encoding: SmtEncodingLimits {
            max_constants: 7,
            ..SmtEncodingLimits::default()
        },
    };
    let limited_identity = HyperProductIdentity::of(&request)
        .expect("encoding limits do not change the product identity");
    assert_eq!(limited_identity, default_identity);
    assert!(matches!(
        encode_hyper_query(request, SmtQueryKind::InductionBase { depth: 1 }),
        Err(SmtEncodingRefusal::Refused(Refusal::TooLarge {
            measure: Measure::Constants,
            value: 8,
            limit: 7,
            setting: "smt_encoding.max_constants",
        }))
    ));
}

#[trace("TC-1250", "FR-315-AC-42")]
#[test]
fn model_check_request_smt_limits_have_the_specified_identity_effects() {
    let world = World::new(&["v"], false, &non_interference("", ""));
    let request = |smt_encoding, smt_run| ModelCheckRequest {
        subject: world.subject(),
        temporal_clock_selections: &[],
        item: fsl_replay::ModelCheckItem::DeadlockFreedom,
        max_depth: None,
        limits: fsl_replay::ModelCheckLimits::default(),
        reductions: fsl_route::reductions::Reductions::default(),
        smt_encoding,
        smt_run,
    };

    let defaults = request(
        fsl_replay::SmtEncodingLimits::default(),
        fsl_replay::SmtRunLimits::default(),
    );
    assert_eq!(
        defaults.smt_encoding,
        fsl_replay::SmtEncodingLimits::default()
    );
    assert_eq!(defaults.smt_run, fsl_replay::SmtRunLimits::default());
    assert_eq!(
        defaults.certificate_request().smt_encoding,
        defaults.smt_encoding,
        "the certificate checker receives the run's encoding limits"
    );
    let default_identity: ModelCheckRequestIdentity = defaults.identity();

    let constants_7 = request(
        fsl_replay::SmtEncodingLimits {
            max_constants: 7,
            ..fsl_replay::SmtEncodingLimits::default()
        },
        fsl_replay::SmtRunLimits::default(),
    );
    let constants_8 = request(
        fsl_replay::SmtEncodingLimits {
            max_constants: 8,
            ..fsl_replay::SmtEncodingLimits::default()
        },
        fsl_replay::SmtRunLimits::default(),
    );
    assert_ne!(constants_7.identity(), constants_8.identity());
    assert_eq!(
        constants_7.certificate_request().smt_encoding,
        constants_7.smt_encoding
    );

    let time_1 = request(
        fsl_replay::SmtEncodingLimits::default(),
        fsl_replay::SmtRunLimits {
            max_solver_time: 1,
            ..fsl_replay::SmtRunLimits::default()
        },
    );
    assert_eq!(default_identity, time_1.identity());

    let steps_1 = request(
        fsl_replay::SmtEncodingLimits::default(),
        fsl_replay::SmtRunLimits {
            max_solver_steps: 1,
            ..fsl_replay::SmtRunLimits::default()
        },
    );
    assert_ne!(default_identity, steps_1.identity());
}
