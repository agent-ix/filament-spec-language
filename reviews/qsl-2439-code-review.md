---
id: SR-2026111612172400000042
title: "Code and Rust review of QSL-2000 slice B delay_distribution emission (filament-spec-language PR 2934)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@a5386b9f5ebbef2e3ccaea2144efd648743fd4db; git diff origin/main...HEAD (PR 2934, branch qsl-2439-delay-distribution, 10 files); focus: fsl-package/src/emit/probabilistic.rs, fsl-package/src/probabilistic_read.rs, fsl-semantics/src/check/probabilistic/model.rs, fsl-semantics/src/check/probabilistic/model/package.rs, tests in fsl-package/src/emit/tests/probabilistic.rs, fsl-package/src/emit/tests/admission_corpus.rs, fsl-analyze/tests/it/digital_clocks.rs, fsl-semantics/tests/it/probabilistic_declarations.rs"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-2439 (QSL-2000 slice B). The PR emits each checked `Delay` as an FSpec
FR-640 `probabilistic`/`delay_distribution` node (`uniform`, `uniform_interval`,
`exponential`, `discrete`), types delay times as quantity literals at the unit node
of the dense time source of the timed root whose `model` is the delay's declaring
node, reads the nodes back (`BarePackage::probabilistic`) and rebuilds `Delay`
records (`StateModel::probabilistic_model`); the S4 delay refusal guard is removed
(rewards still refuse, QSL-2440). Rust lane (rust-review) folded in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | S3 admits a delay when the operation's model alias/package has a dense time source (`time_source(&operation.model, &operation.key.package)`), but S4 (mirroring IR v0.1.4 `join.delay`) requires the timed root's `model` (the one state-object node) to be the delay's declaring node; a checked delay on an operation declared by any other object type of a timed model refuses at S4 as ProbabilisticModelNotEmitted, contrary to FR-740 "SHALL emit a delay_distribution node for each checked delay declaration". The refusal carries the declaration locus (fail-safe), but the path is untested, undocumented as a deferral and has no ticket | fsl-package/src/emit/probabilistic.rs:403-422,441-449; fsl-semantics/src/check/probabilistic/model.rs:1198-1212 |
| FND-002 | medium | The read-back rebuilds `DelayDistribution` with no re-check of FR-253's record invariants (`0 <= a <= b`; `discrete` delays non-negative and distinct). IR's reader rule 5 checks only weight/rate > 0, so a package from another producer with `uniform_interval` lower > upper, a negative delay or a repeated discrete delay is rebuilt as a record S3 can never produce and reaches the production route (fsl-eval simulation/model/input.rs:295); a repeated delay makes `delay_probability` take the first weight over the total, so step probabilities silently fail to sum to 1. Random parameters re-check (`ZeroSum`, `ValueOutsideType`); delays add no `PackageProbabilisticRefusal` | fsl-semantics/src/check/probabilistic/model/package.rs:205-225; fsl-package/src/probabilistic_read.rs:191-210; fsl-eval/src/simulation/model/timed/race.rs:648-664 |
| FND-003 | low | Tests tagged FSpec FR-640-AC-2 do not assert the values that AC states: no test checks a discrete node's delays `1` and `3` in unit `ms` with weights 90 and 10 (fsl-analyze `each_delay_family_is_emitted_as_a_delay_distribution_node` asserts only the family string), and `uniform[1 ms, 4 ms]` / `exponential(rational(1, 2))` are replaced by `[1 ms, 3 ms]` / `3/2`; values are only checked indirectly through the unit-less FSL read-back | fsl-package/src/emit/tests/probabilistic.rs:990-1044; fsl-analyze/tests/it/digital_clocks.rs:889-923 |
| FND-004 | low | Stale doc comments: `append_probabilistic_nodes` still says "for each checked random parameter and workload"; the `probabilistic_read` module doc still names only the IR typed reads of random parameters and workloads | fsl-package/src/emit/probabilistic.rs:36-37; fsl-package/src/probabilistic_read.rs:6-10 |

## Scope and evidence

- Read every changed production file in full, FSpec FR-640 and FR-322 "Probabilistic-model
  nodes" (filament-specification origin/main 06d4f215, FR-640 unchanged from local), FSL
  FR-740 and FR-253, and IR v0.1.4 (`probabilistic.rs` shape check, `probabilistic_join.rs`
  `operation`, `delay`, `dense_unit`).
