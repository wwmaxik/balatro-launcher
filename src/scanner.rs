use std::path::{Path, PathBuf};
use which::which;

pub fn find_love_binary() -> Option<PathBuf> {
    which("love").ok()
}

pub fn get_default_mods_dir() -> PathBuf {
    dirs::data_dir()
        .map(|p| p.join("love").join("Balatro").join("Mods"))
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|p| p.join(".local/share/love/Balatro/Mods"))
                .unwrap_or_else(|| PathBuf::from("Mods"))
        })
}

pub fn find_game_executable(custom_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(custom) = custom_path {
        if custom.exists() {
            return Some(custom.to_path_buf());
        }
    }

    let home = dirs::home_dir()?;

    let candidates = [
        home.join("Balatro").join("balatro"),
        home.join("Balatro").join("Balatro.love"),
        home.join("Balatro").join("Balatro.exe"),
        home.join(".steam/steam/steamapps/common/Balatro/Balatro.exe"),
        home.join(".local/share/Steam/steamapps/common/Balatro/Balatro.exe"),
        home.join(".steam/root/steamapps/common/Balatro/Balatro.exe"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_love() {
        let love = find_love_binary();
        assert!(love.is_some(), "Love binary should be found on Debian");
    }

    #[test]
    fn test_mods_dir_path() {
        let mods_dir = get_default_mods_dir();
        assert!(mods_dir.to_str().unwrap().contains("love/Balatro/Mods"));
    }
}
