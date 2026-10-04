use super::*;

#[test]
fn legacy_import_preserves_precedence_nested_data_and_recovery_sources() {
    let temp = tempfile::tempdir().unwrap();
    let primary = temp.path().join(".warp-oss");
    let fallback = temp.path().join(".warp");
    let target = temp.path().join(".config/.warpai");
    fs::create_dir_all(primary.join("themes")).unwrap();
    fs::create_dir_all(&fallback).unwrap();
    fs::write(primary.join("settings.toml"), "primary").unwrap();
    fs::write(primary.join("themes/test.yaml"), "theme").unwrap();
    fs::write(fallback.join("settings.toml"), "fallback").unwrap();
    fs::write(fallback.join("keybindings.yaml"), "bindings").unwrap();
    import_directory(&primary, &target).unwrap();
    import_directory(&fallback, &target).unwrap();
    assert_eq!(
        fs::read_to_string(target.join("settings.toml")).unwrap(),
        "primary"
    );
    assert_eq!(
        fs::read_to_string(target.join("themes/test.yaml")).unwrap(),
        "theme"
    );
    assert_eq!(
        fs::read_to_string(target.join("keybindings.yaml")).unwrap(),
        "bindings"
    );
    assert!(primary.join("settings.toml").exists());
    fs::write(target.join("settings.toml"), "new settings").unwrap();
    import_directory(&primary, &target).unwrap();
    assert_eq!(
        fs::read_to_string(target.join("settings.toml")).unwrap(),
        "new settings"
    );
    assert_eq!(
        fs::read_to_string(primary.join("settings.toml")).unwrap(),
        "primary"
    );
}

#[test]
fn import_missing_source_is_noop_and_invalid_destination_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    import_directory(&temp.path().join("missing"), &target).unwrap();
    assert!(!target.exists());
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(&target, "existing file").unwrap();
    assert!(import_directory(&source, &target).is_err());
    assert_eq!(fs::read_to_string(target).unwrap(), "existing file");
}

#[cfg(unix)]
#[test]
fn import_never_traverses_source_or_destination_symlinks() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let outside = temp.path().join("outside");
    let target = temp.path().join("target");
    fs::create_dir_all(source.join("themes")).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("untouched"), "keep").unwrap();
    symlink(&outside, source.join("linked")).unwrap();
    symlink(outside.join("untouched"), source.join("linked-file")).unwrap();
    import_directory(&source, &target).unwrap();
    assert!(!target.join("linked").exists());
    assert!(!target.join("linked-file").exists());
    assert_eq!(
        target.metadata().unwrap().permissions().mode() & 0o777,
        0o700
    );
    fs::remove_dir(target.join("themes")).unwrap();
    symlink(&outside, target.join("themes")).unwrap();
    fs::write(source.join("themes/new"), "new").unwrap();
    import_directory(&source, &target).unwrap();
    assert!(!outside.join("new").exists());
}

#[test]
fn public_and_private_file_preferences_do_not_overwrite_each_other() {
    use warpui_extras::user_preferences::{
        file_backed::FileBackedUserPreferences, UserPreferences,
    };
    let temp = tempfile::tempdir().unwrap();
    let public_path = temp.path().join("user_preferences.json");
    let private_path = temp.path().join("private_preferences.json");
    fs::write(
        &public_path,
        r#"{"prefs":{"theme":"\"Claude Warm Light\"","private_flag":"true"}}"#,
    )
    .unwrap();
    import_file(&public_path, &private_path).unwrap();
    let public = FileBackedUserPreferences::new(public_path.clone()).unwrap();
    let private = FileBackedUserPreferences::new(private_path.clone()).unwrap();
    assert_eq!(
        public.read_value("theme").unwrap().as_deref(),
        Some("\"Claude Warm Light\"")
    );
    assert_eq!(
        private.read_value("private_flag").unwrap().as_deref(),
        Some("true")
    );
    private.write_value("private_flag", "false".into()).unwrap();
    public.write_value("theme", "\"Light\"".into()).unwrap();
    assert_eq!(
        FileBackedUserPreferences::new(private_path)
            .unwrap()
            .read_value("private_flag")
            .unwrap()
            .as_deref(),
        Some("false")
    );
    assert_eq!(
        FileBackedUserPreferences::new(public_path)
            .unwrap()
            .read_value("theme")
            .unwrap()
            .as_deref(),
        Some("\"Light\"")
    );
}

