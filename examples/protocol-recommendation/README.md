---
type: tutorial
description: Return an advisory operation call in a protocol counterexample.
---

# Return a protocol recommendation

An advisory role recommends incrementing a counter. A second role applies the
increment to the model, and a failing check exposes both steps in a counterexample.

Run from the repository root with the `fsl` executable from
[filament-driver](https://github.com/agent-ix/filament-driver):

```sh
fsl analyze --format json examples/protocol-recommendation/analyze.json
```

The command exits `10` because `check Blocked` deliberately fails. The
`items[0].counterexample.prefix` array contains two transitions:

1. `Main/Advise`, with `step` `recommend`. Its `recommendation` member names
   `Auditor`, operation `inc`, receiver `counter`, and an empty argument vector.
   The returned model still has counter value `0`.
2. `Main/Apply`, with `step` `attempt`. The returned model has counter value `1`.

The `replay.reproduced` member is `true`: analysis rechecks the counterexample
before returning it. Each step's `post_state.state.semantic` member contains
the model state after that step.

The source selects `SpaceAvailable` as the recommendation's precondition.
It permits the call when the counter value is below `2`. The attempt also
selects `AppendOne`, which requires the model value to increase by one.

The recommendation binder `advice` is unused because recommendations have no
operation result. Writing `run observe sequence Main` instead of
`run sequence Main` refuses the attempt during checking, before analysis steps
the protocol.

The input model is the repository's
[counter fixture](../../fsl-command/tests/fixtures/cli-contract/counter.semantic-ir.json).
The [request](analyze.json) starts from the nonempty [snapshot](start.json) and
checks the [protocol](workflow.native) for deadlock freedom.
