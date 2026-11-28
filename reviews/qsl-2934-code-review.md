---
id: SR-967175026302197768900474181941935084853
title: "Code review: FSL PR #3892 package-route pre/post operation-contract negotiation"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@34f048e03595db5bb0a5f93a5d2413f883077946; PR #3892 against origin/main (merge base 8da4a15d16); fsl-analyze/src/analyze/document/package_run.rs, fsl-analyze/tests/it/analyze_package_provider.rs, fsl-analyze/tests/it/analyze_package_selection.rs; context fsl-analyze/src/analyze.rs (unadapted, settle), fsl-analyze/src/analyze/request.rs (contract_checker_bound)"
review_set: subset
---

## Summary

Ticket: QSL-2934. Code review with the rust-review lane, at head `34f048e03595db5bb0a5f93a5d2413f883077946`. The PR is a partial delivery: a red-main fix. FR-940-AC-28 evaluation parity and AC-33 to AC-38 remain open on the ticket.

The change removes the early `pending_requirement_record()` refusal in `ClaimReads::state_clause` for an explicitly named `pre`/`post` item. The item is now built from its admitted requirement row and negotiated, as an invariant item already was.

What I examined:

- Negotiation-settled outcomes. Ambiguous backend, unknown backend, empty candidate set and `requires_bound` now settle per item, with the source route's record and exit. Before, the whole request was refused `not-yet-implemented`. This matches FR-962-AC-1, FR-962-AC-3 and FR-940-AC-28's "never refuses with requirement record has no package form". Sibling items are no longer lost.
- Outcomes that need an evaluator. In `package_run.rs`, a pre/post item has `form: None`, so the package item is `AnalyzeItem::unadapted`, with `work: None`. `contract_checker_bound` adds work only for an unbounded extent (`RequiresBound`). So when negotiation picks `contract-checker` for a bounded extent, `settle` (analyze.rs:3141) reaches `work: None` and settles `unsupported` / `unsupported-requested-capability`. When negotiation picks a provider, it settles the same way and the provider is sent nothing (the asserted SMT branch). The source route would evaluate the item.
- The rewritten tests (judged in Verdict).
- Trace tags on the touched tests.

## Verdict

**CONDITIONAL: mergeable at this head as the red-main fix.** FND-001 and FND-002 stay tracked on QSL-2934, which must stay open. FND-003 is low.

Judgement on the questions asked:

1. **The `unsupported` settlement for evaluator-needing pre/post items.** No criterion allows it:
   - FR-940-AC-37 forbids it ("No operation-contract item settles `unsupported` ... for a contract evaluator the source route has").
   - FR-940 "Requirement records" forbids it ("never `unsupported` for a valid record").
   - The same clause also forbids main's refusal, as does FR-940-AC-28.
   - FR-940-AC-13 covers value-validity rows and FR-961's `Even` item. It says nothing about operation-contract items.
   - FR-950-AC-1 is the value-validity provider request.
   - FR-962-AC-1 is met: a `post` reaches the negotiator with exactly `[contract-checker]`.

   So both main and head violate FR-940. For a user, head is better on balance:
   - Every negotiation-decided case now equals the source route.
   - One item no longer takes its siblings down.

   It is worse in one narrow way. A bounded pre/post on `contract-checker` (or a provider-backed item) used to fail loudly: the whole request was refused, exit 20, with "not-yet-implemented ... no package form". Now it gets a per-item verdict that says the requested capability is unsupported. That record looks authoritative, but the source route proves the same item. The module doc still says the opposite policy (FND-001).
2. **The retained SMT assertion in `contract_package_reads_operation_contract_requirement_records`.** Keeping it as a known-gap pin is acceptable: it flips when the provider route lands. But it is traced to `FR-940-AC-6`, which requires "no package-route record is `unsupported`", so it inflates coverage (FND-002).
3. **The rewritten tests.** No assertion was weakened:
   - `named_operation_clauses_resolve_over_a_package` used to assert the source item's candidates. Now it asserts the package item's candidates `[contract-checker]`, `claim_name`, ordinal 0 and cause `requires-bound`, plus whole-item and exit equality with the source. That implies the old source-candidate assertion.
   - `shared_operation_contract_node_keeps_each_named_occurrence` keeps each name's ordinal (0 and 1) and adds `request.claim` equality, which carries the node and ordinal. That covers the old "the later name must not fall back to the first" message check. It also adds record equality and whole-document and exit parity for both items.
   - `unbounded_operation_contract_item_is_written_requires_bound`: the package half used `request_members`, which names `start.json`, a file `staying_counter_request` never writes. It now reuses the source request's own members. Every assertion line is unchanged.