#[cfg(target_os = "macos")]
#[test]
fn native_preferences_import_preserves_values_and_existing_destination() {
    use warpui_extras::user_preferences::{
        file_backed::FileBackedUserPreferences, UserPreferences,
    };
    let temp = tempfile::tempdir().unwrap();
    let legacy = temp.path().join("Library/Preferences");
    let root = temp.path().join(".config/.warpai");
    fs::create_dir_all(&legacy).unwrap();
    private_directory(&root).unwrap();
    let mut values = plist::Dictionary::new();
    values.insert(
        "theme".into(),
        plist::Value::String("\"Claude Warm Light\"".into()),
    );
    values.insert("private_flag".into(), plist::Value::String("true".into()));
    plist::Value::Dictionary(values)
        .to_file_xml(legacy.join("dev.warp-lite.WarpLite.plist"))
        .unwrap();
    import_native_preferences(temp.path(), &root, Channel::Oss).unwrap();
    let file = root.join("user_preferences.json");
    let prefs = FileBackedUserPreferences::new(file.clone()).unwrap();
    assert_eq!(
        prefs.read_value("theme").unwrap().as_deref(),
        Some("\"Claude Warm Light\"")
    );
    assert_eq!(
        prefs.read_value("private_flag").unwrap().as_deref(),
        Some("true")
    );
    prefs.write_value("theme", "\"Light\"".into()).unwrap();
    import_native_preferences(temp.path(), &root, Channel::Oss).unwrap();
    assert_eq!(
        FileBackedUserPreferences::new(file)
            .unwrap()
            .read_value("theme")
            .unwrap()
            .as_deref(),
        Some("\"Light\"")
    );
}

#[test]
fn completed_migration_does_not_restore_intentionally_reset_settings() {
    let home = tempfile::tempdir().unwrap();
    let source = home.path().join(".warp-oss-isolated-test");
    let root = home.path().join(".config/.warpai");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("settings.toml"), "old").unwrap();
    migrate_to(home.path(), &root, Some("isolated-test"), Channel::Oss).unwrap();
    assert!(root.join(".legacy-migration-complete").is_file());
    assert_eq!(
        fs::read_to_string(root.join("settings.toml")).unwrap(),
        "old"
    );
    fs::write(root.join("settings.toml"), "new").unwrap();
    fs::remove_file(root.join("settings.toml")).unwrap();
    migrate_to(home.path(), &root, Some("isolated-test"), Channel::Oss).unwrap();
    assert!(!root.join("settings.toml").exists());
}

#[test]
fn failed_migration_never_marks_completion_or_discards_legacy_data() {
    let home = tempfile::tempdir().unwrap();
    let source = home.path().join(".warp-oss-test");
    let root = home.path().join(".config/.warpai");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&root).unwrap();
    fs::create_dir(source.join("themes")).unwrap();
    fs::write(root.join("themes"), "existing destination").unwrap();
    // A blocked nested entry is retained rather than overwritten.
    fs::write(source.join("themes/retained"), "legacy").unwrap();
    migrate_to(home.path(), &root, Some("test"), Channel::Oss).unwrap();
    assert!(source.join("themes/retained").exists());
    let blocked = home.path().join("blocked");
    fs::write(&blocked, "existing file").unwrap();
    assert!(migrate_to(home.path(), &blocked, Some("test"), Channel::Oss).is_err());
    assert!(!blocked.join(".legacy-migration-complete").exists());
    assert!(source.join("themes/retained").exists());
}

#[test]
fn isolated_profile_never_imports_ordinary_user_settings() {
    let home = tempfile::tempdir().unwrap();
    let source = home.path().join(".warp-oss");
    let root = home.path().join(".config/.warpai/profiles/test");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("settings.toml"), "ordinary settings").unwrap();
    migrate_to(home.path(), &root, Some("test"), Channel::Oss).unwrap();
    assert!(!root.join("settings.toml").exists());
}

#[test]
fn channel_migration_does_not_import_other_channel_settings() {
    let home = tempfile::tempdir().unwrap();
    fs::create_dir(home.path().join(".warp-oss-test")).unwrap();
    fs::write(home.path().join(".warp-oss-test/settings.toml"), "OSS").unwrap();
    fs::create_dir(home.path().join(".warp-dev-test")).unwrap();
    fs::write(home.path().join(".warp-dev-test/settings.toml"), "Dev").unwrap();
    let root = home
        .path()
        .join(".config/.warpai/channels/dev/profiles/test");
    migrate_to(home.path(), &root, Some("test"), Channel::Dev).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("settings.toml")).unwrap(),
        "Dev"
    );
}
