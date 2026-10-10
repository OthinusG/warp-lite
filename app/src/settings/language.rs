use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};
use warpui::localization::Language;

define_settings_group!(LanguageSettings, settings: [
    language: InterfaceLanguage {
        type: Language,
        default: Language::English,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        toml_path: "appearance.language",
        description: "The interface language; restart Warpai to apply changes.",
    },
]);
