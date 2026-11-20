// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-763-AC-5: the message of an S2 refusal for a construct with no form
//! begins with the construct's row sentence.

use super::CompileRefusal;
use fsl_cst::Production;
use fsl_forms::{ConstructKind, FormsCause, FormsRefusal};
use ix_trace_rs::trace;

#[trace("FR-763-AC-5")]
#[test]
fn a_declaration_construct_message_begins_with_the_construct_and_names_its_owner() {
    for (declaration, expected) in [
        (None, "`unclaimed` declarations are not supported yet"),
        (
            Some("Owner"),
            "`unclaimed` declarations are not supported yet (declaration Owner)",
        ),
    ] {
        let failure = FormsRefusal {
            cause: FormsCause::UnrepresentedConstruct {
                production: Production::Declaration,
                construct: ConstructKind::Declaration,
                text: "unclaimed".into(),
                declaration: declaration.map(Into::into),
                profile: None,
            },
            span: None,
            recoveries: Vec::new(),
        };
        let refusal = CompileRefusal::Forms {
            failure,
            region: None,
        };
        assert_eq!(refusal.to_string(), expected);
    }
}
