// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-099-AC-8 (TC-894), the spine layer: an imported call whose argument
//! obligation is unproved is refused as the same call to a local function is,
//! and when the unproved call is inside an imported library the refusal is
//! the library's, wrapped in `CompileRefusal::Dependency` with the library's
//! dependency path. A call whose obligation is proved compiles in all three
//! positions.

use super::*;

const CALLER_PROVED: &str = "function caller using v(x: Int[0, 9]): Boolean pure { g::f(x) }\n";
const CALLER_UNPROVED: &str = "function caller using v(y: Int[0, 20]): Boolean pure { g::f(y) }\n";
const LOCAL_UNPROVED: &str = "function caller using v(y: Int[0, 20]): Boolean pure { f(y) }\n";

fn refusal_shape(refusal: &CompileRefusal) -> (SpineStage, Code) {
    (refusal.stage(), refusal.code())
}

/// FR-099-AC-8 (TC-894): the importing unit's unproved call to an imported
/// function refuses with the code, stage and cause of the same local call, and
/// as the unit's own `Check` refusal, never as a dependency refusal; the
/// proved call compiles.
#[trace("TC-894", "FR-099-AC-8")]
#[test]
fn an_unproved_imported_call_is_refused_as_the_local_call_is() {
    let geometry = library("test/geometry", "geometry", F);
    let dependencies = input(vec![geometry]);

    compile_as(
        "u",
        &unit(&format!("{}{CALLER_PROVED}", import("test/geometry", "g"))),
        &dependencies,
    )
    .expect("the call whose argument is proved compiles");

    let imported = compile_as(
        "u",
        &unit(&format!(
            "{}{CALLER_UNPROVED}",
            import("test/geometry", "g")
        )),
        &dependencies,
    )
    .expect_err("y: Int[0, 20] does not prove f's x: Int[0, 9]");
    let local = compile_as("u", &unit(&format!("{F}{LOCAL_UNPROVED}")), &dependencies)
        .expect_err("the same local call is refused");
    assert_eq!(refusal_shape(&imported), refusal_shape(&local));
    assert_eq!(imported.code(), Code::UndefinedExpression);
    let (
        CompileRefusal::Check {
            refusals: imported_refusals,
            ..
        },
        CompileRefusal::Check {
            refusals: local_refusals,
            ..
        },
    ) = (&*imported, &*local)
    else {
        panic!("both refuse as the unit's own check: {imported:?} / {local:?}");
    };
    assert_eq!(imported_refusals.len(), 1, "{imported_refusals:?}");
    assert_eq!(local_refusals.len(), 1, "{local_refusals:?}");
    assert_eq!(
        imported_refusals[0].cause.cause(),
        local_refusals[0].cause.cause()
    );
    assert_eq!(
        imported_refusals[0].location.path(),
        local_refusals[0].location.path()
    );
}

/// FR-099-AC-8 (TC-894): a library that itself makes the unproved call fails
/// when it is compiled for the importing unit: the refusal is the library's
/// own check refusal, wrapped as `Dependency` with path `[test/a]` and located
/// in the library's source, not the importing unit's. The same library with
/// the call proved compiles, and imports nothing from the broken one.
#[trace("TC-894", "FR-099-AC-8")]
#[test]
fn a_library_that_makes_an_unproved_call_is_refused_as_a_dependency() {
    let source = unit(&format!("{}{H}", import("test/a", "la")));
    let broken = library(
        "test/a",
        "a",
        &format!("{F}function bad using v(y: Int[0, 20]): Boolean pure {{ f(y) }}\n"),
    );
    let wrapped =
        compile_as("u", &source, &input(vec![broken])).expect_err("test/a's own call is unproved");
    assert_eq!(wrapped.code(), Code::UndefinedExpression);
    let CompileRefusal::Dependency { path, refusal } = &*wrapped else {
        panic!("expected a wrapped refusal, got {wrapped:?}");
    };
    assert_eq!(*path, [lib("test/a")]);
    let CompileRefusal::Check { refusals, .. } = &**refusal else {
        panic!("expected the library's check refusal, got {refusal:?}");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].cause.code(), Code::UndefinedExpression);
    assert_eq!(refusal.region().unwrap().source().identity(), "a");

    let proved = library(
        "test/a",
        "a",
        &format!("{F}function good using v(x: Int[0, 9]): Boolean pure {{ f(x) }}\n"),
    );
    compile_as("u", &source, &input(vec![proved])).expect("the proved library compiles");
}
