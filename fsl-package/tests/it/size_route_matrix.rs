// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked-package reader probes for the QSL-3457 route-by-size matrix.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{CheckedPackage, EmitLimits, V2ReadLimits};
use fsl_semantics::check::PackageDeclarations;
use ix_trace_rs::trace;

const NESTED: usize = 1_000;
const FLAT_TERMS: usize = 10_000;
const SEQUENCE_VALUES: usize = 100_000;

fn nested_source() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function nested using v(): Boolean pure {{ {}true{} }}\n",
        "not (".repeat(NESTED),
        ")".repeat(NESTED)
    )
}

fn link_source(redefines: bool) -> String {
    let mut source = String::from("language \"ix:native\" edition \"1-draft\";\n");
    for index in 0_u64..=1_000 {
        let name = format!("Link{index:04}");
        if index == 0 {
            if redefines {
                source.push_str(&format!(
                    "object {name} {{ operation query(): Integer; }}\n"
                ));
            } else {
                source.push_str(&format!("object {name} {{ }}\n"));
            }
            continue;
        }
        let parent = format!("Link{:04}", index - 1);
        let operation = if redefines {
            format!(" operation query(): Integer redefines {parent}::query;")
        } else {
            String::new()
        };
        source.push_str(&format!(
            "object {name} extends {parent} {{{operation} }}\n"
        ));
    }
    source
}

fn enum_source() -> String {
    let mut source = String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
         domain \"test/qsl-3570-enums\";\n\
         profile v = \"filament.value.complete/v1\";\n",
    );
    for index in 0..1_000 {
        source.push_str(&format!("enum E{index:04} {{ A, B }}\n"));
    }
    source.push_str("function ready using v(): Boolean pure { true }\n");
    source
}

fn flat_source() -> String {
    let terms = std::iter::repeat_n("1", FLAT_TERMS)
        .collect::<Vec<_>>()
        .join(" + ");
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function total using v(): Integer pure {{ {terms} }}\n"
    )
}

fn sequence_source() -> String {
    let values = (0..SEQUENCE_VALUES)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function allValues using v(): Sequence<Integer>[0, {SEQUENCE_VALUES}] pure {{ sequence[{values}] }}\n"
    )
}

fn checked_package(source_text: &str) -> CheckedPackage {
    let parsed = fsl_cst::parse(
        &fsl_cst::ParserContext::new(),
        fsl_foundation::SourceIdentity::new("agent-ix", "test:qsl-3570-package"),
        "unit.native",
        source_text.as_bytes(),
        fsl_cst::Limits {
            source_bytes: 1 << 28,
            tokens: 2_000_000,
            nodes: 2_000_000,
            work_units: 1 << 30,
            diagnostics: 100,
        },
    )
    .expect("S1 reads the source");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the source");
    let empty_domain = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the empty model closure admits");
    let unit_models = fsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &empty_domain,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("source model selections admit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "qsl-3570".to_owned(),
        unit,
        unit_models,
        Vec::new(),
    )
    .expect("the unit assembles");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::new(2_000_000)
                .with_input_bytes(1 << 40)
                .with_work_budget(1 << 40),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    CheckedPackage::link(graph)
}

