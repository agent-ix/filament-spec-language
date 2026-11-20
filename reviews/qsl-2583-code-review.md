---
id: SR-2026111919340478491681419
title: "Code review — filament-spec-language Decimal checked types"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; Cargo.toml, Cargo.lock, fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/lowering/model.rs, fsl-semantics/src/check/source_model.rs, fsl-semantics/src/model/domain_package.rs, fsl-semantics/src/model/index.rs, fsl-semantics/src/model/intake.rs, fsl-semantics/tests/it/decimal_value_types.rs, tests/corpus/operations/parameters/pending/README.md, tests/fixtures/decimal-value-types.semantic-ir.json, tests/it/source_corpus.rs; FR-056-AC-43, FR-056-AC-63, FR-056-AC-66, FR-056-AC-67, FR-056-AC-68, FR-056-AC-71, FR-056-AC-72"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-056-admit-domain-package-model-declarations
---

## Summary

Reviewed PR #3138 at `0b002d1fe3caf0550050e5d88db69f67d33d28d0`, including the Decimal intake, checked-type propagation, tests, and Rust boundary behavior. The Decimal paths are substantially implemented, but recursive alias resolution has no depth bound and can exhaust the intake stack on a sufficiently long valid alias chain.

## Verdict

**CONDITIONAL** — one medium resource-bound defect needs resolution; the assigned focused test/build lane was not run by this reviewer.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Decimal alias resolution recursively follows caller-controlled type chains without a depth limit, allowing stack exhaustion during package intake | fsl-semantics/src/model/intake.rs:5402-5443 | implementation-bug-despite-evidence |

## Review Notes

`decimal_type_for_identity` recursively follows each alias target and allocates a `Vec<String>` entry per level. The chain is represented by a flat `types[]` array, so the package parser's nesting-depth behavior does not bound this recursion. Package input is byte-limited, but no alias-chain maximum is enforced here; a sufficiently long valid chain can overflow the thread stack rather than returning a refusal. Resolve chains iteratively or enforce a documented bounded depth before recursion. The repeated full-array identity lookup also makes repeated alias walks unnecessarily expensive.

The direct Decimal reader validates policy precision/scale and constraint coefficient bounds, and carries `DecimalType` through the checked domain, lowering, and source model. I did not identify another concrete implementation defect in the reviewed diff. No cargo commands were run per the dispatch instruction; focused checks remain with the lead's combined lane.

## Verdict Scope

- Examined FR-056-AC-43, AC-63, AC-66, AC-67, AC-68, AC-71, and AC-72 and the changed source/test files listed in `scope`.
- Rust style and safety references were applied. No applicable AssuranceProfile was found.
- The workspace CI configuration was not changed.
- Subsequent fixture/pin changes through `a6def083ab4b2322e6712ecb5c5f16a907ab01c8` were reviewed. The Decimal `adjust` result now explicitly declares multiplicity `[1, 1]` (`ordered: false`, `unique: true`) and `nullable: false`, matching other required scalar operation results in the repository. This preserves the AC-66 Decimal result type and adds no mismatch with the FR-056 operation-result contract. Fixture JSON syntax was checked; no builds or tests were run.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `decimal_type_for_identity` now walks aliases iteratively, uses the indexed identity map and cache, and detects cycles without recursive calls. The 4,096-alias regression fixture covers a long chain. |

## New findings (disposition pass 2)

