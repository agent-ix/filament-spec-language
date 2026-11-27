// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 over hand-written fixture workspaces, and over this repository.

use std::{fs, path::Path};

use ix_trace_rs::trace;
use tempfile::TempDir;

use super::{
    check, data, spec, BaseRow, Classes, Entry, ExceptionRow, Extent, Outcome, Policy, Rule,
};

/// The base rows the fixtures use, with nothing recorded.
const BASE: &[BaseRow] = &[
    BaseRow {
        id: "B-2",
        modules: &["std", "fsl_route"],
        prefix: "",
        extent: Extent::WholeCrate,
        except: &[],
        recorded: &[],
    },
    BaseRow {
        id: "B-4",
        modules: &["filament_exact"],
        prefix: "",
        extent: Extent::WholeCrate,
        except: &[],
        recorded: &[],
    },
    BaseRow {
        id: "B-5",
        modules: &["fsl_eval::simulation"],
        prefix: "",
        extent: Extent::Module,
        except: &[],
        recorded: &[],
    },
];

/// The real classes and producer-built types over empty lists.
const BARE: Policy = Policy {
    classes: data::FSL.classes,
    base: BASE,
    exceptions: &[],
    routes: &[],
    producer_types: data::FSL.producer_types,
    check_entries: &[],
    codec: &[],
    codec_classes: data::FSL.codec_classes,
    entries: &[],
};

const EVAL: &str = "pub mod simulation {
    pub struct StateKey;
    pub mod model {
        pub struct ModelSystem;
        pub trait ModelLetters {}
    }
}
pub mod probabilistic {
    pub struct MeasurePlan;
    impl MeasurePlan {
        pub fn derive() -> u32 { 0 }
    }
    pub fn step() {}
}
pub mod value {
    pub fn model_value() {}
}
";

struct Fixture(TempDir);

impl Fixture {
    /// A workspace of `fsl-eval`, `fsl-replay` and `fsl-analyze`.
    fn new() -> Self {
        let fixture = Self(tempfile::tempdir().unwrap());
        fixture.write(
            "Cargo.toml",
            "[workspace]\nmembers = [\"fsl-eval\", \"fsl-replay\", \"fsl-analyze\"]\nresolver = \"2\"\n",
        );
        fixture.package("fsl-eval", "");
        fixture.package(
            "fsl-replay",
            "fsl-eval = { path = \"../fsl-eval\" }\nfilament-exact = \"1\"\n",
        );
        fixture.package(
            "fsl-analyze",
            "fsl-eval = { path = \"../fsl-eval\" }\nfsl-replay = { path = \"../fsl-replay\" }\n",
        );
        fixture.write("fsl-eval/src/lib.rs", EVAL);
        fixture.write("fsl-replay/src/lib.rs", "");
        fixture.write("fsl-analyze/src/lib.rs", "");
        fixture
    }

    fn write(&self, relative: &str, contents: &str) -> &Self {
        let path = self.0.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
        self
    }

    fn package(&self, name: &str, dependencies: &str) {
        self.write(
            &format!("{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                 [dependencies]\n{dependencies}"
            ),
        );
    }

    /// `fsl-replay` with `certificate.rs`, a checker file, holding `source`.
    fn checker(&self, source: &str) -> &Self {
        self.write("fsl-replay/src/lib.rs", "pub mod certificate;\n")
            .write("fsl-replay/src/certificate.rs", source)
    }

    fn analyze(&self, source: &str) -> &Self {
        self.write("fsl-analyze/src/lib.rs", source)
    }

    fn run(&self, policy: &Policy) -> Outcome {
        check(self.0.path(), policy).unwrap()
    }
}

fn has(lines: &[String], expected: &str) -> bool {
    lines.iter().any(|line| line == expected)
}

