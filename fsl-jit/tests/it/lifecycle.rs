// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-294, FR-561, FR-568: prepare limits, cancellation, kernel statuses
//! passed up unchanged, and the confinement of `unsafe`.

use filament_eval::{CallFailure, UnsupportedCause, UntranslatableConstruct};
use filament_exact::{Cancel, CancelCause, Integer, Value, ValueType};
use filament_semantic_value::supplied_stop::{SuppliedLimits, SuppliedRun};
use fsl_eval::execution::{ExecutionBackend, PrepareFailure, PrepareLimits, PreparedEntries};
use fsl_jit::Jit;
use ix_trace_rs::trace;

use crate::common::{
    int, interpreted, jitted, jitted_call, package_of, prepare, select, UNLIMITED,
};

const SOURCE: &str = "\
 function narrow using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 18] pure { a + b }\n\
 function id using v(a: Int[0, 9]): Integer pure { a + 0 }\n";

#[trace("TC-1385")]
#[test]
fn a_declared_result_bound_gives_the_interpreters_outcome_and_charges() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["narrow"]);
    for (a, b) in [(0, 0), (4, 5), (9, 9)] {
        let arguments = [int(a), int(b)];
        let want = interpreted(&package, "narrow", &arguments, UNLIMITED);
        let got = jitted(&prepared, "narrow", &arguments, UNLIMITED);
        assert_eq!(got.result, want.result, "{a} {b}");
        assert_eq!(got.charges, want.charges, "{a} {b} charges");
    }
}

#[trace("TC-1382")]
#[test]
fn a_call_with_a_cancelled_handle_returns_cancelled_requested() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["id"]);
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let result = fsl_eval::execution::PreparedEntries::call(
        &prepared,
        &crate::common::name("id"),
        &[int(1)],
        &Default::default(),
        &fsl_eval::execution::CallLimits {
            accounting: UNLIMITED,
            session: Default::default(),
        },
        &mut SuppliedRun::new(SuppliedLimits::default(), cancel.clone()).expect("a run"),
        &cancel,
    );
    assert!(matches!(
        result,
        Err(CallFailure::Cancelled(CancelCause::Requested))
    ));
    assert!(jitted_call(&prepared, "id", &[int(1)]).is_ok());
}

#[test]
fn prepare_stops_at_a_compile_work_unit_or_code_byte_limit() {
    let package = package_of(SOURCE);
    for limits in [
        PrepareLimits {
            compile_work_units: 0,
            ..PrepareLimits::default()
        },
        PrepareLimits {
            code_bytes: 0,
            ..PrepareLimits::default()
        },
    ] {
        let result = Jit.prepare(&package, &select(&["id"]), &limits, &Cancel::new());
        assert!(matches!(result, Err(PrepareFailure::Limit(_))));
    }
}

#[test]
fn prepare_with_a_cancelled_handle_is_cancelled() {
    let package = package_of(SOURCE);
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let result = Jit.prepare(
        &package,
        &select(&["id"]),
        &PrepareLimits::default(),
        &cancel,
    );
    assert!(matches!(
        result,
        Err(PrepareFailure::Cancelled(CancelCause::Requested))
    ));
}

/// FR-568: no `unsafe` outside `exec.rs`. `make arch-lint-unsafe` checks the
/// whole workspace; this is the in-crate check of the same rule.
#[trace("TC-1400")]
#[test]
fn unsafe_code_appears_only_in_the_exec_module() {
    let sources = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(sources).expect("src exists") {
        let path = entry.expect("an entry").path();
        let text = std::fs::read_to_string(&path).expect("readable");
        let code: String = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let uses_unsafe = code.contains("unsafe ") || code.contains("unsafe{");
        let allows = code.contains("allow(unsafe_code)");
        let is_exec = path.file_name().is_some_and(|name| name == "exec.rs");
        assert_eq!(uses_unsafe, is_exec, "{path:?}");
        assert!(!allows || is_exec || path.ends_with("lib.rs"), "{path:?}");
    }
}

