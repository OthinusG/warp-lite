use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use warp_core::settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};

/// Legacy variants remain readable for saved preferences; all use Warpai artwork.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Sequence,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "The app icon displayed in the dock.",
    rename_all = "snake_case"
)]
pub enum AppIcon {
    /// White bound-page/terminal glyph on the black liquid-material Warpai tile.
    #[default]
    #[schemars(description = "Default")]
    Default,
    #[schemars(description = "Aurora")]
    Aurora,
    #[schemars(description = "Classic 1")]
    Classic1,
    #[schemars(description = "Classic 2")]
    Classic2,
    #[schemars(description = "Classic 3")]
    Classic3,
    #[schemars(description = "Comets")]
    Comets,
    /// Cow icon, for Code on Warp launch.
    #[schemars(description = "Cow")]
    Cow,
    #[schemars(description = "Glass Sky")]
    GlassSky,
    #[schemars(description = "Glitch")]
    Glitch,
    /// White glyph on black background with blue/green glow on the side, set in Oct 2024 brand refresh.
    #[schemars(description = "Glow")]
    Glow,
    #[schemars(description = "Holographic")]
    Holographic,
    #[schemars(description = "Mono")]
    Mono,
    #[schemars(description = "Neon")]
    Neon,
    /// Blue/green glyph on black background.
    #[schemars(description = "Original")]
    Original,
    #[schemars(description = "Starburst")]
    Starburst,
    #[schemars(description = "Sticker")]
    Sticker,
    /// Previous default icon with solid blue background.
    #[schemars(description = "Warpai 1")]
    WarpOne,
}

impl std::fmt::Display for AppIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match &self {
            AppIcon::Default => "Default",
            AppIcon::Aurora => "Aurora",
            AppIcon::Classic1 => "Classic 1",
            AppIcon::Classic2 => "Classic 2",
            AppIcon::Classic3 => "Classic 3",
            AppIcon::Comets => "Comets",
            AppIcon::GlassSky => "Glass Sky",
            AppIcon::Glitch => "Glitch",
            AppIcon::Cow => "Cow",
            AppIcon::Glow => "Glow",
            AppIcon::Holographic => "Holographic",
            AppIcon::Mono => "Mono",
            AppIcon::Neon => "Neon",
            AppIcon::Original => "Original",
            AppIcon::Starburst => "Starburst",
            AppIcon::Sticker => "Sticker",
            AppIcon::WarpOne => "Warpai 1",
        };
        write!(f, "{value}")
    }
}

impl AppIconSettings {
    pub fn get_base_icon_file_name(_icon: AppIcon) -> &'static str {
        "warpai"
    }
}

define_settings_group!(AppIconSettings, settings: [
    app_icon: AppIconState {
        type: AppIcon,
        default: AppIcon::Default,
        supported_platforms: SupportedPlatforms::MAC,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        storage_key: "AppIcon",
        toml_path: "appearance.icon.app_icon",
        description: "The app icon displayed in the dock.",
    },
    show_dock_icon: ShowDockIconState {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::MAC,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        storage_key: "ShowDockIcon",
        toml_path: "appearance.icon.show_dock_icon",
        description: "Whether Warpai is shown in the macOS Dock and Cmd-Tab switcher.",
    },
]);
