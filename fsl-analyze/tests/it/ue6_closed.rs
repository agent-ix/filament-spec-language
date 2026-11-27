// SPDX-License-Identifier: AGPL-3.0-or-later
//! UE-6, closed (event-position) universal and forall-exists forms: a state
//! whose guard has no value must refute with a replayable `undefined`
//! counterexample (ADR-023 HV-8), not the `ClosedUndefined` refusal.

use filament_exact::Cancel;
use fsl_replay::{HyperCounterexampleKind, HyperOutcome, ModelCheckLimits};
use ix_trace_rs::trace;

use crate::hyper_world::{Vault, World};

const QUOTIENT: &str = "function quotient using v(x: Integer): Integer pure { 1 div x }\n";
const GUARDED: Vault = Vault::Custom(
    "pre Guard using v on V::Vault::step { quotient(1 - self.l + self.h) >= 0 }\n\
     post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n",
);

fn closed(quantifier: &str, body: &str) {
    let declaration = format!(
        "hyper H using ep over behaviours (v: V::Vault) \
         forall trace a of V {quantifier} trace b of V {{ {body} }}"
    );
    let world = World::new_with_kernel_function(GUARDED, false, &[(0, 0)], &declaration, QUOTIENT);
    let run = world.run(ModelCheckLimits::default(), &Cancel::new());
    assert!(
        matches!(&run, Ok(HyperOutcome::Violated { counterexample, .. })
            if matches!(counterexample.kind, HyperCounterexampleKind::Undefined(_))),
        "expected an undefined counterexample (HV-8), got {run:?}"
    );
}

#[trace("FR-173-AC-7")]
#[test]
fn ue6_closed_universal_refutes_with_an_undefined_counterexample() {
    closed("forall", "eventually [0, 3] holds(v.h @ a = v.h @ a)");
}

#[trace("FR-173-AC-7")]
#[test]
fn ue6_closed_forall_exists_refutes_with_an_undefined_counterexample() {
    closed("exists", "eventually [0, 3] holds(v.h @ a = v.h @ b)");
}
