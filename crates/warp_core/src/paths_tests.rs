use dirs::home_dir;

use super::*;

#[test]
fn test_managed_directories_share_warpai_root() {
    let mut expected = home_dir()
        .expect("Home directory")
        .join(".config")
        .join("warpai");
    if let Some(profile) = ChannelState::data_profile() {
        expected = expected.join("profiles").join(profile);
    }
    assert_eq!(data_dir(), expected);
    assert_eq!(config_local_dir(), expected);
    assert_eq!(warp_home_config_dir(), Some(expected.clone()));
    assert_eq!(warp_home_skills_dir(), Some(expected.join("skills")));
    assert_eq!(
        warp_home_mcp_config_file_path(),
        Some(expected.join(".mcp.json"))
    );
    assert_eq!(state_dir(), expected.join("state"));
    assert_eq!(secure_state_dir(), Some(expected.join("state")));
    assert_eq!(cache_dir(), expected.join("cache"));
}
