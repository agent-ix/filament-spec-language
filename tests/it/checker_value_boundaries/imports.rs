// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{check, library};
use fsl_foundation::Code;
use fsl_replay::spine::DependencyInput;
use ix_trace_rs::trace;

#[test]
#[trace("FR-099-AC-3", "TC-024")]
fn failed_missing_linkage_does_not_poison_a_later_qualified_request() {
    let dependencies = DependencyInput::new([
        library(
            "original:east",
            "function toll using v(n: Integer): Integer pure { n + 3 }",
        ),
        library(
            "original:west",
            "function toll using v(n: Integer): Integer pure { n + 11 }",
        ),
    ])
    .unwrap();
    let valid = "import \"original:east\" as east;\nimport \"original:west\" as west;\nfunction difference using v(n: Integer): Integer pure { west::toll(n) - east::toll(n) }";
    for position in 0..=2 {
        let absent = format!("original:lost-{position}");
        let mut imports = vec![
            "import \"original:east\" as east;".to_owned(),
            "import \"original:west\" as west;".to_owned(),
        ];
        imports.insert(position, format!("import \"{absent}\" as lost;"));
        let refused = check(&format!("{}\nfunction difference using v(n: Integer): Integer pure {{ west::toll(n) - east::toll(n) }}",imports.join("\n")), &dependencies)
            .expect_err("an unresolved import cannot produce a checked package");
        assert_eq!(refused.code(), Code::MissingImport, "{refused:?}");
        assert!(refused.to_string().contains(&absent), "{refused:?}");
        check(valid, &dependencies).expect("failed selection must not poison the next request");
    }
}

#[test]
#[trace("FR-099-AC-8", "TC-894")]
fn imported_parameter_boundary_matches_local_discharge() {
    let dependencies = DependencyInput::new([library(
        "original:threshold",
        "function accepts using v(x: Int[2,11]): Boolean pure { x >= 2 }",
    )])
    .unwrap();
    let local = "function accepts using v(x: Int[2,11]): Boolean pure { x >= 2 }\n";
    let imported = "import \"original:threshold\" as threshold;\n";
    for (argument, accepted) in [("2", true), ("11", true), ("1", false), ("12", false)] {
        let own = check(
            &format!("{local}function entry using v(): Boolean pure {{ accepts({argument}) }}"),
            &DependencyInput::default(),
        );
        let linked = check(&format!("{imported}function entry using v(): Boolean pure {{ threshold::accepts({argument}) }}"), &dependencies);
        assert_eq!(own.is_ok(), accepted, "local argument {argument}: {own:?}");
        assert_eq!(
            linked.is_ok(),
            accepted,
            "imported argument {argument}: {linked:?}"
        );
        if let (Err(own), Err(linked)) = (own, linked) {
            assert_eq!(own.code(), linked.code(), "argument {argument}");
            let own_diagnostics = own.outcome_diagnostics();
            let linked_diagnostics = linked.outcome_diagnostics();
            assert_eq!(own_diagnostics.len(), 1, "{own:?}");
            assert_eq!(linked_diagnostics.len(), 1, "{linked:?}");
            assert_eq!(
                own_diagnostics[0].cause(),
                linked_diagnostics[0].cause(),
                "argument {argument}"
            );
            assert!(own_diagnostics[0].locus().is_some());
            assert!(linked_diagnostics[0].locus().is_some());
        }
    }
}

#[test]
#[trace("TC-020", "TC-022")]
fn qualified_same_named_exports_keep_their_distinct_function_bodies() {
    use filament_exact::{Integer, Value};
    let dependencies = DependencyInput::new([
        library(
            "original:harbor",
            "function tariff using v(n: Integer): Integer pure { n + 5 }",
        ),
        library(
            "original:ridge",
            "function tariff using v(n: Integer): Integer pure { n + 19 }",
        ),
    ])
    .unwrap();
    let checked = check("import \"original:harbor\" as harbor; import \"original:ridge\" as ridge; function margin using v(n: Integer): Integer pure { ridge::tariff(n) - harbor::tariff(n) }",&dependencies).unwrap();
    for value in [-9_i64, 0, 27] {
        let result = super::evaluation::call(
            &checked,
            "margin",
            vec![Value::Integer(Integer::from(value))],
        );
        assert!(
            matches!(result,Value::Integer(ref n) if *n == Integer::from(14_i64)),
            "{result:?}"
        );
    }
    let error = check("import \"original:harbor\" as shared; import \"original:ridge\" as shared; function margin using v(): Integer pure { shared::tariff(0) }",&dependencies).unwrap_err();
    assert_eq!(error.code(), Code::AmbiguousDeclaration, "{error:?}");
    let missing = check("import \"original:harbor\" as harbor; function missing_export using v(): Integer pure { harbor::tariff_extra(0) }",&dependencies).unwrap_err();
    assert_eq!(
        missing.code(),
        Code::MissingDeclaration,
        "a prefix match must not resolve an absent export: {missing:?}"
    );
}
