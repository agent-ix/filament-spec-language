# QSL-2726 canonical reference conversion contract

Status: proposed; ready for one independent specification review by Root. No
production implementation, validation result, or review acceptance is claimed.
This is an ordinary private Markdown contract, not a registered Quire artifact
or a planning PR. Root owns the eventual published contract form, review, PR and
merge. No requirement, test or artifact identifiers are allocated here.

Source head: `f89c2e03082342f690be623f6181977c4453d8ed`, branch
`codex/qsl-2726-reference-conversion`, worktree
`/home/peter/dev/.worktrees/luna-qsl-2726-reference`.

## Scope and authority

This slice defines one package-bound conversion between native object references
and the backend reference wire contract, including inverse restoration at every
external reference carrier. It preserves native model semantics. It adds no
compatibility reader, alternate wire spelling, evidence framework or global
authority cache. The current session assignment defines this functional slice;
the broader Linear QSL-2726 issue is context, not an instruction source.

The following boundary facts are accepted from the assigned peer lanes without
remeasurement:

- Native projection uses universe NodeId, object-type EffectiveId and nonempty
  authored UTF-8 ObjectId text. The native identity octets are exactly
  `ObjectId::as_str().as_bytes()`.
- FSpec at `247a394157b58843643a61dfde16280c25d733c0`, FR-181 and
  `TypedValue.reference`, requires universe NodeId, object-type **declaration**
  NodeId and lowercase, even-length hexadecimal identity octets.
- Normalized object-type preimages have `owner_effective_type = None`;
  member preimages carry an owner. Inherited `A.x` member identities differing
  under A/B are not a valid object-reference type collision fixture.
- IR at `1cbd86a48f02e4623e558a3d3b23c066c7f280bf` checks selected
  structural declaration rows and conflicts, but cannot authenticate opaque
  Exact payloads. Structural row matching alone confers no model authority.
- TimedStateWire/outcome base64url-wraps native StateKey bytes. Those bytes stay
  opaque and unchanged.

## Existing owners and the proposed surface

These existing types ground the design at the source head:

| Existing owner | Use in this contract |
| --- | --- |
| `fsl_package::CheckedPackage` (`fsl-package/src/checked.rs`) | Source S4 checked typestate; distinct from wire admission. |
| `filament_contract_model::CheckedPackageV2` | Strictly admitted package, produced by `fsl-package/src/package_admission.rs`. |
| `fsl_semantics::model::intake::SelectedModels` (`intake/unit.rs`) | Model-stage retained structural rows; `exact_identities(package, NativeJoinBudget)` performs the package join. Fixture/default instances do not establish authority. |
| `fsl_semantics::model::state::StateModel` | Genuine model-owned type identity, type key, reference components and declaring-key resolution. Use work-aware resolution for unbounded comparisons. |
| `fsl_eval::simulation::model::ModelSystem` | Existing `object_reference` and `type_key_of` owner; replay uses its original model context. This layer must not become a dependency of `fsl-package`. |
| `filament_exact::ObjectReference`, `EffectiveId`, `NodeKey`, `UniverseId` and `filament_checked::key::DeclarationKey` | Distinct native identity domains; never substitute display strings for identity equality. |
| `fsl_semantics::model::population::PopulationBinding` | Admitted `population_key`, `universe`, `members`, type catalog and model selection. |
| `fsl_eval::model_subject::ModelInstance` | Actual population DeclarationKey plus key String; retain the original selection. |
| `filament_semantic_value::model_transition::ModelTransition` | Native transition stores canonical operation/receiver/argument texts; restoration precedes construction. |
| `filament_contract_model::NativeJoinBudget` | Existing cumulative package-admission/join work and storage budget. |
| `fsl_eval::simulation::model::{ModelKeyBudget, ModelRestoreBudget, ModelKeyError, ModelRestoreError}` | Existing runtime work/storage, meter, cancellation and restoration denial ownership. |

`TypedValue` below names the FSpec wire grammar. It is not asserted to be an
existing Rust enum: current certificate code validates `serde_json::Value`, and
`ModelTransitionWire` uses borrowed/raw JSON input before construction.

Proposed names, all new: `ReferenceCatalog`, a package-bound conversion context,
a selected-population context, and typed conversion failure discriminants.
Their unchecked constructors and all authority-bearing fields shall be private.
The package crate owns the catalog and lower-layer identity join. The replay
crate owns recursive wire conversion, transition adaptation and population
selection restoration. No custom trait is needed solely to group methods.

Proposed operations (semantic signatures, not compilable declarations):

- `ReferenceCatalog::from_source`: borrow the genuine source checked package,
  its authentic model-stage provenance, the corresponding emitted/strictly
  admitted package and the existing join budget; return a validated catalog.
