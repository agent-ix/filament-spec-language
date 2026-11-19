---
id: SR-2026111612172400000005
title: "spec review of PR 2786 role-internality consumer fact retirement"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-761-bind-every-consumer-fact-to-a-public-accessor.md, spec/functional/FR-519-publish-the-protocol-read-surface-the-filament-protocol-link-layer-imports.md, spec/test-cases/TC-2144-the-consumer-facade-contract-check-passes-over-the-committed-bindings.md, spec/test-cases/TC-2146-an-unbound-inventory-fact-fails-the-contract-check.md"
review_set: subset
---

## Summary

Ticket: QSL-1961 (S4b). PR agent-ix/filament-spec-language#2786 at pushed head 8454d9e4d, scoped to the diff. Base checklist, checked against FSpec origin/main e56f2959 (after FSpec #664). FSpec retired `CF-PR-04` and kept `CF-PR-05`, `CF-PR-06` and `CF-PR-07`. FSpec FR-661 says "the id of a removed fact is retired and never reused" and "An id is never reassigned to another fact". FSpec FR-217 Status says internality is a binding fact that the checked package and its read view never carry.

Examined and clean: FR-761 binding prose (the `role_internality` sentence is gone); FR-761-AC-1 and TC-2144's expected result (the `CF-PR-04` / `role_internality` clause is gone). FR-519:139 and FR-519-AC-19 still say a role carries no internality member and that `ChannelEndpoints` has no internality field. That agrees with FSpec FR-217 and FR-465. No other `role_internality` or `RoleInternality` reference remains in the tree.

At the pushed head, the commit renumbers the retired fact's successors. The coder's worktree has an unpushed amend, 82d6051a3, that restores `CF-PR-05..07` and moves AC-3 to `CF-PR-05`. It is not on the PR and this review did not assess it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-519's dependency line renumbers the protocol facts to `CF-PR-04` (`await_anchor`), `CF-PR-05` (`choice_visibility`) and `CF-PR-06` (`attempts_maximum`). FSpec main keeps these facts as `CF-PR-05`, `CF-PR-06` and `CF-PR-07`, and FR-661 forbids reassigning an id. So FR-519 now cites a retired id and two ids that FSpec gives to other facts (FSpec FR-661-AC-4). | spec/functional/FR-519-publish-the-protocol-read-surface-the-filament-protocol-link-layer-imports.md:256 |
| FND-002 | high | FR-761-AC-3 (and TC-2146) omits `CF-PR-04` and expects the check to name it as unbound. `CF-PR-04` is retired and is no longer an inventory fact. Against FSpec's inventory, omitting its binding is not an unbound fact, so a correct check cannot produce the result AC-3 expects. The criterion has to omit a live fact. | spec/functional/FR-761-bind-every-consumer-fact-to-a-public-accessor.md:108, spec/test-cases/TC-2146-an-unbound-inventory-fact-fails-the-contract-check.md:19,26 |

## Verdict

FAIL at 8454d9e4d. The internality removal itself is complete and correct. The renumbering contradicts FSpec FR-661 and the FSpec #664 retirement.
