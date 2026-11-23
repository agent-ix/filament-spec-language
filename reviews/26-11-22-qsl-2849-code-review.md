---
id: SR-3554001
title: "code-review of QSL-2849 slice 2: emit source-map charge and chunked protocol BFS (filament-spec-language PR 3554)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; fsl-package/src/emit.rs; fsl-package/src/emit/tests/output_limit.rs; fsl-eval/src/simulation/protocol/reach.rs"
review_set: subset
---

# Code review of QSL-2849 slice 2 (filament-spec-language PR 3554)

## Summary

Ticket: QSL-2849. One code-review pass, with the rust-review lane folded in, over
`git diff origin/main...HEAD` at head 0a62aae839c92181eef1ffa0abd3dfb29113d3eb (5 commits,
3 files, +136/-6). Governing text: FR-264 Behavior 5 and FR-264-AC-6/7, FR-126-AC-22/23,
FR-850-AC-15, FR-255 defaults.

Focused tests run on titan: `cargo test -p fsl-package --lib output_limit` (8 passed) and
`cargo test -p fsl-eval --lib simulation::protocol::reach` (7 passed).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new allocation-count test may not tell the fix apart from main. Its fixture, `nested_not_package(1_000)`, has 1,000 `body_locations` rows. On main, `checked_body_locations` already charges each row against `emit.output_bytes` before building it, so at bound 0 main refuses on the first body row, inside `checked_term_data`. The only typed rows built before that point are the temporal, claim and equality rows, which are empty or tiny for one pure function. Both main and this PR skip the 1,000 serde-encoded body rows and everything after them at bound 0. So `early*3 < late*2` probably holds on main too, and nothing shows that the test fails when `charge_source_map` is removed. Fix: use a fixture with no metered body rows (for example `nested_let_package`, whose body carries no location row), or a bound between the source-map bytes and the first body-row charge, and confirm that the test fails with the call removed. | fsl-package/src/emit/tests/output_limit.rs:298-326 | correct-requirement-no-evidence |
| FND-002 | low | `charge_source_map` canonically encodes the whole source map a second time on every successful emission. That cost is linear in the source map, with the writer's sort buffers, and the encoder throws the result away. It is correct and deliberate (it is a floor check), but it is extra work on the success path that the PR does not mention. It could be avoided by charging only when the bound is below a cheap upper estimate, or accepted on purpose. | fsl-package/src/emit.rs:500-516,7840-7843 | implementation-bug-despite-evidence |
| FND-003 | low | A level is now expanded in chunks of `worker_limit * 64` states, with a fresh `std::thread::scope` spawn and join for each chunk. That is about one thread spawn per 64 expansions and a barrier every 64 expansions per worker, where main had one spawn per level. Results and the reported `workers` are unchanged, but parallel throughput on large under-budget levels (fsl-bench `mc`) may fall, and the PR has no measurement. Measure with fsl-bench, or reuse one scope across the chunks of a level. | fsl-eval/src/simulation/protocol/reach.rs:229-245 | implementation-bug-despite-evidence |

## Verdict

Correct as written; one test-quality fix needed before merge (FND-001).

Emit (`output_bytes`): `charge_source_map` uses a separate `quire_canonical::Writer` over a
discarding sink with `limits.canonical()` (byte ceiling only). It encodes `[` + each entry's
`encode_into` + `]`, which is the same per-entry encoding `encode::elements` uses for the
`source_map` member of the V2 envelope (filament-contract-model `checked_package/v2/encode.rs:80`).
`source_map` is only borrowed after the charge point, so the standalone bytes are a strict subset of
the package bytes, and the check never refuses a package that would fit. There is no double charge:
the final encoder starts with the full bound. A ceiling error maps through the existing
`From<quire_canonical::Error>` to `output_limit`, which clamps `actual` to `bound + 1`. Code
(`stage_limit_exceeded`), cause (`input-bytes-exceeded`), kind, setting and `actual` are therefore
identical whichever check fires, including for a package whose source map alone exceeds the bound.
The locus still comes from `locate_output_limit` at every wrapped entry. The check runs after
`write_index_segments` and before the temporal, claim, equality, term-data, source-type, requirement
and operand rows.

Protocol `max_states`: there is no shared meter across expansions, because each `expand_bounded`
uses its own `self.model.budget()`. Admission stays in frontier position order, chunk by chunk. The
refusal, its `Limit::States(max_states)` value, the state indices, the steps and the first refusal in
breadth-first order are therefore the same as a sequential FIFO run for every worker count. The
reported `workers` is unchanged, because each chunk is at least `worker_limit` long. `worker_limit` is
a `NonZeroUsize`, so `chunk` is at least 64 and `chunks(0)` cannot happen; `saturating_mul` covers
overflow. The new expansion-count test is exact (513 for 1, 2, 4 and 8 workers, against 1,001 on
main), so it does fail on main. Existing assertions are unchanged.

Replay `input_bytes`, verified independently: `replay_model_trace_from_package`
(model_check/replay.rs:991), `replay_protocol_trace_from_package` (:1511) and the graph-replay
package route (graph_replay.rs:351) each call `check_external_package_bytes` before `package_input`
and `admit_package`. The request's own encoded bytes are checked first, in `decode_with`
(request.rs:918-919), and the source route charges each library as it is emitted (slice 1). Nothing
is left at the request level.

fsl-bench uses `Limits::default().max_states`, which is the FR-255 published default of 10,000,000.
Agreed: a benchmark should run the default configuration, and chunking now bounds the overshoot.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-004 | low | The `body_locations` bytes are not added to the running `OutputMeter`. `checked_body_locations` gets `meter.bound` and counts its own total from zero, so the floor charged before and after the body rows leaves them out. A package whose source map plus body rows plus later rows exceeds the bound, when no one of those does alone, is refused only at the final encode, after the remaining rows are built. This is sound (it never refuses a package that fits) but refuses later than it could. Fix: pass the meter, or add the body rows' charged total to `meter.charged`. | fsl-package/src/emit.rs:4752-4759 | implementation-bug-despite-evidence |
| FND-005 | low | `OutputMeter::charge` builds a new `quire_canonical::Writer` for every row and canonically encodes every charged section (source map, temporal, claim, equality, dispatch, protocol-location, probabilistic and requirement rows) a second time. The roughly 3% success-path cost in the PR body was measured on a `let` chain, where only the source map is large; packages heavy in claims or term data pay the second encode, and the per-row writer buffers, on more bytes. Measure a term-data-heavy package, or reuse one writer's buffers across rows. | fsl-package/src/emit.rs:520-543 | implementation-bug-despite-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The test now uses `nested_let_package(1_000)` (no body rows) at bound N/10, so only the source-map charge can refuse early; main refuses at the final encode there. A second test pins the claim-name charge, and the PR records it failing with that charge removed (331440 against 333849). |
| FND-002 | accepted-no-change | Measured at about 3% in a debug build. Removing it needs quire-canonical to accept pre-encoded canonical bytes (a raw-value write on `Writer`), which belongs in agent-ix/quire-canonical, not this PR. FND-005 tracks the wider cost of the per-section meter. |
| FND-003 | fixed | Workers are spawned once per search inside one `thread::scope` and take chunks over per-worker mpsc channels; a chunk costs channel hand-offs, not spawns. On the PR body's fsl-bench numbers, 4 workers went 18.96 to 11.9 ms against 9.25 ms on main; the rest is the chunk barrier the refusal bound needs. |
