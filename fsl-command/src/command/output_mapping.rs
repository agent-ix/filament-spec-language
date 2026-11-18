// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-814's injective target-name mangling shared by the TLA+ and Lean mappers.

/// Mangle one FSL identifier into the ASCII-safe target spelling.
///
/// ASCII letters and digits are retained, underscores are doubled, namespace
/// separators become `_N_`, and every other Unicode scalar is encoded by its
/// uppercase hexadecimal code point. The mapping is injective because each
/// escape has a reserved delimiter and source underscores are escaped.
pub fn mangle_name(name: &str) -> String {
    let mut mangled = String::with_capacity(name.len());
    let mut characters = name.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            'A'..='Z' | 'a'..='z' | '0'..='9' => mangled.push(character),
            '_' => mangled.push_str("__"),
            ':' if characters.peek() == Some(&':') => {
                characters.next();
                mangled.push_str("_N_");
            }
            other => {
                use std::fmt::Write as _;
                write!(&mut mangled, "_u{:X}_", other as u32)
                    .expect("writing into a String cannot fail");
            }
        }
    }
    mangled
}

/// Build a generated target identifier from its distinct kind prefix.
pub fn target_name(prefix: &str, fsl_name: &str) -> String {
    format!("{prefix}_{}", mangle_name(fsl_name))
}

/// The target module or namespace name for one checked package.
pub fn module_name(package_name: &str) -> String {
    target_name("M", package_name)
}

/// One already-lowered definition in canonical dependency order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    /// Generated target identifier.
    pub name: String,
    /// Stable checked-package node identity.
    pub node: String,
    /// Half-open source byte region.
    pub region: [u32; 2],
    /// Target-language definition body, without the FSL comment.
    pub body: String,
}

/// A canonical target file and its reverse name map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedModule {
    /// Target module or namespace identifier.
    pub module: String,
    /// Canonical ASCII source, ending in exactly one line feed.
    pub text: String,
    /// Generated identifiers mapped to source nodes and regions.
    pub name_map: Vec<NameMapEntry>,
}

/// One generated-name to checked-node mapping.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NameMapEntry {
    /// Generated target identifier.
    pub name: String,
    /// Stable checked-package node identity.
    pub node: String,
    /// Half-open source byte region.
    pub region: [u32; 2],
}

/// Why target text could not be rendered canonically.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RenderError {
    /// A definition name occurs more than once.
    #[error("duplicate generated definition name: {0}")]
    DuplicateName(String),
    /// Target output must be ASCII; source text belongs in escaped comments.
    #[error("generated definition is not ASCII: {0}")]
    NonAscii(String),
    /// A definition body has trailing whitespace or blank outer lines.
    #[error("generated definition is not canonically trimmed: {0}")]
    NonCanonicalBody(String),
    /// The source byte region is reversed.
    #[error("source region is reversed for definition: {0}")]
    ReversedRegion(String),
}

/// Render one canonical TLA+ module from definitions in dependency order.
pub fn render_tla(
    package_name: &str,
    definitions: &[Definition],
) -> Result<GeneratedModule, RenderError> {
    render(package_name, definitions, Target::Tla)
}

/// Render one canonical Lean namespace from definitions in dependency order.
pub fn render_lean(
    package_name: &str,
    definitions: &[Definition],
) -> Result<GeneratedModule, RenderError> {
    render(package_name, definitions, Target::Lean)
}

#[derive(Clone, Copy)]
enum Target {
    Tla,
    Lean,
}

