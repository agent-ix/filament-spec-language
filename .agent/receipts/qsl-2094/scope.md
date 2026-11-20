# QSL-2094 terminal emission boundaries

This delivery exercises terminal emission and FSL's independent I2 reader through the package crate's existing real source/domain-package fixture. It adds six active behavior tests and one strict owner-pending test under the existing terminal test module; it changes no production code, dependency, feature, or runtime limit.

The cases cover:

- Equal declarations on two aliases selecting one model retain both exact declaration spans and one semantic terminal row.
- Different declarations on those aliases retain structural row ordering, ordinals, mode distinctions, and equal semantic bodies after source reordering.
- Constant true and false predicates retain their distinct Boolean literals and terminal identities without a self parameter; neither is the `any` declaration.
- An `any` declaration carrying both a parameter and condition is refused at the parameter binding first.
- Duplicate self parameter entries are refused at the parameter binding after dependencies are rebuilt as the unique body-target set.
- Wrong binding order is refused at the body before a simultaneously invalid mode is considered.

Owning contracts are current FSL FR-411 and FSpec FR-476's terminal body, several-declarations, and reader-order rules. FSpec was read at the accepted `f32bca6c7937e0652742b2bbe27a52419bf5a0ae` reference. Assertions use real compiled source, emitted graph keys, admitted terminal rows, exact source regions, and typed reader refusals with exact loci. No proof success, evaluation outcome, checked package, or diagnostic is manufactured.

The wider historical QSL-2094 families (SMT, IEEE, units, hosting, conformance, protocol emission, union handling, and runner gates) are not newly claimed by this terminal slice. Accepted earlier deliveries are not duplicated for tags. The no_std hosting and evaluator-retirement work remains with its existing owners. No matrix or historical-count completion claim is made.

QSL-2718 blocks completion of the terminal model-binding facet. The original focused run reached an emitted and admitted row naming the model's object-type key rather than the selection's required model-root key. The desired model-root assertion remains strict in the ignored `a_terminal_row_names_the_selected_model_root_instead_of_its_object_type` case and is excluded from coverage. The active ordering case covers structural row order and lookups without claiming the pending binding identity has been repaired. No additional execution of the known defect is requested.
