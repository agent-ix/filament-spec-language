// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original checker and value boundary programs through the public source route.

use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{CheckedUnit, CompileRefusal, DependencyInput, SuppliedLibrary};
use fsl_semantics::model::intake::PackageInput;

mod evaluation;
mod guards;
mod imports;
mod observations;
mod state_keys;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

fn check(body: &str, dependencies: &DependencyInput) -> Result<CheckedUnit, Box<CompileRefusal>> {
    crate::support::front_end::checked(
        SourceIdentity::new("boundary-tests", "original:checker-boundaries"),
        "boundaries.native",
        format!("{HEADER}{body}").as_bytes(),
        &PackageInput::default(),
        dependencies,
    )
}

fn library(identity: &str, body: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.into(),
        source: SourceIdentity::new("boundary-tests", identity),
        path: format!("{identity}.native"),
        bytes: format!("{HEADER}{body}").into_bytes(),
    }
}
