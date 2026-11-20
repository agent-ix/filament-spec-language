---
id: SR-1493
title: "Code review — forms and package behavior contracts"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2089; tests/it/forms_package_contracts.rs; tests/it/forms_package_contracts/routing.rs; tests/it/forms_package_contracts/source_package.rs; tests/it/main.rs"
review_set: subset
---

## Summary

Reviewed the four-file QSL-2089 diff frozen at `3b51f5d8f9ed7a5f2f71392fc29e9f8998ff8239`, applying code-review with its Rust lane. Eight new active tests exercise public conformance, relationship, registry and source/package boundaries with explicit positive and adverse controls.

## Verdict

**PASS** — no defect found in the changed tests or registration.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review evidence

The conformance table covers all eight independent parameter-type/result-multiplicity/effect combinations and asserts exactly the failing axes and typed causes, with a mandatory compatible control. Target resolution must agree with the actual ancestor relationship, and cyclic normalization must refuse both involved declarations. The three-edge budget checks the exact limit/consumed/next-charge tuple before the successful exact bound; the 10,000-edge case observes its worker result on the specified small stack.

Relationship normalization must retain the complete declared record, including absent multiplicity, and each independently missing end yields exactly one typed refusal identifying its relationship, missing declaration and member. Registry cases require stable complete registry state across three registration orders, exact evidence-kind candidates, permanent withdrawal under a conflicting identity, and the distinction between unknown and held wrong-kind backends. The malformed manifest pairs exercise descriptor admission before a valid counterpart reaches the real registry.

The source test performs actual parse/forms/check/link/emission/strict I2 admission. It requires three distinct named identities to appear as real decoded graph nodes, then compares them under unrelated declaration reordering. It does not equate a fixture to itself or manufacture an admitted result.

No production behavior, CI gate, warning setting, unsafe operation, compatibility branch, external copied fixture or schema is added. Test panics are deliberate failures of fixture/output invariants, not newly reachable production panic paths.

## Evidence and limits

Accepted the author's eight distinct focused passes and preserved compile/setup correction receipts without rerunning any test. No tests, builds, lint, matrix, complete-spec scan, source inventory, main-refresh check or Done-ticket audit was run. Existing warnings and lint limitations remain disclosed; this is not a green lint claim. The lead owns one final affected component gate. The broader historical QSL-2089 family list remains unclaimed by this slice; this review does not establish whole-ticket completion.

## Examined scope

```yaml
{
  "scope": [
    {
      "id": "FR-082-AC-1",
      "path": "spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md",
      "role": "examined",
      "excerpt": "Given a redefining operation whose parameter type, result multiplicity and effect frame each independently violate their axis, the checker's `Refused` outcome names all three failing axes with their own typed cause, not only the first one checked."
    },
    {
      "id": "FR-082-AC-2",
      "path": "spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md",
      "role": "examined",
      "excerpt": "Given a member redefinition naming a target absent from any supertype the owner conforms to, and separately a declared `supertypes` cycle, each is refused with its own named cause (redefinition-target, specialization-cycle) and no conformance edge is derived across the cycle."
    },
    {
      "id": "FR-082-AC-3",
      "path": "spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md",
      "role": "examined",
      "excerpt": "Given a conformance walk that follows `n` `supertypes` edges, the checker with `ancestor_steps` at `n − 1` stops `Incomplete` naming `ancestor_steps`, bound `n − 1` and count `n`, and reports neither conformance nor non-conformance for that walk; at `n` it gives the conformance verdict. A 10,000-long chain's conformance walk completes on a thread with a 512 KiB stack at the default `ancestor_steps`, and no outcome names a depth."
    },
    {
      "id": "FR-085-AC-1",
      "path": "spec/functional/FR-085-resolve-relationship-end-references.md",
      "role": "examined",
      "excerpt": "Given a relationship whose target end names a type absent from the admitted domain package, resolution refuses that relationship record with a `missing_declaration`/`missing-name` cause naming the missing type and the `target` end, and admits no binding for it; a relationship whose both ends resolve is admitted."
    },
    {
      "id": "FR-085-AC-2",
      "path": "spec/functional/FR-085-resolve-relationship-end-references.md",
      "role": "examined",
      "excerpt": "Given a relationship whose source end declares a role and a bounded multiplicity, the resolved binding retains that exact role and multiplicity; no default or inferred multiplicity appears for an end that declares none."
    },
    {
      "id": "FR-075-AC-1",
      "path": "spec/functional/FR-075-compute-candidates-from-registered-backends.md",
      "role": "examined",
      "excerpt": "For a non-probabilistic item, given a registry holding one backend that advertises the item's capability kind and no named backend, the candidate set is exactly that backend; a backend not advertising the kind is excluded. For `probabilistic-satisfaction`, a backend is a candidate only when it advertises both the kind and the item's exact evidence kind; neither `statistical` nor `exact` substitutes for the other. A state-graph item is also matched by exact form as FR-075-AC-21 specifies."
    },
    {
      "id": "FR-075-AC-2",
      "path": "spec/functional/FR-075-compute-candidates-from-registered-backends.md",
      "role": "examined",
      "excerpt": "Given two registries built by adding the same set of `BackendDescriptor` values in two different orders, the two registries are equal, they report the same registration refusals in the same order, and computing candidate sets for the same set of items against each yields identical sets in identical order for every item -- including probabilistic items and a set that contains an identical repeat of one identity's complete descriptor and a conflicting pair under another."
    },
    {
      "id": "FR-075-AC-3",
      "path": "spec/functional/FR-075-compute-candidates-from-registered-backends.md",
      "role": "examined",
      "excerpt": "Given a request naming a `BackendId` the registry does not hold, the result is the unknown-backend marker carrying that identity, distinguishable from an empty candidate set (which arises when a registered backend does not advertise the item's kind or exact probabilistic evidence kind, or no registrant advertises that required combination)."
    },
    {
      "id": "FR-075-AC-4",
      "path": "spec/functional/FR-075-compute-candidates-from-registered-backends.md",
      "role": "examined",
      "excerpt": "Given a registration naming a `BackendId` already held by the registry with an unequal descriptor, including one differing in `state_graph_forms`, the identity is refused once with `invalid_capability`/`duplicate-backend`, keyed by the identity; the held registration is withdrawn, the identity is permanently unregistered (a later registration of it, even an identical one, is refused on arrival, and a request naming it receives the unknown-backend marker), and registrations of other identities are unaffected."
    },
    {
      "id": "FR-057-AC-8",
      "path": "spec/functional/FR-057-admit-shared-capability-kinds.md",
      "role": "examined",
      "excerpt": "A backend registration advertising an absent or unknown kind or an unknown mode is refused with `invalid_capability` and its cause, keyed by backend identity; the refused registration contributes nothing, and any registration already held under that identity stands."
    },
    {
      "id": "FR-065-AC-2",
      "path": "spec/functional/FR-065-migrate-function-application-to-checked-family.md",
      "role": "examined",
      "excerpt": "Given a source file declaring one function and one call to it, the function declaration's checked identity is the same value read at three points: immediately after `check`, again after S4 linking, and again after decoding the emitted v2 bytes. Reordering unrelated top-level declarations in the source leaves that identity unchanged at all three points."
    }
  ]
}
```

