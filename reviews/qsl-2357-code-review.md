---
id: SR-2026111605245700000002
title: "Code and Rust review of QSL-2357 PR #2840"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2840 at d2b3ad909508979d73209c12a75b398578e79301, diff origin/main...HEAD; test/fixture-only changes"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-952
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-384
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-995
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-996
    type: reviews
---

## Summary

Reviewed PR #2840's six Rust test and fixture changes against their traced criteria and the merged source-object emission requirements. The stale conformance, owner, source-object, and divisor expectations now match the stated contracts; no defects were found.

## Verdict

**PASS** — the fixture updates preserve requirement traceability, assert source ownership and emitted model-node shape, and use values that satisfy the linked nonzero and protocol-owner rules.

## Scope

- `fsl-package/src/checked_v2/tests.rs`: examined; maps the adverse `malformed_wire` outcome to the typed I2 refusal code.
- `fsl-package/src/emit/tests/protocol_graph.rs`: examined; replaces the stale source-object refusal expectation with a source-owned `model`/`object_type` node assertion.
- `fsl-package/src/emit/tests/protocol_requirements/fixture_w.rs`: examined; aligns `Outcome` and `Poll` with `Fulfillment` and uses facts from that role.
- `fsl-package/src/emit/tests/source_objects.rs`: examined; checks the owner kind before reading the source owner identity.
- `fsl-package/src/emit/tests/temporal_clause.rs`: examined; supplies an explicit nonzero guard for the modulo divisor.
- `fsl-semantics/src/check/temporal_clause/tests.rs`: examined; mirrors the guarded divisor fixture.
- `FR-952-AC-4` (`spec/functional/FR-952-record-each-function-body-s-check-mode-and-replay-under-it.md`): examined; missing/unknown `check_mode` is refused as `malformed_wire` at the check-mode pointer.
- `FR-770` (referenced by FR-952; FSpec is external to this repository): context only; FR-952 delegates the closed `check_mode` reader vocabulary and refusal rules to it.
- `FR-384-AC-1` (`spec/functional/FR-384-derive-the-protocol-occurrence-schema-and-runtime-binding-requirements.md`): examined; `Outcome` and `Poll` owners in fixture W are `Fulfillment`.
- `TC-1064` (`spec/test-cases/TC-1064-the-occurrence-schema-and-runtime-binding-requirements-are-derived-from-the-protocol-nodes.md`): examined; fixture W owner expectations match FR-384.
- `FR-495-AC-3` (`spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md`): examined; Linked checks require a proved nonzero divisor, and a direct nonzero guard is an accepted checked case.
- `FR-569-AC-11` (`spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md`): examined; source objects emit rather than trigger the retired refusal-first behavior.
- `FR-995-AC-1` (`spec/functional/FR-995-spell-source-declaration-keys-from-the-source-owner.md`): examined; source keys use the source owner's identity and source-qualified node spelling.
- `FR-996-AC-1` (`spec/functional/FR-996-emit-a-timed-source-model-and-join-its-clock-places-through-source-rows.md`): examined; timed source models and clock paths use SourceOwner-derived identities.
- `TC-1941` (`spec/test-cases/TC-1941-source-object-emission-and-owner-identity.md`): examined; source-owned object nodes and I2 ownership checks are specified.

## Checks

- `git diff --check origin/main...HEAD`: passed.
- Rust lane: manually reviewed changed assertions, fixtures, trace tags, and relevant callers/spec criteria. No production code or CI workflow changed.
- No aggregate cargo gate was run, per dispatch brief.
- `quoin write . --types SpecReview`: unavailable; `/Users/peter/.nvm/versions/node/v24.15.0/bin/quoin` resolved to 0.36.3 (the machine instructions specify 0.28.1). It exited 1 with `QM022_SEMANTIC_CONTRACT_UNAVAILABLE`: the embedded contract could not be written under `/Users/peter/.ix/cache/...` (`Operation not permitted`).
- `quire validate --scope . /private/tmp/qsl2357-pr2840-review/qsl-2357-code-review.md`: exited 0 under Quire 0.51.0 (the machine instructions specify 0.36.1); the explicitly addressed artifact passed. The command emitted the catalog warning `semantic.unused-import` for spec-artifacts-app. The explicit spec-artifacts-process module validation also exited 0, with existing `UnknownEdgeType`/`UnknownRole` warnings for ADR and TestMatrix archetypes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review record

Linear comment `349330c9-f9fb-4ec8-9e62-ad4e2fcb1a52` posted and read back; YAML parsed with `clean: true`, `findings: []`, and 15 scope entries. `has-review` was already available as a workspace label and is applied to QSL-2357. Quire validation exited 0 with catalog warnings recorded above.