#[trace("FR-1051-AC-1")]
#[test]
fn checker_code_naming_outside_the_base_fails_with_its_site() {
    let fixture = Fixture::new();
    fixture.checker(
        "use fsl_analyze::run;
use fsl_eval::probabilistic::MeasurePlan;
",
    );
    fixture.analyze("pub fn run() {}\n");
    fixture.package(
        "fsl-replay",
        "fsl-eval = { path = \"../fsl-eval\" }\nfsl-analyze = \"1\"\n",
    );
    let outcome = fixture.run(&BARE);
    assert!(!outcome.passed());
    assert_eq!(
        outcome.findings,
        [
            "checker code names fsl_analyze::run outside the trusted base: \
             fsl-replay/src/certificate.rs:1",
            "checker code names fsl_eval::probabilistic::MeasurePlan outside the trusted base: \
             fsl-replay/src/certificate.rs:2",
        ]
    );
}

#[trace("FR-1051-AC-1")]
#[test]
fn checker_code_naming_only_the_base_passes() {
    let fixture = Fixture::new();
    fixture.checker(
        "use std::collections::BTreeMap;
use filament_exact::Integer;
use fsl_eval::simulation::{model::ModelSystem, StateKey};
pub fn f(_: BTreeMap<StateKey, Integer>) -> Option<ModelSystem> { None }
",
    );
    let outcome = fixture.run(&BARE);
    assert!(outcome.passed(), "{}", outcome.report());
}

#[trace("FR-1051-AC-2")]
#[test]
fn a_function_named_by_both_sides_is_reported_with_a_site_on_each() {
    let fixture = Fixture::new();
    fixture
        .checker("pub fn f() -> u32 {\n    fsl_eval::probabilistic::MeasurePlan::derive()\n}\n")
        .analyze(
            "use fsl_eval::probabilistic::MeasurePlan as Plan;
pub fn g() -> u32 {
    Plan::derive()
}
",
        );
    let outcome = fixture.run(&BARE);
    assert!(
        has(
            &outcome.findings,
            "function fsl_eval::probabilistic::MeasurePlan::derive is shared outside the trusted \
             base: checker code at fsl-replay/src/certificate.rs:2, producer code at \
             fsl-analyze/src/lib.rs:3"
        ),
        "{}",
        outcome.report()
    );
}

#[trace("FR-1051-AC-2")]
#[test]
fn a_one_sided_or_codec_function_is_not_reported() {
    let fixture = Fixture::new();
    fixture
        .checker("pub fn f() {}\n")
        .analyze("pub fn g() {\n    fsl_eval::probabilistic::step();\n}\n");
    let one_side = fixture.run(&BARE);
    assert!(one_side.passed(), "{}", one_side.report());

    fixture.checker("pub fn f() {\n    fsl_eval::probabilistic::step();\n}\n");
    let codec = Policy {
        codec: &[("fsl_eval::probabilistic::step", "C-4")],
        ..BARE
    };
    let outcome = fixture.run(&codec);
    assert!(outcome.passed(), "{}", outcome.report());
}

const DOCUMENT: &str = "pub struct Certificate;
pub fn check_probability_certificate_document() {}
pub fn write_probability_certificate() {}
pub fn derive_rank() {}
";

const DOCUMENT_POLICY: Policy = Policy {
    routes: &["R-1"],
    check_entries: &[(
        "fsl_replay::certificate::check_probability_certificate_document",
        &["R-1"],
    )],
    codec: &[(
        "fsl_replay::certificate::write_probability_certificate",
        "C-2",
    )],
    ..BARE
};

#[trace("FR-1051-AC-3")]
#[test]
fn producer_code_names_checker_code_only_through_entries_codec_and_types() {
    let fixture = Fixture::new();
    fixture.checker(DOCUMENT).analyze(
        "use fsl_replay::certificate::{
    check_probability_certificate_document, write_probability_certificate, Certificate,
};
",
    );
    let allowed = fixture.run(&DOCUMENT_POLICY);
    assert!(allowed.passed(), "{}", allowed.report());

    fixture.analyze("pub fn g() {\n    fsl_replay::certificate::derive_rank();\n}\n");
    let outcome = fixture.run(&DOCUMENT_POLICY);
    assert_eq!(
        outcome.findings,
        [
            "producer code names checker item fsl_replay::certificate::derive_rank, neither a \
             check entry nor a codec function: fsl-analyze/src/lib.rs:2"
        ]
    );
}

