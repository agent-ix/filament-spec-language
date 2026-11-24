// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed timed-model inputs for package emit, package read and replay/request.
//!
//! Each model document describes one object type, with one clock field, one
//! integer value field and one operation. The corresponding source unit has
//! one time declaration, a timed pre/post transition, and one temporal claim
//! per selected model.

use crate::world::{Model, Operation};

/// A package fixture prepared once and reused by the package axes.
pub struct Fixture {
    /// The selected, checked source unit.
    pub checked: fsl_replay::spine::CheckedUnit,
    /// The admitted model documents used by package read.
    pub packages: fsl_semantics::model::intake::PackageInput,
    /// The checked package's current wire emission.
    pub emission: fsl_package::Emission,
    /// One valid source region supplied to checked-package emission.
    pub region: fsl_foundation::source::provenance::SourceRegion,
    /// Raw domain-package documents provisioned to replay requests.
    pub documents: Vec<Vec<u8>>,
}

/// Compile one timed-model workload and prepare its read input.
pub fn fixture(count: usize) -> Result<Fixture, String> {
    let models = models(count);
    let documents: Vec<Vec<u8>> = models.iter().map(Model::document).collect();
    let input = fsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice))
        .map_err(|error| format!("model package admission: {error:?}"))?;
    let unit = source(count);
    let identity = fsl_foundation::SourceIdentity::new("bench", "package");
    let region = fsl_foundation::source::provenance::SourceRegion::new(
        fsl_foundation::source::provenance::RawSourceRef::new("bench", "package")
            .map_err(|error| format!("source identity: {error:?}"))?,
        0,
        u64::try_from(unit.len()).map_err(|error| format!("source length: {error}"))?,
    )
    .map_err(|error| format!("source region: {error:?}"))?;
    let cancel = filament_exact::Cancel::new();
    let limits = fsl_replay::spine::SpineLimits::default();
    let parsed = fsl_replay::spine::parse(
        &fsl_replay::spine::ParseRequest {
            source: &identity,
            path: "package.native",
            bytes: unit.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(|error| format!("source parse: {error:?}"))?
    .into_value();
    let selected = fsl_replay::spine::select(&parsed, &input, limits.model, &cancel)
        .map_err(|error| format!("model selection: {error:?}"))?
        .into_value();
    let checked = fsl_replay::spine::check(
        &parsed,
        "package".to_owned(),
        &selected,
        &fsl_replay::spine::DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|error| format!("source check: {error:?}"))?
    .into_value();
    let emission = fsl_package::emit_package(
        checked.package(),
        |_| Some(region.clone()),
        fsl_package::EmitLimits::new(1 << 32),
    )
    .map_err(|error| format!("package emission: {error:?}"))?;
    Ok(Fixture {
        checked,
        packages: input,
        emission,
        region,
        documents,
    })
}

/// The workload sizes required for package and replay measurements.
pub const MODEL_COUNTS: [usize; 3] = [1, 10, 100];

/// Build the checked-domain inputs for one workload size.
///
/// # Panics
/// Panics only if `count` is zero, since a package axis requires a model.
pub fn models(count: usize) -> Vec<Model> {
    assert!(count > 0, "a package workload has at least one model");
    (0..count)
        .map(|index| Model {
            package: format!("bench/timed{index}"),
            object: format!("Clocked{index}"),
            population: "clocked".to_owned(),
            symmetric: false,
            integers: Vec::new(),
            fields: vec![
                ("clock".to_owned(), "ix://filament/native/Clock".to_owned()),
                (
                    "value".to_owned(),
                    "ix://filament/native/Integer".to_owned(),
                ),
            ],
            operations: vec![Operation {
                name: "step".to_owned(),
                parameters: Vec::new(),
                modifies: vec!["clock".to_owned(), "value".to_owned()],
            }],
            profiles: String::new(),
            body: String::new(),
            objects: Vec::new(),
            unrelated: 0,
        })
        .collect()
}

/// Generate the source unit that times every selected model and gives it a
/// single transition and temporal claim.
pub fn source(count: usize) -> String {
    assert!(count > 0, "a package workload has at least one model");
    let mut source = String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile q = \"filament.value.complete/v1\";\n\
         profile t = \"filament.temporal.complete/v1\";\n\
         profile tt = \"filament.temporal.timed/v1\";\n",
    );
    for index in 0..count {
        source.push_str(&format!("model M{index} = \"bench/timed{index}\";\n"));
    }
    source.push_str("dimension Time;\nunit ms : Time = rational(1, 1);\n");
    for index in 0..count {
        source.push_str(&format!("time M{index} dense unit ms;\n"));
    }
    for index in 0..count {
        source.push_str(&format!(
            "pre Ready{index} using v on M{index}::Clocked{index}::step {{ self.value >= 0 }}\n\
             post Advanced{index} using v on M{index}::Clocked{index}::step {{ self.value = pre(self.value) + 1 }}\n\
             temporal Claim{index} using tt over (m: M{index}::Clocked{index}) clock \"model-time\" on origin {{ eventually holds(m.value >= 0) }}\n"
        ));
    }
    source.push_str("function replay_probe using q(x: Integer): Boolean pure { x >= 0 }\n");
    source
}

