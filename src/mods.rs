use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    pub id: Option<String>,
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModInfo {
    pub name: String,
    pub folder_name: String,
    pub path: PathBuf,
    pub is_enabled: bool,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub is_smods: bool,
}

pub fn scan_mods(mods_dir: &Path) -> Vec<ModInfo> {
    let mut mods = Vec::new();
    if !mods_dir.exists() {
        return mods;
    }

    if let Ok(entries) = fs::read_dir(mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let folder_name = entry.file_name().to_string_lossy().to_string();

                // Skip internal lovely cache directories (dump, game-dump, log)
                if folder_name == "dump"
                    || folder_name == "game-dump"
                    || folder_name == "log"
                {
                    continue;
                }

                // If folder is named lovely, check if it's just lovely cache container
                if folder_name.to_lowercase() == "lovely"
                    && !path.join("lovely.toml").exists()
                    && !path.join("manifest.json").exists()
                {
                    continue;
                }

                let is_enabled = !folder_name.ends_with(".disabled");
                let clean_name = if is_enabled {
                    folder_name.clone()
                } else {
                    folder_name.trim_end_matches(".disabled").to_string()
                };

                let (manifest_name, version, author, description) = read_mod_metadata(&path);
                let display_name = manifest_name.unwrap_or_else(|| clean_name.clone());
                let is_smods = clean_name.to_lowercase().contains("smods")
                    || clean_name.to_lowercase().contains("steamodded")
                    || display_name.to_lowercase().contains("steamodded");

                mods.push(ModInfo {
                    name: display_name,
                    folder_name,
                    path,
                    is_enabled,
                    version,
                    author,
                    description,
                    is_smods,
                });
            }
        }
    }

    mods.sort_by(|a, b| {
        if a.is_smods && !b.is_smods {
            std::cmp::Ordering::Less
        } else if !a.is_smods && b.is_smods {
            std::cmp::Ordering::Greater
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    mods
}

fn read_mod_metadata(
    mod_path: &Path,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let candidates = ["manifest.json", "metadata.json", "mod.json"];
    for candidate in candidates {
        let file_path = mod_path.join(candidate);
        if file_path.exists() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Ok(manifest) = serde_json::from_str::<ModManifest>(&content) {
                    return (
                        manifest.name.or(manifest.id),
                        manifest.version,
                        manifest.author,
                        manifest.description,
                    );
                }
            }
        }
    }

    // Try parsing lovely.toml if present
    let lovely_toml = mod_path.join("lovely.toml");
    if lovely_toml.exists() {
        if let Ok(content) = fs::read_to_string(&lovely_toml) {
            let mut version = None;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("version") {
                    if let Some(val) = trimmed.split('=').nth(1) {
                        version = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
                        break;
                    }
                }
            }
            return (None, version, None, None);
        }
    }

    (None, None, None, None)
}

pub fn toggle_mod(mod_info: &ModInfo) -> std::io::Result<PathBuf> {
    let parent = mod_info
        .path
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No parent directory"))?;

    let new_folder_name = if mod_info.is_enabled {
        format!("{}.disabled", mod_info.folder_name)
    } else {
        mod_info
            .folder_name
            .trim_end_matches(".disabled")
            .to_string()
    };

    let target_path = parent.join(&new_folder_name);
    fs::rename(&mod_info.path, &target_path)?;
    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_mod_scanning_and_toggle() {
        let dir = tempdir().unwrap();
        let mods_dir = dir.path();

        let mod1_path = mods_dir.join("CoolMod");
        let mod2_path = mods_dir.join("DisabledMod.disabled");
        let smods_path = mods_dir.join("Steamodded");
        let dump_path = mods_dir.join("dump");
        fs::create_dir_all(&mod1_path).unwrap();
        fs::create_dir_all(&mod2_path).unwrap();
        fs::create_dir_all(&smods_path).unwrap();
        fs::create_dir_all(&dump_path).unwrap();

        let scanned = scan_mods(mods_dir);
        assert_eq!(scanned.len(), 3, "dump folder should be ignored");
        assert!(scanned[0].is_smods, "Steamodded should be sorted first");

        let cool_mod = scanned.iter().find(|m| m.name == "CoolMod").unwrap();
        assert!(cool_mod.is_enabled);

        let toggled_path = toggle_mod(cool_mod).unwrap();
        assert!(toggled_path.to_str().unwrap().ends_with(".disabled"));
        assert!(toggled_path.exists());

        let rescan = scan_mods(mods_dir);
        let cool_mod_rescanned = rescan.iter().find(|m| m.name == "CoolMod").unwrap();
        assert!(!cool_mod_rescanned.is_enabled);
    }
}
