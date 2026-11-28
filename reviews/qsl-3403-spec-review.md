---
id: SR-2026112520400002
title: "Spec review — filament-spec-language QSL-3403 replay-wire setting rulings"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-263-replay-at-any-depth-under-the-request-limits.md, spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md, spec/functional/FR-399-set-a-named-stage-limit-through-the-replay-root.md, spec/test-cases/TC-1093-a-consumer-sets-a-named-stage-limit-through-the-replay-root.md, spec/test-cases/TC-1250-smt-encoding-and-solver-run-limits-are-request-settings.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-255", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/FR-263", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/FR-399", type: reviews }
---

## Summary

Ticket: QSL-3403. PR #3969 at head `512f1536a56fa260548122ced17857baa330f648`. Each ruling judged against FSL text on `origin/main` and FSpec (filament-specification `origin/main` `a06642a6`) FR-323 and FR-461.

- `emit.output_bytes` accepted: correct. FR-255-AC-20 on main already required a replay request's `stage_limits` to set it, FR-263 Behavior 1/AC-4 require every stage the recompile reaches to take its entry, and replay's recompile runs E4 under `limits.spine.package`. FSpec FR-323 ("each recompile stage under exactly these values") and FR-461 Behavior 4 agree. This fixes the product, not the spec.
- `exact_prob.max_witness_paths` refused: correct. FR-255's list on main already said "Replay does not execute `exact_prob.*`" and AC-24 says the replay entry point accepts exactly the listed settings; replay never reads it.
- `smt_encoding.*` refused and FR-255-AC-14 edited: the old AC-14 sentence contradicted AC-24 on main (smt_encoding was never on the replay list). Replay's execute path encodes no SMT query (the `smt_query`/`hyper_smt_query` modules are not reached from `execute`), so the old sentence asserted something replay cannot do. Resolving toward AC-24 is not a weakening of real behaviour.
- Behaviour 5, 11, the Description and FR-290's table are narrowed to "settings replay consumes". This matches FR-255's own coverage list and FSpec FR-323 line 337 ("`stage_limits` SHALL continue to carry only FR-460/461 recompile stage settings"), but now states in the behaviour text a divergence from FSpec FR-461 Behavior 4 and AC-2/AC-6 (FND-002).

## Verdict

**CONDITIONAL** — two medium findings; neither is introduced as a weakening by this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-263-AC-1 still says the request's `stage_limits` carries the raised `s1.*` and `s3.*` settings; its fixture (TC-736) also needs `emit.output_bytes` (and raises `i2.*`), so AC-1 as written cannot pass. Now that a request can carry `emit.output_bytes`, AC-1 should name it; this is QSL-3253's requested resolution and the PR leaves it. | spec/functional/FR-263-replay-at-any-depth-under-the-request-limits.md:62 |
| FND-002 | medium | FSpec conflict: FR-461 Behavior 4 ("A replay request carries every stage limit") and AC-2/AC-6 (an engine limit raised "through ... the replay request") contradict FSpec FR-323 ("`stage_limits` SHALL continue to carry only FR-460/461 recompile stage settings"). FSL follows FR-323; FR-255 Behaviour 5/11 now say so explicitly. Needs an FSpec ticket (drafted in the review comment). | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:658, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:715 |

## Coverage

- Examined: FR-255 Description, Lifecycle coverage replay paragraph, Behaviour 5 and 11, AC-14, AC-20, AC-24; FR-263 Behaviour 1-2, AC-1, AC-3, AC-4; FR-290 replay-context `stage_limits` row and AC-14; FR-399 Behaviour, AC-4, AC-5; TC-1093 steps 4-5; TC-1250 step 5.
- Pre-existing nits not raised as findings: FR-255's replay refusal paragraph names `evaluation.live_occurrences`, which is no setting, and does not name `format.*`, `render.*`, `export.*`, `inspect.*`, `explore.*` (covered by "exactly" in AC-24).