/// I2 admits the package emitted from the 1,000-level nested expression at
/// its exact byte boundary and returns its typed reader limit one byte under.
#[trace("TC-3242", "FR-264-AC-10")]
#[test]
fn i2_reads_the_thousand_nested_expression_package_at_its_exact_byte_bound() {
    let package = checked_package(&nested_source());
    let emit = EmitLimits::new(1 << 32);
    let emission = fsl_package::emit_checked(&package, emit).expect("the package emits");
    let package_bytes = emission.package().bytes();
    let wire: serde_json::Value = serde_json::from_slice(package_bytes).expect("package JSON");
    let node_count = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .len();
    let domain = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the empty model closure admits");
    let identity =
        fsl_semantics::library::LibraryName::new("qsl-3570").expect("a library identity");

    let mut limits = V2ReadLimits::bounded(1 << 32);
    limits.artifact_bytes = package_bytes.len();
    limits.nodes = node_count as u64;
    let read =
        fsl_package::read_evaluation_package(&package, identity.clone(), &domain, limits, emit)
            .expect("I2 reads the full artifact at its exact byte count");
    assert_eq!(read.canonical_bytes(), package_bytes);
    assert_eq!(read.admitted().name(), "qsl-3570");
    assert_eq!(read.effective_limits().artifact_bytes, package_bytes.len());
    assert_eq!(read.effective_limits().nodes, node_count as u64);

    limits.artifact_bytes = package_bytes.len() - 1;
    let refusal =
        fsl_package::read_evaluation_package(&package, identity.clone(), &domain, limits, emit)
            .err()
            .expect("I2 refuses one byte below the artifact length");
    let fsl_package::ImportViewRefusal::Read { refusal, .. } = refusal else {
        panic!("expected the typed I2 reader limit, got {refusal:?}");
    };
    let fsl_foundation::diagnostic::StageFailure::Limit(limit) = *refusal else {
        panic!("expected StageFailure::Limit, got {refusal:?}");
    };
    assert_eq!(limit.setting(), fsl_foundation::Setting::I2InputBytes);
    assert_eq!(limit.configured_bound(), (package_bytes.len() - 1) as u64);
    assert_eq!(limit.actual(), package_bytes.len() as u128);

    limits.artifact_bytes = package_bytes.len();
    limits.nodes = (node_count - 1) as u64;
    let refusal =
        fsl_package::read_evaluation_package(&package, identity.clone(), &domain, limits, emit)
            .err()
            .expect("I2 refuses one node below the graph's node count");
    let fsl_package::ImportViewRefusal::Read { refusal, .. } = refusal else {
        panic!("expected the typed I2 node limit, got {refusal:?}");
    };
    let fsl_foundation::diagnostic::StageFailure::Limit(limit) = *refusal else {
        panic!("expected StageFailure::Limit, got {refusal:?}");
    };
    assert_eq!(limit.setting(), fsl_foundation::Setting::I2Nodes);
    assert_eq!(limit.configured_bound(), (node_count - 1) as u64);
    assert_eq!(limit.actual(), node_count as u128);
}

/// I2 admits the package carrying 1,000 enum types and their 2,000 variants
/// at its exact encoded byte and node counts, refusing one node below.
#[trace("TC-3242", "FR-264-AC-12")]
#[test]
fn i2_reads_one_thousand_enum_declarations_at_its_counted_boundaries() {
    let package = checked_package(&enum_source());
    let emit = EmitLimits::new(1 << 32);
    let emission = fsl_package::emit_checked(&package, emit).expect("the package emits");
    let package_bytes = emission.package().bytes();
    let wire: serde_json::Value = serde_json::from_slice(package_bytes).expect("package JSON");
    let node_count = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .len();
    let enum_count = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .filter(|node| node["node_tag"] == "model" && node["semantic_form"] == "variant_type")
        .count();
    assert_eq!(enum_count, 1_000);

    let domain = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the empty model closure admits");
    let identity =
        fsl_semantics::library::LibraryName::new("qsl-3570").expect("a library identity");
    let mut limits = V2ReadLimits::bounded(1 << 24);
    limits.artifact_bytes = package_bytes.len();
    limits.nodes = node_count as u64;
    let read =
        fsl_package::read_evaluation_package(&package, identity.clone(), &domain, limits, emit)
            .expect("I2 admits all enum declarations at exact byte and node counts");
    assert_eq!(read.admitted().name(), "qsl-3570");
    assert_eq!(read.canonical_bytes(), package_bytes);

    limits.nodes = (node_count - 1) as u64;
    let refusal =
        fsl_package::read_evaluation_package(&package, identity.clone(), &domain, limits, emit)
            .err()
            .expect("I2 refuses one node below the enum package count");
    let fsl_package::ImportViewRefusal::Read { refusal, .. } = refusal else {
        panic!("expected the typed I2 node limit, got {refusal:?}");
    };
    let fsl_foundation::diagnostic::StageFailure::Limit(limit) = *refusal else {
        panic!("expected StageFailure::Limit, got {refusal:?}");
    };
    assert_eq!(limit.setting(), fsl_foundation::Setting::I2Nodes);
    assert_eq!(limit.configured_bound(), (node_count - 1) as u64);
    assert_eq!(limit.actual(), node_count as u128);

    limits.nodes = node_count as u64;
    limits.artifact_bytes = package_bytes.len() - 1;
    let refusal = fsl_package::read_evaluation_package(&package, identity, &domain, limits, emit)
        .err()
        .expect("I2 refuses one byte below the enum package length");
    let fsl_package::ImportViewRefusal::Read { refusal, .. } = refusal else {
        panic!("expected the typed I2 byte limit, got {refusal:?}");
    };
    let fsl_foundation::diagnostic::StageFailure::Limit(limit) = *refusal else {
        panic!("expected StageFailure::Limit, got {refusal:?}");
    };
    assert_eq!(limit.setting(), fsl_foundation::Setting::I2InputBytes);
    assert_eq!(limit.configured_bound(), (package_bytes.len() - 1) as u64);
    assert_eq!(limit.actual(), package_bytes.len() as u128);
}