const SIGNATURE_SOURCE: &str = " function id using v(a: Int[0, 9]): Integer pure { a + 0 }\n";

/// A test hook records a wrong signature: the call returns `Fault` without
/// entering the code.
#[trace("TC-1400", "FR-568-AC-3")]
#[test]
fn an_entry_with_a_wrongly_recorded_signature_returns_fault_without_being_called() {
    let package = package_of(SIGNATURE_SOURCE);
    let mut prepared = prepare(&package, &["id"]);
    prepared.record_signature_for_test(&crate::common::name("id"), 2, true);
    let result = jitted_call(&prepared, "id", &[int(1)]);
    let Err(CallFailure::Fault(fault)) = result else {
        panic!("expected a Fault");
    };
    assert_eq!(fault.invariant(), "jit-entry-signature-mismatch");
}

/// A test entry returns a status outside the closed status type: `Fault`
/// and no value.
#[trace("TC-1400", "FR-568-AC-3")]
#[test]
fn a_status_outside_the_closed_status_type_returns_fault_and_no_value() {
    let package = package_of(SIGNATURE_SOURCE);
    let mut prepared = prepare(&package, &["id"]);
    prepared.return_out_of_range_status_for_test(&crate::common::name("id"));
    let result = jitted_call(&prepared, "id", &[int(1)]);
    let Err(CallFailure::Fault(fault)) = result else {
        panic!("expected a Fault");
    };
    assert_eq!(fault.invariant(), "jit-entry-status-out-of-range");
}

/// The signature recorded at generation is read from the declared Cranelift
/// signature: one pointer parameter and a `u32` result.
#[trace("TC-1400")]
#[test]
fn a_generated_entry_records_the_entry_signature() {
    let package = package_of(SIGNATURE_SOURCE);
    let prepared = prepare(&package, &["id"]);
    assert_eq!(
        prepared.recorded_signature_for_test(&crate::common::name("id")),
        Some((1, true))
    );
}

const EDGE_SOURCE: &str = "\
 function narrow_mod using v(a: Int[0, 10], b: Int[0, 4000000000000000000]): Integer pure { (a mod 5) * b }\n\
 function fitting_mod using v(a: Int[0, 10], b: Int[0, 1000000000000000000]): Integer pure { (a mod 5) * b }\n\
 function unread using v(x: Int[0, 100000000000000000000]): Boolean pure { true }\n\
 function other using v(a: Int[0, 9]): Integer pure { a + 1 }\n";

/// The remainder is not monotone: `a mod 5` is 0 at both ends of `[0, 10]`
/// and 4 between them, so a product with `b` must not be judged to fit 64
/// bits from the ends. The entry is listed unsupported instead of faulting
/// at run time where the interpreter completes.
#[trace("TC-1385")]
#[test]
fn a_remainder_times_a_large_operand_is_not_judged_to_fit_from_the_interval_ends() {
    let package = package_of(EDGE_SOURCE);
    let prepared = prepare(&package, &["narrow_mod", "fitting_mod"]);
    assert_eq!(
        crate::common::unsupported_names(&prepared),
        vec!["narrow_mod".to_owned()]
    );
    let arguments = [int(9), int(1_000_000_000_000_000_000)];
    let want = interpreted(&package, "fitting_mod", &arguments, UNLIMITED);
    let got = jitted(&prepared, "fitting_mod", &arguments, UNLIMITED);
    assert_eq!(got.result, want.result);
    assert_eq!(got.charges, want.charges);
}

/// A parameter the body does not read, of a type wider than 64 bits, is
/// admitted and ignored exactly as the interpreter does.
#[test]
fn an_unread_wide_parameter_is_not_a_fault() {
    let package = package_of(EDGE_SOURCE);
    let prepared = prepare(&package, &["unread"]);
    assert!(crate::common::unsupported_names(&prepared).is_empty());
    let big = Value::Integer(Integer::from(1_i128 << 65));
    let want = interpreted(&package, "unread", std::slice::from_ref(&big), UNLIMITED);
    let got = jitted(&prepared, "unread", std::slice::from_ref(&big), UNLIMITED);
    assert_eq!(got.result, want.result);
    assert_eq!(got.charges, want.charges);
}

