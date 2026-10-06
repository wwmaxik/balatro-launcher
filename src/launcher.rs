use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

pub const COMPAT_NATIVEFS: &str = include_str!("../assets/nativefs_linux_compat.lua");

#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub love_binary: PathBuf,
    pub game_target: PathBuf,
    pub modded: bool,
    pub lovely_lib_path: Option<PathBuf>,
    pub mods_dir: Option<PathBuf>,
}

pub fn launch_game(config: &LaunchConfig) -> std::io::Result<Child> {
    // If launched in modded mode, ensure Linux nativefs compatibility across all mods
    if config.modded {
        if let Some(ref mods_dir) = config.mods_dir {
            ensure_linux_nativefs_compatibility(mods_dir);
        }
    }

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

#[cfg(unix)]
pub fn ensure_linux_environment() {
    if let Some(data_dir) = dirs::data_dir() {
        let love_dir = data_dir.join("love");
        let _ = fs::create_dir_all(&love_dir);
        let lower = love_dir.join("balatro");
        let upper = love_dir.join("Balatro");
        let standalone = data_dir.join("balatro");

        let target = if lower.exists() {
            lower.clone()
        } else if upper.exists() {
            upper.clone()
        } else {
            let _ = fs::create_dir_all(&lower);
            lower.clone()
        };

        let mods_dir = target.join("Mods");
        let _ = fs::create_dir_all(&mods_dir);

        if !upper.exists() && target != upper {
            let _ = std::os::unix::fs::symlink(&target, &upper);
        }
        if !standalone.exists() && target != standalone {
            let _ = std::os::unix::fs::symlink(&target, &standalone);
        }
    }
}

#[cfg(not(unix))]
pub fn ensure_linux_environment() {}

/// Recursively scans mods directory and updates any `nativefs.lua` files
/// to be Linux-compatible (POSIX stat/opendir fallback, no undefined symbol PHYSFS crash,
/// guarded cdef definitions).
pub fn ensure_linux_nativefs_compatibility(mods_dir: &Path) {
    ensure_linux_environment();
    if !mods_dir.exists() {
        return;
    }
    walk_and_patch_nativefs(mods_dir);
    ensure_steamodded_compatibility(mods_dir);
}

fn ensure_steamodded_compatibility(mods_dir: &Path) {
    let smods_dir = mods_dir.join("Steamodded");
    let utils_lua = smods_dir.join("src").join("utils.lua");
    if utils_lua.exists() {
        if let Ok(content) = fs::read_to_string(&utils_lua) {
            let mut patched = content;
            if patched.contains("math.max(1, G.GAME.starting_params.play_limit)")
                && !patched.contains("G.GAME.starting_params.play_limit = G.GAME.starting_params.play_limit or 5")
            {
                patched = patched.replace(
                    "function SMODS.update_hand_limit_text(play, discard)\n    if play then",
                    "function SMODS.update_hand_limit_text(play, discard)\n    if not G.GAME or not G.GAME.starting_params then return end\n    G.GAME.starting_params.play_limit = G.GAME.starting_params.play_limit or 5\n    G.GAME.starting_params.discard_limit = G.GAME.starting_params.discard_limit or 5\n    if play then",
                );
            }
            if patched.contains("value = value + card.ability[property]") {
                patched = patched.replace(
                    "value = value + card.ability[property]",
                    "if card and card.ability and card.ability[property] then value = value + (tonumber(card.ability[property]) or 0) end",
                );
            }
            let _ = fs::write(&utils_lua, patched);
        }
    }
    let hand_limit_toml = smods_dir.join("lovely").join("hand_limit.toml");
    if hand_limit_toml.exists() {
        if let Ok(content) = fs::read_to_string(&hand_limit_toml) {
            if !content.contains("self.GAME.starting_params.play_limit = self.GAME.starting_params.play_limit or 5") {
                let patched = content.replace(
                    "payload = '''\nSMODS.update_hand_limit_text(true, true)",
                    "payload = '''\nif self.GAME and self.GAME.starting_params then\n    self.GAME.starting_params.play_limit = self.GAME.starting_params.play_limit or 5\n    self.GAME.starting_params.discard_limit = self.GAME.starting_params.discard_limit or 5\nend\nSMODS.update_hand_limit_text(true, true)",
                );
                let _ = fs::write(&hand_limit_toml, patched);
            }
        }
    }
    let scoring_calc_toml = smods_dir.join("lovely").join("scoring_calculation.toml");
    if scoring_calc_toml.exists() {
        if let Ok(content) = fs::read_to_string(&scoring_calc_toml) {
            if content.contains("if saveTable then\n    self.GAME.current_scoring_calculation = SMODS.Scoring_Calculations[saveTable.SCORING_CALC.key]") {
                let patched = content.replace(
                    "if saveTable then\n    self.GAME.current_scoring_calculation = SMODS.Scoring_Calculations[saveTable.SCORING_CALC.key]",
                    "if saveTable and saveTable.SCORING_CALC and saveTable.SCORING_CALC.key and SMODS.Scoring_Calculations[saveTable.SCORING_CALC.key] then\n    self.GAME.current_scoring_calculation = SMODS.Scoring_Calculations[saveTable.SCORING_CALC.key]"
                );
                let _ = fs::write(&scoring_calc_toml, patched);
            }
        }
    }
    let better_calc_toml = smods_dir.join("lovely").join("better_calc.toml");
    if better_calc_toml.exists() {
        if let Ok(content) = fs::read_to_string(&better_calc_toml) {
            if content.contains("if saveTable.SMODS then\n        SMODS.last_hand = {scoring_hand = {}, full_hand = {}, scoring_name = saveTable.SMODS.last_hand.scoring_name}") {
                let patched = content.replace(
                    "if saveTable.SMODS then\n        SMODS.last_hand = {scoring_hand = {}, full_hand = {}, scoring_name = saveTable.SMODS.last_hand.scoring_name}",
                    "if saveTable.SMODS and saveTable.SMODS.last_hand then\n        SMODS.last_hand = {scoring_hand = {}, full_hand = {}, scoring_name = saveTable.SMODS.last_hand.scoring_name}"
                );
                let _ = fs::write(&better_calc_toml, patched);
            }
        }
    }
    let card_limit_toml = smods_dir.join("lovely").join("card_limit.toml");
    if card_limit_toml.exists() {
        if let Ok(content) = fs::read_to_string(&card_limit_toml) {
            if content.contains("self.config = setmetatable(cardAreaTable.config, {")
                && !content.contains("if not cardAreaTable.config.card_limits then")
            {
                let patched = content
                    .replace(
                        "payload = '''\nself.config = setmetatable(cardAreaTable.config, {",
                        "payload = '''\nif not cardAreaTable.config.card_limits then\n    local clim = cardAreaTable.config.card_limit or 8\n    cardAreaTable.config.card_limits = {\n        base = clim,\n        total_slots = clim,\n        mod = 0,\n        extra_slots = 0,\n        extra_slots_used = 0,\n    }\nend\nself.config = setmetatable(cardAreaTable.config, {",
                    )
                    .replace(
                        "if self.config.type == 'hand' and cardAreaTable.config.card_limits.total_slots ~= (cardAreaTable.config.card_limits.base or 0) + (cardAreaTable.config.card_limits.mod or 0) + (cardAreaTable.config.card_limits.extra_slots or 0) then",
                        "if self.config.type == 'hand' and cardAreaTable.config.card_limits and cardAreaTable.config.card_limits.total_slots and cardAreaTable.config.card_limits.total_slots ~= (cardAreaTable.config.card_limits.base or 0) + (cardAreaTable.config.card_limits.mod or 0) + (cardAreaTable.config.card_limits.extra_slots or 0) then",
                    );
                let _ = fs::write(&card_limit_toml, patched);
            }
        }
    }
    let game_obj_lua = smods_dir.join("src").join("game_object.lua");
    if game_obj_lua.exists() {
        if let Ok(content) = fs::read_to_string(&game_obj_lua) {
            if content.contains("math.min(cfg.choose + (G.GAME.modifiers.booster_choice_mod or 0)") {
                let patched = content.replace(
                    "local cfg = (card and card.ability) or self.config\n        return {\n            vars = { math.min(cfg.choose + (G.GAME.modifiers.booster_choice_mod or 0), math.max(1, cfg.extra + (G.GAME.modifiers.booster_size_mod or 0))), math.max(1, cfg.extra + (G.GAME.modifiers.booster_size_mod or 0)) },",
                    "local cfg = (card and card.ability) or self.config or {}\n        local choose = cfg.choose or (self.config and self.config.choose) or 1\n        local extra = cfg.extra or (self.config and self.config.extra) or 3\n        local choice_mod = (G.GAME and G.GAME.modifiers and G.GAME.modifiers.booster_choice_mod) or 0\n        local size_mod = (G.GAME and G.GAME.modifiers and G.GAME.modifiers.booster_size_mod) or 0\n        local total_size = math.max(1, extra + size_mod)\n        local total_choose = math.min(choose + choice_mod, total_size)\n        return {\n            vars = { total_choose, total_size },",
                );
                let _ = fs::write(&game_obj_lua, patched);
            }
        }
    }
}

fn walk_and_patch_nativefs(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != ".git" && name != "dump" && name != "game-dump" && name != "lsp_def" {
                    walk_and_patch_nativefs(&path);
                }
            } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.eq_ignore_ascii_case("nativefs.lua") {
                    let needs_patch = match fs::read_to_string(&path) {
                        Ok(content) => {
                            !content.contains("posix_getInfo")
                                || !content.contains("pcall(ffi.cdef")
                                || content.contains("local mountPoint = _ptr(loveC.PHYSFS_getMountPoint(dir))")
                        }
                        Err(_) => true,
                    };
                    if needs_patch {
                        let _ = fs::write(&path, COMPAT_NATIVEFS);
                    }
                }
            }
        }
    }
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
            mods_dir: None,
        };
        assert_eq!(config.modded, false);
    }

    #[test]
    fn test_nativefs_patching() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mod_dir = temp_dir.path().join("TestMod");
        fs::create_dir_all(&mod_dir).unwrap();
        let dummy_nativefs = mod_dir.join("nativefs.lua");
        fs::write(
            &dummy_nativefs,
            "local mountPoint = _ptr(loveC.PHYSFS_getMountPoint(dir))\n",
        )
        .unwrap();

        ensure_linux_nativefs_compatibility(temp_dir.path());

        let patched_content = fs::read_to_string(&dummy_nativefs).unwrap();
        assert!(patched_content.contains("posix_getInfo"));
        assert!(patched_content.contains("pcall(ffi.cdef"));
    }
}
