// SPDX-License-Identifier: AGPL-3.0-or-later
//! Valid unbounded Integer arguments wider than a machine word.

use crate::common::{interpreted, jitted, package_of, prepare, UNLIMITED};
use filament_exact::{Integer, Value};
use ix_trace_rs::trace;

/// FR-562-AC-5: an unbounded Integer argument remains valid when its value
/// exceeds the JIT's machine-width fast path.
#[trace("FR-560-AC-1", "FR-562-AC-5", "FR-563-AC-2")]
#[test]
fn a_wide_unbounded_integer_argument_matches_the_interpreter() {
    let package = package_of("function echoValue using v(x: Integer): Integer pure { x }\n");
    let prepared = prepare(&package, &["echoValue"]);
    let expected = "9223372036854775808";
    let arguments = vec![Value::Integer(
        expected
            .parse::<Integer>()
            .expect("a valid unbounded Integer"),
    )];

    let reference = interpreted(&package, "echoValue", &arguments, UNLIMITED);
    let actual = jitted(&prepared, "echoValue", &arguments, UNLIMITED);

    assert_eq!(actual.result, reference.result);
    assert!(actual.result.contains(expected), "{}", actual.result);
}
