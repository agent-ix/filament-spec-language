# Final affected-component gate

Independent reviews **SR-1491 / SR-1492 passed without findings**. Their latest artifacts were committed unchanged at `c50d2dd51`. The reviewed test source was `716f44b511daec8cf8028b7f8abd7094be078d68`.

One final gate covered all existing and new tests in the four changed parents. Each filter ends in `::` and the four parent names do not overlap. The initial loop stopped after the hyper-replay segment's actual failure, so probabilistic replay had not yet run.

| Original segment | Passed | Failed | Ignored | Exit |
| --- | ---: | ---: | ---: | ---: |
| `closure_certificate::` | 21 | 0 | 0 | 0 |
| `exact_probabilistic_item::` | 18 | 0 | 2 | 0 |
| `hyper_replay::` | 22 | 5 | 0 | 101 |

The five failures were existing fixtures refused at S3 because they placed bare division or remainder in Linked code. Each was corrected to call the same operation through the existing Kernel-checked `q` function and `World::new_with_kernel_function` fixture. The shared test-only guarded vault and division-function constants were made `pub(crate)` for reuse; their bodies were unchanged. Every original runtime Undefined, replay, witness and settlement assertion was retained. No production behavior changed and no product bug was inferred from these fixture failures.

Only those five changed fully qualified names were then run with `--exact`; each returned **1 passed / 0 failed / 0 ignored**, exit **0**:

- `hyper_replay::a_witness_set_computed_past_a_universal_contract_with_no_value_reproduces_nothing`
- `hyper_replay::a_witness_set_emptied_by_an_undefined_match_reproduces_nothing`
- `hyper_replay::a_witness_set_that_lost_a_member_to_an_undefined_contract_reproduces_nothing`
- `hyper_replay::an_undefined_match_counterexample_replays_to_its_value`
- `hyper_replay::an_undefined_witness_counterexample_reads_each_match_over_its_own_variables`

The previously unrun `probabilistic_replay::` segment then returned **43 passed / 0 failed / 0 ignored**, exit **0**. No passing segment or unchanged successful name was repeated.

The combined accepted evidence therefore covers **109 passed, 0 unresolved failures, 2 inherited ignored**. This is a composition of the preserved original receipts and the five exact corrections, not a claim that the original failed invocation was green. Both ignored cases remain owned by QSL-2158 and are excluded from coverage. All eight new cases were exercised by their affected parent segments.

Every segment used:

```sh
CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2087/target \
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_INCREMENTAL=0 cargo test --locked -p fsl-analyze --test it FILTER \
  -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

For the five confined corrections, `FILTER` was the complete name above and `--exact` was added after `--`. Private command outputs and actual exits are retained under `/home/peter/dev/.agent/qsl-2087/logs/final-<parent>.{log,exit}` and `corrected-<test-name>.{log,exit}`. All processes exited.

Disk before the gate: **210 GiB available**. The existing owned target was reused. `git diff --check` exited **0** after the fixture correction. No workspace/proof/Kani/matrix/lint sweep, shared binary build, main refresh or extra review ran. FR-163 product-closure checking and hyper certified proof settlement remain unimplemented under QSL-964 and are not represented as coverage.
