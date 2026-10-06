use std::path::{Path, PathBuf};
use std::process::{Child, Command};

#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub love_binary: PathBuf,
    pub game_target: PathBuf,
    pub modded: bool,
    pub lovely_lib_path: Option<PathBuf>,
}

pub fn launch_game(config: &LaunchConfig) -> std::io::Result<Child> {
    // If game_target is a standalone fused binary (like ~/Balatro/balatro), run it directly!
    // Otherwise run via love <path>.
    let is_direct_binary = config.game_target.is_file()
        && config
            .game_target
            .file_name()
            .map(|n| n == "balatro" || n == "Balatro")
            .unwrap_or(false);

    let mut cmd = if is_direct_binary {
        Command::new(&config.game_target)
    } else {
        let mut c = Command::new(&config.love_binary);
        c.arg(&config.game_target);
        c
    };

    // If launched in modded mode and lovely injector lib exists
    if config.modded {
        if let Some(ref lovely_path) = config.lovely_lib_path {
            if lovely_path.exists() {
                cmd.env("LD_PRELOAD", lovely_path);
            }
        }
    }

    // Set working directory to game parent dir if possible
    if let Some(parent) = config.game_target.parent() {
        cmd.current_dir(parent);
    }

    cmd.spawn()
}

pub fn find_lovely_lib(game_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = game_dir {
        let local_lovely = dir.join("liblovely.so");
        if local_lovely.exists() {
            return Some(local_lovely);
        }
    }

    if let Some(data_dir) = dirs::data_dir() {
        let global_lovely = data_dir.join("love").join("Balatro").join("liblovely.so");
        if global_lovely.exists() {
            return Some(global_lovely);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_config() {
        let config = LaunchConfig {
            love_binary: PathBuf::from("/usr/bin/love"),
            game_target: PathBuf::from("/tmp/Balatro.love"),
            modded: false,
            lovely_lib_path: None,
        };
        assert_eq!(config.modded, false);
    }
}