#[trace("FR-1051-AC-4")]
#[test]
fn a_check_entry_taking_a_producer_built_value_fails() {
    let fixture = Fixture::new();
    fixture.checker(
        "use fsl_eval::simulation::model::{ModelLetters, ModelSystem};
pub struct Request<'a> {
    pub system: &'a ModelSystem,
}
pub struct Checker {
    system: ModelSystem,
}
pub struct MeasurePlan;
pub struct FiniteQuantifier;
pub struct AutomatonStateKey;
impl Checker {
    pub fn on_self(&self) {}
}
pub fn by_parameter(system: &ModelSystem) {}
pub fn by_field(request: Request<'_>) {}
pub fn by_closure(item_of: impl Fn(u32) -> u32) {}
pub fn by_generic<L: ModelLetters>(letters: L) {}
pub fn by_object(letters: &dyn ModelLetters) {}
pub fn by_behaviour(plan: &MeasurePlan) {}
pub fn by_reading(quantifiers: &[FiniteQuantifier]) {}
pub fn clean(bytes: &[u8], key: AutomatonStateKey) {}
",
    );
    let policy = Policy {
        routes: &["R-1"],
        check_entries: &[
            ("fsl_replay::certificate::Checker::on_self", &["R-1"]),
            ("fsl_replay::certificate::by_parameter", &["R-1"]),
            ("fsl_replay::certificate::by_field", &["R-1"]),
            ("fsl_replay::certificate::by_closure", &["R-1"]),
            ("fsl_replay::certificate::by_generic", &["R-1"]),
            ("fsl_replay::certificate::by_object", &["R-1"]),
            ("fsl_replay::certificate::by_behaviour", &["R-1"]),
            ("fsl_replay::certificate::by_reading", &["R-1"]),
            ("fsl_replay::certificate::clean", &["R-1"]),
        ],
        ..BARE
    };
    let outcome = fixture.run(&policy);
    let entry = "check entry fsl_replay::certificate";
    assert_eq!(
        outcome.findings,
        [
            format!(
                "{entry}::Checker::on_self takes producer-built ModelSystem through self.system"
            ),
            format!("{entry}::by_parameter takes producer-built ModelSystem through system"),
            format!("{entry}::by_field takes producer-built ModelSystem through request.system"),
            format!("{entry}::by_closure takes producer-built closure through item_of"),
            format!("{entry}::by_generic takes producer-built ModelLetters through letters"),
            format!("{entry}::by_object takes producer-built ModelLetters through letters"),
            format!("{entry}::by_behaviour takes producer-built MeasurePlan through plan"),
            format!(
                "{entry}::by_reading takes producer-built FiniteQuantifier through quantifiers"
            ),
        ]
    );
}

