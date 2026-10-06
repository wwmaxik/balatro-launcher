use flate2::read::GzDecoder;
use serde::Deserialize;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tar::Archive;

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub async fn download_and_install_lovely(
    target_dir: &Path,
    on_status: impl Fn(String),
) -> Result<PathBuf, String> {
    on_status("Checking latest Lovely release on GitHub...".to_string());

    let client = reqwest::Client::builder()
        .user_agent("balatro-launcher-linux")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let release_url = "https://api.github.com/repos/ethangreen-dev/lovely-injector/releases/latest";
    let resp = client
        .get(release_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned status: {}", resp.status()));
    }

    let release: GithubRelease = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse release JSON: {e}"))?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name.contains("linux") && a.name.ends_with(".tar.gz"))
        .ok_or_else(|| "Could not find Linux asset in Lovely release".to_string())?;

    on_status(format!(
        "Downloading Lovely {} ({})...",
        release.tag_name, asset.name
    ));

    let download_resp = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download asset: {e}"))?;

    let bytes = download_resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read asset bytes: {e}"))?;

    on_status("Extracting liblovely.so...".to_string());
    fs::create_dir_all(target_dir)
        .map_err(|e| format!("Failed to create target directory: {e}"))?;

    let tar = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(tar);

    let output_lib_path = target_dir.join("liblovely.so");

    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?;
        if path.file_name().and_then(|n| n.to_str()) == Some("liblovely.so") {
            entry
                .unpack(&output_lib_path)
                .map_err(|e| format!("Failed to unpack liblovely.so: {e}"))?;
            on_status(format!(
                "Lovely {} successfully installed!",
                release.tag_name
            ));
            return Ok(output_lib_path);
        }
    }

    Err("liblovely.so not found inside release archive".to_string())
}