/// I2 reads the 10,000-term checked package at its encoded byte and node
/// counts, and returns the typed node limit one below the count.
#[trace("TC-3242", "FR-264-AC-11")]
#[test]
fn i2_reads_the_ten_thousand_term_package_at_its_counted_node_bound() {
    let package = checked_package(&flat_source());
    let emit = EmitLimits::new(1 << 32);
    let emission = fsl_package::emit_checked(&package, emit).expect("the package emits");
    let package_bytes = emission.package().bytes();
    let wire: serde_json::Value = serde_json::from_slice(package_bytes).expect("package JSON");
    let node_count = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .len();
    let domain = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the empty model closure admits");
    let identity =
        fsl_semantics::library::LibraryName::new("qsl-3570").expect("a library identity");
    let mut limits = V2ReadLimits::bounded(1 << 24);
    limits.artifact_bytes = package_bytes.len();
    limits.nodes = node_count as u64;
    let read =
        fsl_package::read_evaluation_package(&package, identity.clone(), &domain, limits, emit)
            .expect("I2 admits the package at exact byte and node counts");
    assert_eq!(read.admitted().name(), "qsl-3570");
    assert_eq!(read.canonical_bytes(), package_bytes);

    limits.nodes = (node_count - 1) as u64;
    let refusal = fsl_package::read_evaluation_package(&package, identity, &domain, limits, emit)
        .err()
        .expect("I2 refuses one node below the package count");
    let fsl_package::ImportViewRefusal::Read { refusal, .. } = refusal else {
        panic!("expected the typed I2 node limit, got {refusal:?}");
    };
    let fsl_foundation::diagnostic::StageFailure::Limit(limit) = *refusal else {
        panic!("expected StageFailure::Limit, got {refusal:?}");
    };
    assert_eq!(limit.setting(), fsl_foundation::Setting::I2Nodes);
    assert_eq!(limit.configured_bound(), (node_count - 1) as u64);
    assert_eq!(limit.actual(), node_count as u128);
}

