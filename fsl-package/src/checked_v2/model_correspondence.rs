// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed projection of FR-081 model-correspondence flat terms (FR-400-AC-14).

use filament_contract_model::{CheckedLiteralValue, CheckedNodeTag, CheckedPackageV2, CheckedTerm};
use filament_exact::{EffectiveId, OriginalDeclarationKey};
use fsl_semantics::model::key::{
    EffectiveDeclarationPreimage, Fact, RULE_INHERIT, RULE_QUALIFY, RULE_REDEFINE, RULE_SUBSET,
};

/// Read the admitted `correspondence/model_correspondence` scalar sequences.
/// Each body starts with a row count, followed by the closed preimage grammar
/// documented by the writer. Only literal terms from admitted model-
/// correspondence nodes are considered.
pub(super) fn read(package: &CheckedPackageV2) -> Result<Vec<EffectiveDeclarationPreimage>, ()> {
    let mut rows = Vec::new();
    for node in &package.graph().nodes {
        if node.node_tag.as_ref() != CheckedNodeTag::Correspondence.as_wire()
            || node.semantic_form.as_ref() != "model_correspondence"
        {
            continue;
        }
        let CheckedTerm::Aggregate { members, .. } = &node.body else {
            return Err(());
        };
        let mut cursor = Cursor {
            terms: members,
            at: 0,
        };
        for _ in 0..cursor.count_bounded()? {
            rows.push(cursor.preimage()?);
        }
        if cursor.at != members.len() {
            return Err(());
        }
    }
    rows.sort_by_key(EffectiveDeclarationPreimage::identity);
    if rows
        .windows(2)
        .any(|pair| pair[0].identity() == pair[1].identity())
    {
        return Err(());
    }
    Ok(rows)
}

struct Cursor<'a> {
    terms: &'a [CheckedTerm],
    at: usize,
}

impl Cursor<'_> {
    fn next(&mut self) -> Result<&CheckedTerm, ()> {
        let term = self.terms.get(self.at).ok_or(())?;
        self.at += 1;
        Ok(term)
    }

    fn text(&mut self) -> Result<String, ()> {
        let literal = self.next()?.literal_read().ok_or(())?;
        if literal.kind != filament_contract_model::LiteralKind::Text {
            return Err(());
        }
        match literal.value {
            CheckedLiteralValue::Text(value) => Ok(value.to_owned()),
            _ => Err(()),
        }
    }

    fn integer(&mut self) -> Result<usize, ()> {
        let literal = self.next()?.literal_read().ok_or(())?;
        if literal.kind != filament_contract_model::LiteralKind::Integer {
            return Err(());
        }
        match literal.value {
            CheckedLiteralValue::Integer(value) => value.to_string().parse().map_err(|_| ()),
            _ => Err(()),
        }
    }

    fn count(&mut self) -> Result<usize, ()> {
        self.integer()
    }

    fn count_bounded(&mut self) -> Result<usize, ()> {
        let count = self.count()?;
        if count > self.terms.len().saturating_sub(self.at) {
            return Err(());
        }
        Ok(count)
    }

    fn optional_effective_id(&mut self) -> Result<Option<EffectiveId>, ()> {
        let marker = self.text()?;
        if marker == "none" {
            return Ok(None);
        }
        if marker != "some" {
            return Err(());
        }
        let mut depth = 1usize;
        loop {
            match self.text()?.as_str() {
                "some" => depth = depth.checked_add(1).ok_or(())?,
                "none" => break,
                _ => return Err(()),
            }
        }

        // The owner chain is encoded recursively but consumed iteratively:
        // one original key per `some`, from its innermost owner outward.
        let mut identity = None;
        for _ in 0..depth {
            let original = self.original_key()?;
            identity = Some(EffectiveId::new(identity, original));
        }
        Ok(identity)
    }

    fn original_key(&mut self) -> Result<OriginalDeclarationKey, ()> {
        Ok(OriginalDeclarationKey::new(self.text()?, self.text()?))
    }

    fn declaration(&mut self) -> Result<filament_checked::key::DeclarationKey, ()> {
        Ok(filament_checked::key::DeclarationKey {
            package: self.text()?,
            node: self.text()?,
        })
    }

    fn preimage(&mut self) -> Result<EffectiveDeclarationPreimage, ()> {
        let owner_effective_type = self.optional_effective_id()?;
        let original = self.declaration()?;
        let fact_count = self.count_bounded()?;
        let mut derivation = Vec::with_capacity(fact_count);
        for _ in 0..fact_count {
            derivation.push(self.fact()?);
        }
        Ok(EffectiveDeclarationPreimage {
            owner_effective_type,
            original,
            derivation,
        })
    }

    fn fact(&mut self) -> Result<Fact, ()> {
        let ordinal = self.integer()?;
        let rule = match self.text()?.as_str() {
            RULE_QUALIFY_ID => RULE_QUALIFY,
            RULE_INHERIT_ID => RULE_INHERIT,
            RULE_SUBSET_ID => RULE_SUBSET,
            RULE_REDEFINE_ID => RULE_REDEFINE,
            _ => return Err(()),
        };
        let input_count = self.count_bounded()?;
        let mut inputs = Vec::with_capacity(input_count);
        for _ in 0..input_count {
            inputs.push(self.declaration()?);
        }
        let parent = self.optional_effective_id()?;
        Ok(Fact {
            ordinal,
            rule,
            inputs,
            parent,
        })
    }
}

const RULE_QUALIFY_ID: &str = "filament.model.normalize.qualify/v1";
const RULE_INHERIT_ID: &str = "filament.model.normalize.inherit/v2";
const RULE_SUBSET_ID: &str = "filament.model.normalize.subset/v2";
const RULE_REDEFINE_ID: &str = "filament.model.normalize.redefine/v2";
