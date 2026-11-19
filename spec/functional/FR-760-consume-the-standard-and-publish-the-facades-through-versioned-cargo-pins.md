---
id: FR-760
title: "Consume the standard and publish the facades through versioned Cargo pins"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/US-037
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/NFR-017
    type: references
  - target: ix://agent-ix/filament-spec-language/NFR-018
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-480
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-761
    type: references
  - target: ix://agent-ix/filament-specification/FR-660
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-760: Consume the standard and publish the facades through versioned Cargo pins

## Description

FSL SHALL build against a released version, named in Cargo, of every other
Filament repository it depends on. FSL SHALL read the standard's published
files only through the pinned standard library. FSL's consumer facades SHALL
be versioned with FSL's release, so a consumer that pins an FSL release keeps
building whatever FSL merges next (FSpec FR-660).

## Rationale

FSL's shared dependencies followed other repositories' `main` branches, and its
conformance tests read the standard's vectors from a checkout next to FSL's
own. A change on either side broke the other's build. Pinned releases make each
side move when it chooses, and let a clean clone run the conformance gate.

## Inputs

The workspace's Cargo manifests and lockfile, the pinned standard library
`filament_specification`, and a checked-package document.

## Outputs

A workspace build that resolves every Filament dependency to one release, vector
tests that read the pinned standard, and a reader that refuses another
checked-package document type.

## Behavior

### The standard as a pinned dependency

The workspace SHALL take the standard's Cargo package
`filament-specification-qualification` as a git dev-dependency with
`tag = "vX.Y.Z"`, used by every crate whose tests read a standard vector,
fixture, schema or catalog. Each such test SHALL read the bytes through
`filament_specification::published_file`, `published_files` or a named
constant of that library. No Makefile target, CI workflow, build script, `xtask` command,
source or test SHALL locate the standard through an environment variable, a
sibling checkout or another filesystem path. Because the standard arrives
through Cargo, the conformance and qualification targets run in a clean clone
under NFR-017 like every other target.

### Pins on every Filament dependency

Every dependency of a workspace manifest on another `agent-ix` repository SHALL
name a release (`tag = "vX.Y.Z"`) or a registry `version`; a `branch` dependency
SHALL NOT be committed. `cargo xtask pin-check [--manifest <path>]` reads every workspace manifest,
or the one manifest `--manifest` names, and fails with exit 1, naming each manifest and dependency, on an `agent-ix` git dependency
without a `tag`. `Cargo.toml` and `Cargo.lock` are the only record of which
release FSL builds against: no other file holds an expected version, hash or
digest of another repository. Local co-development uses `make use-local`'s
uncommitted `[patch]` override, which is never committed.

### FSL's facade version

Every crate that holds a module named in FSL's consumer-facade bindings
(FR-761) SHALL carry the workspace version of the root package, and FSL releases
them together under its release tag `vX.Y.Z` (NFR-018). A consumer pins them with
a git dependency on this repository with that tag and a `package` name. FSpec
FR-660's bump rules apply to FSL's release: removing or renaming a bound
accessor, changing its signature or meaning, or moving FSL's standard pin to a
breaking standard release is a breaking FSL release; adding an accessor is a
compatible one.

### Document type of the wire

FSL's emitter writes, and FSL's reader admits, the `contract_version` of the
pinned standard release, `filament.checked-package/v2` today. A document of any
other document type refuses through Contract IR's strict reader as
`unknown_contract_version` at `/contract_version`, and FSL's read path returns
`V2ReadRefusal::UnsupportedVersion` carrying the tag it read, with no package.
A dependency package supplied for linking (FR-480) is refused the same way
before it is linked.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-760-AC-1 | `cargo metadata --locked` for the workspace lists `filament-specification-qualification` from `https://github.com/agent-ix/filament-specification` with a `tag` source, and the published positive fixture `proposals/checked-package-v2/fixtures/positive-control-operations.json`, read from that pinned package, carries the `contract_version` FSL writes and reads (`fsl_package::CHECKED_PACKAGE_V2`). | Test (TC-2140) |
| FR-760-AC-2 | No file of the `Makefile`, `.github/workflows`, `xtask/src`, `src`, `tests` or any `fsl-*/src` and `fsl-*/tests` reads or sets the `FSPEC_DIR` variable or names a sibling `filament-specification` path, and no workflow step checks out `agent-ix/filament-specification`. | Test (TC-2140) |
| FR-760-AC-3 | `cargo xtask pin-check` passes over the workspace manifests, every `agent-ix` git dependency of which names a `tag`; over a scratch manifest with `filament-contract-model = { git = "https://github.com/agent-ix/filament-contract-ir", branch = "main" }` it exits 1 and names the manifest and `filament-contract-model`. | Test (TC-2141) |
| FR-760-AC-4 | The pinned standard's positive fixture with `contract_version` replaced by `filament.checked-package/v3` refuses through Contract IR's strict reader as `unknown_contract_version` carrying the tag `filament.checked-package/v3`, and no package is admitted. | Test (TC-2142) |
| FR-760-AC-5 | `cargo metadata` reports for `fsl-package`, `fsl-replay` and `fsl-analyze`, the crates holding FR-761's bound modules, the version of the root package `filament-spec-language`. | Test (TC-2143) |

## Dependencies

- FSpec FR-660 owns the versioning, pinning and bump rules this requirement
  applies.
- [NFR-017](../non-functional/NFR-017-build-a-clean-clone-with-locked-dependencies-and-run-the-documented-examples.md)
  runs the clean-clone build; [NFR-018](../non-functional/NFR-018-keep-every-release-tag-a-buildable-commit.md)
  owns release tags.
- [FR-761](FR-761-bind-every-consumer-fact-to-a-public-accessor.md) names the
  bound facade modules.