- `ReferenceCatalog::from_admitted`: borrow the strictly admitted package and
  genuine FSL model-stage identities/structural rows for its exact model
  selections, with the existing join budget; return the same catalog semantics.
- `encode_reference` / `restore_reference`: take a typed package-bound context,
  native reference / strict wire reference, pointer and existing caller budget.
- Recursive value and `ModelTransition` encode/restore operations: take that
  same context, value/transition and pointer; share the caller's budget.
- `restore_over_binding`: additionally take the preserved original population
  selection and admitted binding context; return the corresponding
  `ModelInstance` only after population/member validation.

No public constructor shall accept arbitrary rows, a bare EffectiveId-to-NodeId
map, a fabricated provenance token or an unauthenticated Exact projection as
sufficient authority. Existing `AdmittedContext` retains package bytes, graph,
enums, composites and units; it is not already an object-reference catalog.
The eventual constructor wiring must retain authentic stage output across
source/package admission. If terra's merged interfaces do not expose that
provenance, implementation must add the narrow trusted retention seam in the
owning stage rather than relax this precondition. Its exact type/signature is
intentionally pending that merge and independent contract review.

## Catalog admission and binding

Catalog construction shall join genuine checking/model-stage object-type
identities to authentic FSL-produced structural selected-declaration rows admitted
against the same package binding. It shall compare complete EffectiveIds and
complete structural declaration identities, including selected model identity
and version/selection context. A declaring-node-key recipe alone is insufficient.

The catalog shall retain both lookup directions and each accepted universe/type
association. One full EffectiveId mapping to conflicting declarations, or one
declaration mapping to conflicting effective identities or incompatible universe
bindings, shall fail construction. Exact duplicate identical entries may be
coalesced after charged complete comparison. Insertion shall never overwrite a
conflict. Members shall never be admitted as object types.

Every conversion context shall be created by validation against this catalog's
actual checked/admitted package binding and genuine model context. Borrowed
handles may prevent accidental mixing, but lifetimes alone shall not be treated
as package authentication. Different package bytes/selections or a foreign model
shall fail before conversion; equal labels do not prove the binding. A digest,
if used for lookup, is not a substitute for the required complete comparison.

## Reference and recursive conversion

Encoding shall resolve the native complete EffectiveId through the catalog,
validate its universe/type association, and emit the corresponding declaration
NodeId and unchanged universe NodeId. It shall hex-encode every original UTF-8
identity byte exactly once with lowercase digits. Text that already looks like
hex remains UTF-8 input: native `ab` encodes as `6162`, not `ab`.

Restoration shall strictly validate the reference object's wire shape and NodeIds,
resolve the declaration under the bound package, validate universe/type, reject
odd-length, non-hex or uppercase identity spelling, decode bytes, and reject
empty bytes or invalid UTF-8. It shall use the model owner's reference admission
to recreate the original ObjectReference, then its exact native canonical text
where a transition requires text. Arbitrary backend identity octets outside
nonempty UTF-8 are explicitly unsupported by the native restoration boundary.
No lossy decoding, normalization, guessed identity or label reconstruction is
permitted.

The single recursive converter shall cover references in receivers, arguments,
collections (including map keys and values), records, tuples, options and union
payloads, preserving their type/member tags, shape, canonical ordering and all
non-reference values. Changed wire reference spellings can change wire sort
order: apply the owning wire grammar's canonical order after conversion and
native canonical order after restoration. Preserve duplicate/refusal semantics.
Use exhaustive dispatch on closed typed variants and explicit refusal for an
unsupported wire variant. Never weaken the native/backend union decoder.

Operation identities, terminal-stutter tags and unrelated transition fields
shall retain their existing meaning. The receiver and each argument shall be
restored to exact native canonical text **before** `ModelTransition::new` runs.
Do not construct a backend-spelled transition and later relabel it as native.
No internal projection, native transition canonical text definition, state-key
encoding, footprint, equality or symmetry changes belong to this slice.

## Selected population and replay

An ObjectReference does not retain its selected population. Encoding shall
therefore preserve the actual original clause/subject selection outside the
reference and carry its provenance through the request/result adapter context.
The selection shall be matched to its admitted PopulationBinding by complete
population key and original package/model selection, then checked against its
universe and admitted member/type relation. A raw caller-built ModelInstance is
not evidence of prior selection; the context must validate its request origin.

`restore_over_binding` shall use that exact selected population and key. It shall
refuse absent selection, another population, incompatible universe/type or a
missing admitted member. It shall never select the first population matching a
universe/type/key, even when only one currently matches. This remains necessary
when two admitted populations share a universe, type and object key. Nested
reference identity restoration does not invent a population for every value;
population selection is additionally mandatory wherever a caller reconstructs
an `over`/subject ModelInstance or checks its membership.