#[trace("FR-1051-AC-5")]
#[test]
fn sharing_through_every_naming_form_is_reported() {
    let finding = "checker code names fsl_eval::probabilistic::step outside the trusted base: \
                   fsl-replay/src/certificate.rs";
    for (source, line) in [
        (
            "#[cfg(feature = \"test-support\")]\npub fn f() {\n    fsl_eval::probabilistic::step();\n}\n",
            3,
        ),
        (
            "pub fn f() {\n    assert_eq!((), fsl_eval::probabilistic::step());\n}\n",
            2,
        ),
        (
            "use fsl_eval::probabilistic::*;\npub fn f() {\n    step();\n}\n",
            3,
        ),
        (
            "use fsl_eval::probabilistic as measure;\npub fn f() {\n    measure::step();\n}\n",
            3,
        ),
        (
            "use fsl_eval::shim::advance;\npub fn f() {\n    advance();\n}\n",
            1,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.checker(source).write(
            "fsl-eval/src/lib.rs",
            &format!(
                "{EVAL}pub mod shim {{\n    pub use crate::probabilistic::step as advance;\n}}\n"
            ),
        );
        let outcome = fixture.run(&BARE);
        let expected = if line == 1 {
            // The use leaf and the call each name the re-exported item.
            format!("{finding}:1 (+1 more)")
        } else {
            format!("{finding}:{line}")
        };
        assert_eq!(outcome.findings, [expected], "{source}");
    }
}

#[trace("FR-1051-AC-2", "FR-1051-AC-5")]
#[test]
fn a_module_does_not_hide_the_function_of_the_same_name() {
    let eval = format!(
        "{EVAL}pub mod shim {{\n    mod advance {{\n        pub fn advance() {{}}\n    }}\n    \
         pub use advance::advance;\n}}\n"
    );
    for source in [
        "pub fn f() {\n    fsl_eval::shim::advance();\n}\n",
        "use fsl_eval::shim::advance;\npub fn f() {\n    advance();\n}\n",
        "use fsl_eval::shim;\npub fn f() {\n    shim::advance();\n}\n",
    ] {
        let fixture = Fixture::new();
        fixture
            .write("fsl-eval/src/lib.rs", &eval)
            .checker(source)
            .analyze("pub fn g() {\n    fsl_eval::shim::advance();\n}\n");
        let outcome = fixture.run(&BARE);
        assert_eq!(outcome.findings.len(), 1, "{source}: {}", outcome.report());
        assert!(
            outcome.findings[0].starts_with(
                "function fsl_eval::shim::advance::advance is shared outside the trusted base: "
            ),
            "{source}: {}",
            outcome.report()
        );
    }
}

#[trace("FR-1051-AC-3", "FR-1051-AC-5")]
#[test]
fn a_function_path_in_a_serde_attribute_is_a_naming() {
    for attribute in [
        "serde(default = \"fsl_replay::certificate::derive_rank\")",
        "serde(serialize_with = \"fsl_replay::certificate::derive_rank\")",
        "cfg_attr(feature = \"x\", serde(deserialize_with = \"fsl_replay::certificate::derive_rank\"))",
        "serde(with = \"fsl_replay::certificate\")",
    ] {
        let fixture = Fixture::new();
        fixture
            .checker(
                "pub fn derive_rank() {}\npub fn serialize() {}\npub fn deserialize() {}\n",
            )
            .analyze(&format!("pub struct Wire {{\n    #[{attribute}]\n    pub x: u32,\n}}\n"));
        let outcome = fixture.run(&BARE);
        assert!(!outcome.findings.is_empty(), "{attribute}");
        assert!(
            outcome.findings.iter().all(|finding| finding
                .starts_with("producer code names checker item fsl_replay::certificate::")
                && finding.ends_with("fsl-analyze/src/lib.rs:2")),
            "{attribute}: {}",
            outcome.report()
        );
    }
}

#[trace("FR-1051-AC-4")]
#[test]
fn a_producer_built_value_inside_another_type_is_found() {
    let fixture = Fixture::new();
    fixture
        .checker(
            "use crate::planner::{Imported, Wrapper};
use fsl_eval::simulation::model::{ModelLetters, ModelSystem};
use std::sync::Arc;
type Shared = Arc<ModelSystem>;
pub trait ProbeLetters: ModelLetters {}
pub struct OwnLetters;
impl ModelLetters for OwnLetters {}
pub fn by_wrapper(wrapper: &Wrapper) {}
pub fn by_alias(shared: &Shared) {}
pub fn by_imported_alias(shared: &Imported) {}
pub fn by_subtrait(letters: &dyn ProbeLetters) {}
pub fn by_implementation(letters: &crate::planner::Letters) {}
pub fn clean(own: &OwnLetters, package: &crate::planner::CheckedPackage) {}
",
        )
        .write(
            "fsl-replay/src/lib.rs",
            "pub mod certificate;\npub mod planner;\n",
        )
        .write(
            "fsl-replay/src/planner.rs",
            "use fsl_eval::simulation::model::{ModelLetters, ModelSystem};
use std::sync::Arc;
pub struct Wrapper {
    pub system: ModelSystem,
}
pub type Imported = Arc<ModelSystem>;
pub struct Letters;
impl ModelLetters for Letters {}
pub struct CheckedPackage;
impl ModelLetters for CheckedPackage {}
",
        );
    let policy = Policy {
        routes: &["R-1"],
        check_entries: &[
            ("fsl_replay::certificate::by_wrapper", &["R-1"]),
            ("fsl_replay::certificate::by_alias", &["R-1"]),
            ("fsl_replay::certificate::by_imported_alias", &["R-1"]),
            ("fsl_replay::certificate::by_subtrait", &["R-1"]),
            ("fsl_replay::certificate::by_implementation", &["R-1"]),
            ("fsl_replay::certificate::clean", &["R-1"]),
        ],
        ..BARE
    };
    let outcome = fixture.run(&policy);
    let entry = "check entry fsl_replay::certificate";
    let taken: Vec<&String> = outcome
        .findings
        .iter()
        .filter(|finding| finding.starts_with(entry))
        .collect();
    assert_eq!(
        taken,
        [
            &format!("{entry}::by_wrapper takes producer-built ModelSystem through wrapper.system"),
            &format!("{entry}::by_alias takes producer-built ModelSystem through shared"),
            &format!("{entry}::by_imported_alias takes producer-built ModelSystem through shared"),
            &format!("{entry}::by_subtrait takes producer-built ModelLetters through letters"),
            &format!("{entry}::by_implementation takes producer-built Letters through letters"),
        ]
    );
}

#[trace("FR-1051-AC-5")]
#[test]
fn test_only_code_is_not_read() {
    let fixture = Fixture::new();
    fixture.checker(
        "#[cfg(test)]
mod tests;
#[cfg(any(test, kani))]
pub fn f() {
    fsl_eval::probabilistic::step();
}
",
    );
    fixture.write(
        "fsl-replay/src/certificate/tests.rs",
        "use fsl_eval::probabilistic::step;\n",
    );
    let outcome = fixture.run(&BARE);
    assert!(outcome.passed(), "{}", outcome.report());
}

#[trace("FR-1051-AC-6")]
#[test]
fn a_crate_or_file_in_no_class_or_two_fails() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"fsl-eval\", \"fsl-replay\", \"fsl-analyze\", \"fsl-new\"]\nresolver = \"2\"\n",
    );
    fixture.package("fsl-new", "");
    fixture.write("fsl-new/src/lib.rs", "");
    fixture.write("fsl-replay/build.rs", "fn main() {}\n");
    fixture.write("fsl-replay/src/lib.rs", "mod new_checker;\n");
    fixture.write("fsl-replay/src/new_checker.rs", "pub fn check() {}\n");
    let outcome = fixture.run(&BARE);
    assert_eq!(
        outcome.errors,
        [
            "crate fsl-new is in no class",
            "file fsl-replay/src/new_checker.rs is in no class",
            "file fsl-replay/build.rs is in no class",
        ]
    );

    const TWICE: Classes = Classes {
        settling_files: &["certificate.rs"],
        tool_crates: &["fsl-analyze", "fsl-new"],
        ..data::FSL.classes
    };
    fixture.checker("pub fn f() {}\n");
    fs::remove_file(fixture.0.path().join("fsl-replay/build.rs")).unwrap();
    let outcome = fixture.run(&Policy {
        classes: TWICE,
        ..BARE
    });
    assert_eq!(
        outcome.errors,
        [
            "crate fsl-analyze is in two classes",
            "file fsl-replay/src/certificate.rs is in two classes",
        ]
    );
}

