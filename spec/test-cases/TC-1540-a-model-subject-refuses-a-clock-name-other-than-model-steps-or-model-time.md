---
id: TC-1540
title: "A model subject refuses a clock name other than model-steps or model-time"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-233
    type: verifies
---
# TC-1540: A model subject refuses a clock name other than model-steps or model-time

## Description

Verify that a temporal clause over a model subject admits only the two model binding names as its clock.

Scope: FR-233-AC-4.

## Test Procedure

Fixture: a unit selecting the infinite-trace profile and a model with a Boolean attribute.

1. Check `eventually[0,3]` over the model subject with `clock "ms"`, `clock "wall"` and `clock "steps"`.
2. Check the same clause with `clock "model-steps"`.
3. Repeat step 1 under the event-position profile.

Tag the tests `#[trace("TC-1540", "FR-233-AC-4")]`.

## Expected Results

- Steps 1 and 3: `unsupported_construct`/`declaration-form` at the clock string, message naming the written name, `model-steps` and `model-time`.
- Step 2: checks with the `ModelSteps` binding.
