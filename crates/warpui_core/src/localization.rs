//! Explicit interface text localization; terminal and editor content bypass this module.

use std::{collections::BTreeMap, sync::{OnceLock, atomic::{AtomicU8, Ordering}}};

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
    pub const ALL: [Self; 3] = [Self::English, Self::SimplifiedChinese, Self::TraditionalChinese];

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
    CATALOG.get_or_init(|| serde_json::from_str(include_str!("locales/interface.json"))
        .expect("Bundled interface translations must be valid"))
}

pub fn text(source: &'static str) -> &'static str {
    text_in(language(), source)
}

pub fn text_in(language: Language, source: &'static str) -> &'static str {
    match language {
        Language::English => source,
        Language::SimplifiedChinese => catalog().get(source).map_or(source, |entry| entry[0].as_str()),
        Language::TraditionalChinese => catalog().get(source).map_or(source, |entry| entry[1].as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes_and_english_interface_remain_stable() {
        for (language, code) in Language::ALL.into_iter().zip(["en", "zh-Hans", "zh-Hant"]) {
            assert_eq!(serde_json::to_string(&language).unwrap(), format!("\"{code}\""));
            assert_eq!(serde_json::from_str::<Language>(&format!("\"{code}\"")).unwrap(), language);
        }
        assert_eq!(text_in(Language::English, "Language"), "Language");
        assert_eq!(text_in(Language::SimplifiedChinese, "Language"), "语言");
        assert_eq!(text_in(Language::TraditionalChinese, "Language"), "語言");
        assert_eq!(text_in(Language::SimplifiedChinese, "Unclassified fixture"), "Unclassified fixture");
        assert!(catalog().values().all(|entry| entry.iter().all(|value| !value.is_empty())));
    }
}
