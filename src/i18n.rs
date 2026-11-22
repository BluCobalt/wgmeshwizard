//! Small i18n layer over `rust-i18n`.
//!
//! Translations live in `res/i18n/<locale>.yml`. Keys are the English source
//! strings, which makes it easy to add a translation without inventing a
//! second naming scheme. When a key is missing from the active locale (and its
//! fallback chain), the English key itself is returned.

use std::borrow::Cow;

/// Look up a translation, falling back to the English source string.
pub fn translate(key: &str) -> String {
    let locale = rust_i18n::locale();
    crate::_rust_i18n_try_translate(&locale, key)
        .map(Cow::into_owned)
        .unwrap_or_else(|| key.to_owned())
}

/// Apply a stored language preference. `system` selects the process locale if
/// one is available, otherwise English.
pub fn set_language(language: &str) {
    let locale = if language == "system" {
        system_locale()
    } else {
        language.to_owned()
    };
    rust_i18n::set_locale(&locale);
}

/// Locales that have a file in `res/i18n/`.
pub fn available_languages() -> Vec<String> {
    rust_i18n::available_locales!()
        .into_iter()
        .map(Cow::into_owned)
        .collect()
}

fn system_locale() -> String {
    // Mainstream desktop Linux/macOS set LANG or LC_ALL. Windows tends to leave
    // both unset unless a Unix-like environment is present.
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en".to_owned());

    let language = raw
        .split(['.', '@'])
        .next()
        .unwrap_or("en")
        .split('_')
        .next()
        .unwrap_or("en")
        .trim();

    if language.is_empty() {
        "en".to_owned()
    } else {
        language.to_ascii_lowercase()
    }
}

/// Translate a literal source string and substitute `%{name}` placeholders.
///
/// ```
/// tr!("ui.add_host_near.added_name_and_centered_the_canvas", name = "alpha")
/// ```
macro_rules! tr {
    ($key:literal $(, $name:ident = $value:expr)* $(,)?) => {{
        let text = $crate::i18n::translate($key);
        $(
            let text = text.replace(
                concat!("%{", stringify!($name), "}"),
                &format!("{}", $value),
            );
        )*
        text
    }};
}

pub(crate) use tr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_locale_parses_common_values() {
        // The function reads the process environment, so just ensure the
        // result is a non-empty language code.
        assert!(!system_locale().is_empty());
    }

    #[test]
    fn missing_translation_falls_back_to_source() {
        rust_i18n::set_locale("en");
        assert_eq!(translate("ui.canvas.no_hosts_yet"), "No hosts yet");
        assert_eq!(
            translate("A string that is not translated"),
            "A string that is not translated"
        );
        assert!(
            available_languages()
                .iter()
                .any(|language| language == "en")
        );
    }
}

#[cfg(test)]
mod catalogue_tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs")
                && path.file_name().and_then(|name| name.to_str()) != Some("i18n.rs")
            {
                out.push(path);
            }
        }
    }

    fn tr_keys(source: &str) -> Vec<String> {
        let mut keys = Vec::new();
        let bytes = source.as_bytes();
        let mut index = 0;
        while let Some(found) = source[index..].find("tr!(") {
            let mut cursor = index + found + 4;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'"' {
                cursor += 1;
                let start = cursor;
                while cursor < bytes.len() && bytes[cursor] != b'"' {
                    if bytes[cursor] == b'\\' {
                        cursor += 1;
                    }
                    cursor += 1;
                }
                if cursor < bytes.len() {
                    keys.push(source[start..cursor].to_owned());
                }
                index = cursor + 1;
            } else {
                index += found + 4;
            }
        }
        keys
    }

    fn catalogue_keys() -> HashSet<String> {
        let text =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("res/i18n/en.yml"))
                .unwrap();
        text.lines()
            .filter_map(|line| {
                let line = line.trim();
                if !line.starts_with('"') {
                    return None;
                }
                let rest = &line[1..];
                let end = rest.find('"')?;
                Some(rest[..end].to_owned())
            })
            .collect()
    }

    #[test]
    fn every_tr_key_exists_in_english_catalogue() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        rust_files(&root.join("src"), &mut files);

        let catalogue = catalogue_keys();
        let mut used = HashSet::new();
        for file in files {
            let source = fs::read_to_string(&file).unwrap();
            for key in tr_keys(&source) {
                used.insert(key);
            }
        }

        let missing: Vec<_> = used.difference(&catalogue).cloned().collect();
        assert!(
            missing.is_empty(),
            "missing keys in res/i18n/en.yml: {missing:?}"
        );

        let unused: Vec<_> = catalogue.difference(&used).cloned().collect();
        assert!(
            unused.is_empty(),
            "unused keys in res/i18n/en.yml: {unused:?}"
        );
    }
}
