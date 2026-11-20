// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original S1 synthesis and ordered verification-plan near-neighbours.
use super::time_declaration::{accepts, refuses};
use fsl_cst::Production;
use ix_trace_rs::trace;

/// Provenance: QSL-2076; synthesis-decl mandatory grammar/domain/goal sequence.
#[test]
#[trace("FSpec-FR-339-AC-1")]
fn synthesis_declaration_refuses_near_misses() {
    accepts(
        "synthesis Controller using p grammar Candidate domain Inputs satisfies { valid };",
        Production::SynthesisDeclaration,
    );
    for (bad, token, expected) in [
        (
            "synthesis Controller using p grammar Candidate domain Inputs;",
            ";",
            "expected `satisfies`",
        ),
        (
            "synthesis Controller using p domain Inputs grammar Candidate satisfies { valid };",
            "domain",
            "expected `grammar`",
        ),
        (
            "synthesis Controller using p grammar Candidate domain { valid };",
            "{",
            "expected identifier",
        ),
        (
            "synthesis Controller using p grammar Candidate domain Inputs satisfies { valid }",
            "",
            "expected `;`",
        ),
    ] {
        refuses(bad, token, expected);
    }
}

/// Provenance: QSL-2076; verification-plan and verification-step ordered clauses.
#[test]
#[trace("FSpec-FR-339-AC-1")]
fn verification_plan_refuses_near_misses() {
    accepts("verify Safety using p { check Base claim Bounded method Exhaustive domain Inputs; check Next claim Stable method Induction domain Inputs depends [Base] bound 5; }", Production::VerificationPlan);
    for (bad,token,expected) in [
        ("verify Safety using p { check Base claim ; }", ";", "expected identifier"),
        ("verify Safety using p { check Base claim Bounded domain Inputs method Exhaustive; }", "domain", "expected `method`"),
        ("verify Safety using p { check Base claim Bounded method Exhaustive domain Inputs bound 5 depends [Earlier]; }", "depends", "expected `;`"),
        ("verify Safety using p { check Base claim Bounded method Exhaustive domain Inputs }", "}", "expected `;`"),
    ] {refuses(bad,token,expected);}
}