fn render(
    package_name: &str,
    definitions: &[Definition],
    target: Target,
) -> Result<GeneratedModule, RenderError> {
    use std::fmt::Write as _;

    let module = module_name(package_name);
    let mut names = std::collections::BTreeSet::new();
    let mut name_map = Vec::with_capacity(definitions.len());
    let mut text = match target {
        Target::Tla => format!("---- MODULE {module} ----\n"),
        Target::Lean => format!("namespace {module}\n\n"),
    };
    for (index, definition) in definitions.iter().enumerate() {
        if !names.insert(definition.name.as_str()) {
            return Err(RenderError::DuplicateName(definition.name.clone()));
        }
        if definition.region[0] > definition.region[1] {
            return Err(RenderError::ReversedRegion(definition.name.clone()));
        }
        if !definition.name.is_ascii() || !definition.node.is_ascii() || !definition.body.is_ascii()
        {
            return Err(RenderError::NonAscii(definition.name.clone()));
        }
        if definition.body.trim() != definition.body
            || definition.body.contains('\r')
            || definition
                .body
                .lines()
                .any(|line| line.ends_with(' ') || line.ends_with('\t'))
        {
            return Err(RenderError::NonCanonicalBody(definition.name.clone()));
        }
        if index > 0 {
            text.push_str("\n\n");
        }
        let comment = match target {
            Target::Tla => "\\* fsl:",
            Target::Lean => "-- fsl:",
        };
        writeln!(
            &mut text,
            "{comment} {} [{}, {})",
            definition.node, definition.region[0], definition.region[1]
        )
        .expect("writing into a String cannot fail");
        text.push_str(&definition.body);
        name_map.push(NameMapEntry {
            name: definition.name.clone(),
            node: definition.node.clone(),
            region: definition.region,
        });
    }
    match target {
        Target::Tla => {
            if !definitions.is_empty() {
                text.push('\n');
            }
            text.push_str("====\n");
        }
        Target::Lean => {
            if !definitions.is_empty() {
                text.push_str("\n\n");
            }
            writeln!(&mut text, "end {module}").expect("writing into a String cannot fail");
        }
    }
    Ok(GeneratedModule {
        module,
        text,
        name_map,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        mangle_name, module_name, render_lean, render_tla, target_name, Definition, RenderError,
    };

    #[test]
    fn escapes_names_injectively_and_emits_ascii() {
        assert_eq!(mangle_name("a_b"), "a__b");
        assert_eq!(mangle_name("a__b"), "a____b");
        assert_eq!(mangle_name("Shop::Order"), "Shop_N_Order");
        assert_eq!(mangle_name("café"), "caf_uE9_");
        assert_eq!(target_name("op", "MODULE"), "op_MODULE");
        assert_eq!(target_name("op", "Next"), "op_Next");
        assert_eq!(target_name("op", "theorem"), "op_theorem");
        assert_eq!(module_name("Shop"), "M_Shop");
    }

    #[test]
    fn emits_canonical_text_and_name_maps_for_both_targets() {
        let definitions = [Definition {
            name: "Init".to_owned(),
            node: "node-7".to_owned(),
            region: [10, 20],
            body: "Init == TRUE".to_owned(),
        }];
        let tla = render_tla("Shop", &definitions).unwrap();
        assert_eq!(
            tla.text,
            "---- MODULE M_Shop ----\n\\* fsl: node-7 [10, 20)\nInit == TRUE\n====\n"
        );
        assert_eq!(tla.name_map[0].node, "node-7");
        let lean = render_lean("Shop", &definitions).unwrap();
        assert_eq!(
            lean.text,
            "namespace M_Shop\n\n-- fsl: node-7 [10, 20)\nInit == TRUE\n\nend M_Shop\n"
        );
        assert!(tla.text.is_ascii() && lean.text.is_ascii());
    }

    #[test]
    fn refuses_duplicate_names_non_ascii_and_noncanonical_bodies() {
        let definition = Definition {
            name: "Init".to_owned(),
            node: "node-7".to_owned(),
            region: [0, 1],
            body: "Init == TRUE".to_owned(),
        };
        assert!(matches!(
            render_tla("Shop", &[definition.clone(), definition.clone()]),
            Err(RenderError::DuplicateName(_))
        ));
        let mut non_ascii = definition.clone();
        non_ascii.body = "Init == \"é\"".to_owned();
        assert!(matches!(
            render_lean("Shop", &[non_ascii]),
            Err(RenderError::NonAscii(_))
        ));
        let mut trailing = definition;
        trailing.body.push(' ');
        assert!(matches!(
            render_tla("Shop", &[trailing]),
            Err(RenderError::NonCanonicalBody(_))
        ));
        let crlf = Definition {
            name: "Init".to_owned(),
            node: "node-7".to_owned(),
            region: [0, 1],
            body: "Init == TRUE\r".to_owned(),
        };
        assert!(matches!(
            render_tla("Shop", &[crlf]),
            Err(RenderError::NonCanonicalBody(_))
        ));
    }
}
