// SPDX-License-Identifier: AGPL-3.0-or-later
//! A declared model variant with an unknown case stays a type mismatch and
//! names the owner and case in the public check refusal.

use filament_semantic_value::location::{ExpressionLocation as Location, ExpressionOwner as Origin};
use fsl_foundation::Code;
use fsl_semantics::check::{CheckCause, CheckRefusal};
use fsl_replay::spine::CompileRefusal;
use ix_trace_rs::trace;

#[trace("FR-360-AC-5")]
#[test]
fn undeclared_variant_case_message_names_its_owner_and_case() {
    let refusal = CheckRefusal {
        location: Location::root(Origin::Expression),
        cause: CheckCause::UndeclaredVariantCase {
            owner: "OrderStatus".to_owned(),
            case: "Lost".to_owned(),
        },
    };

    assert_eq!(refusal.cause.code(), Code::IllTyped);
    assert_eq!(refusal.cause.cause(), Some("type-mismatch"));
    let compile_refusal = CompileRefusal::Check {
        refusals: vec![refusal],
        region: None,
    };
    let message = compile_refusal.to_string();
    assert!(message.contains("`OrderStatus`"), "{message}");
    assert!(message.contains("`Lost`"), "{message}");
}
