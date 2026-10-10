//! Public UI defaults selected for Warpai; explicit user preferences take precedence.

use std::{collections::BTreeMap, sync::OnceLock};

use serde_json::Value;

use crate::SettingsValue;

pub fn owner_defaults() -> &'static BTreeMap<String, Value> {
    static VALUES: OnceLock<BTreeMap<String, Value>> = OnceLock::new();
    VALUES.get_or_init(|| {
        serde_json::from_str(include_str!("owner-defaults.json"))
            .expect("Bundled public defaults must be valid JSON")
    })
}

/// Uses the same typed default for initialization, Reset and the settings schema.
pub fn configured_default<T: SettingsValue>(path: Option<&str>, private: bool, fallback: T) -> T {
    if private {
        return fallback;
    }
    path.and_then(|path| owner_defaults().get(path))
        .and_then(T::from_file_value)
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_typed_and_never_override_private_settings() {
        assert_eq!(configured_default(Some("appearance.text.font_size"), false, 11.0_f32), 14.0);
        assert_eq!(configured_default(Some("appearance.text.font_size"), true, 11.0_f32), 11.0);
        assert_eq!(configured_default(Some("appearance.text.font_size"), false, false), false);
        assert_eq!(configured_default(Some("unrelated.setting"), false, true), true);
        assert!(owner_defaults().keys().all(|key| !key.contains("account") && !key.contains("path")));
    }
}
