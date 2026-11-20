# Certificate and witness behavior cases

The new tests exercise real EN-1 or EN-5 evidence through independent replay/checking APIs. The scope includes canonical instance-context admission, separation of two object instances over the same model-state set, the Link model's changed one-loss Bellman value and exact rejection locus, lossless canonical certificate transport, progress ranks independent of otherwise correct values, the exact certificate byte ceiling, hyper-counterexample instance admission, and probabilistic witness initial-state authentication.

FR-163's hyper product-closure certificate and checker remain unimplemented. The production `HyperOutcome::Holds` carries no certificate and settlement returns `ProductClosureCheckerNotBuilt`. Existing QSL-964 owns the product-closure implementation and blocks QSL-2087. No certificate acceptance or rejection test is fabricated for that missing API. Hyper certified proof settlement remains pending for the same reason.

The older ticket's matrix method and historical criterion counts are not execution instructions. Tests target current committed requirements. The source contract is authoritative where old ticket examples differ.

The original checked-subject replay fixtures remain the harness; no external source, model, schema or binary is copied. No production behavior or feature configuration changes are intended.
