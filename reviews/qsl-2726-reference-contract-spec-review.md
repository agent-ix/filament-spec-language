---
id: SR-1541
title: "Canonical external reference conversion contract review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; /home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md"
review_set: base
---

## Summary

PASS. One independent specification review of the proposed QSL-2726 conversion contract, before production implementation. No high or medium defects found. No low findings retained.

Ticket: QSL-2726. PR: null. Branch: `codex/qsl-2726-reference-conversion`. Source SHA: `f89c2e03082342f690be623f6181977c4453d8ed`. Reviewer method: spec-review/base. Process module: `spec-artifacts-process@0.2.0`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

The contract requires authentic FSL model-stage authority joined with admitted structural rows; it explicitly prevents caller-built rows and opaque Exact projections from becoming authority. Both catalog directions compare full identities and deny conflicting declarations or universe bindings. Constructor and package authentication responsibilities remain with their actual owning stages, with a narrow trusted retention seam required if the pending merge lacks one. Semantic API signatures remain proposed rather than asserting nonexistent exports.

Native UTF-8 identity bytes have an unambiguous lowercase hexadecimal encoding and strict inverse; hex-looking input, Unicode, empty/invalid decoded identities, foreign declarations and member identities have observable acceptance or refusal obligations. Recursive conversion covers map keys and values and other nested carriers, restores native transition text before construction, and reapplies the owning canonical orders. The producer list includes temporal/task, protocol, probability, graph, sampled, refinement, hyper and certificate paths.

Selected population provenance is retained separately and checked against original request, package/model selection, full population key and admitted membership. The two-population counterexample prevents first-match reconstruction. Conversion leaves replay enabledness, membership, fairness, loop checks, state provenance and state-key bytes intact.

The resource contract charges complete comparisons, catalog storage, traversal, hexadecimal expansion and decoded text before the corresponding operations, preserves cumulative budgets and cancellation, and forbids partial publication and subtree copying. Existing typed refusal owners and RFC 6901 pointers remain authoritative. Future tests exercise functional errors and identity preservation rather than source inventories or tracking records.

Existing source-head/FSpec references are review context supplied by the assigned lanes, not a new file-tracking system. Package/model and population provenance are necessary feature authority, not a parallel audit framework. The review does not introduce a compatibility layer, new ticket, test debt or performance scope.

## Applicability and limits

This is a private unregistered contract with no allocated FR/AC identifiers. Identifier numbering, repository-wide link scans and implementation test matrices do not apply to this bounded contract review. No assurance-profile inspection or enforced workflow was authorized by the dispatch. No implementation, build, test, lint, runtime, matrix, code review or external-reference conformance result is claimed. No Rust API implementation was designed or reviewed; exact constructor types remain owned by the post-merge integration.

Only spec-review/base ran, as explicitly assigned. Optional analysis methods and implementation review methods were outside this one-review assignment. Source lane facts and file reservations were accepted without remeasurement. All existing Rust files and exports remain reserved until the owning model/report slice merges. This PASS concerns the written contract and is not human acceptance of an implemented converter.

## Examined scope

The following records include every nonempty authored line, including clean obligations, ownership boundaries and acceptance oracles. All entries are examined; none merely context-only.

