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

#[cfg(test)]
mod tests {
    use super::{mangle_name, module_name, target_name};

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
}
