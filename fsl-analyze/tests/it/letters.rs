// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared letters helpers for the library and integration test targets.

/// What a letters seam reports for an atom's footprint.
pub(crate) type Reads = Result<
    std::collections::BTreeSet<fsl_semantics::check::footprint::SymbolicLocation>,
    fsl_semantics::check::footprint::FootprintRefusal,
>;

/// The footprint a letters seam reports when its run selects no
/// partial-order reduction, which is the only run that reads it.
pub(crate) fn unread() -> Reads {
    Ok(std::collections::BTreeSet::new())
}

/// The two guard methods of a letters seam for a run whose clauses are all
/// unguarded: the engine reads `activation` and `activation_reads` only for
/// a clause with a `when` condition, so reaching either is a defect.
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