- Family shapes match FR-640 and IR's `bind`/`family_bind`: bindings `declaring, operation,
  family, parameters`; `uniform` empty; `uniform_interval` `lower, upper` quantities;
  `exponential` `rate` exact rational; `discrete` rows `delay, weight` in declared order.
- Exact rationals: `spelling` of reduced `filament_exact::Rational`; S3 converts each time
  to the time source's unit (FR-253 `distribution`, `units.convert`) before S4 writes it;
  quantity literals carry `type_id` = the dense source's `unit` reference, which is what
  IR's `join.delay` compares.
- `dense_unit` reads timed root members 0 (`model`) and 2 (`source`) and source members 0
  (`kind`) and 1 (`unit`), matching `write_root`/`write_source`; timed nodes are written
  before `append_probabilistic_nodes` (emit.rs:6968-7005).
- Refusal paths (`model_node`, `dense_unit`) go through `refusal()` after `region_of`
  succeeded, so the locus is non-null.
- Node id: body-derived operand `[declaring key, operation, family, content]` with
  canonical rational spelling (slice A convention); distinct families never collide;
  `pending` is sorted by node id (determinism).
- S3 refuses non-positive rate, `uniform[a,b]` outside `0 <= a <= b`, negative/repeated
  delays and non-positive weights (FR-253), so emission never sees them.
- Read-back: `Delay.declared` is the read ordinal, not the source index (the documented
  slice A convention for random parameters); no consumer reads it for delays.
- No panics on package input in the new reader code (all `Option`/`ok_or`); no `#[allow]`,
  `unwrap` only in tests; changed files rustfmt-clean.
- Tests run (focused, `--test-threads=1`, house skips): fsl-package `emit::tests::probabilistic`,
  `every_emitted_node_family_is_admitted`, `every_emitted_body_of_the_corpus_is_in_the_stratified_grammar`
  (21 passed, 2 ignored QSL-2440/2441); fsl-analyze `delay` filter with `--include-ignored`
  (`each_delay_family_is_emitted_as_a_delay_distribution_node` ok; failures are only tests
  ignored on main, QSL-2156, and `discrete_delays_are_emitted_as_delay_distribution_nodes`,
  ignored IR-1569); fsl-semantics `probabilistic_declarations::`.

## Verdict

Emission conforms to FR-640's delay body, units, occurrences and determinism, and the
strict IR reader admits every family. FND-001 and FND-002 are real edge gaps (an S3/S4
admission mismatch and an unvalidated rebuild from untrusted packages); both fail safe or
silently on inputs this repository's producer does not generate. Mergeable once FND-001 and
FND-002 are fixed in this PR or deferred to tickets; FND-003/FND-004 are cheap in-PR fixes.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | `a_delay_beside_a_second_object_type_is_refused_at_the_time_source` is tagged FR-253-AC-3 but asserts an `AmbiguousTimedModel` time-source refusal (FR-230), none of FR-253-AC-3's delay refusals; its doc claims a delay on a non-root object type "cannot reach S4", which the round-1 probe disproves (see FND-001 disposition) | fsl-package/src/emit/tests/probabilistic.rs:1212-1250 |

## Dispositions

Round 1, head c4aea60878421a962cdf80497d98628a059ecd80 (fix commit 5d65b7f59, then a merge of origin/main 67fb1c431).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The coder's claim that the S4 refusal arms are unreachable is false. A temporary probe at c4aea608 (reverted) on a source-declared timed model `object O { w: W; d: Clock; operation step() modifies self.d; } time O dense unit ms; delay O::step ~ uniform;` is admitted by S3 (one delay) and refused by S4 as ProbabilisticModelNotEmitted { declaration: "step", feature: Delay, region 284..308 }; adding `object Other { n: Integer; operation poke() modifies self.n; } delay Other::poke ~ uniform;` behaves the same. So every delay of a timed source model refuses at S4. The refusal carries the locus (fail-safe), but FR-740 says SHALL emit: either emit (IR/FSpec support for source-object declaring nodes may be needed) or file a deferral ticket and add a test pinning this refusal and its locus |
| FND-002 | fixed | `delay_fault` re-checks FR-253 on read-back and refuses `PackageProbabilisticRefusal::InvalidDelay`; `a_package_delay_that_breaks_fr_253_is_refused_on_read_back` covers all six faults |
| FND-003 | fixed | `delay_round_trips` and `each_delay_family_is_emitted_as_a_delay_distribution_node` now assert FR-640-AC-2's literal values (1/4 ms, 1/2, 1 ms:90, 3 ms:10) and that each time is typed by the unit `ms` |
| FND-004 | fixed | `append_probabilistic_nodes` and the `probabilistic_read` module doc now name delays |