const SHARED_ROWS: &[ExceptionRow] = &[ExceptionRow {
    id: "S-3",
    rule: Rule::SharedSemantics,
    concerns: &[("fsl_eval::probabilistic", &["fsl_eval::probabilistic"])],
    planned: false,
}];

const fn shared(row: &'static str, item: &'static str) -> Entry {
    Entry {
        row,
        rule: Rule::SharedSemantics,
        item,
        takes: "",
    }
}

#[trace("FR-1051-AC-7")]
#[test]
fn an_entry_covers_its_finding_and_a_stale_entry_fails() {
    const ONE: &[Entry] = &[shared("S-3", "fsl_eval::probabilistic::step")];
    let policy = Policy {
        exceptions: SHARED_ROWS,
        entries: ONE,
        ..BARE
    };
    let fixture = Fixture::new();
    fixture.checker("pub fn f() {\n    fsl_eval::probabilistic::step();\n}\n");
    let covered = fixture.run(&policy);
    assert!(covered.passed(), "{}", covered.report());
    assert_eq!(covered.recorded, 1);

    fixture.checker("pub fn f() {\n    fsl_eval::value::model_value();\n}\n");
    let outcome = fixture.run(&policy);
    assert_eq!(
        outcome.findings,
        [
            "checker code names fsl_eval::value::model_value outside the trusted base: \
             fsl-replay/src/certificate.rs:2"
        ]
    );
    assert_eq!(
        outcome.errors,
        ["stale entry: row S-3 fsl_eval::probabilistic::step  matches no finding"]
    );
}

