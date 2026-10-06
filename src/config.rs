use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub game_path: Option<PathBuf>,
    pub love_binary: Option<PathBuf>,
    pub modded_mode: bool,
    pub custom_mods_dir: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            game_path: None,
            love_binary: None,
            modded_mode: false,
            custom_mods_dir: None,
        }
    }
}

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("balatro-launcher"))
        .unwrap_or_else(|| PathBuf::from(".config/balatro-launcher"))
}

pub fn get_config_file_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = get_config_file_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) -> std::io::Result<()> {
    let dir = get_config_dir();
    fs::create_dir_all(&dir)?;
    let content = serde_json::to_string_pretty(config)?;
    fs::write(get_config_file_path(), content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = AppConfig {
            game_path: Some(PathBuf::from("/tmp/Balatro.exe")),
            love_binary: Some(PathBuf::from("/usr/bin/love")),
            modded_mode: true,
            custom_mods_dir: None,
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config.game_path, deserialized.game_path);
        assert_eq!(config.love_binary, deserialized.love_binary);
        assert_eq!(config.modded_mode, deserialized.modded_mode);
    }
}
