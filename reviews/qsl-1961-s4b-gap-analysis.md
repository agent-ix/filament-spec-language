---
id: SR-2026111612172400000006
title: "gap analysis of PR 2786 role-internality consumer fact retirement"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; docs/consumer-facades.json, xtask/tests/consumer_facades.rs, tests/it/standard_pin.rs"
review_set: subset
---

## Summary

Ticket: QSL-1961 (S4b). PR agent-ix/filament-spec-language#2786 at pushed head 8454d9e4d, scoped to the diff. Plan completion: not assessed. This PR has no product code. It changes the bindings document and two sets of `#[ignore]` stubs (QSL-2228). They are checked here against FSpec origin/main `proposals/consumer-facades/consumer-facts.json`, whose protocol facts are `CF-PR-01`, `02`, `03`, `05`, `06` and `07`.

Examined and clean: the TC-2144 stub no longer expects `fsl_package::protocol_view::role_internality`; the `("protocol", "internality")` phrase is gone from `the_pinned_inventory_lists_the_required_facts`; the bindings no longer name `role_internality` or `RoleInternality`. The FR-760 standard pin is not built (there is no `filament-specification` Cargo dependency), so no tag move is needed. A tree-wide grep finds no `role_internality`, `RoleInternality` or internality reference outside FR-519's correct "no internality" text and an old review record.

The coder's unpushed amend 82d6051a3 fixes FND-001 and the TC-2146 stub. It does not change FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The bindings renumber `await_anchor`, `choice_visibility` and `attempts_maximum` to `CF-PR-04`, `05` and `06`. Against FSpec's inventory, the check will report `CF-PR-04` as a binding with no inventory fact. It will report `CF-PR-07` (attempts maximum) as unbound. `CF-PR-05` and `CF-PR-06` will pass while bound to the wrong facts. The TC-2146 stub (`retain(... != "CF-PR-04")`) also removes a retired id. | docs/consumer-facades.json:21-23, xtask/tests/consumer_facades.rs:146-150 |
| FND-002 | medium | `the_pinned_inventory_has_its_closed_shape` (the TC-1645 / FSpec FR-661-AC-1 stub) asserts consecutive ordinals with `format!("CF-{code}-{:02}", ordinal + 1)`. FSpec FR-661-AC-1 now says "strictly increasing ordinals from `01`" with retired ids never reused. Against the FSpec main inventory this assertion fails at `CF-PR-05`. Once QSL-2228 removes the ignore, the test will fail. Assert strictly increasing ordinals starting at 01 instead. | tests/it/standard_pin.rs:323 |

## Verdict

FAIL at 8454d9e4d. The internality accessor, phrase and stub are fully removed. The renumbered bindings mis-bind three facts. The closed-shape stub still encodes the consecutive-ordinal rule that FSpec #664 relaxed.
