// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-100-AC-8: replay-owned package model inputs and retained window results.

/// The evaluator selected by a package temporal request.
#[derive(Clone, Copy)]
pub struct PackageEvaluator<'a> {
    inner: &'a dyn fsl_eval::simulation::model::ModelEvaluator,
}

impl<'a> From<&'a dyn fsl_eval::simulation::model::ModelEvaluator> for PackageEvaluator<'a> {
    fn from(inner: &'a dyn fsl_eval::simulation::model::ModelEvaluator) -> Self {
        Self { inner }
    }
}

impl<'a, 'p> From<&'a fsl_package::BarePackage<'p>> for PackageEvaluator<'a> {
    fn from(inner: &'a fsl_package::BarePackage<'p>) -> Self {
        Self { inner }
    }
}

impl PackageEvaluator<'_> {
    pub(crate) fn as_inner(&self) -> &dyn fsl_eval::simulation::model::ModelEvaluator {
        self.inner
    }
}

/// The model tables and evaluator retained for a package temporal run.
#[derive(Clone)]
pub struct PackageModelCheckInput<'a> {
    inner: fsl_eval::simulation::model::ModelCheckInput<'a>,
}

impl<'a> From<fsl_eval::simulation::model::ModelCheckInput<'a>> for PackageModelCheckInput<'a> {
    fn from(inner: fsl_eval::simulation::model::ModelCheckInput<'a>) -> Self {
        Self { inner }
    }
}

impl<'a> PackageModelCheckInput<'a> {
    pub(crate) fn as_inner(&self) -> &fsl_eval::simulation::model::ModelCheckInput<'a> {
        &self.inner
    }

    pub(crate) fn into_inner(self) -> fsl_eval::simulation::model::ModelCheckInput<'a> {
        self.inner
    }
}

/// The original finite-window result retained when source attribution fails.
#[derive(Debug)]
pub struct PackageWindowEvaluation {
    inner: fsl_eval::temporal::WindowEvaluation,
}

impl From<fsl_eval::temporal::WindowEvaluation> for PackageWindowEvaluation {
    fn from(inner: fsl_eval::temporal::WindowEvaluation) -> Self {
        Self { inner }
    }
}

impl PackageWindowEvaluation {
    /// The original temporal result, before attempting source attribution.
    pub fn evaluation(&self) -> &fsl_eval::temporal::WindowEvaluation {
        &self.inner
    }
}