Reviewed head `d758e24d25e4b9f5f2ae8960755914a9a18438a3` (`origin/codex/qsl-2583-decimal`, 20 commits over `origin/main` `5865aa2ffe`), with the rust-review lane folded in (rust-style applied). The reviewer ran no cargo build or test; findings come from reading the diff, FR-056, TC-3031/TC-3042 and the pinned dependency sources (`agent-ix-semantic-ir` at `c23980e`, `filament-exact` at `b26a8e4`). Cargo.toml and Cargo.lock are unchanged against main, so no pin was edited. No gate was narrowed: five `#[ignore]` markers were removed and none were added.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | high | `decimal_schema_diagnostic` sets aside every SCHEMA_VIOLATION whose pointer is exactly `/ir/types/N/decimal` once precision and scale are in range. That includes the validator's `forbid_extra` finding ("the member rounding is not one this schema admits", `semantic-ir` schema.rs:406, emitted at the policy pointer). The reader then reads only `precision` and `scale`, so a policy such as `{"precision":5,"scale":2,"rounding":"half_up"}` is admitted as an `exact` Decimal and a required validator diagnostic is lost | fsl-semantics/src/model/intake.rs:1955-2018 (line 2003), fsl-semantics/src/model/intake.rs:3676-3700 | implementation-bug-despite-evidence |
| FND-003 | medium | The AC-63 Cash test was replaced instead of being satisfied. Commit cdc63255bf deleted `target_bound_cash_without_policy_is_operator_ineligible`, which asserted AC-63/TC-3031 step 2 (`self.cash = self.cash` refuses `ill_typed`/`operator-ineligible`), and added `policy_less_cash_alias_member_refuses_once`: a policy-less alias that refuses at intake with DECIMAL_POLICY_MISSING. The new test is still tagged FR-056-AC-63, but AC-63 states the opposite outcome (Cash is admitted). If the arch ruling that a value_type may not carry `target` stands, AC-63 and TC-3031 need a spec amendment. Until then, the tag claims an outcome the test does not assert | fsl-semantics/tests/it/decimal_value_types.rs:298-334 | test-bent-to-implementation |
| FND-004 | medium | The AC-72 row "policy on a target-bound value type" passes for the wrong reason. It clones Price, removes `scalar` and adds `target`, so the validator's SCHEMA_VIOLATION comes from the value_type construct carrying `target`, whatever the policy is. Its oracle was changed from DECIMAL_POLICY_CONFLICT (the diagnostic AC-72 names) to SCHEMA_VIOLATION in cdc63255bf. The row cannot fail if policy handling breaks, and it disagrees with the AC-72 text | fsl-semantics/tests/it/decimal_value_types.rs:577-593 | test-bent-to-implementation |
| FND-005 | medium | `original-decimal-model/workflow.native-decimal.json` is now orphaned. The README paragraph that documented it was deleted, and `corpus_decimal_model_and_native_parameter_cases` stages only `workflow.decimal.json`. The original QSL-2583 native-Decimal-parameter repro (`delta >= 0.00` over a policy-typed native Decimal parameter) is neither run nor recorded as pending | tests/it/source_corpus.rs:1187-1284, tests/corpus/operations/parameters/pending/README.md:3 | correct-requirement-no-evidence |
| FND-006 | medium | A Decimal bound held by another node is silently dropped. That is a constraint whose `appliesTo` names the Decimal type but which sits in another node's `constraints`. The reader reads only the Decimal node's own list, and nothing refuses the foreign case, unlike the Rational guard at intake.rs:5651-5666. FR-056 says intake "never drops" a bound. Unverified whether the pinned validator admits such a document | fsl-semantics/src/model/intake.rs:3720-3760, fsl-semantics/src/model/intake.rs:5651-5666 | implementation-bug-despite-evidence |
| FND-007 | low | The new `constraint_owner` suppression in `read_domain_package` drops any reader refusal whose node contains `/constraints/` when the owner was validator-refused. That covers every value-type family, not just Decimal. It bypasses the `distinct_family` exemption and depends on a constraint-identity naming convention | fsl-semantics/src/model/intake.rs:6503-6516 | implementation-bug-despite-evidence |
| FND-008 | low | The AC-71 named items (`Decimal[0, 100; 2, 2; exact]`, coefficient, scale) are asserted only for soc 101/2. The soc -1/2 and 5/3, margin 1/-1 and tax 0.124 refusals check only `field` | fsl-semantics/tests/it/decimal_value_types.rs:503-524 | weak-oracle |
| FND-009 | low | `cyclic_decimal_alias_refuses_at_intake` checks only that the detail contains `target`, so any refusal mentioning `target` passes. It should assert ALIAS_CYCLE. It is also tagged FR-056-AC-63, which says nothing about cycles | fsl-semantics/tests/it/decimal_value_types.rs:336-354 | weak-oracle |
| FND-010 | low | `decimal_alias_walk_handles_long_chains_and_stops_cycles` has no `#[trace]`. The key-eligibility table adds an id `PM-34` that no spec defines, and its comment still lists PM-31..33 | fsl-semantics/src/model/intake.rs:7416-7483, fsl-semantics/src/model/intake.rs:13180-13196 | trace |
| FND-011 | low | Code smells: an unused `_package` parameter on `read_decimal_value_type`; a validator-message prefix match (`"the member unit is not one"`) used as control flow; and the cache-poison loop `for identity in visited { cache.borrow_mut().insert(identity, None) }` repeated six times in `decimal_type_for_identity`, which should be one helper | fsl-semantics/src/model/intake.rs:3651, fsl-semantics/src/model/intake.rs:2000, fsl-semantics/src/model/intake.rs:5531-5626 | style |

Checked and clean on this head:

- Map-key refusal of `ScalarValueDomain::Decimal` (FSpec FR-154): fixed at intake.rs:7106-7111 and tested as PM-32.
- DECIMAL_POLICY_MISSING and INVALID_OPERAND are kept once per policy-less root.
- The AC-71 refusal fields come from observation/document.rs:1683-1704.
- Leading-zero bounds (`0.0001` at scale 4) are read by `decimal_coefficient`.
- The `Half` walk re-reads `Soc` but is collapsed by the `(node, detail)` dedup at intake.rs:6481-6490, so no second refusal is added.
- `DecimalType::new` takes `u64` scales and narrows them with `u32::try_from` (filament-exact decimal.rs:397-408).
- The iterative alias walk is bounded by its `seen` set. No new panic sits on untrusted input: `unreachable!` is guarded by the keyword pre-pass, and the padding arithmetic is guarded by `fraction.len() <= scale`.
- Matches stay exhaustive in assemble.rs, lowering/model.rs and index.rs.
- Formatting changes are confined to new code. Every touched file is Decimal work.
