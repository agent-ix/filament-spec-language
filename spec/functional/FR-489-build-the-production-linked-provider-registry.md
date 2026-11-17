---
id: FR-489
title: "Build the production linked-provider registry for refinement witnesses"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-345
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-288
    type: depends_on
---
# FR-489: Build the production linked-provider registry for refinement witnesses

## Description

FSL SHALL build one production candidate registry from the real linked provider
engines exposed by `fsl_analyze::Engines::fsl()`. The `fsl-route::Registry`
remains the owner of registration admission and candidate computation under
FR-075. The engine collection supplies registrations from each engine's own
FR-288 descriptor; a descriptor without its corresponding implementation is
not a production provider registration.

For each FR-345 CLI invocation, the layering command SHALL obtain provider
registrations from `fsl_analyze::Engines::fsl()`, register each descriptor
once in a fresh `fsl_route::Registry`, and freeze the result as the invocation's
snapshot. The command SHALL NOT use a default empty snapshot, test-only
descriptors, or a process-global registry.

The current linked production sources in `fsl_analyze::Engines::fsl()` are
EN-1 (`temporal-satisfaction`), EN-4 (`probabilistic-satisfaction`), EN-5 (exact `probabilistic-satisfaction`), EN-7 (`schedulability`, FR-246, FR-633) and EN-6
(`temporal-satisfaction` over a dense-time subject, FR-243) under FR-281, and
the two refinement-witness providers FR-345 uses. The `ValueValidity` witness
is supplied by the value-validity check arm (`value-checker`, FSpec FR-483 and
FR-484). The `OperationContract` witness is supplied by the operation-contract
check arm (`contract-checker`, FSpec FR-483 and FR-485), which advertises
`operation-contract` in `bounded` mode. QSL-905 supplies the operation-contract
record the arm settles; it does not itself register or implement a provider.
EN-1 also advertises `operation-contract`, in the mode it advertises for
`temporal-satisfaction`, for an
`invariant` state clause over a model subject. Such an invariant claims that
it holds at every reachable state of the subject, a reachability question,
which EN-1's explicit-state search decides exhaustively with a certificate and
a replayed counterexample (FR-625); the contract checker decides an
operation's precondition, postcondition and frame over bounded inputs and has
no reachable-state set to search. The production snapshot SHALL therefore
settle the tie between the two `operation-contract` providers by the claim:

| `operation-contract` claim | Provider |
|---|---|
| An `invariant` state clause, when the request names a model subject | EN-1 |
| An `invariant` state clause with no model subject | `contract-checker` |
| An operation's `pre` or `post` contract, or a frame obligation | `contract-checker` |

The rule reads only the claim's form and whether a model subject is named,
never a caller preference, so one item always routes to one provider. The
production snapshot SHALL return each arm's descriptor as the candidate
outcome for its witness capability. The registry builder SHALL NOT insert a
placeholder for either capability.

| Witness capability | Registration owner/source | Current production status |
|---|---|---|
| `temporal-satisfaction` | EN-1 and EN-6 in `fsl_analyze::Engines::fsl()`; FR-281 owns the engine behavior and FR-288 owns conversion of each provider manifest into a descriptor. | Present; expose and register the existing EN-1 and EN-6 descriptors. |
| `probabilistic-satisfaction` | EN-4 in `fsl_analyze::Engines::fsl()`; FR-281 owns the engine behavior and FR-288 owns conversion of its provider manifest into a descriptor. | Present; expose and register the existing EN-4 descriptor. |
| `schedulability` | EN-7, the closed-form schedulability engine, in `fsl_analyze::Engines::fsl()` under `en-7`; FR-246 to FR-248 own its behavior and FR-288 the conversion of its provider manifest. FR-620 and FR-633 route schedulability items to it through this registry, so a linked set without it would leave every schedulability item with no candidate. | Register the existing EN-7 descriptor. |
| `ValueValidity` | The value-validity arm in `fsl_analyze::Engines::fsl()`; FSpec FR-483 and FR-484 own its behavior. | Present; expose and register its descriptor. |
| `OperationContract` | The operation-contract arm in `fsl_analyze::Engines::fsl()`; FSpec FR-483 and FR-485 own its behavior. QSL-905 supplies the operation-contract record it settles, not a provider. | Present; expose and register its descriptor. |

The current `fsl_analyze::Engines::fsl()` API does not expose its engine
descriptors, and `xtask` does not yet depend on `fsl-analyze`. FSL SHALL add a
public `Engines::registrations()` accessor that yields the descriptors owned
by the contained engines without constructing replacements. `xtask` SHALL add
`fsl-analyze` as a normal dependency and build each production snapshot by
iterating that accessor, registering each yielded descriptor exactly once in
a fresh `fsl_route::Registry`, and freezing the registry. These are required
API and dependency changes, not claims about existing code.

