//! Locating the proxy binary: PATH first, then GitHub releases.

use std::fs;

use zed_extension_api::{
    self as zed, LanguageServerId, LanguageServerInstallationStatus, Result, Worktree,
};

const PROXY_BINARY: &str = "comment-translate-proxy";
/// The repository whose GitHub releases provide prebuilt proxy binaries.
const GITHUB_REPO: &str = "273Do/zed-comment-translator";

#[derive(Default)]
pub struct ProxyBinary {
    cached_path: Option<String>,
}

impl ProxyBinary {
    pub fn path(&mut self, id: &LanguageServerId, worktree: &Worktree) -> Result<String> {
        if let Some(path) = worktree.which(PROXY_BINARY) {
            return Ok(path);
        }
        if let Some(path) = &self.cached_path {
            if is_file(path) {
                return Ok(path.clone());
            }
        }
        let path = download(id)?;
        self.cached_path = Some(path.clone());
        Ok(path)
    }
}

/// Downloads the latest release for this platform, removing older versions.
fn download(id: &LanguageServerId) -> Result<String> {
    zed::set_language_server_installation_status(
        id,
        &LanguageServerInstallationStatus::CheckingForUpdate,
    );
    let release = zed::latest_github_release(
        GITHUB_REPO,
        zed::GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )?;

    let (os, arch) = zed::current_platform();
    let target = match (os, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => "aarch64-apple-darwin",
        (zed::Os::Mac, zed::Architecture::X8664) => "x86_64-apple-darwin",
        (zed::Os::Linux, zed::Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
        (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-gnu",
        (zed::Os::Windows, zed::Architecture::X8664) => "x86_64-pc-windows-msvc",
        (os, arch) => return Err(format!("unsupported platform: {os:?} {arch:?}")),
    };
    let (extension, file_type, exe) = match os {
        zed::Os::Windows => ("zip", zed::DownloadedFileType::Zip, ".exe"),
        _ => ("tar.gz", zed::DownloadedFileType::GzipTar, ""),
    };
    let asset_name = format!("{PROXY_BINARY}-{target}.{extension}");
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| format!("no asset named {asset_name} in release {}", release.version))?;

    let version_dir = format!("{PROXY_BINARY}-{}", release.version);
    let binary_path = format!("{version_dir}/{PROXY_BINARY}{exe}");
    if !is_file(&binary_path) {
        zed::set_language_server_installation_status(
            id,
            &LanguageServerInstallationStatus::Downloading,
        );
        zed::download_file(&asset.download_url, &version_dir, file_type)
            .map_err(|error| format!("failed to download {asset_name}: {error}"))?;
        zed::make_file_executable(&binary_path)?;

        for entry in fs::read_dir(".")
            .map_err(|error| error.to_string())?
            .flatten()
        {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(PROXY_BINARY) && name != version_dir {
                fs::remove_dir_all(entry.path()).ok();
            }
        }
    }

    zed::set_language_server_installation_status(id, &LanguageServerInstallationStatus::None);
    Ok(binary_path)
}

fn is_file(path: &str) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}
