//! Helper functions for retrieving base paths for storing config/data files.
//!
//! This file should not be directly exposed to or used in integration tests;
//! any paths computed using these functions should be exposed to integration
//! tests through use-case-specific helper functions.
//!
//! `_local_dir` variants of functions are for storing non-portable data, where
//! "portable" refers to the ability to copy that file to another machine.
//! Some examples of non-portable data include things that reference local
//! paths (which may not exist on a different machine), such as paths to shell
//! binaries or user-added theme files.
//!
//! TODO(vorporeal): In general, we should be returning Option<PathBuf> or
//! Result<PathBuf> when we can't compute the home directory instead of
//! returning a relative path.

use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::channel::{Channel, ChannelState};

/// The name of the directory in which to put non-global Warp-specific files.
///
/// This should be used, for example, as the base directory under which
/// repository workflows would be stored (in "./.warp/workflows").
pub const WARP_CONFIG_DIR: &str = ".warp";

/// The name of the folder that stores Warp execution logs and network logs.
/// This is currently only used on Windows to maintain backwards compatibility.
pub const WARP_LOGS_DIR: &str = "logs";

/// Returns the home-relative managed root. Debug profiles remain inside it.
pub fn warp_home_config_dir_name() -> String {
    let root = PathBuf::from(".config").join("warpai");
    let root = match ChannelState::channel() {
        Channel::Oss | Channel::Stable => root,
        channel => root.join("channels").join(channel.to_string()),
    };
    let root = if let Some(profile) = ChannelState::data_profile() {
        assert!(
            !profile.is_empty()
                && profile != "."
                && profile != ".."
                && profile
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c)),
            "Data profile must be a single safe directory name"
        );
        root.join("profiles").join(profile)
    } else {
        root
    };
    root.to_string_lossy().into_owned()
}

pub fn warp_home_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(warp_home_config_dir_name()))
}

pub fn warp_home_skills_dir() -> Option<PathBuf> {
    warp_home_config_dir().map(|root| root.join("skills"))
}

pub fn warp_home_mcp_config_file_path() -> Option<PathBuf> {
    warp_home_config_dir().map(|root| root.join(".mcp.json"))
}

/// Portable configuration and non-portable settings share the managed root.
pub fn data_dir() -> PathBuf {
    warp_home_config_dir().expect("Cannot determine Warpai home directory")
}

pub fn config_local_dir() -> PathBuf {
    data_dir()
}

/// Returns the base directory for general config files. Useful for accessing the config files for
/// other programs.
pub fn base_config_dir() -> PathBuf {
    BaseDirs::new()
        .map(|dirs| dirs.config_dir().to_owned())
        .unwrap_or_default()
}

/// Returns the path to the directory where non-portable application state data
/// should be stored.
///
/// This is the appropriate home for files like our sqlite database, which
/// contains durable but non-critical and non-portable data like what windows
/// the user had open and cached state of known Warp Drive objects.
pub fn state_dir() -> PathBuf {
    data_dir().join("state")
}

/// Keep durable state under the same private user-owned configuration root.
pub fn secure_state_dir() -> Option<PathBuf> {
    if ChannelState::channel() == Channel::Integration {
        None
    } else {
        Some(state_dir())
    }
}

/// Returns the path to the directory containing the user's custom themes.
pub fn themes_dir() -> PathBuf {
    data_dir().join("themes")
}

/// Returns the path to the directory where files can be stored for caching
/// purposes.
///
/// This is a good place to store things like user profile pictures, which
/// we don't want to fetch on every launch of the app but can be safely
/// deleted by the OS.
pub fn cache_dir() -> PathBuf {
    data_dir().join("cache")
}

/// Returns a display-ready version of the path that is formatted in a
/// home-dir-relative manner, if appropriate.
pub fn home_relative_path(path: &Path) -> String {
    #[cfg(unix)]
    if let Some(base_dirs) = directories::BaseDirs::new() {
        if let Ok(relative_path) = path.strip_prefix(base_dirs.home_dir()) {
            return format!("~/{}", relative_path.display());
        }
    };

    path.display().to_string()
}

/// Returns the path to resources included in the Warp distribution.
///
/// Unlike [`warpui::AssetProvider`] assets, which are generally embedded in the binary, these are
/// stored on the filesystem alongside the rest of Warp.
///
/// ## macOS
/// The resources directory is `$APP_DIR/Contents/Resources` (e.g. `/Applications/Warp.app/Contents/Resources`).
///
/// ## Linux
/// The resources directory is `$INSTALL_DIR/resources`, where `$INSTALL_DIR` depends on the
/// specific package manager. For example, on Ubuntu this might be `/opt/warpdotdev/warp-terminal/resources`.
///
/// ## Windows
/// The resources directory is `$INSTALL_DIR/resources`, where `$INSTALL_DIR` is the directory
/// containing the Warp executable (e.g. `C:\Program Files\WarpDev\resources`).
pub fn bundled_resources_dir() -> Option<PathBuf> {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            crate::macos::get_bundle_path().ok()
                .map(|bundle_path| {
                    PathBuf::from(bundle_path)
                        .join("Contents")
                        .join("Resources")
                })
        } else if #[cfg(target_os = "linux")] {
            std::env::current_exe()
                .ok()
                .and_then(|executable| std::fs::canonicalize(executable).ok())
                .and_then(|executable| executable.parent().map(|parent| parent.join("resources")))
        } else if #[cfg(target_os = "windows")] {
            std::env::current_exe()
                .ok()
                .and_then(|executable| std::fs::canonicalize(executable).ok())
                .and_then(|executable| executable.parent().map(|parent| parent.join("resources")))
        } else {
            None
        }
    }
}

#[cfg(all(test, feature = "local_fs"))]
#[path = "paths_tests.rs"]
mod tests;
