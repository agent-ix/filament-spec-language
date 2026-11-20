# QSL-2094 authoring validation

One owned target: `/home/peter/dev/.worktrees/luna-qsl-2094/target`.

The focused authoring command selects only the new `emit::tests::terminal::emission_boundaries::` functions in the `fsl-package` library test binary, with jobs 2, development/test debug information disabled, incremental compilation disabled, one test thread, and the two program skips. Before compiling, `/` had 208 GiB available.

The initial focused command exited 101 after the actual first owned build completed in 9m00s: five new cases passed and one failed at the independently expected model-root identity. Its original private receipt is `/home/peter/dev/.agent/qsl-2094/logs/focused.log` and `focused.exit`. The typed terminal row actually named `example/config-version:[model,object_type,ix://example/config-version/ConfigVersion]:0` instead of the contract's `example/config-version:[model,model_root]:0`. QSL-2718 owns this actual emitter/read-back contract defect and blocks QSL-2094. No root cause is inferred and the known failure is not rerun.

The desired root-binding assertion was separated as a strict ignored case, not passing coverage. The source-order case retains its already-executed graph/body equality and row equality, and separately exercises its later ordinal and lookup assertions that the original failure prevented from running. Only that changed qualified source-order name was run: exit 0, one passed, zero failed, zero ignored. No already-passing name was repeated. Its actual log/exit are `source-order-corrected.log` and `source-order-corrected.exit` in the same private directory.

The accepted focused composition is six new active functions passed, with one strict QSL-2718 owner-pending function excluded from coverage. This is the composition of the original five passes and the one corrected source-order pass; it is not a rerun of the whole group or the final component gate.

The new leaf was formatted alone. The parent changes only its module registration. No workspace formatting or unrelated source edits were made. Known semantics/package library warnings and the program's accepted lint limitations are not suppressed or retried; there is no green Clippy claim.

One independent feature review and one final complete affected terminal-component gate remain reserved for the lead. The authoring command is not that final gate.
