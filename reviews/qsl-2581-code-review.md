---
id: SR-20261121044238
title: "code review of PR 3259 reserved slot-parameter names"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3259 head 6eae944d71 diff against main: fsl-semantics/src/check/{check.rs,mod.rs,protocol_obligation.rs,refusal.rs,region.rs}, fsl-semantics/src/check/protocol_obligation/tests/reserved_slot.rs, fsl-replay/src/spine/refinement.rs"
review_set: subset
---

## Summary

Ticket: QSL-2581. Code review with the Rust lane (rust-review, rust-style) of the change that refuses source identifiers in FSpec FR-402's reserved slot-parameter form (FR-488 "Reserved slot names", FR-488-AC-29, TC-3082). The reserved set comes only from contract-ir's exported `filament_contract_model::is_slot_parameter_name` (rev 7b76c8b, unchanged by this PR), the tag is the text before the first `_`, which is exactly the decoder's own `split_once('_')`, and the cause is boxed. Coverage of AC-29 is incomplete and one trace tag is wrong.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-29's clock-guard binder case (TC-3082 step 3, `pre F using v on Config::ConfigVersion::attemptUpdate { forall(slot_sx_sy in self.us: ...) }` over `us: Sequence<Timer>`) has no test; the quantifier test substitutes a function `forall` over `Sequence<Integer>`, a different checking path | fsl-semantics/src/check/protocol_obligation/tests/reserved_slot.rs:87-104 |
| FND-002 | medium | AC-29 / TC-3082 step 4 require each near-miss name to be admitted and to emit a `value`/`parameter` node of that name; the test only asserts the check returns no refusals and never emits or inspects the package | fsl-semantics/src/check/protocol_obligation/tests/reserved_slot.rs:132-144 |
| FND-003 | medium | Names refused through `Typer::bind` (quantifier, let, case, fold binders, model operation parameters) carry `span: None`, so the region is the enclosing binding node or clause, not the identifier's declaring region FR-488 names; the quantifier test only asserts the region contains the identifier | fsl-semantics/src/check/check.rs:3324; fsl-semantics/src/check/protocol_obligation/tests/reserved_slot.rs:103 |
| FND-004 | medium | Tests tag bare `FR-402-AC-12`, which resolves to this repo's FR-402 (no-std evaluator differential, no AC-12); the repo writes FSpec ids as `FSpec-FR-402-AC-..` (TC-3007, TC-3010) and TC-3082 asks for `("TC-3082", "FR-488-AC-29")` only | fsl-semantics/src/check/protocol_obligation/tests/reserved_slot.rs:73, fsl-semantics/src/check/protocol_obligation/tests/reserved_slot.rs:134 |
| FND-005 | low | Every protocol binder refusal (over input, capture, trigger, forward, retry, recovery) gets the `ProtocolSite::ActivationGuard` location whichever binder it is; the source region is right through the span, but the refusal's location misnames the site | fsl-semantics/src/check/protocol_obligation.rs:595 |
| FND-006 | low | `ReservedSlotName.tag` is a public `String` for a closed set of four tags (`slot`, `op`, `rcv`, `arg`), re-derived by a local split rather than taken from the decoder; an enum would make an invalid tag unrepresentable | fsl-semantics/src/check/refusal.rs:2903, fsl-semantics/src/check/check.rs:6202 |

## Verdict

Changes requested (medium findings FND-001 to FND-004). Checked and clean: the reserved set is contract-ir's decoder with no copied grammar, and the tag decoding matches it; `CheckCause::ReservedSlotName(Box<ReservedSlotName>)` keeps `CheckCause` from growing; code and cause (`ambiguous_declaration` / `ambiguous-name`) are catalogued beside the other ambiguous-name causes; the fsl-replay `check_limit` arm correctly returns `None` (not a limit); function parameters are refused once, at the identifier (exact-region assertion), before bodies are checked; protocol binders refuse at their `DeclaredName` span and `run` stops before re-binding them; the non-canonical escape `op_s_x41` (FR-488:309) and the near misses `op_s_xC3` (invalid UTF-8) and `op_s_xc3` (lowercase hex) are tested; the `bind` hook cannot refuse the checker's own synthesised hyper slot names (`v@a`, `a.step.op` are not identifiers); no assertion was weakened and no behaviour changes for names outside the form. Gates were not run (read-only review, no cargo); coder reports `reserved_slot` 4 passed, fsl-semantics clippy red on pre-existing main lints, fsl-replay compile check pending.

## Scope and evidence

- FR-488-AC-29 (spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md): examined.
- FR-488 "Reserved slot names" (spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md:301-324): examined.
- TC-3082 (spec/test-cases/TC-3082-a-source-identifier-in-the-reserved-slot-form-is-refused.md): examined.
- FSpec FR-402-AC-12: context only, through contract-ir `is_slot_parameter_name` at rev 7b76c8b.

## Dispositions

Disposition round 1, independently inspected at head `87cdd77465a665b8b80b798ee1e54e61a2a6124c` against original review head `6eae944d71`. The original findings above are preserved verbatim. No cargo or build command ran in this pass; focused execution evidence is supplied separately by draco:QSL:code.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `clock_places::a_clock_guard_binder_in_the_reserved_slot_form_is_refused` supplies a `Sequence<Timer>` field and `pre ... using` guard, asserts the reserved name, tag, quantifier span, code and cause. Both timed guard and time-invariant checking paths now call `reserved_slot_cause`. |
| FND-002 | fixed | The fsl-package near-miss test emits all seven names and requires a `parameter` node with the supplied name. |
| FND-003 | accepted-no-change | Arch explicitly accepts the declaring binding form as the smallest region carried today, including no available model-operation parameter region under FR-103. Commit `87cdd77465` states this fallback as system behaviour in FR-488. Exact binder-identifier spans remain scope on open QSL-2581 with binder-link work QSL-2728. |
| FND-004 | fixed | The two incorrect bare FR-402-AC-12 tags are removed; new and retained tests use TC-3082 and FR-488-AC-29. |
| FND-005 | fixed | The reserved protocol binder pass associates declaration, capture and compensation binders with their respective ProtocolSite values while preserving each DeclaredName span. |
| FND-006 | fixed | ReservedSlotName.tag is SlotTag, an explicit closed enum of Slot, Op, Rcv and Arg, with Display returning the wire spelling; the upstream decoder remains the authority for reserved-set membership. |

The reviewed fixes introduce no further finding. The added FR-488 conditional sentence is a precise exception to the preceding general region rule; it does not promise a source region when none is carried. Code-review disposition is mergeable under the arch ruling, subject to the lead's separate focused test result. QSL-2581 remains open for exact identifier spans with QSL-2728.