/// A function the caller did not select at `prepare` is a caller condition,
/// not a backend fault.
#[test]
fn a_function_not_selected_at_prepare_is_refused_not_faulted() {
    let package = package_of(EDGE_SOURCE);
    let prepared = prepare(&package, &["other"]);
    let result = jitted_call(&prepared, "unread", &[int(1)]);
    assert!(matches!(result, Err(CallFailure::Input(_))), "{result:?}");
}

const WIDE_SOURCE: &str = "\
 record Wide { w: Int[0, 100000000000000000000]; }\n\
 function wide_read using v(a: Int[0, 100000000000000000000]): Boolean pure { a > 0 }\n\
 function calls_wide using v(): Boolean pure { wide_read(5) }\n\
 function wide_field using v(r: Wide): Boolean pure { r.w > 0 }\n";

/// A value of an integer type wider than 64 bits has no register image, so a
/// body that reads one is not translated. It is never read as another value.
#[trace("TC-1385")]
#[test]
fn a_read_wide_integer_is_unsupported_and_never_substituted() {
    let package = package_of(WIDE_SOURCE);
    let names = ["wide_read", "calls_wide", "wide_field"];
    let prepared = prepare(&package, &names);
    let result = jitted_call(&prepared, "wide_read", &[int(5)]);
    assert!(
        matches!(result, Err(CallFailure::Unsupported(_))),
        "{result:?}"
    );
    assert_eq!(crate::common::unsupported_names(&prepared), names);
    let cause = &prepared.unsupported()[0].1.cause;
    assert!(
        matches!(
            cause,
            UnsupportedCause::Untranslatable {
                construct: UntranslatableConstruct::ValueType(value_type),
                ..
            } if matches!(**value_type, ValueType::Int(_))
        ),
        "{cause:?}"
    );
}

const GUARDED_SOURCE: &str = "\
 profile law = \"filament.value.integer-division.euclidean/v1\";\n\
 function narrow_div using v(b: Int[-5, 5], c: Int[0, 4000000000000000000]): Integer pure { if b != 0 then (10 div b) * c else 0 }\n\
 function fitting_div using v(b: Int[-5, 5], c: Int[0, 900000000000000000]): Integer pure { if b != 0 then (10 div b) * c else 0 }\n\
 function from_zero using v(b: Int[0, 5]): Integer pure { if b != 0 then 10 div b else 0 }\n\
 function nonnegative_mod using v(a: Int[0, 10], b: Int[-9223372036854775808, 0]): Integer pure { (a mod 5) + b }\n";

/// A guarded divisor that spans 0 reaches its extreme quotients at -1 and 1,
/// not at its ends, so the product is not judged to fit 64 bits from the ends.
#[trace("TC-1385")]
#[test]
fn a_quotient_by_a_divisor_spanning_zero_is_bounded_at_both_signs() {
    let package = package_of(GUARDED_SOURCE);
    let prepared = prepare(&package, &["narrow_div", "fitting_div", "from_zero"]);
    let result = jitted_call(
        &prepared,
        "narrow_div",
        &[int(1), int(4_000_000_000_000_000_000)],
    );
    assert!(
        matches!(result, Err(CallFailure::Unsupported(_))),
        "{result:?}"
    );
    assert_eq!(
        crate::common::unsupported_names(&prepared),
        vec!["narrow_div".to_owned()]
    );
    for b in -5..=5 {
        let arguments = [int(b), int(900_000_000_000_000_000)];
        let want = interpreted(&package, "fitting_div", &arguments, UNLIMITED);
        let got = jitted(&prepared, "fitting_div", &arguments, UNLIMITED);
        assert_eq!(got.result, want.result, "{b}");
        assert_eq!(got.charges, want.charges, "{b} charges");
    }
    for b in 0..=5 {
        let want = interpreted(&package, "from_zero", &[int(b)], UNLIMITED);
        let got = jitted(&prepared, "from_zero", &[int(b)], UNLIMITED);
        assert_eq!(got.result, want.result, "{b}");
        assert_eq!(got.charges, want.charges, "{b} charges");
    }
}

