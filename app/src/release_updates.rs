//! Public release checks never modify the installed application.
use crate::{
    channel::{Channel, ChannelState},
    view_components::{DismissibleToast, ToastLink},
    workspace::ToastStack,
};
use async_compat::CompatExt as _;
use settings::{macros::define_settings_group, Setting as _, SupportedPlatforms, SyncToCloud};
use std::time::Duration;
use warpui::{Entity, ModelContext, SingletonEntity};

const RELEASE_API: &str = "https://api.github.com/repos/OthinusG/warpai/releases/latest";
pub(crate) const RELEASES_URL: &str = "https://github.com/OthinusG/warpai/releases";

define_settings_group!(UpdateSettings, settings: [
    check_on_startup: CheckForUpdatesOnStartup {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: false,
        storage_key: "CheckForUpdatesOnStartup",
        toml_path: "general.check_for_updates_on_startup",
        description: "Check public Warpai releases once at startup without installing updates.",
    },
]);

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(serde::Deserialize)]
struct ReleaseAsset {
    name: String,
}

pub(crate) fn installer_url(tag: &str, os: &str, arch: &str) -> Option<String> {
    version(tag)?;
    let number = tag.strip_prefix('v')?;
    let name = match (os, arch) {
        ("macos", "aarch64") => format!("Warpai-{number}-macos-arm64.dmg"),
        ("windows", "x86_64") => format!("WarpaiSetup-{number}-windows-x64.exe"),
        _ => return None,
    };
    Some(format!("{RELEASES_URL}/download/{tag}/{name}"))
}

fn published_installer(release: &Release, os: &str, arch: &str) -> Option<String> {
    let url = installer_url(&release.tag_name, os, arch)?;
    release
        .assets
        .iter()
        .any(|asset| url.rsplit('/').next() == Some(asset.name.as_str()))
        .then_some(url)
}

fn version(value: &str) -> Option<[u64; 3]> {
    let mut parts = value.strip_prefix('v').unwrap_or(value).split('.');
    let parse = |part: &str| {
        part.bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    let result = [
        parse(parts.next()?)?,
        parse(parts.next()?)?,
        parse(parts.next()?)?,
    ];
    parts.next().is_none().then_some(result)
}

fn newer_release(release: Release, current: &str) -> anyhow::Result<Option<String>> {
    anyhow::ensure!(
        !release.draft && !release.prerelease && release.tag_name.starts_with('v'),
        "Not a stable application release"
    );
    let latest =
        version(&release.tag_name).ok_or_else(|| anyhow::anyhow!("Invalid release version"))?;
    let current = version(current).ok_or_else(|| anyhow::anyhow!("Unknown application version"))?;
    Ok((latest > current).then_some(release.tag_name))
}

async fn fetch_update(current: &str) -> anyhow::Result<Option<(String, Option<String>)>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Warpai-update-check")
        .build()?;
    let mut response = client
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        anyhow::ensure!(
            body.len() + chunk.len() <= 256 * 1024,
            "Release response too large"
        );
        body.extend_from_slice(&chunk);
    }
    let release: Release = serde_json::from_slice(&body)?;
    let download = published_installer(&release, std::env::consts::OS, std::env::consts::ARCH);
    Ok(newer_release(release, current)?.map(|tag| (tag, download)))
}

#[derive(Default)]
pub(crate) struct ReleaseUpdates {
    pub(crate) checking: bool,
    pub(crate) available: Option<String>,
    pub(crate) download_url: Option<String>,
    pub(crate) status: String,
}

