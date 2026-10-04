//! Import legacy local data before any settings or durable state are opened.

use std::{collections::BTreeMap, fs, io, path::Path};

use warp_core::{
    channel::{Channel, ChannelState},
    paths,
};

/// Imports only missing entries, preserving both existing settings and recovery sources.
/// A failure aborts startup instead of silently opening an empty preferences store.
pub(crate) fn migrate() -> anyhow::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine user home"))?;
    migrate_to(
        &home,
        &paths::data_dir(),
        ChannelState::data_profile().as_deref(),
        ChannelState::channel(),
    )
}

fn migrate_to(
    home: &Path,
    root: &Path,
    profile: Option<&str>,
    channel: Channel,
) -> anyhow::Result<()> {
    private_directory(root)?;
    let marker = root.join(".legacy-migration-complete");
    if marker.try_exists()? {
        return Ok(());
    }
    let suffix = profile
        .as_ref()
        .map(|p| format!("-{p}"))
        .unwrap_or_default();
    let legacy_name = match channel {
        Channel::Oss => ".warp-oss",
        Channel::Stable => ".warp",
        Channel::Preview => ".warp-preview",
        Channel::Dev => ".warp-dev",
        Channel::Local => ".warp-local",
        Channel::Integration => ".warp-integration",
    };
    let fallback = if matches!(channel, Channel::Oss | Channel::Preview) {
        ".warp"
    } else {
        legacy_name
    };
    for name in [
        format!("{legacy_name}{suffix}"),
        format!("{fallback}{suffix}"),
    ] {
        import_directory(&home.join(name), &root)?;
    }

    // Native review profiles must never import ordinary user data.
    if profile.is_none() {
        #[cfg(target_os = "macos")]
        for domain in legacy_macos_domains(channel) {
            let legacy = home.join("Library/Application Support").join(domain);
            import_directory(&legacy, &root.join("state"))?;
        }
        #[cfg(target_os = "windows")]
        for name in legacy_windows_names(channel) {
            let Some(legacy) = directories::ProjectDirs::from("dev", "warp", name) else {
                continue;
            };
            import_directory(legacy.config_local_dir(), &root)?;
            import_directory(legacy.data_dir(), &root)?;
            import_directory(legacy.data_local_dir(), &root.join("state"))?;
            import_directory(legacy.cache_dir(), &root.join("cache"))?;
        }
        import_native_preferences(home, root, channel)?;
    }

    // The former JSON backend mixed public and private settings. Separate files
    // prevent independent cached stores from overwriting each other's changes.
    let public = root.join("user_preferences.json");
    if public.is_file() {
        import_file(&public, &root.join("private_preferences.json"))?;
    }
    let temporary = tempfile::NamedTempFile::new_in(root)?;
    temporary.as_file().sync_all()?;
    match temporary.persist_noclobber(marker) {
        Ok(_) => (),
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.error.into()),
    }
    Ok(())
}

fn private_directory(path: &Path) -> io::Result<()> {
    if path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Managed directory is a symlink",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    fs::create_dir_all(path)?;
    Ok(())
}

fn import_directory(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = match source.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(());
    }
    private_directory(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            if target
                .symlink_metadata()
                .is_ok_and(|m| !m.is_dir() || m.file_type().is_symlink())
            {
                continue;
            }
            import_directory(&entry.path(), &target)?;
        } else if kind.is_file() {
            import_file(&entry.path(), &target)?;
        }
    }
    Ok(())
}

fn import_file(source: &Path, destination: &Path) -> io::Result<()> {
    if destination.symlink_metadata().is_ok() {
        return Ok(());
    }
    let mut temporary = tempfile::NamedTempFile::new_in(destination.parent().unwrap())?;
    io::copy(&mut fs::File::open(source)?, temporary.as_file_mut())?;
    temporary
        .as_file()
        .set_permissions(source.metadata()?.permissions())?;
    temporary.as_file().sync_all()?;
    match temporary.persist_noclobber(destination) {
        Ok(_) => Ok(()),
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e.error),
    }
}

fn import_native_preferences(home: &Path, root: &Path, channel: Channel) -> anyhow::Result<()> {
    let destination = root.join("user_preferences.json");
    if destination.exists() {
        return Ok(());
    }
    let mut values = BTreeMap::<String, String>::new();
    #[cfg(target_os = "macos")]
    for domain in legacy_macos_domains(channel) {
        let source = home
            .join("Library/Preferences")
            .join(format!("{domain}.plist"));
        if !source.exists() {
            continue;
        }
        let plist = plist::Value::from_file(&source)?;
        if let Some(dictionary) = plist.as_dictionary() {
            for (key, value) in dictionary {
                if let Some(value) = value.as_string() {
                    values
                        .entry(key.clone())
                        .or_insert_with(|| value.to_owned());
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    for name in legacy_windows_names(channel) {
        let key = match windows_registry::CURRENT_USER.open(format!("Software\\Warp.dev\\{name}")) {
            Ok(key) => key,
            Err(e) if e.code() == windows_result::HRESULT::from_win32(2) => continue,
            Err(e) => return Err(e.into()),
        };
        for (key, value) in key.values()? {
            if let windows_registry::Value::String(value) = value {
                values.entry(key).or_insert(value);
            }
        }
    }
    let _ = (home, channel);
    if !values.is_empty() {
        let mut temporary = tempfile::NamedTempFile::new_in(root)?;
        serde_json::to_writer(
            temporary.as_file_mut(),
            &serde_json::json!({"prefs": values}),
        )?;
        temporary.as_file().sync_all()?;
        match temporary.persist_noclobber(destination) {
            Ok(_) => (),
            Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.error.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "user_data_migration_tests.rs"]
mod tests;

#[cfg(target_os = "macos")]
fn legacy_macos_domains(channel: Channel) -> &'static [&'static str] {
    match channel {
        Channel::Oss => &["dev.warp-lite.WarpLite", "dev.warp.WarpOss"],
        Channel::Stable => &["dev.warp.Warp-Stable", "dev.warp.Warp"],
        Channel::Preview => &["dev.warp.Warp-Preview", "dev.warp.WarpPreview"],
        Channel::Dev => &["dev.warp.Warp-Dev", "dev.warp.WarpDev"],
        Channel::Local => &["dev.warp.Warp-Local", "dev.warp.WarpLocal"],
        Channel::Integration => &["dev.warp.WarpIntegration"],
    }
}

#[cfg(target_os = "windows")]
fn legacy_windows_names(channel: Channel) -> &'static [&'static str] {
    match channel {
        Channel::Oss => &["WarpOss", "Warpai", "WarpLite"],
        Channel::Stable => &["Warp"],
        Channel::Preview => &["WarpPreview"],
        Channel::Dev => &["WarpDev"],
        Channel::Local => &["WarpLocal"],
        Channel::Integration => &["WarpIntegration"],
    }
}
