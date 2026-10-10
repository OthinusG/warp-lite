//! Import legacy local data before any settings or durable state are opened.

use std::{collections::BTreeMap, fs, io, io::Read, path::Path};

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
    let root_marker = root.join(".config-root-migration-complete");
    if !root_marker.try_exists()? {
        let relative = root.strip_prefix(home.join(".config/warpai"))?;
        let former = home.join(".config/.warpai").join(relative);
        if former.symlink_metadata().is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            anyhow::bail!("Former managed root is a symlink");
        }
        import_directory(&former, root)?;
        mark_complete(root, &root_marker)?;
    }
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
    mark_complete(root, &marker)?;
    Ok(())
}

fn mark_complete(root: &Path, marker: &Path) -> io::Result<()> {
    let temporary = tempfile::NamedTempFile::new_in(root)?;
    temporary.as_file().sync_all()?;
    match temporary.persist_noclobber(marker) {
        Ok(_) => (),
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.error),
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
            if sqlite_sidecar(&entry.path())? {
                continue;
            }
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
    if is_sqlite(source)? {
        snapshot_sqlite(source, temporary.path())?;
    } else {
        io::copy(&mut fs::File::open(source)?, temporary.as_file_mut())?;
    }
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

fn is_sqlite(path: &Path) -> io::Result<bool> {
    let mut header = [0_u8; 16];
    match fs::File::open(path)?.read_exact(&mut header) {
        Ok(()) => Ok(&header == b"SQLite format 3\0"),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(error),
    }
}

fn sqlite_sidecar(path: &Path) -> io::Result<bool> {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else { return Ok(false); };
    for suffix in ["-wal", "-shm", "-journal"] {
        if let Some(base) = name.strip_suffix(suffix) {
            let database = path.with_file_name(base);
            if database.symlink_metadata().is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink()) {
                return is_sqlite(&database);
            }
        }
    }
    Ok(false)
}

/// Preserve row IDs and include committed WAL data in one native SQLite snapshot.
fn snapshot_sqlite(source: &Path, destination: &Path) -> io::Result<()> {
    use libsqlite3_sys as sqlite;
    let filename = |path: &Path| {
        std::ffi::CString::new(path.to_str().ok_or_else(|| io::Error::other("Invalid database path"))?)
            .map_err(|_| io::Error::other("Invalid database path"))
    };
    let source = filename(source)?;
    let destination = filename(destination)?;
    let mut input = std::ptr::null_mut();
    let mut output = std::ptr::null_mut();
    // SAFETY: filenames outlive both connections; every opened handle and backup is closed.
    let success = unsafe {
        let opened = sqlite::sqlite3_open_v2(source.as_ptr(), &mut input, sqlite::SQLITE_OPEN_READONLY, std::ptr::null()) == sqlite::SQLITE_OK
            && sqlite::sqlite3_open_v2(destination.as_ptr(), &mut output, sqlite::SQLITE_OPEN_READWRITE, std::ptr::null()) == sqlite::SQLITE_OK;
        let mut success = false;
        if opened {
            let backup = sqlite::sqlite3_backup_init(output, c"main".as_ptr(), input, c"main".as_ptr());
            if !backup.is_null() {
                let copied = sqlite::sqlite3_backup_step(backup, -1) == sqlite::SQLITE_DONE;
                success = sqlite::sqlite3_backup_finish(backup) == sqlite::SQLITE_OK && copied;
            }
        }
        if !output.is_null() { success &= sqlite::sqlite3_close(output) == sqlite::SQLITE_OK; }
        if !input.is_null() { success &= sqlite::sqlite3_close(input) == sqlite::SQLITE_OK; }
        success
    };
    if success { Ok(()) } else { Err(io::Error::other("Cannot snapshot managed SQLite database")) }
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