/// I2 reads both 1,000-link checked packages without changing their canonical
/// package identity or bytes, and retains independent edge/node boundaries.
#[trace("TC-3242", "FR-264-AC-13")]
#[test]
fn i2_reads_both_thousand_link_packages_at_their_counted_boundaries() {
    let emit = EmitLimits::new(1 << 32);
    let domain = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the empty model closure admits");
    let identity =
        fsl_semantics::library::LibraryName::new("qsl-3570").expect("a library identity");

    let inheritance = checked_package(&link_source(false));
    let inheritance_emission =
        fsl_package::emit_checked(&inheritance, emit).expect("inheritance package emits");
    let inheritance_bytes = inheritance_emission.package().bytes();
    let inheritance_wire: serde_json::Value =
        serde_json::from_slice(inheritance_bytes).expect("inheritance package JSON");
    let inheritance_types = inheritance_wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .filter(|node| node["node_tag"] == "model" && node["semantic_form"] == "object_type")
        .count();
    assert_eq!(inheritance_types, 1_001);
    let inheritance_edges = inheritance_wire["semantic_graph"]["operand_rows"]
        .as_array()
        .expect("operand rows")
        .iter()
        .map(|row| row.as_array().expect("operand row").len())
        .sum::<usize>();
    let mut inheritance_limits = V2ReadLimits::bounded(1 << 24);
    inheritance_limits.artifact_bytes = inheritance_bytes.len();
    inheritance_limits.edges = inheritance_edges as u64;
    let inheritance_read = fsl_package::read_evaluation_package(
        &inheritance,
        identity.clone(),
        &domain,
        inheritance_limits,
        emit,
    )
    .expect("I2 reads the inheritance package at exact byte and edge counts");
    assert_eq!(inheritance_read.admitted().name(), "qsl-3570");
    assert_eq!(inheritance_read.canonical_bytes(), inheritance_bytes);
    inheritance_limits.edges = (inheritance_edges - 1) as u64;
    let refusal = fsl_package::read_evaluation_package(
        &inheritance,
        identity.clone(),
        &domain,
        inheritance_limits,
        emit,
    )
    .err()
    .expect("I2 refuses one edge below the inheritance package count");
    let fsl_package::ImportViewRefusal::Read { refusal, .. } = refusal else {
        panic!("expected the typed I2 edge limit, got {refusal:?}");
    };
    let fsl_foundation::diagnostic::StageFailure::Limit(limit) = *refusal else {
        panic!("expected StageFailure::Limit, got {refusal:?}");
    };
    assert_eq!(limit.setting(), fsl_foundation::Setting::I2Edges);
    assert_eq!(limit.configured_bound(), (inheritance_edges - 1) as u64);
    assert_eq!(limit.actual(), inheritance_edges as u128);

    let redefinition = checked_package(&link_source(true));
    let redefinition_emission =
        fsl_package::emit_checked(&redefinition, emit).expect("redefinition package emits");
    let redefinition_bytes = redefinition_emission.package().bytes();
    let redefinition_wire: serde_json::Value =
        serde_json::from_slice(redefinition_bytes).expect("redefinition package JSON");
    let redefinition_types = redefinition_wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .filter(|node| node["node_tag"] == "model" && node["semantic_form"] == "object_type")
        .count();
    assert_eq!(redefinition_types, 1_001);
    let redefinition_nodes = redefinition_wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .len();
    let mut redefinition_limits = V2ReadLimits::bounded(1 << 24);
    redefinition_limits.artifact_bytes = redefinition_bytes.len();
    redefinition_limits.nodes = redefinition_nodes as u64;
    let redefinition_read = fsl_package::read_evaluation_package(
        &redefinition,
        identity.clone(),
        &domain,
        redefinition_limits,
        emit,
    )
    .expect("I2 reads the redefinition package at exact byte and node counts");
    assert_eq!(redefinition_read.admitted().name(), "qsl-3570");
    assert_eq!(redefinition_read.canonical_bytes(), redefinition_bytes);
    redefinition_limits.nodes = (redefinition_nodes - 1) as u64;
    let refusal = fsl_package::read_evaluation_package(
        &redefinition,
        identity,
        &domain,
        redefinition_limits,
        emit,
    )
    .err()
    .expect("I2 refuses one node below the redefinition package count");
    let fsl_package::ImportViewRefusal::Read { refusal, .. } = refusal else {
        panic!("expected the typed I2 node limit, got {refusal:?}");
    };
    let fsl_foundation::diagnostic::StageFailure::Limit(limit) = *refusal else {
        panic!("expected StageFailure::Limit, got {refusal:?}");
    };
    assert_eq!(limit.setting(), fsl_foundation::Setting::I2Nodes);
    assert_eq!(limit.configured_bound(), (redefinition_nodes - 1) as u64);
    assert_eq!(limit.actual(), redefinition_nodes as u128);
}