impl ReleaseUpdates {
    pub(crate) fn new(ctx: &mut ModelContext<Self>) -> Self {
        if *UpdateSettings::as_ref(ctx).check_on_startup.value()
            && warp_core::execution_mode::AppExecutionMode::as_ref(ctx).can_autoupdate()
            && ChannelState::channel() != Channel::Integration
            && ChannelState::app_version().and_then(version).is_some()
        {
            ctx.spawn(
                async { warpui::r#async::Timer::after(Duration::from_secs(5)).await },
                |model, _, ctx| {
                    if *UpdateSettings::as_ref(ctx).check_on_startup.value() {
                        model.check(ctx);
                    }
                },
            );
        }
        Self::default()
    }

    pub(crate) fn check(&mut self, ctx: &mut ModelContext<Self>) {
        if self.checking {
            return;
        }
        self.checking = true;
        self.available = None;
        self.download_url = None;
        self.status = "Checking for updates…".into();
        ctx.notify();
        let current = ChannelState::app_version().unwrap_or("").to_owned();
        ctx.spawn(
            async move { fetch_update(&current).compat().await },
            |model, result, ctx| {
                model.checking = false;
                match result {
                    Ok(Some((tag, download))) => {
                        model.status = format!("Warpai {tag} is available.");
                        model.available = Some(tag.clone());
                        model.download_url = download.clone();
                        if let Some(window) = ctx.windows().active_window() {
                            ToastStack::handle(ctx).update(ctx, |toasts, ctx| {
                                toasts.add_ephemeral_toast(
                                    DismissibleToast::default(format!("Warpai {tag} is available"))
                                        .with_link(
                                            ToastLink::new(
                                                if download.is_some() {
                                                    "Download"
                                                } else {
                                                    "GitHub Releases"
                                                }
                                                .into(),
                                            )
                                            .with_href(download.unwrap_or_else(|| {
                                                format!("{RELEASES_URL}/tag/{tag}")
                                            })),
                                        ),
                                    window,
                                    ctx,
                                );
                            });
                        }
                    }
                    Ok(None) => {
                        model.available = None;
                        model.status = "Warpai is up to date.".into();
                    }
                    Err(_) => {
                        model.status =
                            "Could not check for updates. Try again or open GitHub Releases."
                                .into();
                    }
                }
                ctx.notify();
            },
        );
    }
}

impl Entity for ReleaseUpdates {
    type Event = ();
}
impl SingletonEntity for ReleaseUpdates {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn downloads_select_only_published_desktop_installers() {
        let release = Release {
            tag_name: "v1.4.0".into(),
            draft: false,
            prerelease: false,
            assets: [
                "Warpai-1.4.0-macos-arm64.dmg",
                "WarpaiSetup-1.4.0-windows-x64.exe",
                "WarpaiCompanion-3.1.0-linux-x64.run",
            ]
            .into_iter()
            .map(|name| ReleaseAsset { name: name.into() })
            .collect(),
        };
        assert_eq!(
            published_installer(&release, "macos", "aarch64").unwrap(),
            format!("{RELEASES_URL}/download/v1.4.0/Warpai-1.4.0-macos-arm64.dmg")
        );
        assert_eq!(
            published_installer(&release, "windows", "x86_64").unwrap(),
            format!("{RELEASES_URL}/download/v1.4.0/WarpaiSetup-1.4.0-windows-x64.exe")
        );
        for (os, arch) in [
            ("macos", "x86_64"),
            ("windows", "aarch64"),
            ("linux", "x86_64"),
        ] {
            assert!(published_installer(&release, os, arch).is_none());
        }
        let missing = Release {
            assets: vec![ReleaseAsset {
                name: "WarpaiCompanion-3.1.0-macos-arm64.dmg".into(),
            }],
            ..release
        };
        assert!(published_installer(&missing, "macos", "aarch64").is_none());
        assert!(installer_url("v1.4.0/evil", "macos", "aarch64").is_none());
    }
    #[test]
    fn stable_versions_never_offer_downgrades_or_untrusted_tags() {
        let release = |tag: &str| Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
            assets: Vec::new(),
        };
        assert_eq!(
            newer_release(release("v1.10.0"), "v1.9.9").unwrap(),
            Some("v1.10.0".into())
        );
        for tag in ["v1.3.1", "v1.3.0"] {
            assert_eq!(newer_release(release(tag), "1.3.1").unwrap(), None);
        }
        for tag in [
            "v1.4.0-beta",
            "companion-v3.1.0",
            "v1.4.0/evil",
            "vv1.4.0",
            "v1.4",
            "v1.4.0.1",
            "v+1.4.0",
            "v18446744073709551616.0.0",
        ] {
            assert!(newer_release(release(tag), "v1.3.1").is_err());
        }
        assert!(newer_release(
            Release {
                prerelease: true,
                ..release("v1.4.0")
            },
            "v1.3.1"
        )
        .is_err());
        assert!(newer_release(
            Release {
                draft: true,
                ..release("v1.4.0")
            },
            "v1.3.1"
        )
        .is_err());
        assert!(newer_release(release("v1.4.0"), "development").is_err());
    }
}