/// Construct the end-to-end replay request for a fixture.
pub fn replay_request(
    fixture: &Fixture,
    unit: &str,
) -> Result<fsl_replay::ReplayRequestWire, String> {
    let name =
        fsl_replay::QualifiedName::new(vec![filament_exact::Identifier::new("replay_probe")
            .map_err(|error| format!("function name: {error:?}"))?])
        .map_err(|error| format!("qualified function name: {error:?}"))?;
    let site = fsl_replay::call_site(
        fsl_foundation::SourceIdentity::new("bench", "package"),
        fsl_replay::CallSiteUnit::new("package", "package.native", unit.as_bytes()),
        fixture.documents.iter().map(Vec::as_slice),
        &fsl_replay::spine::DependencyInput::default(),
        &fsl_replay::RequestClocks::default(),
        &name,
    )
    .map_err(|error| format!("replay call site: {error:?}"))?;
    let limits = fsl_replay::ReplayLimits::default().with_input_bytes(1 << 30);
    let package = fsl_replay::ReplayPackage::read(&site.package, limits.input_bytes)
        .map_err(|error| format!("replay package: {error:?}"))?;
    Ok(fsl_replay::ReplayRequestWire {
        package,
        profile_selections: Vec::new(),
        dependencies: Vec::new(),
        replay_target: fsl_replay::RequestTargetWire::function(name, site.site.declaration.clone()),
        source: fsl_replay::ReplaySource::Input(vec![fsl_replay::CanonicalAssignment {
            parameter: site.site.parameters[0].1.clone(),
            value: fsl_replay::WitnessValue::Integer(1),
        }]),
        certificate: None,
        obligation_identity: site.site.obligation_identity().clone(),
        backend: "fsl-bench".to_owned(),
        state_environment: fsl_replay::StateEnvironment::new(Vec::new()),
        accounting_limits: crate::world::UNLIMITED,
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![(
            "bench".to_owned(),
            "package".to_owned(),
            unit.as_bytes().to_vec(),
        )],
        documents: fixture.documents.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{models, source, MODEL_COUNTS};
    use ix_trace_rs::trace;

    #[trace("NFR-141-AC-1", "NFR-141-AC-2", "NFR-141-AC-3")]
    #[test]
    fn every_package_fixture_has_one_timed_model_composition() {
        for count in MODEL_COUNTS {
            let models = models(count);
            assert_eq!(models.len(), count);
            for model in &models {
                assert_eq!(model.fields.len(), 2, "one clock and one value");
                assert_eq!(model.fields[0].1, "ix://filament/native/Clock");
                assert_eq!(model.operations.len(), 1, "one operation");
                assert_eq!(model.operations[0].name, "step");
            }
            let unit = source(count);
            assert_eq!(unit.matches("time M").count(), count);
            assert_eq!(unit.matches("pre Ready").count(), count);
            assert_eq!(unit.matches("post Advanced").count(), count);
            assert_eq!(unit.matches("temporal Claim").count(), count);
        }
    }

    #[trace("NFR-141-AC-1", "NFR-141-AC-2")]
    #[test]
    fn the_smallest_timed_package_compiles_through_replay_spine() {
        let fixture = super::fixture(1).expect("timed package fixture compiles");
        assert!(!fixture.emission.package().bytes().is_empty());
    }

    #[trace("NFR-141-AC-3")]
    #[test]
    fn every_package_size_compiles_emits_and_replays() {
        for count in MODEL_COUNTS {
            let fixture = super::fixture(count).expect("timed package fixture compiles");
            assert!(
                !fixture.emission.package().bytes().is_empty(),
                "M={count} emits checked package bytes"
            );
            let unit = source(count);
            let request = super::replay_request(&fixture, &unit).expect("replay request builds");
            let limits = fsl_replay::ReplayLimits::default().with_input_bytes(1 << 30);
            let result = fsl_replay::replay(request, limits);
            assert!(result.is_ok(), "M={count} replay succeeds: {result:?}");
        }
    }

    #[trace("NFR-141-AC-3")]
    #[test]
    fn the_smallest_fixture_replays_end_to_end() {
        let fixture = super::fixture(1).expect("timed replay fixture compiles");
        let unit = source(1);
        let request = super::replay_request(&fixture, &unit).expect("request builds");
        let result = fsl_replay::replay(request, fsl_replay::ReplayLimits::default());
        assert!(result.is_ok(), "replay request succeeds: {result:?}");
    }

    #[cfg(feature = "counters")]
    #[trace("NFR-141-AC-5")]
    #[test]
    fn replay_parse_counter_counts_calls_for_its_source_document() {
        let fixture = super::fixture(1).expect("timed replay fixture compiles");
        let unit = source(1);
        let request = super::replay_request(&fixture, &unit).expect("request builds");
        let (result, calls) =
            crate::counters::replay_with_parse_calls(request, fsl_replay::ReplayLimits::default());
        assert!(result.is_ok(), "replay request succeeds: {result:?}");
        assert_eq!(calls, 1, "one parser invocation for one source document");
    }
}