/// `mod` is never negative, so adding it to the least 64-bit integer stays
/// inside 64 bits and the entry is translated.
#[trace("TC-1385")]
#[test]
fn a_modulus_has_a_nonnegative_result_interval() {
    let package = package_of(GUARDED_SOURCE);
    let prepared = prepare(&package, &["nonnegative_mod"]);
    assert!(
        crate::common::unsupported_names(&prepared).is_empty(),
        "{:?}",
        prepared.unsupported()
    );
    for a in 0..=10 {
        for b in [i64::MIN, 0] {
            let arguments = [int(a), int(b)];
            let want = interpreted(&package, "nonnegative_mod", &arguments, UNLIMITED);
            let got = jitted(&prepared, "nonnegative_mod", &arguments, UNLIMITED);
            assert_eq!(got.result, want.result, "{a} {b}");
            assert_eq!(got.charges, want.charges, "{a} {b} charges");
        }
    }
}

/// Each function's sum fits 64 bits only when the remainder takes its law's
/// one-sided interval (`p`: dividend and divisor positive; `n`: divisor
/// negative; `z`: dividend not positive; `m`: dividend of both signs).
const REMAINDER_FUNCTIONS: &str = "\
 function p using v(a: Int[0, 10], d: Int[2, 5], b: Int[-9223372036854775808, 0]): Integer pure { (a rem d) + b }\n\
 function n using v(a: Int[0, 10], d: Int[-5, -2], b: Int[0, 9223372036854775807]): Integer pure { (a rem d) + b }\n\
 function z using v(a: Int[-10, 0], d: Int[2, 5], b: Int[0, 9223372036854775807]): Integer pure { (a rem d) + b }\n\
 function m using v(a: Int[-10, 10], d: Int[2, 5], b: Int[-9223372036854775808, 0]): Integer pure { (a rem d) + b }\n";

/// The `rem` result interval is the one the package's division law admits.
#[trace("TC-1385")]
#[test]
fn a_remainder_has_the_interval_of_the_selected_law() {
    for (law, refused) in [
        ("euclidean", vec!["n", "z"]),
        ("floor", vec!["z"]),
        ("truncating", vec!["n", "m"]),
    ] {
        let package = package_of(&format!(
            " profile law = \"filament.value.integer-division.{law}/v1\";\n{REMAINDER_FUNCTIONS}"
        ));
        let prepared = prepare(&package, &["p", "n", "z", "m"]);
        assert_eq!(
            crate::common::unsupported_names(&prepared),
            refused,
            "{law}"
        );
        for (function, dividends, divisors, addends) in [
            ("p", [0, 9, 10], [2, 5], [i64::MIN, 0]),
            ("n", [0, 9, 10], [-5, -2], [4, i64::MAX]),
            ("z", [-10, -9, 0], [2, 5], [4, i64::MAX]),
            ("m", [-9, 0, 9], [2, 5], [i64::MIN, 0]),
        ] {
            if refused.contains(&function) {
                continue;
            }
            // The sum leaves 64 bits at no pair the declared interval admits.
            for a in dividends {
                for d in divisors {
                    for b in addends {
                        let arguments = [int(a), int(d), int(b)];
                        let want = interpreted(&package, function, &arguments, UNLIMITED);
                        let got = jitted(&prepared, function, &arguments, UNLIMITED);
                        assert_eq!(got.result, want.result, "{law} {function} {a} {d} {b}");
                        assert_eq!(got.charges, want.charges, "{law} {function} {a} {d} {b}");
                    }
                }
            }
        }
    }
}
