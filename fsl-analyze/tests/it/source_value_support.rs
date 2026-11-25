// SPDX-License-Identifier: AGPL-3.0-or-later
//! Values of source declarations, shared by split integration test binaries.

use filament_exact::Value;
use fsl_package::CheckedPackage;

/// The value of `case` of the source `enum Color` that `package` declares.
pub(crate) fn source_color(package: &CheckedPackage, case: &str) -> Value {
    let shape = package
        .graph()
        .scope()
        .enums()
        .iter()
        .find(|binding| binding.name == "Color")
        .expect("the unit declares Color")
        .shape();
    let variant = shape
        .variants()
        .find(|variant| variant.case() == case)
        .expect("a case of Color");
    let rank = shape.rank(variant).expect("a ranked case");
    Value::Enum(filament_exact::EnumMember::new(variant.clone(), rank))
}
