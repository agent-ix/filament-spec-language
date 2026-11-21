---
id: SR-202611210022442313144108558
title: "gap-analysis of QSL-2739 PR #3385 registered composition provider path"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-1044-AC-1..14, FR-530-AC-1..10 (bounded to the changed FR-1044/FR-530 behavior); source and tests in BASE 97ac3336d6b6a660a4752486b39a276bb31c82c2..HEAD 13623d294863e357a3d854915a38d74f342c1b11"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: references
---

## Summary

Ticket: QSL-2739, PR #3385 at frozen head 13623d29. This is a planless gap audit, bounded by the brief to the FR-1044 and FR-530 behavior the PR changes and the source routes it touches. It is not a whole-spec audit. The installed `quoin` resolves 0.36.8 (the machine note says 0.30.0). `quoin matrix --repo . --json` exited 1 with `CORE_REFUSED: request exceeds the accepted size`, so the matrix was computed with the skill's documented fallback, `quire matrix --scope . --format json` (quire 0.52.0, engine 0.62.2), which exited 0. FR-1044-AC-7 and FR-530-AC-5 are retired. Every other criterion in scope is tagged except FR-1044-AC-10 and FR-530-AC-10.

## Verdict

**FAIL**: the skill's rule fails any untagged criterion, and two are untagged. FR-530-AC-10 is fixable inside this PR by tagging tests that already exist. FR-1044-AC-10 is the external package-parity half, kept OPEN by the architecture ruling. Reverse-gap and stub checks over the diff found no code without an owning requirement. `ProviderVerdict::Mismatch` in `report()` is a defensive arm that `finish` never reaches, not a stub. The coverage weaknesses inside tagged criteria (FR-1044-AC-6, AC-12 and AC-14) are recorded in the code-review artifact SR-202611210022442313144108557 and are not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-530-AC-10 is untagged. This PR changes its behavior: `guarantee_key` now empties `assumptions` under every rule but `assume-guarantee`, and a `component-guarantee` slot key carries the strictly ascending peer indices. Tests already assert it: fsl-package `a_non_assume_guarantee_rule_derives_a_request_item` checks the assumption arrays, and the CLI test `an_interface_composition_derives_no_assumption_set` checks `[[], []]`. Neither carries the FR-530-AC-10 trace. | fsl-package/src/emit/tests/composition.rs:535 |
| FND-002 | low | FR-1044-AC-10 is untagged, and its package-parity test is `#[ignore]`. Per the architecture ruling this is the external package-consumer API lane's open work (terra:QSL:package-consumer-api), not this PR's. It is recorded so the gap is not mistaken for coverage. | fsl-analyze/tests/it/analyze_composition_cli.rs:1435 |

## Coverage

- Plan completion: not assessed
- Matrix: computed by the `quire matrix` fallback after `quoin matrix` refused on size. In scope: FR-1044 has 12 criteria tagged, 1 untagged (AC-10) and 1 retired (AC-7). FR-530 has 8 tagged, 1 untagged (AC-10) and 1 retired (AC-5).
- Reverse gap and stubs: examined over every changed source file in scope. No unowned behavior, `todo!` or placeholder return was found.
- Semantic review: skipped. It is opt-in and was not requested in this non-interactive dispatch.

## Dispositions

Disposition round 1 at fix head fe5e3881a9987aefbe900da708cfb8743e22def4. Bounded to the fix commit. No repeated matrix or workspace run. The tags were checked in source.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `#[trace("FR-530-AC-1", "FR-530-AC-10", ...)]` is now on `a_non_assume_guarantee_rule_derives_a_request_item`, `#[trace("FR-1044-AC-5", "FR-530-AC-10")]` on `an_interface_composition_derives_no_assumption_set`, and FR-530-AC-10 on the new `components_of_one_package_are_distinct_assumed_peers`. |
| FND-002 | deferred: external | FR-1044-AC-10 package parity stays OPEN with its existing owner, terra:QSL:package-consumer-api. The ignored test's reason says so, and it carries no trace, so it claims no coverage. No new issue or scope. |

### Disposition round 1, continuation (final head 11c4db178296af2965c5a5e1c950c0776c956bc8)

No outcome changed. FR-530-AC-10 stays tagged. The removed fixture is replaced by `an_undeclared_peer_is_no_assumption` (fsl-package) and `only_a_declared_peer_is_an_assumption_of_the_slot_key` (root), both tagged FR-530-AC-10, and both passed at 11c4. FR-1044-AC-10 stays deferred: it is external, OPEN with terra:QSL:package-consumer-api, and its ignored test is untagged. Nothing new was untagged by the continuation.
