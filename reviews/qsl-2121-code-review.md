---
id: SR-2700003
title: "code review of PR 2725 analyze request and result documents"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 2725 head da05353e1420d36513426526843edfe21497949c; fsl-analyze/src/analyze/document.rs, fsl-command/src/command/analyze.rs, fsl-command/src/command/wire.rs, fsl-command/src/command.rs, fsl-replay/src/outcome.rs, fsl-replay/src/model_check/settle.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: references
---

## Summary

Ticket: QSL-2121. Code review with the rust-review lane (rust-style loaded) of `git diff origin/main...HEAD`. The reader, resolver and FR-621 writer are sound on the single-unit Counter path. Four spec divergences remain: default-item order, the certificate `model` source, the default subject's incomplete populations, and refused `package` selections.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Default items rank units by `graph.sources()`, which is canonical `(authority, identity)` order, but FR-620 says the program's units in request order. For a request whose `libraries` unit declares a claim and sorts before the program, the order differs. Fix: pass the request-order unit list (program, then `libraries` as listed) from `run_input` into `resolve_items` and rank by its position. | fsl-analyze/src/analyze/document.rs:818 |
| FND-002 | medium | `certificate.identity.model` comes from the first initial-state snapshot's `model.identity` and is `null` when the subject has none. A protocol subject or an empty `initial_states` writes `model: null`, but the CertificateIdentity schema requires a non-empty string, and FR-621 and FSpec FR-201 name the subject model that owns the obligation. Fix: take it from the checked package's subject model selection (`graph.model_selections()`), and refuse or omit the identity instead of writing `null`. | fsl-analyze/src/analyze/document.rs:1130-1138 |
| FND-003 | medium | The default subject puts every initialization snapshot into `universes`, then `check_universes` refuses `incomplete_population`. FR-620 says the default subject's universes are the complete populations those snapshots declare, so it should leave the incomplete ones out, not refuse. Fix: in `default_over` and `binding`, keep only complete populations, and refuse only for an explicit `subject.universes`. | fsl-analyze/src/analyze/document.rs:90-130 |
| FND-004 | medium | A `package` selection, and any `memory` entry, refuses `invalid-request` as "not read by this build". FR-620 says a selected package is admitted by FR-027's reader with no fallback, and ticket AC-1 says a request names a compiled package. Fix: admit the package through FR-027's reader, or link an owning ticket before merge. | fsl-command/src/command/analyze.rs:102-118 |
| FND-005 | low | `temporal.limits` members that do not fit `usize` are silently dropped (`set_usize`, and `max_depth` via `.ok()`), and they have no 2^53 - 1 cap like the request-level `limit()`. Fix: refuse `invalid-request` naming the member. | fsl-analyze/src/analyze/document.rs:767-780 |
| FND-006 | low | `fairness_wire` maps a serialization failure to `null` and writes it as a valid fairness entry. Fix: propagate the error as `AnalyzeDocumentError::Fault`. | fsl-analyze/src/analyze/document.rs:1056-1061 |

## Verdict

REQUEST CHANGES (CONDITIONAL on FND-001 to FND-004). Correct as written: `limits.observation` accepts only the FR-255 `observation.*` names (`input_bytes`, `objects`, `values`). `post_state` is an admission policy, not a limit row, so dropping it follows FR-620 and is not a narrowing (it never existed on main). `protocol_refinement` is never written because EN-1 settles protocol refinement `unsupported` (items.rs:981) and no `Counterexample` variant carries it. The exhaustive match in `replay_target` will force a target when one is added, and nothing refuses it. `RunCause::AnalyzeUniverse` maps Unprovisioned to `invalid-request` and Incomplete to `incomplete_population` at the request stage. Request-level `limit()` and `deadline()` enforce integers from 0 to 2^53 - 1. The FR-285 exit change from 10 to 22 and the quotient.rs exploration alignment are justified.
