// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-860 "Capture faults", FR-327 "Undefined evaluation": an atom that reads
//! a capture of an `on each` clause is read for each activation with its own
//! captures, and an atom undefined for one activation is ranked against every
//! other activation as an undefined atom of the clause is: a false activation
//! before the first undefined position wins, and the reported position is
//! the first undefined one over all activations.

use std::collections::BTreeMap;

use filament_exact::{Meter, Undefined};
use fsl_eval::temporal::{
    evaluate, AtomReading, CaptureReading, LetterStop, TemporalLetters, TemporalOutcome,
    TemporalPosition, UndefinedCause,
};
use fsl_semantics::check::{FormulaNodeId, SCALAR_LIMITS_UNLIMITED};
use ix_trace_rs::trace;

use crate::temporal_trace::{finite, fixture, violated, EVENT};

/// Letters whose one `holds` atom reads a capture: its reading at a position
/// is scripted per activation, and true where no script says otherwise.
struct PerActivation {
    script: BTreeMap<(u64, u64), AtomReading>,
    fixed: Vec<u64>,
    active: Option<u64>,
}

impl PerActivation {
    fn new(script: &[((u64, u64), AtomReading)]) -> Self {
        Self {
            script: script.iter().cloned().collect(),
            fixed: Vec::new(),
            active: None,
        }
    }
}

impl TemporalLetters for PerActivation {
    fn atom(
        &mut self,
        _atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let active = self.active.ok_or_else(|| {
            LetterStop::Fault(filament_semantic_value::catalog::InternalFault::new(
                "test",
                "an-activation-is-selected",
            ))
        })?;
        Ok(self
            .script
            .get(&(active, position.index()))
            .cloned()
            .unwrap_or(AtomReading::True))
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Ok(AtomReading::True)
    }

    fn captures(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        self.fixed.push(position.index());
        self.active = Some(position.index());
        Ok(CaptureReading::Fixed)
    }

    fn reads_captures(&self, _atom: FormulaNodeId) -> bool {
        true
    }

    fn select_activation(&mut self, activation: TemporalPosition) -> Result<(), LetterStop> {
        assert!(
            self.fixed.contains(&activation.index()),
            "an activation is selected after its captures are fixed"
        );
        self.active = Some(activation.index());
        Ok(())
    }
}

fn undefined() -> AtomReading {
    AtomReading::Undefined(UndefinedCause::Kernel(Undefined::DivisionByZero))
}

/// `always[0,2] holds(c.level >= s)` `on each` with the capture `s`, over a
/// closed trace of four positions read through `letters`.
fn run(letters: &mut PerActivation) -> TemporalOutcome {
    let fixture = fixture(
        EVENT,
        "c: Counter",
        "on each (q: Counter)",
        "capture s: Int[0, 9] = q.level; always[0,2] holds(c.level >= s)",
    );
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    evaluate(&fixture.clause, finite(4), letters, &mut meter).expect("the trace is readable")
}

/// A later activation that is false at a position before an earlier
/// activation's undefined atom is the result: the claim fails at the first
/// refuting activation (FR-327 "Undefined evaluation").
#[trace("TC-2440", "FR-860-AC-4")]
#[test]
fn a_later_false_activation_before_an_earlier_undefined_atom_is_returned() {
    let mut letters = PerActivation::new(&[((0, 2), undefined()), ((1, 1), AtomReading::False)]);
    assert_eq!(run(&mut letters), violated(1));
}

/// The reported undefined position is the first one over every activation: a
/// later activation undefined at an earlier position than an earlier
/// activation's undefined atom names that earlier position.
#[trace("TC-2440", "FR-860-AC-4")]
#[test]
fn the_first_undefined_position_over_every_activation_is_returned() {
    let mut letters = PerActivation::new(&[((0, 3), undefined()), ((1, 2), undefined())]);
    let TemporalOutcome::Undefined(first) = run(&mut letters) else {
        panic!("an undefined outcome");
    };
    assert_eq!(first.at, TemporalPosition::new(2));
    assert_eq!(
        first.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
}
