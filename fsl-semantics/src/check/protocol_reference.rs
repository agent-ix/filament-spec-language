// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218's unit-wide rule on protocol references and the protocols they
//! name: each protocol is paired with what S3 checked of it for lowering.

use fsl_forms::ProtocolDeclarationForm;

use super::lowering::ProtocolInput;
use super::protocol_clause::CheckedProtocol;
use super::refusal::CheckRefusal;

/// Each successfully checked protocol as lowering reads it. A protocol S3
/// refused has no input; its source index is retained for authored edges.
pub(super) fn inputs<'a>(
    protocols: &'a [ProtocolDeclarationForm],
    places: &[Option<usize>],
    checked: &'a [CheckedProtocol],
) -> Result<Vec<ProtocolInput<'a>>, CheckRefusal> {
    Ok(protocols
        .iter()
        .enumerate()
        .filter_map(|(index, form)| {
            places
                .get(index)
                .copied()
                .flatten()
                .and_then(|place| checked.get(place))
                .map(|checked| ProtocolInput {
                    source_index: index,
                    form,
                    checked,
                })
        })
        .collect())
}
