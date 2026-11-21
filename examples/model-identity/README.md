---
type: tutorial
description: Check inherited and redefined fields from an authored domain model.
---
# Inherited and redefined fields

Check a `Leaf` object whose fields come from a three-level type hierarchy.
The example uses an installed `fsl` command and runs from the repository root.

In [model.fsl](model.fsl), `Base` declares `inherited` and `field`.
`Mid` and `Leaf` each redefine `Base::field`.
The effective field selected on `Leaf` is its own redefinition.
Its inherited field keeps the declaration originally authored on `Base`.

An original identity names a declaration in its domain package.
An effective identity pairs that original identity with the type that owns its use.
Thus `Base::inherited` and its use on `Leaf` share an original identity and have distinct effective identities.
The correspondence also retains hidden redefinitions and their derivation facts.

[program.fsl](program.fsl) checks `self.inherited = 3 and self.field = 7`.
[current.json](current.json) supplies those values for `leaf-1`.
[run.json](run.json) selects the model source as `native-model-unit/1` and checks that object:

```sh
fsl run examples/model-identity/run.json
```

The result has a Boolean value of `true` and exits with status 0.
Change `field` in the snapshot to `8` and rerun the request.
The Boolean value becomes `false` and the exit status becomes 10.

Continue with the [language overview](../../README.md#language).