## Inputs

- The linked engine collection returned by `fsl_analyze::Engines::fsl()`.
- The descriptors exposed by the required public
  `fsl_analyze::Engines::registrations()` accessor.
- The `xtask` normal dependency on `fsl-analyze`.
- A fresh `fsl_route::Registry` for the current CLI invocation.

## Outputs

- A `RegistrySnapshot` containing every admitted linked provider descriptor
  supplied by the engine collection.
- A typed registry-build error when construction or registration fails; a
  failed provider registration is not silently omitted.

## Behavior

- For each CLI invocation, `xtask` SHALL build a fresh registry snapshot. A
  later invocation SHALL read no registry state from an earlier invocation.
- The production source SHALL include exactly the registrations exposed by
  `fsl_analyze::Engines::registrations()` on
  `fsl_analyze::Engines::fsl()`. The production source SHALL NOT mint a
  `BackendDescriptor` from a capability label alone.
- The production snapshot SHALL hold no process plugin. A plugin the driver
  starts from its provider configuration joins only an analyze run's
  registry, through FR-635, and never this snapshot.
- `fsl_route::Registry` SHALL admit each descriptor and compute candidates
  under FR-075; this requirement adds no candidate-matching rule. FR-962
  applies this requirement's `operation-contract` tie table when it computes
  an unnamed item's candidates.
- A test-only factory constructor SHALL accept an injectable registration
  source and registry-registration operation so tests can cause, separately,
  a descriptor-source error and a refused registration without changing the
  production engine collection or registry.
- The production factory SHALL return a typed error when the descriptor
  source fails or `Registry::register` refuses a descriptor.
- The CLI SHALL report FR-344 `tool failure`, exit 30, when the production
  factory returns an error.
- The CLI SHALL NOT report the empty candidate set or FR-345's expected
  unsupported result when the production factory returns an error.
- A production registry SHALL contain no registration solely to make a
  refinement-layering witness candidate set nonempty. Each witness
  capability's candidate is its real provider: `value-checker` for
  `ValueValidity` and `contract-checker` for `OperationContract`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-489-AC-1 | A production snapshot built from `fsl_analyze::Engines::fsl()` contains exactly the engine descriptors that collection exposes, including EN-1, EN-4, EN-6, EN-7, `value-checker` and `contract-checker`, and no descriptor without its engine implementation. | Test (TC-1272) |
| FR-489-AC-2 | Candidate queries over the production snapshot include EN-1 and EN-6 for `temporal-satisfaction` and EN-4 for `probabilistic-satisfaction` when the query supplies the evidence kind EN-4 advertises, as FR-075-AC-13 requires; the FR-345 `ValueValidity` witness capability gives exactly `value-checker` and the `OperationContract` witness capability gives exactly `contract-checker`. An `operation-contract` item naming an `invariant` state clause in a request that names a model subject gives exactly EN-1; the same item with no model subject, and an `operation-contract` item naming an operation's `pre` or `post` contract or a frame obligation, give exactly `contract-checker`. | Test (TC-1272) |
| FR-489-AC-3 | If the injected descriptor source fails or a production registration is refused, the factory returns a typed build error and the CLI reports tool failure; it never drops the descriptor and continues with a partial snapshot. | Test (TC-1272) |
| FR-489-AC-4 | Two independently built production snapshots are equal, and the production builder uses no process-global registry state. | Test (TC-1272) |
| FR-489-AC-5 | `Engines::registrations()` yields exactly the descriptors owned by the EN-1, EN-4, EN-5, EN-6, EN-7 (`en-7`, advertising `schedulability`), `value-checker`, `contract-checker` and `protocol-analysis` engines in `Engines::fsl()`; `xtask` declares `fsl-analyze` as a normal dependency and its production factory registers each yielded descriptor once. | Test (TC-1272) |
| FR-489-AC-6 | Test-only injected descriptor-source failure and registration refusal each return their corresponding typed build error; the CLI maps either to FR-344 tool failure, exit 30, without returning a partial snapshot or the FR-345 expected unsupported result. | Test (TC-1272) |

## Dependencies

- [FR-075](./FR-075-compute-candidates-from-registered-backends.md) owns
  registry admission and candidate computation.
- [FR-281](./FR-281-analyze-claims-with-in-process-engines.md) specifies the
  in-process engine collection and its current provider set.
- [FR-288](./FR-288-build-the-registry-from-provider-manifests.md) specifies
  the provider descriptor source used by each engine.
- [FR-345](./FR-345-run-the-profile-layering-refinement-gate.md) consumes the
  resulting snapshot for its layering witness requests.
- FR-345-AC-9, FR-345-AC-12 and FR-345-AC-13 read the witness candidates
  this snapshot gives.
- FSpec FR-483, FR-484 and FR-485 specify the value-validity and
  operation-contract check arms.