#[trace("FR-1051-AC-8")]
#[test]
fn wrong_data_is_named() {
    const WRONG: &[Entry] = &[
        shared("S-3", "fsl_eval::probabilistic::gone"),
        shared("S-3", "fsl_eval::value::model_value"),
        shared("S-99", "fsl_eval::probabilistic::step"),
        Entry {
            row: "S-3",
            rule: Rule::ProducerCall,
            item: "fsl_eval::probabilistic::step",
            takes: "",
        },
        shared("S-14", "fsl_eval::planned::item"),
        Entry {
            row: "O-1",
            rule: Rule::ProducerObject,
            item: "fsl_eval::probabilistic::step",
            takes: "ModelSystem",
        },
    ];
    const ROWS: &[ExceptionRow] = &[
        ExceptionRow {
            id: "S-3",
            rule: Rule::SharedSemantics,
            concerns: &[("fsl_eval::probabilistic", &["fsl_eval::probabilistic"])],
            planned: false,
        },
        ExceptionRow {
            id: "S-4",
            rule: Rule::SharedSemantics,
            concerns: &[],
            planned: false,
        },
        ExceptionRow {
            id: "S-14",
            rule: Rule::SharedSemantics,
            concerns: &[],
            planned: true,
        },
        ExceptionRow {
            id: "O-1",
            rule: Rule::ProducerObject,
            concerns: &[("fsl_eval::value", &["fsl_eval::value"])],
            planned: false,
        },
    ];
    const DATA_ONLY: &[BaseRow] = &[BaseRow {
        id: "B-10",
        modules: &["fsl-eval"],
        prefix: "",
        extent: Extent::DataItems,
        except: &[],
        recorded: &["fsl_eval::probabilistic::step"],
    }];
    let fixture = Fixture::new();
    let outcome = fixture.run(&Policy {
        base: DATA_ONLY,
        exceptions: ROWS,
        routes: &["R-1"],
        check_entries: &[("fsl_replay::missing", &["R-9"])],
        codec: &[("fsl_eval::probabilistic::step", "C-9")],
        entries: WRONG,
        ..BARE
    });
    for expected in [
        "entry S-3 fsl_eval::probabilistic::gone: the item does not exist",
        "entry S-3 fsl_eval::value::model_value: the item is defined outside the modules row S-3 \
         concerns",
        "entry S-99 fsl_eval::probabilistic::step: no row S-99 in FR-1051",
        "entry S-3 fsl_eval::probabilistic::step: breaks \"Producer call into checker\", but row \
         S-3 is \"Shared semantics\"",
        "row S-4 has no entry",
        "entry O-1 fsl_eval::probabilistic::step: the item is defined outside the modules row O-1 \
         concerns",
        "check entry fsl_replay::missing: the item does not exist",
        "check entry fsl_replay::missing: no route R-9 in FR-1051",
        "codec function fsl_eval::probabilistic::step: no codec class C-9 in FR-1051",
        "B-10 item fsl_eval::probabilistic::step: is a function, not data",
    ] {
        assert!(
            has(&outcome.errors, expected),
            "{expected}\n{}",
            outcome.report()
        );
    }
    assert!(
        !outcome
            .errors
            .iter()
            .any(|error| error.contains("fsl_eval::planned::item")),
        "a planned row's entry is exempt: {}",
        outcome.report()
    );
}

