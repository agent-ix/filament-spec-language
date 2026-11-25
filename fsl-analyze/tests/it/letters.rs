// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared letters helpers for the library and integration test targets.

pub(crate) type Reads = Result<
    std::collections::BTreeSet<fsl_semantics::check::footprint::SymbolicLocation>,
    fsl_semantics::check::footprint::FootprintRefusal,
>;

pub(crate) fn unread() -> Reads {
    Ok(std::collections::BTreeSet::new())
}

macro_rules! unguarded {
    () => {
        fn activation(
            &self,
            _instance: &fsl_eval::model_subject::ModelInstance,
            _state: &fsl_eval::simulation::model::ModelSimState,
            _meter: &mut filament_exact::Meter,
        ) -> Result<fsl_eval::temporal::AtomReading, fsl_eval::model_subject::AtomStop> {
            unreachable!("an unguarded clause has no activation condition")
        }

        fn activation_reads(&self) -> $crate::letters::Reads {
            unreachable!("an unguarded clause has no activation condition")
        }
    };
}
pub(crate) use unguarded;