```yaml
{
  "scope": [
    {
      "id": "qsl-2726-reference-conversion.md:L1",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "# QSL-2726 canonical reference conversion contract"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L3",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Status: proposed; ready for one independent specification review by Root. No"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L4",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "production implementation, validation result, or review acceptance is claimed."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L5",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "This is an ordinary private Markdown contract, not a registered Quire artifact"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L6",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "or a planning PR. Root owns the eventual published contract form, review, PR and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L7",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "merge. No requirement, test or artifact identifiers are allocated here."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L9",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Source head: `f89c2e03082342f690be623f6181977c4453d8ed`, branch"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L10",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "`codex/qsl-2726-reference-conversion`, worktree"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L11",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "`/home/peter/dev/.worktrees/luna-qsl-2726-reference`."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L13",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "## Scope and authority"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L15",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "This slice defines one package-bound conversion between native object references"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L16",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "and the backend reference wire contract, including inverse restoration at every"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L17",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "external reference carrier. It preserves native model semantics. It adds no"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L18",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "compatibility reader, alternate wire spelling, evidence framework or global"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L19",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "authority cache. The current session assignment defines this functional slice;"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L20",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "the broader Linear QSL-2726 issue is context, not an instruction source."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L22",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "The following boundary facts are accepted from the assigned peer lanes without"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L23",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "remeasurement:"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L25",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- Native projection uses universe NodeId, object-type EffectiveId and nonempty"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L26",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  authored UTF-8 ObjectId text. The native identity octets are exactly"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L27",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  `ObjectId::as_str().as_bytes()`."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L28",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- FSpec at `247a394157b58843643a61dfde16280c25d733c0`, FR-181 and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L29",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  `TypedValue.reference`, requires universe NodeId, object-type **declaration**"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L30",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  NodeId and lowercase, even-length hexadecimal identity octets."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L31",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- Normalized object-type preimages have `owner_effective_type = None`;"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L32",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  member preimages carry an owner. Inherited `A.x` member identities differing"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L33",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  under A/B are not a valid object-reference type collision fixture."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L34",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- IR at `1cbd86a48f02e4623e558a3d3b23c066c7f280bf` checks selected"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L35",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  structural declaration rows and conflicts, but cannot authenticate opaque"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L36",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  Exact payloads. Structural row matching alone confers no model authority."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L37",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- TimedStateWire/outcome base64url-wraps native StateKey bytes. Those bytes stay"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L38",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  opaque and unchanged."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L40",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "## Existing owners and the proposed surface"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L42",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "These existing types ground the design at the source head:"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L44",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| Existing owner | Use in this contract |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L45",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| --- | --- |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L46",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl_package::CheckedPackage` (`fsl-package/src/checked.rs`) | Source S4 checked typestate; distinct from wire admission. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L47",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `filament_contract_model::CheckedPackageV2` | Strictly admitted package, produced by `fsl-package/src/package_admission.rs`. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L48",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl_semantics::model::intake::SelectedModels` (`intake/unit.rs`) | Model-stage retained structural rows; `exact_identities(package, NativeJoinBudget)` performs the package join. Fixture/default instances do not establish authority. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L49",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl_semantics::model::state::StateModel` | Genuine model-owned type identity, type key, reference components and declaring-key resolution. Use work-aware resolution for unbounded comparisons. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L50",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl_eval::simulation::model::ModelSystem` | Existing `object_reference` and `type_key_of` owner; replay uses its original model context. This layer must not become a dependency of `fsl-package`. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L51",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `filament_exact::ObjectReference`, `EffectiveId`, `NodeKey`, `UniverseId` and `filament_checked::key::DeclarationKey` | Distinct native identity domains; never substitute display strings for identity equality. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L52",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl_semantics::model::population::PopulationBinding` | Admitted `population_key`, `universe`, `members`, type catalog and model selection. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L53",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl_eval::model_subject::ModelInstance` | Actual population DeclarationKey plus key String; retain the original selection. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L54",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `filament_semantic_value::model_transition::ModelTransition` | Native transition stores canonical operation/receiver/argument texts; restoration precedes construction. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L55",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `filament_contract_model::NativeJoinBudget` | Existing cumulative package-admission/join work and storage budget. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L56",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl_eval::simulation::model::{ModelKeyBudget, ModelRestoreBudget, ModelKeyError, ModelRestoreError}` | Existing runtime work/storage, meter, cancellation and restoration denial ownership. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L58",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "`TypedValue` below names the FSpec wire grammar. It is not asserted to be an"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L59",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "existing Rust enum: current certificate code validates `serde_json::Value`, and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L60",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "`ModelTransitionWire` uses borrowed/raw JSON input before construction."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L62",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Proposed names, all new: `ReferenceCatalog`, a package-bound conversion context,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L63",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "a selected-population context, and typed conversion failure discriminants."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L64",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Their unchecked constructors and all authority-bearing fields shall be private."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L65",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "The package crate owns the catalog and lower-layer identity join. The replay"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L66",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "crate owns recursive wire conversion, transition adaptation and population"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L67",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "selection restoration. No custom trait is needed solely to group methods."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L69",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Proposed operations (semantic signatures, not compilable declarations):"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L71",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- `ReferenceCatalog::from_source`: borrow the genuine source checked package,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L72",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  its authentic model-stage provenance, the corresponding emitted/strictly"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L73",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  admitted package and the existing join budget; return a validated catalog."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L74",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- `ReferenceCatalog::from_admitted`: borrow the strictly admitted package and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L75",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  genuine FSL model-stage identities/structural rows for its exact model"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L76",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  selections, with the existing join budget; return the same catalog semantics."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L77",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- `encode_reference` / `restore_reference`: take a typed package-bound context,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L78",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  native reference / strict wire reference, pointer and existing caller budget."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L79",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- Recursive value and `ModelTransition` encode/restore operations: take that"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L80",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  same context, value/transition and pointer; share the caller's budget."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L81",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "- `restore_over_binding`: additionally take the preserved original population"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L82",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  selection and admitted binding context; return the corresponding"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L83",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "  `ModelInstance` only after population/member validation."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L85",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "No public constructor shall accept arbitrary rows, a bare EffectiveId-to-NodeId"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L86",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "map, a fabricated provenance token or an unauthenticated Exact projection as"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L87",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "sufficient authority. Existing `AdmittedContext` retains package bytes, graph,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L88",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "enums, composites and units; it is not already an object-reference catalog."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L89",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "The eventual constructor wiring must retain authentic stage output across"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L90",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "source/package admission. If terra's merged interfaces do not expose that"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L91",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "provenance, implementation must add the narrow trusted retention seam in the"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L92",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "owning stage rather than relax this precondition. Its exact type/signature is"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L93",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "intentionally pending that merge and independent contract review."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L95",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "## Catalog admission and binding"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L97",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Catalog construction shall join genuine checking/model-stage object-type"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L98",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "identities to authentic FSL-produced structural selected-declaration rows admitted"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L99",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "against the same package binding. It shall compare complete EffectiveIds and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L100",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "complete structural declaration identities, including selected model identity"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L101",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "and version/selection context. A declaring-node-key recipe alone is insufficient."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L103",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "The catalog shall retain both lookup directions and each accepted universe/type"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L104",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "association. One full EffectiveId mapping to conflicting declarations, or one"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L105",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "declaration mapping to conflicting effective identities or incompatible universe"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L106",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "bindings, shall fail construction. Exact duplicate identical entries may be"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L107",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "coalesced after charged complete comparison. Insertion shall never overwrite a"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L108",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "conflict. Members shall never be admitted as object types."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L110",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Every conversion context shall be created by validation against this catalog's"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L111",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "actual checked/admitted package binding and genuine model context. Borrowed"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L112",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "handles may prevent accidental mixing, but lifetimes alone shall not be treated"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L113",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "as package authentication. Different package bytes/selections or a foreign model"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L114",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "shall fail before conversion; equal labels do not prove the binding. A digest,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L115",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "if used for lookup, is not a substitute for the required complete comparison."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L117",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "## Reference and recursive conversion"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L119",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Encoding shall resolve the native complete EffectiveId through the catalog,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L120",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "validate its universe/type association, and emit the corresponding declaration"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L121",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "NodeId and unchanged universe NodeId. It shall hex-encode every original UTF-8"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L122",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "identity byte exactly once with lowercase digits. Text that already looks like"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L123",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "hex remains UTF-8 input: native `ab` encodes as `6162`, not `ab`."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L125",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Restoration shall strictly validate the reference object's wire shape and NodeIds,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L126",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "resolve the declaration under the bound package, validate universe/type, reject"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L127",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "odd-length, non-hex or uppercase identity spelling, decode bytes, and reject"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L128",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "empty bytes or invalid UTF-8. It shall use the model owner's reference admission"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L129",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "to recreate the original ObjectReference, then its exact native canonical text"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L130",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "where a transition requires text. Arbitrary backend identity octets outside"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L131",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "nonempty UTF-8 are explicitly unsupported by the native restoration boundary."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L132",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "No lossy decoding, normalization, guessed identity or label reconstruction is"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L133",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "permitted."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L135",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "The single recursive converter shall cover references in receivers, arguments,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L136",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "collections (including map keys and values), records, tuples, options and union"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L137",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "payloads, preserving their type/member tags, shape, canonical ordering and all"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L138",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "non-reference values. Changed wire reference spellings can change wire sort"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L139",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "order: apply the owning wire grammar's canonical order after conversion and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L140",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "native canonical order after restoration. Preserve duplicate/refusal semantics."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L141",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Use exhaustive dispatch on closed typed variants and explicit refusal for an"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L142",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "unsupported wire variant. Never weaken the native/backend union decoder."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L144",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Operation identities, terminal-stutter tags and unrelated transition fields"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L145",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "shall retain their existing meaning. The receiver and each argument shall be"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L146",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "restored to exact native canonical text **before** `ModelTransition::new` runs."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L147",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Do not construct a backend-spelled transition and later relabel it as native."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L148",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "No internal projection, native transition canonical text definition, state-key"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L149",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "encoding, footprint, equality or symmetry changes belong to this slice."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L151",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "## Selected population and replay"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L153",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "An ObjectReference does not retain its selected population. Encoding shall"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L154",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "therefore preserve the actual original clause/subject selection outside the"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L155",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "reference and carry its provenance through the request/result adapter context."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L156",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "The selection shall be matched to its admitted PopulationBinding by complete"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L157",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "population key and original package/model selection, then checked against its"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L158",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "universe and admitted member/type relation. A raw caller-built ModelInstance is"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L159",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "not evidence of prior selection; the context must validate its request origin."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L161",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "`restore_over_binding` shall use that exact selected population and key. It shall"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L162",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "refuse absent selection, another population, incompatible universe/type or a"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L163",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "missing admitted member. It shall never select the first population matching a"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L164",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "universe/type/key, even when only one currently matches. This remains necessary"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L165",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "when two admitted populations share a universe, type and object key. Nested"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L166",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "reference identity restoration does not invent a population for every value;"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L167",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "population selection is additionally mandatory wherever a caller reconstructs"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L168",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "an `over`/subject ModelInstance or checks its membership."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L170",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Conversion is an identity boundary, not replay authentication. Existing"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L171",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "enabledness, membership, successor, fairness and loop checks shall still run on"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L172",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "the restored native values. Retain original package/request, initial-state and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L173",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "universe provisions and exact supplied post-state bytes. RestoredFromKey state"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L174",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "provenance shall not become an anchored initial observation or native witness."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L176",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "## Failure and resource contract"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L178",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Callers shall distinguish typed conditions for malformed wire, foreign"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L179",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "package/catalog, unknown declaration, conflicting mapping, incompatible"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L180",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "universe/type, invalid native identity bytes, missing population, wrong"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L181",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "population/member and resource denial. These are semantic condition names, not"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L182",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "new catalogued code spellings. Map them through the owning existing refusal"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L183",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "envelopes and RFC 6901 pointers; extend typed discriminants where needed without"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L184",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "introducing a parallel result/evidence system. Preserve cause objects and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L185",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "cancellation/resource denials; never branch on rendered messages."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L187",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Before performing the corresponding operation, charge catalog insertion and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L188",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "storage, full identity comparisons (including owner chains), recursive visits,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L189",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "wire output capacity, hex expansion and decoded/native text storage against"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L190",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "the caller's existing cumulative budgets. Check size arithmetic and capacity"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L191",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "conversion before allocation. A denied reservation stops the operation without"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L192",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "publishing a partially converted value/catalog. Do not reset budgets per value,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L193",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "refund prior charges, use a global cache to bypass accounting, or clone a full"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L194",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "JSON subtree per node. Retain existing depth/input ceilings and cancellation;"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L195",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "use borrowed traversal and a bounded work stack where needed."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L197",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "## File ownership and later integration"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L199",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Currently authorized: this private contract and worker report only. Future new"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L200",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Rust files: `fsl-package/src/reference_catalog.rs` and"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L201",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "`fsl-replay/src/reference_codec.rs`. All existing Rust files, including exports,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L202",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "remain reserved until terra's `peter/qsl-2726-package-remainder` model/report"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L203",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "slice (reported head `4d06bbabb7`, not frozen/PR) lands."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L205",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "After that merge, this slice owns the external egress/ingress/restoration"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L206",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "callers and package/replay exports below. All producers shall use the same"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L207",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "converter; temporal/task-only coverage is insufficient."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L209",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| Future integration owner | Required member/caller coverage |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L210",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| --- | --- |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L211",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl-replay/src/outcome.rs`, `outcome/counterexample.rs` | Temporal/task, queue growth, protocol `over` (handoff 1778), probability receiver (1880), graph binding (2038); shared transition guard (957), sampled (1058), refinement (1143), hyper (1750), graph (1987/2122). Numbers are peer handoff anchors, not stable source locations. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L212",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl-replay/src/certificate/task_product/wire.rs`, `certificate/zone_certificate/wire.rs` | Certificate transition encode/read/restore. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L213",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl-replay/src/spine/native_result/temporal.rs` and future model reader | Native-result writer/readback. Handoff path `temporal/model_read.rs` is absent at this source head; resolve its landed owner after terra's merge. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L214",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl-replay/src/model_check/replay.rs` | Restore with the actual package/model/request and selected context before replay. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L215",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| `fsl-analyze/src/analyze.rs`, `analyze/task_product_item.rs`, `zone_check/task_certificate.rs` | Model result and certificate writers. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L216",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "| Package/replay module exports | Expose the one authoritative catalog/converter after reservation ends. |"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L218",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "## Acceptance obligations for future implementation"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L220",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "These are proposed proofs, not passing tests. Use genuine local ConfigVersion"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L221",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "`model_state` fixtures (`two_objects`, `admit_initial`,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L222",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "`admit_current_snapshot`, `declared_type_universe_for`) and real model admission."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L223",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Do not fabricate packages or copy external fixtures. Tests shall show:"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L225",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "1. Genuine source and admitted catalogs yield the same actual declaration"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L226",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   NodeId; Unicode ObjectId bytes round-trip through a real"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L227",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   StateModel::object_reference, including hex-looking text."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L228",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "2. Invalid hex/odd length/uppercase, invalid UTF-8, empty ObjectId, unknown or"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L229",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   foreign declaration/package, incompatible universe/type, conflicting full"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L230",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   mapping, caller-created rows and inherited MEMBER EffectiveIds are refused"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L231",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   through typed failures at the relevant pointers."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L232",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "3. Two populations sharing universe/type/key restore only the explicitly"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L233",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   selected population; absent or wrong selection is refused."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L234",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "4. A reference at each nested value position and actual receiver/argument"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L235",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   round-trips to the same native canonical transition and equality behavior;"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L236",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   StateKey bytes remain exactly unchanged."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L237",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "5. Real protocol, probability and graph reference carriers, plus shared sampled,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L238",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   hyper and refinement adapters, use the converter. Task/certificate"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L239",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   serialize/read/serialize preserves canonical wire values."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L240",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "6. Existing replay checks still reject a well-shaped but semantically invalid"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L241",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   transition/witness with original request and state provisions retained."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L242",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "7. Denial at catalog insert/identity compare/traversal/hex output/decoded storage"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L243",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   stops before the denied operation and preserves the existing caller denial"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L244",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "   or cancellation without panic or partial result."
    },
    {
      "id": "qsl-2726-reference-conversion.md:L246",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "Contract-stage execution: no Cargo, tests, full gates, implementation review,"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L247",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "new ticket, PR, relay or authentication/configuration changes. Independent spec"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L248",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "review remains pending with Root. Final public artifact placement and exact"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L249",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "post-merge trusted constructor signatures remain Root/reviewer decisions; the"
    },
    {
      "id": "qsl-2726-reference-conversion.md:L250",
      "path": "/home/peter/dev/.agent/contracts/qsl-2726-reference-conversion.md",
      "role": "examined",
      "excerpt": "authority, population and accounting requirements above are not optional."
    }
  ]
}
```
