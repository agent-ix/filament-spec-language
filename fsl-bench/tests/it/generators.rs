// SPDX-License-Identifier: AGPL-3.0-or-later
use filament_exact::Cancel;
use fsl_bench::prop::{self, value};
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{self, DependencyInput, FrontEndFailure, ParseRequest, SpineLimits};
use proptest::prelude::TestCaseError;
use std::collections::{BTreeMap, BTreeSet};

fn check(source: &str) -> Result<(), FrontEndFailure> {
    let cancel = Cancel::new();
    let identity = SourceIdentity::new("agent-ix", "property-generators");
    let limits = SpineLimits::default();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "generated.native",
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )?
    .into_value();
    let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &cancel)?.into_value();
    spine::check(
        &parsed,
        "property-generators".into(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )?;
    Ok(())
}

/// Provenance: QSL-2054. Each generated function reaches the real Linked checker.
#[test]
fn value_generator_output_parses_and_checks() {
    prop::run(
        "typed expressions",
        prop::config(1000),
        &value::expressions(),
        |input| {
            let source = input.source();
            check(&source).map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))
        },
    )
    .unwrap();
}

/// Provenance: QSL-2054. Coverage is measured from the parsed CST, never labels.
#[test]
fn grammar_unit_generator_covers_every_production() {
    let start = std::time::Instant::now();
    let cases = fsl_cst::grammar::generate::covering_derivations();
    let mut covered = BTreeSet::new();
    let mut refused = Vec::new();
    for case in cases {
        let parsed = fsl_bench::parse::parse(&case.source).unwrap();
        if !parsed.diagnostics().is_empty() {
            refused.push(format!(
                "{:?}: {:?}\n{}",
                case.target,
                parsed.diagnostics(),
                case.source
            ));
        } else {
            covered.extend(parsed.cst().nodes().iter().map(|node| node.production()));
        }
    }
    let missing = fsl_cst::Production::all()
        .iter()
        .filter(|production| !covered.contains(production))
        .collect::<Vec<_>>();
    eprintln!("property suite grammar: {:?}", start.elapsed());
    assert!(refused.is_empty(), "{}", refused.join("\n"));
    assert!(
        missing.is_empty(),
        "missing actual parser productions: {missing:?}"
    );
}

/// Provenance: QSL-2054. Refusals must be typed and located at the authored expression.
#[test]
fn near_miss_generator_output_is_refused() {
    prop::run(
        "near misses",
        prop::config(256),
        &prop::near_miss::near_misses(),
        |input| {
            let failure = check(&input.source).expect_err("a near miss must never be accepted");
            let fsl_foundation::diagnostic::StageFailure::Refused(refusal) = failure else {
                return Err(TestCaseError::fail(format!(
                    "expected typed refusal, got {failure:?}"
                )));
            };
            proptest::prop_assert_eq!(refusal.code(), input.code, "{}", input.source);
            proptest::prop_assert_eq!(
                refusal.cause(),
                Some(input.cause),
                "{:?}\n{}",
                refusal,
                input.source
            );
            let region = refusal
                .region()
                .ok_or_else(|| TestCaseError::fail(format!("missing location: {refusal:?}")))?;
            proptest::prop_assert_eq!(
                region.start(),
                u64::try_from(input.expression_span.start).unwrap(),
                "{:?}\n{}",
                refusal,
                input.source
            );
            proptest::prop_assert_eq!(
                region.end(),
                u64::try_from(input.expression_span.end).unwrap(),
                "{:?}\n{}",
                refusal,
                input.source
            );
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2054. Plain enumeration is independent of the production model system.
#[test]
fn model_generator_models_are_enumerable() {
    prop::run(
        "small models",
        prop::config(256),
        &prop::model::small_models(),
        |input| {
            check(&input.source()).map_err(|error| {
                TestCaseError::fail(format!("source: {error:?}\n{}", input.source()))
            })?;
            let library = input.library();
            let _world = fsl_bench::world::World::build(&library)
                .map_err(|error| TestCaseError::fail(format!("library: {error:?}")))?;
            let mut queue = std::collections::VecDeque::from([input.initial()]);
            let mut visited = BTreeSet::new();
            while let Some(state) = queue.pop_front() {
                if visited.insert(state.clone()) {
                    proptest::prop_assert!(visited.len() <= input.state_bound());
                    queue.extend(input.successors(&state));
                }
            }
            proptest::prop_assert_eq!(visited.len(), input.state_bound());
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2054. Generated shapes use the evaluator's actual public trace type.
#[test]
fn trace_generator_respects_profile() {
    prop::run(
        "proposition traces",
        prop::config(256),
        &prop::trace::traces(),
        |input| {
            let len = u64::try_from(input.valuations.len()).unwrap();
            proptest::prop_assert!(len > 0);
            proptest::prop_assert_eq!(input.shape.represented(), Some(len));
            match input.shape {
                fsl_eval::temporal::TemporalTrace::Finite { len: represented } => {
                    proptest::prop_assert_eq!(represented, len)
                }
                fsl_eval::temporal::TemporalTrace::Lasso { prefix, loop_len } => {
                    proptest::prop_assert_eq!(
                        input.profile,
                        fsl_semantics::check::TemporalProfile::InfiniteTrace
                    );
                    proptest::prop_assert!(prefix < len && loop_len > 0);
                }
            }
            proptest::prop_assert!(input.valuations.iter().all(|valuation| *valuation < 8));
            for pair in input.timestamps.windows(2) {
                proptest::prop_assert!(pair[0] < pair[1]);
            }
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2054. Typed round-trip inputs reach real package emission.
#[test]
fn package_generator_emits_checked_units() {
    prop::run(
        "package inputs",
        prop::config(256),
        &prop::unit::package_units(),
        |input| {
            let source = input.source();
            let cancel = Cancel::new();
            let identity = SourceIdentity::new("agent-ix", "property-generators");
            let limits = SpineLimits::default();
            let parsed = spine::parse(
                &ParseRequest {
                    source: &identity,
                    path: "generated.native",
                    bytes: source.as_bytes(),
                },
                limits.source,
                &cancel,
            )
            .map_err(|error| TestCaseError::fail(format!("{error:?}")))?
            .into_value();
            let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &cancel)
                .map_err(|error| TestCaseError::fail(format!("{error:?}")))?
                .into_value();
            let checked = spine::check(
                &parsed,
                "property-generators".into(),
                &models,
                &DependencyInput::default(),
                limits,
                &cancel,
            )
            .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?
            .into_value();
            spine::package(&checked, spine::PackageLimits::default(), &cancel)
                .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
            Ok(())
        },
    )
    .unwrap();
}
