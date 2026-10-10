//! Explicit interface text localization; terminal and editor content bypass this module.

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU8, Ordering},
        OnceLock,
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-Hans")]
    SimplifiedChinese,
    #[serde(rename = "zh-Hant")]
    TraditionalChinese,
}

#[cfg(feature = "settings_value")]
impl settings_value::SettingsValue for Language {}

impl Language {
    pub const ALL: [Self; 3] = [
        Self::English,
        Self::SimplifiedChinese,
        Self::TraditionalChinese,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
            Self::TraditionalChinese => "繁體中文",
        }
    }
}

static LANGUAGE: AtomicU8 = AtomicU8::new(0);

/// Configure once at application startup so cached menus never mix languages.
pub fn set_language(language: Language) {
    LANGUAGE.store(language as u8, Ordering::Relaxed);
}

pub fn language() -> Language {
    match LANGUAGE.load(Ordering::Relaxed) {
        1 => Language::SimplifiedChinese,
        2 => Language::TraditionalChinese,
        _ => Language::English,
    }
}

fn catalog() -> &'static BTreeMap<String, [String; 2]> {
    static CATALOG: OnceLock<BTreeMap<String, [String; 2]>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("locales/interface.json"))
            .expect("Bundled interface translations must be valid")
    })
}

pub fn text(source: &str) -> &str {
    text_in(language(), source)
}

pub fn text_in(language: Language, source: &str) -> &str {
    match language {
        Language::English => source,
        Language::SimplifiedChinese => catalog()
            .get(source)
            .map_or(source, |entry| entry[0].as_str()),
        Language::TraditionalChinese => catalog()
            .get(source)
            .map_or(source, |entry| entry[1].as_str()),
    }
}

/// Substitute only catalog placeholders; braces in user-provided values stay literal.
pub fn format_text(source: &'static str, values: &[(&str, &str)]) -> String {
    format_text_in(language(), source, values)
}

pub fn format_text_in(language: Language, source: &'static str, values: &[(&str, &str)]) -> String {
    let template = text_in(language, source);
    let mut characters = template.chars().peekable();
    let mut result = String::with_capacity(template.len());
    let mut positional = 0;
    while let Some(character) = characters.next() {
        if (character == '{' || character == '}') && characters.peek() == Some(&character) {
            characters.next();
            result.push(character);
        } else if character == '{' {
            let mut field = String::new();
            for character in characters.by_ref() {
                if character == '}' {
                    break;
                }
                field.push(character);
            }
            if field.is_empty() || field.starts_with(':') {
                field = format!("{positional}{field}");
                positional += 1;
            }
            if let Some((_, value)) = values.iter().find(|(key, _)| *key == field) {
                result.push_str(value);
            } else {
                debug_assert!(false, "Interface template arguments must match the catalog");
                result.push('{');
                result.push_str(&field);
                result.push('}');
            }
        } else {
            result.push(character);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes_and_english_interface_remain_stable() {
        for (language, code) in Language::ALL.into_iter().zip(["en", "zh-Hans", "zh-Hant"]) {
            assert_eq!(
                serde_json::to_string(&language).unwrap(),
                format!("\"{code}\"")
            );
            assert_eq!(
                serde_json::from_str::<Language>(&format!("\"{code}\"")).unwrap(),
                language
            );
        }
        assert_eq!(text_in(Language::English, "Language"), "Language");
        assert_eq!(text_in(Language::SimplifiedChinese, "Language"), "语言");
        assert_eq!(text_in(Language::TraditionalChinese, "Language"), "語言");
        assert_eq!(
            text_in(Language::SimplifiedChinese, "Unclassified fixture"),
            "Unclassified fixture"
        );
        assert!(catalog()
            .values()
            .all(|entry| entry.iter().all(|value| !value.is_empty())));
    }

    #[test]
    fn templates_preserve_user_text_and_literal_braces() {
        for language in Language::ALL {
            let text = format_text_in(language, "Open {name}", &[("name", "owned {name} 中文")]);
            assert!(text.ends_with("owned {name} 中文"));
        }
        assert_eq!(
            format_text_in(Language::English, "{{literal}} {}", &[("0", "{value}")]),
            "{literal} {value}"
        );
    }
}
