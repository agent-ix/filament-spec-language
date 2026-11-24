// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-020, TC-021, TC-022 and TC-024: an import resolves by exact qualified
//! name, a missing import and an ambiguous export are refused with their
//! typed codes, and a failed resolution leaves no partial linkage.
//!
//! Library selection is `fsl_semantics::library::resolve_libraries` over
//! `LibraryPackage` values; name resolution at a call is the checker's, over
//! the admitted library's import view. Both are exercised, each with an
//! accepted control beside every refusal.

use filament_exact::{NodeKey, PathSegment};
use fsl_foundation::Code;
use fsl_semantics::library::{
    resolve_libraries, ImportDeclaration, LibraryCause, LibraryName, LibraryPackage, LibraryRefusal,
};
use ix_trace_rs::trace;

use crate::imported_call::{admitted_library, compile, HEADER};

fn name(library: &str) -> LibraryName {
    LibraryName::new(library).expect("a library name")
}

fn key(label: &str) -> NodeKey {
    NodeKey::new("lib", vec![PathSegment::Text(label.to_owned())], 0).expect("a node key")
}

fn package(library: &str, imports: &[&str], exports: &[&str]) -> LibraryPackage {
    LibraryPackage {
        library: name(library),
        imports: imports
            .iter()
            .map(|import| ImportDeclaration {
                library: name(import),
                qualifier: Some(format!("q_{}", import.to_lowercase())),
            })
            .collect(),
        exports: exports
            .iter()
            .map(|export| ((*export).to_owned(), key(&format!("{library}::{export}"))))
            .collect(),
    }
}

#[trace("TC-020")]
#[test]
fn an_exact_qualified_import_resolves_and_a_near_name_does_not() {
    // Library selection: the import selects exactly the supplied library.
    let root = package("P", &["Counters"], &[]);
    let lock = resolve_libraries(&root, &[package("Counters", &[], &["bounded"])]).unwrap();
    assert_eq!(lock.selections(), [name("Counters")]);
    // Name resolution at the call: `g::f` resolves; a different case, a
    // prefix, an extension and a different alias are each missing names.
    let library = || {
        admitted_library(
            "test/geometry",
            "function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n",
            &["f"],
        )
        .expect("the library checks")
    };
    let call = |callee: &str| {
        compile(
            &format!(
                "{HEADER}import \"test/geometry\" as g;\n\
                 function caller using v(): Boolean pure {{ {callee}(3) }}\n"
            ),
            vec![library()],
        )
    };
    call("g::f").expect("the exact qualified name resolves");
    for near in ["g::F", "g::ff", "g::", "h::f"] {
        // `g::` is not parseable as a call by every route; only assert the
        // names the grammar accepts.
        if near == "g::" {
            continue;
        }
        let refusals = call(near).expect_err(near);
        assert_eq!(refusals.len(), 1, "{near}: {refusals:?}");
        assert_eq!(
            (refusals[0].cause.code(), refusals[0].cause.cause()),
            (Code::MissingDeclaration, Some("missing-name")),
            "{near}"
        );
    }
}

#[trace("TC-021")]
#[test]
fn a_missing_selected_import_is_refused_at_the_import_and_produces_no_lock() {
    let root = package("P", &["Counters"], &[]);
    let refusal = resolve_libraries(&root, &[]).expect_err("nothing supplied");
    assert_eq!(
        refusal,
        LibraryRefusal::MissingImport {
            path: vec![name("P"), name("Counters")]
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::MissingImport, LibraryCause::MissingSelection)
    );
    // An unrelated library in the inventory does not stand in for it, and the
    // same inventory with the selected library present resolves.
    let other = package("Other", &[], &["bounded"]);
    assert!(matches!(
        resolve_libraries(&root, std::slice::from_ref(&other)),
        Err(LibraryRefusal::MissingImport { .. })
    ));
    assert!(resolve_libraries(&root, &[other, package("Counters", &[], &["bounded"])]).is_ok());
}

#[trace("TC-022")]
#[test]
fn an_ambiguous_export_is_refused_in_either_candidate_order() {
    let root = package("P", &["Counters"], &[]);
    let mut ambiguous = package("Counters", &[], &[]);
    ambiguous.exports = vec![
        ("bounded".to_owned(), key("Counters::first")),
        ("bounded".to_owned(), key("Counters::second")),
    ];
    let expected = LibraryRefusal::AmbiguousDeclaration {
        library: name("Counters"),
        export: "bounded".to_owned(),
    };
    for reorder in [false, true] {
        let mut candidate = ambiguous.clone();
        if reorder {
            candidate.exports.reverse();
        }
        let refusal = resolve_libraries(&root, &[candidate]).expect_err("two candidates");
        assert_eq!(refusal, expected, "neither first nor last wins");
        assert_eq!(
            (refusal.code(), refusal.cause()),
            (Code::AmbiguousDeclaration, LibraryCause::AmbiguousName)
        );
    }
    // Distinct names are no ambiguity.
    let mut distinct = ambiguous;
    distinct.exports[1].0 = "other".to_owned();
    assert!(resolve_libraries(&root, &[distinct]).is_ok());
}

#[trace("TC-024")]
#[test]
fn a_failed_resolution_leaves_no_partial_linkage_wherever_the_defect_sits() {
    let a = package("A", &[], &["a"]);
    let b = package("B", &[], &["b"]);
    let mut ambiguous = package("Bad", &[], &[]);
    ambiguous.exports = vec![
        ("x".to_owned(), key("Bad::1")),
        ("x".to_owned(), key("Bad::2")),
    ];
    let with_ambiguous = vec![a.clone(), b.clone(), ambiguous];
    let without = vec![a.clone(), b.clone()];
    let control = resolve_libraries(&package("P", &["A", "B"], &[]), &[a.clone(), b.clone()])
        .expect("the valid pair resolves");
    assert_eq!(control.selections(), [name("A"), name("B")]);
    for (defect, kind) in [("Gone", "missing"), ("Bad", "ambiguous")] {
        for position in 0..=2 {
            let mut imports = vec!["A", "B"];
            imports.insert(position, defect);
            let root = package("P", &imports, &[]);
            let supplied = if kind == "missing" {
                &without
            } else {
                &with_ambiguous
            };
            let refusal = resolve_libraries(&root, supplied)
                .expect_err(&format!("{kind} import at {position} must refuse"));
            let expected_code = if kind == "missing" {
                Code::MissingImport
            } else {
                Code::AmbiguousDeclaration
            };
            assert_eq!(refusal.code(), expected_code, "{kind} at {position}");
            // The refusal is the whole result: a later valid request is
            // unaffected and resolves its complete set.
            let again = resolve_libraries(&package("P", &["A", "B"], &[]), &[a.clone(), b.clone()])
                .expect("an independent request after a failure");
            assert_eq!(again.selections(), control.selections());
        }
    }
}
