# Focused validation

The new `certificate_boundary_cases` modules contain eight tests. The focused invocation ran all eight once after compilation succeeded: **8 passed, 0 failed, 0 ignored**, with 2,191 unrelated cases filtered out. Execution took 0.73 seconds. This is a focused authoring receipt, not the final component gate.

```sh
CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2087/target \
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_INCREMENTAL=0 cargo test --locked -p fsl-analyze --test it \
  certificate_boundary_cases -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Exit: **0**. Private receipt: `/home/peter/dev/.agent/qsl-2087/logs/focused-certificate-corrected.log`; its sibling `.exit` contains the process exit. All child processes exited.

The initial compile exited **101** before any tests ran: the new hyper test used named fields for a counterexample binding that is represented by a two-string tuple. The fixture access was corrected to `.0`/`.1`. No production change, successful case rerun, or product regression is claimed from that compile failure. Original receipt: `focused-certificate.log` and `.exit` in the same private log directory.

The four new leaf files were formatted directly with `rustfmt --edition 2021`; no recursive repository formatter ran. `git diff --check` exited **0**. No lint sweep was run: existing unchanged semantics/analyze lint failures are accepted from the program's earlier receipts, so lint is not claimed green.

Disk before compilation: **210 GiB available**. One owned target directory was reused, with two build jobs, debug information disabled, incremental compilation disabled and one test thread. No product binary, workspace gate, matrix, Kani, fuzz or mutation job ran. The independent review and one final complete affected-component gate remain with the code lead.
