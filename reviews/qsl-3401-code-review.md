---
id: SR-96720
title: "QSL-3400/QSL-3401 code review of the transition operation string and zone step verdict (filament-spec-language PR 3979)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@0b9befd5ad62ef2bc971fee598d8c7d6a299ce6c; git diff origin/main...HEAD (fsl-replay certificate, outcome, reference_codec, spine/request, statistical/settle; fsl-analyze tests; FR-245)"
review_set: subset
date: 2026-11-25
---

## Summary

Ticket: QSL-3401 (zone step-fails verdict), QSL-3400 (transition operation
string; remaining receiver work QSL-2790). PR filament-spec-language#3979,
head 0b9befd5ad62ef2bc971fee598d8c7d6a299ce6c. Rust lane (rust-review) folded
in.

Writer ruling: correct. FSL FR-120 ("Applications and transition
identities") writes `operation` as `"<qualified-name>"`, the declaration
identity text; FSpec FR-181 and schema `TransitionIdentity.operation`
(`Nonempty` string) agree. `ModelTransition::canonical_bytes` already wrote the
operation as a JSON string, so the identity bytes, every digest or key over
them, and the anchor text are unchanged; only the JSON outcome/codec spellings
change. No certificate or outcome digest is computed over these spellings
(searched fsl-replay certificate/product, outcome, reference_codec). The HP-1
relation edges have no reader.

Readers: `spine/request.rs::operation_text` and
`ReferenceCodec::restore_transition_in` read the new string and still read the
legacy object form (canonicalised as before); a non-canonical inner text is a
typed `NotCanonicalJson` refusal from `ModelTransition::new`.
`outcome/counterexample.rs::source_transition` already parsed a JSON-text
string; `proof_result/plugin_read.rs` already required a string. No downstream
repo under ~/dev reads the object form.

Zone verdict: `FailedStep::of` is one conversion shared by the simulation and
zone checkers; `refine_edge` is the only zone `StepFails` producer;
post-differs, backward-edge and extra-edge producers write `verdict: None`.
`unsound()` halts on `Stopped`/`Fault` before `FailedStep::of`, so the new
`RefinementStep` fault path is unreachable with a transition present.

Test substitution: `assert_tuple_conforms` now validates operation and
arguments and nulls only the reference receiver, asserting it is the native
FR-267 form; that gap is QSL-2790's (FR-267 "Remaining QSL-2726 canonical
reference conversion"). Acceptable.

Main interaction: origin/main 7fa83d2b05 (#3978) adds `#[implements]` tags in
spine/clause/temporal.rs and spine/lifecycle.rs only; no file overlap, merge
tree clean.

## Verdict

MERGEABLE. No high finding; the medium findings are pre-existing spec/wire
conflicts outside this change's scope, to be ticketed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The zone step-fails verdict now reaches the wire in forms FSpec FR-331's StepVerdict refuses: `NoAbstractMatch` with `transition` (FSL FR-149 l.114 requires it, FR-331 admits `{kind, position}` only), timed `AbstractClockGuard`/`AbstractTimeInvariant` (FR-379 kinds FR-331 excludes), `Undetermined` cause `ProtocolAssumptionNotEstablished` and locus `capture`/`invariant` (outside FSL FR-149 and FR-331). Pre-existing on FR-149; newly reachable on FR-245. Real conflicts; ticket. | fsl-replay/src/certificate/simulation.rs:304-399 |
| FND-002 | low | The operation reader rule (a string holding JSON text is identity text; any other string is a typed string value; an object is legacy) is in no FSL spec text, and a name spelled as a JSON literal (`true`, `1`) reads as a different identity. | fsl-replay/src/spine/request.rs:979-990; fsl-replay/src/reference_codec.rs:1380-1388 |
| FND-003 | medium | Task-source/temporal writers spell a source operation as the node URI, not FR-120's declaration identity text; readers rebuild the package by splitting the URI (counterexample.rs) or wrap it as a JSON string (task_product source_transition, used by native-result model_read), so the round trip is not FR-120's identity. One temporal counterexample now spells one operation two ways (steps vs where.transition / AbstractStepRejected). Pre-existing; ticket, not this PR. | fsl-replay/src/outcome.rs:618-629; fsl-replay/src/outcome/counterexample.rs:452-475; fsl-replay/src/certificate/task_product/wire.rs:446-481 |
| FND-004 | medium | State-graph evidence writes `undefined.where.state_key` and each GraphStep `post_state` as a bare base64url string; FSpec `Where.state_key` and `GraphStep.post_state` are State `{key}`, and GraphStep admits no `delay`. Confirmed by reading code and schema, not by a run. Separate producer and criterion; ticket with a failing test, not this PR. | fsl-replay/src/outcome/counterexample.rs:2297-2309, 2428-2432 |
| FND-005 | low | `FailedStep::of` is tagged FR-149-AC-8 (the wire form); it implements the verdict a StepFails rejection carries (FR-149-AC-3, FR-245-AC-8), and writes nothing. | fsl-replay/src/certificate/simulation.rs:280 |

## Coverage

- Producers checked: TimedTransitionWire (relation edges, graph steps, not-stochastic, undefined where.transition, AbstractStepRejected), reference codec generic and task arms, CertificateLocus::ZoneEdge (zone_certificate, product, task_check).
- Readers checked: spine/request model_step, reference codec restore, counterexample source_transition, task_product source_transition, plugin_read, native_result model_read.
- Tests run at head: see the Linear comment.