#[trace("FR-1051-AC-10")]
#[test]
fn routes_and_check_entries_cover_each_other() {
    let fixture = Fixture::new();
    fixture.checker(DOCUMENT);
    let outcome = fixture.run(&Policy {
        routes: &["R-1", "R-2"],
        check_entries: &[
            (
                "fsl_replay::certificate::check_probability_certificate_document",
                &["R-1"],
            ),
            ("fsl_replay::certificate::derive_rank", &[]),
        ],
        ..BARE
    });
    assert_eq!(
        outcome.errors,
        [
            "check entry fsl_replay::certificate::derive_rank: names no route",
            "route R-2 has no check entry",
        ]
    );
}

#[trace("FR-1051-AC-10")]
#[test]
fn settling_code_naming_an_undeclared_checker_function_fails() {
    let fixture = Fixture::new();
    fixture
        .checker(DOCUMENT)
        .write(
            "fsl-replay/src/lib.rs",
            "pub mod certificate;\npub mod proof_result;\n",
        )
        .write(
            "fsl-replay/src/proof_result.rs",
            "pub fn settle() {\n    crate::certificate::derive_rank();\n}\n",
        );
    let outcome = fixture.run(&DOCUMENT_POLICY);
    assert_eq!(
        outcome.findings,
        [
            "settling code names checker item fsl_replay::certificate::derive_rank, neither a \
             check entry nor a codec function: fsl-replay/src/proof_result.rs:2"
        ]
    );

    const ROWS: &[ExceptionRow] = &[ExceptionRow {
        id: "P-1",
        rule: Rule::ProducerCall,
        concerns: &[("certificate.rs", &["fsl-replay/src/certificate.rs"])],
        planned: false,
    }];
    const RECORDED: &[Entry] = &[Entry {
        row: "P-1",
        rule: Rule::ProducerCall,
        item: "fsl_replay::certificate::derive_rank",
        takes: "",
    }];
    let outcome = fixture.run(&Policy {
        exceptions: ROWS,
        entries: RECORDED,
        ..DOCUMENT_POLICY
    });
    assert!(outcome.passed(), "{}", outcome.report());
    assert_eq!(outcome.recorded, 1);
}

fn repository() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(relative: &str) -> String {
    fs::read_to_string(repository().join(relative)).unwrap()
}

const REQUIREMENT: &str =
    "spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md";

#[trace("FR-1051-AC-7", "FR-1051-AC-8", "FR-1051-AC-10")]
#[test]
fn the_repository_passes_with_its_recorded_sharings() {
    let outcome = check(repository(), &data::FSL).unwrap();
    assert!(outcome.passed(), "{}", outcome.report());
}

#[trace("FR-1051-AC-9")]
#[test]
fn the_requirement_and_the_data_agree() {
    assert_eq!(
        spec::differences(&read(REQUIREMENT), &data::FSL),
        Vec::<String>::new()
    );
}