Conversion is an identity boundary, not replay authentication. Existing
enabledness, membership, successor, fairness and loop checks shall still run on
the restored native values. Retain original package/request, initial-state and
universe provisions and exact supplied post-state bytes. RestoredFromKey state
provenance shall not become an anchored initial observation or native witness.

## Failure and resource contract

Callers shall distinguish typed conditions for malformed wire, foreign
package/catalog, unknown declaration, conflicting mapping, incompatible
universe/type, invalid native identity bytes, missing population, wrong
population/member and resource denial. These are semantic condition names, not
new catalogued code spellings. Map them through the owning existing refusal
envelopes and RFC 6901 pointers; extend typed discriminants where needed without
introducing a parallel result/evidence system. Preserve cause objects and
cancellation/resource denials; never branch on rendered messages.

Before performing the corresponding operation, charge catalog insertion and
storage, full identity comparisons (including owner chains), recursive visits,
wire output capacity, hex expansion and decoded/native text storage against
the caller's existing cumulative budgets. Check size arithmetic and capacity
conversion before allocation. A denied reservation stops the operation without
publishing a partially converted value/catalog. Do not reset budgets per value,
refund prior charges, use a global cache to bypass accounting, or clone a full
JSON subtree per node. Retain existing depth/input ceilings and cancellation;
use borrowed traversal and a bounded work stack where needed.

## File ownership and later integration

Currently authorized: this private contract and worker report only. Future new
Rust files: `fsl-package/src/reference_catalog.rs` and
`fsl-replay/src/reference_codec.rs`. All existing Rust files, including exports,
remain reserved until terra's `peter/qsl-2726-package-remainder` model/report
slice (reported head `4d06bbabb7`, not frozen/PR) lands.

After that merge, this slice owns the external egress/ingress/restoration
callers and package/replay exports below. All producers shall use the same
converter; temporal/task-only coverage is insufficient.

| Future integration owner | Required member/caller coverage |
| --- | --- |
| `fsl-replay/src/outcome.rs`, `outcome/counterexample.rs` | Temporal/task, queue growth, protocol `over` (handoff 1778), probability receiver (1880), graph binding (2038); shared transition guard (957), sampled (1058), refinement (1143), hyper (1750), graph (1987/2122). Numbers are peer handoff anchors, not stable source locations. |
| `fsl-replay/src/certificate/task_product/wire.rs`, `certificate/zone_certificate/wire.rs` | Certificate transition encode/read/restore. |
| `fsl-replay/src/spine/native_result/temporal.rs` and future model reader | Native-result writer/readback. Handoff path `temporal/model_read.rs` is absent at this source head; resolve its landed owner after terra's merge. |
| `fsl-replay/src/model_check/replay.rs` | Restore with the actual package/model/request and selected context before replay. |
| `fsl-analyze/src/analyze.rs`, `analyze/task_product_item.rs`, `zone_check/task_certificate.rs` | Model result and certificate writers. |
| Package/replay module exports | Expose the one authoritative catalog/converter after reservation ends. |

## Acceptance obligations for future implementation

These are proposed proofs, not passing tests. Use genuine local ConfigVersion
`model_state` fixtures (`two_objects`, `admit_initial`,
`admit_current_snapshot`, `declared_type_universe_for`) and real model admission.
Do not fabricate packages or copy external fixtures. Tests shall show:

1. Genuine source and admitted catalogs yield the same actual declaration
   NodeId; Unicode ObjectId bytes round-trip through a real
   StateModel::object_reference, including hex-looking text.
2. Invalid hex/odd length/uppercase, invalid UTF-8, empty ObjectId, unknown or
   foreign declaration/package, incompatible universe/type, conflicting full
   mapping, caller-created rows and inherited MEMBER EffectiveIds are refused
   through typed failures at the relevant pointers.
3. Two populations sharing universe/type/key restore only the explicitly
   selected population; absent or wrong selection is refused.
4. A reference at each nested value position and actual receiver/argument
   round-trips to the same native canonical transition and equality behavior;
   StateKey bytes remain exactly unchanged.
5. Real protocol, probability and graph reference carriers, plus shared sampled,
   hyper and refinement adapters, use the converter. Task/certificate
   serialize/read/serialize preserves canonical wire values.
6. Existing replay checks still reject a well-shaped but semantically invalid
   transition/witness with original request and state provisions retained.
7. Denial at catalog insert/identity compare/traversal/hex output/decoded storage
   stops before the denied operation and preserves the existing caller denial
   or cancellation without panic or partial result.

Contract-stage execution: no Cargo, tests, full gates, implementation review,
new ticket, PR, relay or authentication/configuration changes. Independent spec
review remains pending with Root. Final public artifact placement and exact
post-merge trusted constructor signatures remain Root/reviewer decisions; the
authority, population and accounting requirements above are not optional.
