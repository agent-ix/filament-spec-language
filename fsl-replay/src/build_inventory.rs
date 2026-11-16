// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-661: the tables and providers admitted by this build.

use serde::Serialize;

/// The deterministic inventory of the FSL build (FSpec FR-560).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BuildInventory {
    /// The application identity and its version.
    pub application: Application,
    /// The wire format identifier.
    pub format: &'static str,
    /// The language editions accepted by S1.
    pub editions: Vec<&'static str>,
    /// The header-selectable profile identities and direct requirements.
    pub profiles: Vec<Profile>,
    /// Complete-V1 capability families and their feature identifiers.
    pub families: Vec<Family>,
    /// Every complete-V1 feature identifier in normative order.
    pub features: Vec<String>,
    /// Providers linked into the supplied registry.
    pub consumers: Vec<Consumer>,
}

/// The application named by the inventory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Application {
    /// Stable application name.
    pub name: &'static str,
    /// The FSL crate version used to build this inventory.
    pub version: &'static str,
}

/// One header-selectable profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Profile {
    /// Exact header identity.
    pub identity: &'static str,
    /// Direct profile requirements in the layer table's order.
    pub requires: Vec<&'static str>,
}

/// One normative feature family.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Family {
    /// The family identifier (for example, `SRC`).
    pub identity: String,
    /// Feature identifiers owned by this family.
    pub features: Vec<String>,
}

/// One linked provider registered for the build.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Consumer {
    /// Provider backend identity.
    pub identity: String,
    /// Whether the provider was linked into the binary.
    pub origin: &'static str,
    /// Capability and mode pairs advertised by this provider.
    pub advertises: Vec<Advertisement>,
}

/// One advertised capability/mode pair.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Advertisement {
    /// Capability kind.
    pub kind: &'static str,
    /// Provider mode.
    pub mode: &'static str,
}

impl BuildInventory {
    /// Serialize this inventory as compact JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

/// Read the editions, profiles, families, features and providers this build
/// admits. All compiler-owned tables are read directly; no files, environment
/// variables or clocks are consulted.
pub fn build_inventory(registry: &fsl_route::Registry) -> BuildInventory {
    use fsl_semantics::value::layer::HeaderLayer;

    let ids = fsl_semantics::library::bundle::CapabilityId::complete_inventory();
    let features: Vec<_> = ids.iter().map(|id| id.as_str().to_owned()).collect();
    let mut families = Vec::<Family>::new();
    for feature in &features {
        let family = feature
            .strip_prefix("V1-")
            .and_then(|value| value.rsplit_once('-'))
            .map(|(family, _)| family)
            .unwrap_or_default();
        if families.last().is_none_or(|entry| entry.identity != family) {
            families.push(Family {
                identity: family.to_owned(),
                features: Vec::new(),
            });
        }
        families
            .last_mut()
            .expect("a family was added")
            .features
            .push(feature.clone());
    }

    BuildInventory {
        application: Application {
            name: "filament-spec-language",
            version: env!("CARGO_PKG_VERSION"),
        },
        format: "filament.build-inventory/v1",
        editions: vec!["1-draft"],
        profiles: HeaderLayer::ALL
            .into_iter()
            .map(|layer| Profile {
                identity: layer.identity(),
                requires: layer
                    .requires()
                    .iter()
                    .map(|required| required.identity())
                    .collect(),
            })
            .collect(),
        families,
        features,
        consumers: registry
            .descriptors()
            .map(|descriptor| {
                let mut advertises: Vec<_> = descriptor
                    .advertises()
                    .map(|(kind, mode)| Advertisement {
                        kind: kind.to_wire(),
                        mode: mode.to_wire(),
                    })
                    .collect();
                advertises
                    .sort_by(|left, right| (left.kind, left.mode).cmp(&(right.kind, right.mode)));
                Consumer {
                    identity: descriptor.id().as_str().to_owned(),
                    origin: match descriptor.origin() {
                        fsl_route::ProviderOrigin::Linked => "linked",
                        fsl_route::ProviderOrigin::Process => "process",
                    },
                    advertises,
                }
            })
            .collect(),
    }
}
