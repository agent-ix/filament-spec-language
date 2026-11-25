// SPDX-License-Identifier: AGPL-3.0-or-later
//! Building a bare package's world refuses two definitions of one unit node
//! key that disagree. The views are attached directly, below the checks the
//! dependency join makes on names.

use super::{BarePackage, BareSupply};
use crate::bare_slot::BareSelectionFailure;
use crate::bare_world::BareWorld;
use crate::protocol_selection::SlotSelectionFailure;
use filament_contract_model::{CheckedPackageV2, NativeJoinBudget};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_semantics::check::PackageDeclarations;
use ix_trace_rs::trace;

struct Owner {
    package: CheckedPackageV2,
    identities: crate::ReadIdentityRows,
    equality: crate::ReadEqualityRows,
}

fn budget() -> NativeJoinBudget {
    crate::V2ReadLimits::fixture().native_join_budget()
}

/// The package `name`, whose unit `ms` is `1/divisor` of a second.
fn owner(name: &str, divisor: u32) -> Owner {
    declaring(name, divisor, "/")
}

/// [`owner`], whose derived dimension `Rate` is `Length {operator} Time`.
fn declaring(name: &str, divisor: u32, operator: &str) -> Owner {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n\
         dimension Time;\ndimension Length;\ndimension Rate = Length {operator} Time;\nunit s : Time = rational(1, 1);\nunit m : Length = rational(1, 1);\nunit r : Rate = rational(1, 1);\nunit ms : Time = rational(1, {divisor}) * s;\n\
         function in_ms_is_3000 using v(): Boolean pure {{ convert<Quantity<ms>>(3 s) = 3000 ms }}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("fixture", name),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the source parses");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        name.to_owned(),
        fsl_forms::build_unit(&parsed).expect("the unit forms"),
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles");
    let checked = crate::CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .expect("the unit checks"),
    );
    let emission =
        crate::emit_checked(&checked, crate::EmitLimits::fixture()).expect("the declarations emit");
    let package = crate::admit_checked_package(
        emission.package().bytes(),
        &Default::default(),
        crate::V2ReadLimits::fixture(),
    )
    .expect("the emission admits");
    let mut budget = budget();
    let identities = emission
        .identity_rows(&package, &mut budget)
        .expect("identity rows");
    let equality = emission
        .equality_witness(&package, &mut budget)
        .expect("equality rows");
    Owner {
        package,
        identities,
        equality,
    }
}

fn joined(owner: &Owner) -> BarePackage<'_> {
    BarePackage::new(&owner.package)
        .with_supply(
            BareSupply {
                identities: &owner.identities,
                equality: Some(&owner.equality),
            },
            &mut budget(),
        )
        .expect("the package joins its own witnesses")
}

/// Both public world constructors over `bare`.
fn worlds(bare: &BarePackage<'_>) -> [Result<BareWorld, BareSelectionFailure>; 2] {
    [bare.world(), bare.world_with_source_objects()]
}

/// Both constructors refuse `bare` with the unit-agreement fault.
fn assert_disagreement(bare: &BarePackage<'_>, case: &str) {
    for world in worlds(bare) {
        let Err(BareSelectionFailure::Slot(SlotSelectionFailure::Fault(fault))) = world else {
            panic!("{case}: {world:?}");
        };
        assert_eq!(
            fault.invariant(),
            "authenticated-unit-definitions-agree",
            "{case}"
        );
    }
}

/// Views that hold unequal unit definitions under one node key refuse both
/// world constructors; equal definitions form one world holding each once.
#[trace("TC-1331", "FR-531-AC-62")]
#[test]
fn unequal_definitions_for_one_unit_key_are_a_world_fault() {
    let (thousand, two_thousand) = (owner("dep", 1000), owner("dep", 2000));
    let other_root = owner("root", 1000);

    let alone = worlds(&joined(&thousand)).map(|world| world.expect("one view forms a world"));
    let mut equal = joined(&thousand);
    equal.dependency_views.push(joined(&thousand));
    for (world, alone) in worlds(&equal).into_iter().zip(alone) {
        let world = world.expect("equal definitions form one world");
        assert_eq!(world.types().units(), alone.types().units());
    }

    for (root, views) in [
        (&thousand, vec![&two_thousand]),
        (&two_thousand, vec![&thousand]),
        (&other_root, vec![&thousand, &two_thousand]),
    ] {
        let mut bare = joined(root);
        bare.dependency_views.extend(views.into_iter().map(joined));
        assert_disagreement(&bare, root.package.name());
    }
}

/// Dimension definitions are held to the same agreement: two views whose
/// derived dimension under one node key has other terms refuse, with every unit
/// definition equal.
#[trace("TC-1331", "FR-531-AC-62")]
#[test]
fn unequal_definitions_for_one_dimension_key_are_a_world_fault() {
    let (quotient, product) = (declaring("dep", 1000, "/"), declaring("dep", 1000, "*"));
    for (root, view) in [(&quotient, &product), (&product, &quotient)] {
        let mut bare = joined(root);
        bare.dependency_views.push(joined(view));
        assert_disagreement(&bare, "a dimension");
    }
}
