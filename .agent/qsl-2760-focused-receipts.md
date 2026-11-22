# QSL-2760 caller override focused receipts

Changed crate: `fsl-command`. Focused-test base: `5b51a5d55`.
After those commands exited, the branch rebased cleanly once onto the owner's
repair main `3415ccde2ceea376b44ea49a0504ed3ed52fc628`. Accepted tests were
not repeated for that integration.
Only the two new acceptance names ran. Commands used the populated shared
CARGO_HOME, this worktree's retained target, debug=0, incremental=0 and
parallelism derived from actual CPU/load/available memory.

| Command / accepted test | Exit | Result | Log |
| --- | ---: | --- | --- |
| `cargo test --locked -p fsl-command --test it caller_overrides_ -- --test-threads=1` | 101 | `cli_contract::run::caller_overrides_reach_call_and_items_accounting` PASS; plan fixture rejected its handler anchor | `logs/qsl-2760-caller-overrides.log` |
| Exact plan name, after changing its fixture to initialization | 101 | Fixture lacked the initialization anchor's required name | `logs/qsl-2760-plan-overrides-2.log` |
| `cargo test --locked -p fsl-command --test it cli_contract::run::caller_overrides_reach_plan_clocks_stages_and_observations -- --exact --test-threads=1` | 0 | PASS with the valid named initialization anchor | `logs/qsl-2760-plan-overrides-3.log` |

The call/items pass was not repeated. It proves that a caller work_units=0
budget overrides an otherwise successful request's accounting through the
provider-aware entry and reaches both independently identified items.
The plan pass includes normal initial-state admission, a caller S1 byte-limit
failure, a caller observation-byte-limit failure, a caller scalar work limit
through the named backend, and duplicate-clock refusal. Source and snapshot
admission remain real; no engine or budget double replaces the owned route.

The authoritative driver's `LimitOverrides::apply_to` updates SpineLimits and
its existing Call.accounting. This API accepts a borrow of that fully resolved
ScalarLimits. No additional caller-owned non-accounting Call resource override
was present at that callsite. Actual driver adoption remains separate.

No whole-crate/whole-integration/workspace test, final check or review ran here.
The arch-owned fairness initializer repair is inherited without modification.