#[trace("FR-1051-AC-9")]
#[test]
fn a_changed_table_is_named() {
    let changed = read(REQUIREMENT)
        .replace("`timed/explore.rs`", "`timed/search.rs`")
        .replace("| module, except S-9 |", "| module |")
        .replace("| S-2 | Shared semantics |", "| S-2 | Producer object |")
        .replace("| R-62 |", "| R-63 |")
        .replace("`FiniteTracePopulation`", "`FiniteTrace`")
        .replace(
            "`num_integer`, `num_traits` | whole crate |",
            "`num_integer`, `num_traits` | type items |",
        )
        .replace(
            "`fsl_eval::model_subject` | module |",
            "`fsl_eval::model_reading` | module |",
        )
        .replace(
            "| `model_check/identity.rs` |",
            "| `model_check/identities.rs` |",
        )
        .replace("| S-11 | Shared semantics |", "| S-99 | Shared semantics |");
    let differences = spec::differences(&changed, &data::FSL);
    for expected in [
        "class \"Checker code\": FR-1051 has timed/search.rs, the data does not",
        "class \"Checker code\": the data has timed/explore.rs, FR-1051 does not",
        "base row B-7 exceptions: the data has S-9, FR-1051 does not",
        "exception row S-2: rule \"Producer object\" is not the data's \"Shared semantics\"",
        "routes: FR-1051 has R-63, the data does not",
        "routes: the data has R-62, FR-1051 does not",
        "producer-built types: FR-1051 has FiniteTrace, the data does not",
        "producer-built types: the data has FiniteTracePopulation, FR-1051 does not",
        "base row B-4: extent \"type items\" is not the data's WholeCrate",
        "base row B-8: FR-1051 has fsl_eval::model_reading, the data does not",
        "base row B-8: the data has fsl_eval::model_subject, FR-1051 does not",
        "exception row S-14: FR-1051 has model_check/identities.rs, the data does not",
        "exception row S-14: the data has model_check/identity.rs, FR-1051 does not",
        "exception row S-99: not in the data",
        "exception row S-11: not in FR-1051",
    ] {
        assert!(has(&differences, expected), "{expected}\n{differences:#?}");
    }
    assert_eq!(differences.len(), 15, "{differences:#?}");
}

#[trace("FR-1051-AC-9")]
#[test]
fn a_selector_wider_than_its_concern_is_named() {
    const WIDENED: &[ExceptionRow] = &[ExceptionRow {
        id: "S-4",
        rule: Rule::SharedSemantics,
        concerns: &[("fsl_eval::fairness", &["fsl_eval"])],
        planned: false,
    }];
    let policy = Policy {
        exceptions: WIDENED,
        ..data::FSL
    };
    let differences = spec::differences(&read(REQUIREMENT), &policy);
    let expected =
        "exception row S-4: selector fsl_eval does not match the concern fsl_eval::fairness";
    assert!(has(&differences, expected), "{differences:#?}");
}

#[trace("FR-1051-AC-15")]
#[test]
fn ce_7_must_say_the_checker_shares_the_concrete_run_alone() {
    let named =
        Some("ADR-028 CE-7 does not limit what the checker shares to the concrete run alone");
    let silent = "| CE-7 | The checker is small. |\n";
    let shared_product = "| CE-7 | What the checker shares is the concrete run alone, and the \
                          product construction is shared with EN-1. |\n";
    let narrowed = "| CE-7 | What the checker shares is the concrete run alone. |\n";
    assert_eq!(spec::ce_7_difference(silent).as_deref(), named);
    assert_eq!(spec::ce_7_difference(shared_product).as_deref(), named);
    assert_eq!(spec::ce_7_difference("").as_deref(), named);
    assert_eq!(spec::ce_7_difference(narrowed), None);
    let adr = read("spec/decisions/ADR-028-exact-probabilistic-engine.md");
    assert_eq!(spec::ce_7_difference(&adr), None);
}