4. **Trace tags.** See FND-002 and FND-003.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An explicitly named `pre`/`post` item that negotiates to `contract-checker` with a bounded extent, or to an external provider, now settles `unsupported`/`unsupported-requested-capability` (`AnalyzeItem::unadapted`, `work: None`). The source route proves the same item. This contradicts FR-940 "Requirement records" ("never `unsupported` for a valid record") and FR-940-AC-37. The cause misattributes the gap: the evaluator is missing on the package route, but the record says the backend does not support the capability. The module doc (package_run.rs:10-15) and `PendingPackageFamily` doc (:51-55) still say a family with no package form "refuses the request whole ... never settling `unsupported`". No test exercises the bounded `contract-checker` case on the package route: every fixture's Counter population is unbounded, so only `requires_bound` is covered. Disclosed in the PR body as remaining QSL-2934 work. | fsl-analyze/src/analyze/document/package_run.rs:3320-3321, 3264-3273, 10-15, 51-55; fsl-analyze/src/analyze.rs:3141-3143 |
| FND-002 | medium | `contract_package_reads_operation_contract_requirement_records` is traced `#[trace("TC-2410", "FR-940-AC-6")]`, which is wrong on three counts. (1) Its SMT branch asserts the package record is `unsupported-requested-capability` while the source record is `proved`, the opposite of AC-6's "no package-route record is `unsupported`". (2) AC-6's family list does not include operation contracts. (3) TC-2410 is FR-850's test case, not AC-6's TC-2958. The computed matrix counts this test as AC-6 coverage, while FR-940-AC-28 and AC-37, which it actually exercises (partially and negatively), stay untagged. The deprecated `/// Trace: FSL FR-940-AC-6, FSpec FR-825-AC-6` doc line remains. | fsl-analyze/tests/it/analyze_package_provider.rs:262-300 |
| FND-003 | low | Both rewritten selection tests keep tags that do not match what they now assert. `named_operation_clauses_resolve_over_a_package` pairs TC-2702 (FR-962's test case) with FR-489-AC-2 (verified by TC-1272); its new doc names FR-962-AC-1 and source/package parity (FR-940-AC-28). `shared_operation_contract_node_keeps_each_named_occurrence` is tagged FR-940-AC-7, which covers `claim_name` of `NotAboveThree`/`Bounded`/`AtMostTwo` and not shared operation-contract occurrences or settlement parity. | fsl-analyze/tests/it/analyze_package_selection.rs:714-719, 760-765 |

## Review Methods

- Code review and rust-review lane: read the `origin/main...HEAD` diff in full. Traced the package item build (`package_run.rs` item adapter, `contract_checker_bound`) through `analyze::settle` for each disposition. Checked `explicit_items == false` (default items): unchanged, because the removed guard only applied to explicit items. No new `unwrap`, panic, integer conversion or `unsafe` in production code. The only compiler warning (`model_check/check.rs` `expand`) predates the PR.

## Coverage

- Re-ran at head under the cargo flock (MemAvailable about 31 GB): `cargo test --locked -j 4 -p fsl-analyze --test it -- analyze_package_provider:: analyze_package_selection:: tied_item_resolution::` with the two QSL-3249 skips. Result: 58 passed, 0 failed. Log: `sr-3892/head-run.log`.
- Base evidence (45 passed, 5 failed on 8da4a15d16) read from the coder's `qsl2934-red.log`, not re-run.
- Not run: the workspace check (owned by the lead), whole-crate suites, or any reproduction of the bounded `contract-checker` case (FND-001 is traced from source, not reproduced).