/// I2 reads the complete 100,000-value artifact at its exact byte and node
/// counts, preserves the checked package identity and ordered wire bytes, and
/// names each independent reader limit one unit under.
#[trace("TC-3242", "FR-264-AC-14")]
#[test]
fn i2_reads_one_hundred_thousand_ordered_values_at_exact_byte_and_node_bounds() {
    let package = checked_package(&sequence_source());
    let emit = EmitLimits::new(1 << 32);
    let emission = fsl_package::emit_checked(&package, emit).expect("the package emits");
    let bytes = emission.package().bytes();
    let wire: serde_json::Value = serde_json::from_slice(bytes).expect("package JSON");
    let node_count = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .len();
    let edge_count = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .map(|node| {
            node["dependencies"]
                .as_array()
                .expect("node dependencies")
                .len()
        })
        .sum::<usize>();
    assert_eq!(wire["name"], "qsl-3570");
    assert!(
        bytes
            .windows(b"99999".len())
            .any(|window| window == b"99999"),
        "the final ordered collection value is carried in the checked artifact"
    );

    let domain = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the empty model closure admits");
    let identity =
        fsl_semantics::library::LibraryName::new("qsl-3570").expect("a library identity");
    let mut limits = V2ReadLimits::bounded(1 << 32);
    limits.artifact_bytes = bytes.len();
    limits.nodes = node_count as u64;
    limits.edges = edge_count as u64;
    limits.occurrences = 1 << 40;
    limits.diagnostics = 1 << 20;
    limits.work = 1 << 40;
    limits.storage = 1 << 40;
    let read =
        fsl_package::read_evaluation_package(&package, identity.clone(), &domain, limits, emit)
            .expect("I2 reads all 100,000 ordered values at exact byte and node counts");
    assert_eq!(read.admitted().name(), "qsl-3570");
    assert_eq!(read.canonical_bytes(), bytes);
    assert_eq!(read.effective_limits().artifact_bytes, bytes.len());
    assert_eq!(read.effective_limits().nodes, node_count as u64);

    limits.nodes = (node_count - 1) as u64;
    let refusal =
        fsl_package::read_evaluation_package(&package, identity.clone(), &domain, limits, emit)
            .err()
            .expect("one node under the graph count is refused");
    let fsl_package::ImportViewRefusal::Read { refusal, .. } = refusal else {
        panic!("expected the typed I2 node limit, got {refusal:?}");
    };
    let fsl_foundation::diagnostic::StageFailure::Limit(limit) = *refusal else {
        panic!("expected StageFailure::Limit, got {refusal:?}");
    };
    assert_eq!(limit.setting(), fsl_foundation::Setting::I2Nodes);
    assert_eq!(limit.configured_bound(), (node_count - 1) as u64);
    assert_eq!(limit.actual(), node_count as u128);

    limits.nodes = node_count as u64;
    limits.artifact_bytes = bytes.len() - 1;
    let refusal = fsl_package::read_evaluation_package(&package, identity, &domain, limits, emit)
        .err()
        .expect("one byte under the encoded artifact length is refused");
    let fsl_package::ImportViewRefusal::Read { refusal, .. } = refusal else {
        panic!("expected the typed I2 byte limit, got {refusal:?}");
    };
    let fsl_foundation::diagnostic::StageFailure::Limit(limit) = *refusal else {
        panic!("expected StageFailure::Limit, got {refusal:?}");
    };
    assert_eq!(limit.setting(), fsl_foundation::Setting::I2InputBytes);
    assert_eq!(limit.configured_bound(), (bytes.len() - 1) as u64);
    assert_eq!(limit.actual(), bytes.len() as u128);
}
