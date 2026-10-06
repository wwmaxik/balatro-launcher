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

    // Also ensure <game_dir>/Mods is linked to mods_dir so Lovely finds it regardless of identity
    #[cfg(unix)]
    if let (Some(parent), Some(ref mods_dir)) = (config.game_target.parent(), &config.mods_dir) {
        let game_mods_dir = parent.join("Mods");
        if !game_mods_dir.exists() {
            let _ = std::os::unix::fs::symlink(mods_dir, &game_mods_dir);
        }
    }

    // If game_target is Balatro.exe or Balatro.love, try to auto-fuse to standalone "balatro" binary
    let effective_target = if config.game_target.is_file() {
        let is_exe = config
            .game_target
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case("Balatro.exe") || n.eq_ignore_ascii_case("balatro.exe"))
            .unwrap_or(false);

        if is_exe {
            let candidate_bin = config.game_target.with_file_name("balatro");
            if candidate_bin.is_file() {
                candidate_bin
            } else if let (Ok(love_bytes), Ok(exe_bytes)) =
                (fs::read(&config.love_binary), fs::read(&config.game_target))
            {
                let mut fused = love_bytes;
                fused.extend_from_slice(&exe_bytes);
                if fs::write(&candidate_bin, fused).is_ok() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(
                            &candidate_bin,
                            fs::Permissions::from_mode(0o755),
                        );
                    }
                    candidate_bin
                } else {
                    config.game_target.clone()
                }
            } else {
                config.game_target.clone()
            }
        } else {
            config.game_target.clone()
        }
    } else {
        config.game_target.clone()
    };

    // If effective_target is a standalone fused binary (like ~/Balatro/balatro), run it directly!
    // Otherwise run via love <path>.
    let is_direct_binary = effective_target.is_file()
        && effective_target
            .file_name()
            .map(|n| n == "balatro" || n == "Balatro")
            .unwrap_or(false);

    let mut cmd = if is_direct_binary {
        Command::new(&effective_target)
    } else {
        let mut c = Command::new(&config.love_binary);
        c.arg(&effective_target);
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
    if let Some(parent) = effective_target.parent() {
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
        let standalone_upper = data_dir.join("Balatro");

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
        if !standalone_upper.exists() && target != standalone_upper {
            let _ = std::os::unix::fs::symlink(&target, &standalone_upper);
        }

        // Link love-11/Mods -> target/Mods for Lovely default search path
        let love_11_dir = data_dir.join("love-11");
        let _ = fs::create_dir_all(&love_11_dir);
        let love_11_mods = love_11_dir.join("Mods");
        if love_11_mods.is_symlink() {
            // Already symlinked
        } else if !love_11_mods.exists() {
            let _ = std::os::unix::fs::symlink(&mods_dir, &love_11_mods);
        } else if !love_11_mods.join("Steamodded").exists() {
            // If lovely auto-created ~/.local/share/love-11/Mods without actual mods, replace it
            let _ = fs::remove_dir_all(&love_11_mods);
            let _ = std::os::unix::fs::symlink(&mods_dir, &love_11_mods);
        }

        // Link love/Mods -> target/Mods
        let love_mods = love_dir.join("Mods");
        if love_mods.is_symlink() {
            // Already symlinked
        } else if !love_mods.exists() {
            let _ = std::os::unix::fs::symlink(&mods_dir, &love_mods);
        } else if !love_mods.join("Steamodded").exists() {
            let _ = fs::remove_dir_all(&love_mods);
            let _ = std::os::unix::fs::symlink(&mods_dir, &love_mods);
        }
    }
}

#[cfg(not(unix))]
pub fn ensure_linux_environment() {}

pub const APP_ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

#[cfg(unix)]
pub fn ensure_desktop_integration() {
    let Ok(current_exe) = std::env::current_exe() else {
        return;
    };

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    let local_bin_dir = home.join(".local").join("bin");
    let target_bin = local_bin_dir.join("balatro-launcher");

    let exec_path = if current_exe == target_bin || current_exe.starts_with("/usr") {
        current_exe.clone()
    } else {
        let _ = fs::create_dir_all(&local_bin_dir);
        let should_copy = if !target_bin.exists() {
            true
        } else {
            let cur_len = fs::metadata(&current_exe).map(|m| m.len()).unwrap_or(0);
            let tgt_len = fs::metadata(&target_bin).map(|m| m.len()).unwrap_or(0);
            cur_len != tgt_len
        };

        if should_copy {
            let _ = fs::copy(&current_exe, &target_bin);
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target_bin, fs::Permissions::from_mode(0o755));
        }

        if target_bin.exists() {
            target_bin
        } else {
            current_exe.clone()
        }
    };

    let icon_dir = home
        .join(".local")
        .join("share")
        .join("icons")
        .join("hicolor")
        .join("256x256")
        .join("apps");
    let _ = fs::create_dir_all(&icon_dir);
    let icon_path = icon_dir.join("balatro-launcher.png");
    if !icon_path.exists() {
        let _ = fs::write(&icon_path, APP_ICON_PNG);
    }

    let desktop_content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Balatro Launcher\n\
         GenericName=Game Launcher\n\
         Comment=Launcher and mod manager for Balatro\n\
         Exec={}\n\
         Icon={}\n\
         Terminal=false\n\
         Categories=Game;CardGame;\n\
         Keywords=balatro;cards;roguelike;launcher;mods;\n\
         StartupNotify=true\n",
        exec_path.display(),
        icon_path.display()
    );

    let apps_dir = home.join(".local").join("share").join("applications");
    let _ = fs::create_dir_all(&apps_dir);
    let app_desktop = apps_dir.join("balatro-launcher.desktop");
    let _ = fs::write(&app_desktop, &desktop_content);

    if let Some(desktop_dir) = dirs::desktop_dir() {
        if desktop_dir.is_dir() {
            let desktop_file = desktop_dir.join("balatro-launcher.desktop");
            let _ = fs::write(&desktop_file, &desktop_content);
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&desktop_file, fs::Permissions::from_mode(0o755));
        }
    }
}

#[cfg(not(unix))]
pub fn ensure_desktop_integration() {}

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
            if patched.contains("local edition = G.P_CENTERS[self.edition.key]") {
                patched = patched.replace(
                    "function Card:calculate_edition(context)\n    if self.edition then\n        local edition = G.P_CENTERS[self.edition.key]",
                    "function Card:calculate_edition(context)\n    if self.edition then\n        if not self.edition.key then\n            for k, v in pairs(self.edition) do\n                if v and G.P_CENTERS['e_' .. k] then\n                    self.edition.key = 'e_' .. k\n                    break\n                end\n            end\n        end\n        local edition = self.edition.key and G.P_CENTERS[self.edition.key]",
                );
            }
            if patched.contains("G.SCORE_DISPLAY_QUEUE = nil") && !patched.contains("self.GAME.starting_params.boosters_in_shop") {
                patched = patched.replace(
                    "function Game:start_run(args)\n    game_start_run(self, args)\n    G.SCORE_DISPLAY_QUEUE = nil",
                    "function Game:start_run(args)\n    game_start_run(self, args)\n    if self.GAME then\n        self.GAME.starting_params = self.GAME.starting_params or {}\n        self.GAME.starting_params.play_limit = self.GAME.starting_params.play_limit or 5\n        self.GAME.starting_params.discard_limit = self.GAME.starting_params.discard_limit or 5\n        self.GAME.starting_params.boosters_in_shop = self.GAME.starting_params.boosters_in_shop or 2\n        self.GAME.starting_params.vouchers_in_shop = self.GAME.starting_params.vouchers_in_shop or 1\n        self.GAME.modifiers = self.GAME.modifiers or {}\n        self.GAME.round_resets = self.GAME.round_resets or {}\n        self.GAME.round_resets.free_rerolls = self.GAME.round_resets.free_rerolls or 0\n        if not self.GAME.current_scoring_calculation and SMODS.Scoring_Calculations and SMODS.Scoring_Calculations['multiply'] then\n            self.GAME.current_scoring_calculation = SMODS.Scoring_Calculations['multiply']:new()\n        end\n        if self.GAME.current_round and type(self.GAME.current_round.voucher) == 'string' then\n            local v_key = self.GAME.current_round.voucher\n            self.GAME.current_round.voucher = { v_key, spawn = { [v_key] = true } }\n        end\n    end\n    G.SCORE_DISPLAY_QUEUE = nil",
                );
            }
            let _ = fs::write(&utils_lua, patched);
        }
    }
    let card_draw_lua = smods_dir.join("src").join("card_draw.lua");
    if card_draw_lua.exists() {
        if let Ok(content) = fs::read_to_string(&card_draw_lua) {
            if content.contains("local edition = G.P_CENTERS[self.edition.key]") {
                let patched = content.replace(
                    "if self.edition then \n                local edition = G.P_CENTERS[self.edition.key]",
                    "if self.edition then \n                if not self.edition.key then\n                    for k, v in pairs(self.edition) do\n                        if v and G.P_CENTERS['e_' .. k] then\n                            self.edition.key = 'e_' .. k\n                            break\n                        end\n                    end\n                end\n                local edition = self.edition.key and G.P_CENTERS[self.edition.key]",
                );
                let _ = fs::write(&card_draw_lua, patched);
            }
        }
    }
    let overrides_lua = smods_dir.join("src").join("overrides.lua");
    if overrides_lua.exists() {
        if let Ok(content) = fs::read_to_string(&overrides_lua) {
            let mut patched = content;
            if !patched.contains("local function init_card_ability_defaults") {
                patched = patched.replace(
                    "local card_init = Card.init",
                    "local function init_card_ability_defaults(card)\n\tif not card or not card.ability then return end\n\tlocal ab = card.ability\n\tab.card_limit = ab.card_limit or 0\n\tab.extra_slots_used = ab.extra_slots_used or 0\n\tab.bonus = ab.bonus or 0\n\tab.perma_bonus = ab.perma_bonus or 0\n\tab.perma_x_chips = ab.perma_x_chips or 0\n\tab.perma_mult = ab.perma_mult or 0\n\tab.perma_x_mult = ab.perma_x_mult or 0\n\tab.perma_h_chips = ab.perma_h_chips or 0\n\tab.perma_h_x_chips = ab.perma_h_x_chips or 0\n\tab.perma_h_mult = ab.perma_h_mult or 0\n\tab.perma_h_x_mult = ab.perma_h_x_mult or 0\n\tab.perma_p_dollars = ab.perma_p_dollars or 0\n\tab.perma_h_dollars = ab.perma_h_dollars or 0\n\tab.perma_repetitions = ab.perma_repetitions or 0\n\tab.perma_score = ab.perma_score or 0\n\tab.perma_h_score = ab.perma_h_score or 0\n\tab.perma_x_score = ab.perma_x_score or 0\n\tab.perma_h_x_score = ab.perma_h_x_score or 0\n\tab.perma_blind_size = ab.perma_blind_size or 0\n\tab.perma_h_blind_size = ab.perma_h_blind_size or 0\n\tab.perma_x_blind_size = ab.perma_x_blind_size or 0\n\tab.perma_h_x_blind_size = ab.perma_h_x_blind_size or 0\n\tab.h_chips = ab.h_chips or 0\n\tab.x_chips = ab.x_chips or 1\n\tab.h_x_chips = ab.h_x_chips or 1\n\tab.repetitions = ab.repetitions or 0\nend\n\nlocal card_init = Card.init",
                );
            }
            if patched.contains("card_init(self, X, Y, W, H, card, center, params)") && !patched.contains("init_card_ability_defaults(self)") {
                patched = patched.replace(
                    "card_init(self, X, Y, W, H, card, center, params)",
                    "card_init(self, X, Y, W, H, card, center, params)\n\tinit_card_ability_defaults(self)",
                );
            }
            if patched.contains("function Card:load(cardTable, other_card)\n\tlocal ret = smods_card_load(self, cardTable, other_card)") && !patched.contains("init_card_ability_defaults(self)") {
                patched = patched.replace(
                    "function Card:load(cardTable, other_card)\n\tlocal ret = smods_card_load(self, cardTable, other_card)",
                    "function Card:load(cardTable, other_card)\n\tlocal ret = smods_card_load(self, cardTable, other_card)\n\tinit_card_ability_defaults(self)",
                );
            }
            if patched.contains("local on_edition_loaded = self.edition and self.edition.key and G.P_CENTERS[self.edition.key].on_load") {
                patched = patched.replace(
                    "function Card:load(cardTable, other_card)\n\tlocal ret = smods_card_load(self, cardTable, other_card)\n\tlocal on_edition_loaded = self.edition and self.edition.key and G.P_CENTERS[self.edition.key].on_load",
                    "function Card:load(cardTable, other_card)\n\tlocal ret = smods_card_load(self, cardTable, other_card)\n\tinit_card_ability_defaults(self)\n\tif self.edition and not self.edition.key then\n\t\tfor k, v in pairs(self.edition) do\n\t\t\tif v and G.P_CENTERS['e_' .. k] then\n\t\t\t\tself.edition.key = 'e_' .. k\n\t\t\t\tbreak\n\t\t\tend\n\t\tend\n\tend\n\tlocal on_edition_loaded = self.edition and self.edition.key and G.P_CENTERS[self.edition.key] and G.P_CENTERS[self.edition.key].on_load",
                );
            }
            if patched.contains("self.ability.card_limit = self.ability.card_limit + (center.config.card_limit or 0)") {
                patched = patched.replace(
                    "self.ability.card_limit = self.ability.card_limit + (center.config.card_limit or 0)",
                    "self.ability.card_limit = (self.ability.card_limit or 0) + (center.config.card_limit or 0)",
                );
            }
            if patched.contains("self.ability.extra_slots_used = self.ability.extra_slots_used + (center.config.extra_slots_used or 0)") {
                patched = patched.replace(
                    "self.ability.extra_slots_used = self.ability.extra_slots_used + (center.config.extra_slots_used or 0)",
                    "self.ability.extra_slots_used = (self.ability.extra_slots_used or 0) + (center.config.extra_slots_used or 0)",
                );
            }
            let _ = fs::write(&overrides_lua, patched);
        }
    }
    let perma_bonus_toml = smods_dir.join("lovely").join("perma_bonus.toml");
    if perma_bonus_toml.exists() {
        if let Ok(content) = fs::read_to_string(&perma_bonus_toml) {
            let mut patched = content;
            if patched.contains("bonus_x_chips = self.ability.perma_x_chips ~= 0 and (self.ability.perma_x_chips + 1) or nil,") {
                patched = patched.replace(
                    "bonus_x_chips = self.ability.perma_x_chips ~= 0 and (self.ability.perma_x_chips + 1) or nil,",
                    "bonus_x_chips = (self.ability.perma_x_chips or 0) ~= 0 and ((self.ability.perma_x_chips or 0) + 1) or nil,",
                );
            }
            if patched.contains("bonus_mult = self.ability.perma_mult ~= 0 and self.ability.perma_mult or nil,") {
                patched = patched.replace(
                    "bonus_mult = self.ability.perma_mult ~= 0 and self.ability.perma_mult or nil,",
                    "bonus_mult = (self.ability.perma_mult or 0) ~= 0 and self.ability.perma_mult or nil,",
                );
            }
            if patched.contains("bonus_x_mult = self.ability.perma_x_mult ~= 0 and (self.ability.perma_x_mult + 1) or nil,") {
                patched = patched.replace(
                    "bonus_x_mult = self.ability.perma_x_mult ~= 0 and (self.ability.perma_x_mult + 1) or nil,",
                    "bonus_x_mult = (self.ability.perma_x_mult or 0) ~= 0 and ((self.ability.perma_x_mult or 0) + 1) or nil,",
                );
            }
            if patched.contains("bonus_h_chips = self.ability.perma_h_chips ~= 0 and self.ability.perma_h_chips or nil,") {
                patched = patched.replace(
                    "bonus_h_chips = self.ability.perma_h_chips ~= 0 and self.ability.perma_h_chips or nil,",
                    "bonus_h_chips = (self.ability.perma_h_chips or 0) ~= 0 and self.ability.perma_h_chips or nil,",
                );
            }
            if patched.contains("bonus_h_x_chips = self.ability.perma_h_x_chips ~= 0 and (self.ability.perma_h_x_chips + 1) or nil,") {
                patched = patched.replace(
                    "bonus_h_x_chips = self.ability.perma_h_x_chips ~= 0 and (self.ability.perma_h_x_chips + 1) or nil,",
                    "bonus_h_x_chips = (self.ability.perma_h_x_chips or 0) ~= 0 and ((self.ability.perma_h_x_chips or 0) + 1) or nil,",
                );
            }
            if patched.contains("bonus_h_mult = self.ability.perma_h_mult ~= 0 and self.ability.perma_h_mult or nil,") {
                patched = patched.replace(
                    "bonus_h_mult = self.ability.perma_h_mult ~= 0 and self.ability.perma_h_mult or nil,",
                    "bonus_h_mult = (self.ability.perma_h_mult or 0) ~= 0 and self.ability.perma_h_mult or nil,",
                );
            }
            if patched.contains("bonus_h_x_mult = self.ability.perma_h_x_mult ~= 0 and (self.ability.perma_h_x_mult + 1) or nil,") {
                patched = patched.replace(
                    "bonus_h_x_mult = self.ability.perma_h_x_mult ~= 0 and (self.ability.perma_h_x_mult + 1) or nil,",
                    "bonus_h_x_mult = (self.ability.perma_h_x_mult or 0) ~= 0 and ((self.ability.perma_h_x_mult or 0) + 1) or nil,",
                );
            }
            if patched.contains("bonus_p_dollars = self.ability.perma_p_dollars ~= 0 and self.ability.perma_p_dollars or nil,") {
                patched = patched.replace(
                    "bonus_p_dollars = self.ability.perma_p_dollars ~= 0 and self.ability.perma_p_dollars or nil,",
                    "bonus_p_dollars = (self.ability.perma_p_dollars or 0) ~= 0 and self.ability.perma_p_dollars or nil,",
                );
            }
            if patched.contains("bonus_h_dollars = self.ability.perma_h_dollars ~= 0 and self.ability.perma_h_dollars or nil,") {
                patched = patched.replace(
                    "bonus_h_dollars = self.ability.perma_h_dollars ~= 0 and self.ability.perma_h_dollars or nil,",
                    "bonus_h_dollars = (self.ability.perma_h_dollars or 0) ~= 0 and self.ability.perma_h_dollars or nil,",
                );
            }
            if patched.contains("bonus_score = self.ability.perma_score ~= 0 and (self.ability.perma_score) or nil,") {
                patched = patched.replace(
                    "bonus_score = self.ability.perma_score ~= 0 and (self.ability.perma_score) or nil,",
                    "bonus_score = (self.ability.perma_score or 0) ~= 0 and (self.ability.perma_score) or nil,",
                );
            }
            if patched.contains("bonus_h_score = self.ability.perma_h_score ~= 0 and (self.ability.perma_h_score) or nil,") {
                patched = patched.replace(
                    "bonus_h_score = self.ability.perma_h_score ~= 0 and (self.ability.perma_h_score) or nil,",
                    "bonus_h_score = (self.ability.perma_h_score or 0) ~= 0 and (self.ability.perma_h_score) or nil,",
                );
            }
            if patched.contains("bonus_x_score = self.ability.perma_x_score ~= 0 and (self.ability.perma_x_score + 1) or nil,") {
                patched = patched.replace(
                    "bonus_x_score = self.ability.perma_x_score ~= 0 and (self.ability.perma_x_score + 1) or nil,",
                    "bonus_x_score = (self.ability.perma_x_score or 0) ~= 0 and ((self.ability.perma_x_score or 0) + 1) or nil,",
                );
            }
            if patched.contains("bonus_h_x_score = self.ability.perma_h_x_score ~= 0 and (self.ability.perma_h_x_score + 1) or nil,") {
                patched = patched.replace(
                    "bonus_h_x_score = self.ability.perma_h_x_score ~= 0 and (self.ability.perma_h_x_score + 1) or nil,",
                    "bonus_h_x_score = (self.ability.perma_h_x_score or 0) ~= 0 and ((self.ability.perma_h_x_score or 0) + 1) or nil,",
                );
            }
            if patched.contains("bonus_blind_size = self.ability.perma_blind_size ~= 0 and (self.ability.perma_blind_size) or nil,") {
                patched = patched.replace(
                    "bonus_blind_size = self.ability.perma_blind_size ~= 0 and (self.ability.perma_blind_size) or nil,",
                    "bonus_blind_size = (self.ability.perma_blind_size or 0) ~= 0 and (self.ability.perma_blind_size) or nil,",
                );
            }
            if patched.contains("bonus_h_blind_size = self.ability.perma_h_blind_size ~= 0 and (self.ability.perma_h_blind_size) or nil,") {
                patched = patched.replace(
                    "bonus_h_blind_size = self.ability.perma_h_blind_size ~= 0 and (self.ability.perma_h_blind_size) or nil,",
                    "bonus_h_blind_size = (self.ability.perma_h_blind_size or 0) ~= 0 and (self.ability.perma_h_blind_size) or nil,",
                );
            }
            if patched.contains("bonus_x_blind_size = self.ability.perma_x_blind_size ~= 0 and (self.ability.perma_x_blind_size + 1) or nil,") {
                patched = patched.replace(
                    "bonus_x_blind_size = self.ability.perma_x_blind_size ~= 0 and (self.ability.perma_x_blind_size + 1) or nil,",
                    "bonus_x_blind_size = (self.ability.perma_x_blind_size or 0) ~= 0 and ((self.ability.perma_x_blind_size or 0) + 1) or nil,",
                );
            }
            if patched.contains("bonus_h_x_blind_size = self.ability.perma_h_x_blind_size ~= 0 and (self.ability.perma_h_x_blind_size + 1) or nil,") {
                patched = patched.replace(
                    "bonus_h_x_blind_size = self.ability.perma_h_x_blind_size ~= 0 and (self.ability.perma_h_x_blind_size + 1) or nil,",
                    "bonus_h_x_blind_size = (self.ability.perma_h_x_blind_size or 0) ~= 0 and ((self.ability.perma_h_x_blind_size or 0) + 1) or nil,",
                );
            }
            if patched.contains("bonus_repetitions = self.ability.perma_repetitions ~= 0 and self.ability.perma_repetitions or nil,") {
                patched = patched.replace(
                    "bonus_repetitions = self.ability.perma_repetitions ~= 0 and self.ability.perma_repetitions or nil,",
                    "bonus_repetitions = (self.ability.perma_repetitions or 0) ~= 0 and self.ability.perma_repetitions or nil,",
                );
            }
            let _ = fs::write(&perma_bonus_toml, patched);
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
    let shop_toml = smods_dir.join("lovely").join("shop.toml");
    if shop_toml.exists() {
        if let Ok(content) = fs::read_to_string(&shop_toml) {
            let mut patched = content;
            if patched.contains("for i=1, G.GAME.starting_params.boosters_in_shop + (G.GAME.modifiers.extra_boosters or 0) do") {
                patched = patched.replace(
                    "for i=1, G.GAME.starting_params.boosters_in_shop + (G.GAME.modifiers.extra_boosters or 0) do",
                    "for i=1, ((G.GAME.starting_params and G.GAME.starting_params.boosters_in_shop) or 2) + ((G.GAME.modifiers and G.GAME.modifiers.extra_boosters) or 0) do",
                );
            }
            if patched.contains("local vouchers_to_spawn = 0\nfor _,_ in pairs(G.GAME.current_round.voucher.spawn) do") {
                patched = patched.replace(
                    "local vouchers_to_spawn = 0\nfor _,_ in pairs(G.GAME.current_round.voucher.spawn) do vouchers_to_spawn = vouchers_to_spawn + 1 end\nif vouchers_to_spawn < G.GAME.starting_params.vouchers_in_shop + (G.GAME.modifiers.extra_vouchers or 0) then",
                    "if G.GAME.current_round and type(G.GAME.current_round.voucher) == 'string' then\n    local v_key = G.GAME.current_round.voucher\n    G.GAME.current_round.voucher = { v_key, spawn = { [v_key] = true } }\nend\nif not (G.GAME.current_round and G.GAME.current_round.voucher and G.GAME.current_round.voucher.spawn) then\n    G.GAME.current_round.voucher = SMODS.get_next_vouchers()\nend\nlocal v_in_shop = (G.GAME.starting_params and G.GAME.starting_params.vouchers_in_shop) or 1\nlocal extra_v = (G.GAME.modifiers and G.GAME.modifiers.extra_vouchers) or 0\nlocal vouchers_to_spawn = 0\nfor _,_ in pairs(G.GAME.current_round.voucher.spawn) do vouchers_to_spawn = vouchers_to_spawn + 1 end\nif vouchers_to_spawn < v_in_shop + extra_v then",
                );
            }
            let _ = fs::write(&shop_toml, patched);
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
            let mut patched = content;
            if patched.contains("self.config = setmetatable(cardAreaTable.config, {")
                && !patched.contains("if not cardAreaTable.config.card_limits then")
            {
                patched = patched
                    .replace(
                        "payload = '''\nself.config = setmetatable(cardAreaTable.config, {",
                        "payload = '''\nif not cardAreaTable.config.card_limits then\n    local clim = cardAreaTable.config.card_limit or 8\n    cardAreaTable.config.card_limits = {\n        base = clim,\n        total_slots = clim,\n        mod = 0,\n        extra_slots = 0,\n        extra_slots_used = 0,\n    }\nend\nself.config = setmetatable(cardAreaTable.config, {",
                    )
                    .replace(
                        "if self.config.type == 'hand' and cardAreaTable.config.card_limits.total_slots ~= (cardAreaTable.config.card_limits.base or 0) + (cardAreaTable.config.card_limits.mod or 0) + (cardAreaTable.config.card_limits.extra_slots or 0) then",
                        "if self.config.type == 'hand' and cardAreaTable.config.card_limits and cardAreaTable.config.card_limits.total_slots and cardAreaTable.config.card_limits.total_slots ~= (cardAreaTable.config.card_limits.base or 0) + (cardAreaTable.config.card_limits.mod or 0) + (cardAreaTable.config.card_limits.extra_slots or 0) then",
                    );
            }
            if patched.contains("local mod = unfixed and (card.ability.card_limit - card.ability.extra_slots_used) or 0") {
                patched = patched.replace(
                    "local mod = unfixed and (card.ability.card_limit - card.ability.extra_slots_used) or 0",
                    "local mod = (unfixed and card and card.ability) and ((card.ability.card_limit or 0) - (card.ability.extra_slots_used or 0)) or 0",
                );
            }
            let _ = fs::write(&card_limit_toml, patched);
        }
    }
    let game_obj_lua = smods_dir.join("src").join("game_object.lua");
    if game_obj_lua.exists() {
        if let Ok(content) = fs::read_to_string(&game_obj_lua) {
            let mut patched = content;
            if patched.contains("math.min(cfg.choose + (G.GAME.modifiers.booster_choice_mod or 0)") {
                patched = patched.replace(
                    "local cfg = (card and card.ability) or self.config\n        return {\n            vars = { math.min(cfg.choose + (G.GAME.modifiers.booster_choice_mod or 0), math.max(1, cfg.extra + (G.GAME.modifiers.booster_size_mod or 0))), math.max(1, cfg.extra + (G.GAME.modifiers.booster_size_mod or 0)) },",
                    "local cfg = (card and card.ability) or self.config or {}\n        local choose = cfg.choose or (self.config and self.config.choose) or 1\n        local extra = cfg.extra or (self.config and self.config.extra) or 3\n        local choice_mod = (G.GAME and G.GAME.modifiers and G.GAME.modifiers.booster_choice_mod) or 0\n        local size_mod = (G.GAME and G.GAME.modifiers and G.GAME.modifiers.booster_size_mod) or 0\n        local total_size = math.max(1, extra + size_mod)\n        local total_choose = math.min(choose + choice_mod, total_size)\n        return {\n            vars = { total_choose, total_size },",
                );
            }
            if patched.contains("function SMODS.Edition.get_card_limit_key(card)\n        return G.P_CENTERS[card.edition.key]:card_limit_key(card)\n    end") {
                patched = patched.replace(
                    "function SMODS.Edition.get_card_limit_key(card)\n        return G.P_CENTERS[card.edition.key]:card_limit_key(card)\n    end",
                    "function SMODS.Edition.get_card_limit_key(card)\n        if card and card.edition then\n            if not card.edition.key then\n                for k, v in pairs(card.edition) do\n                    if v and G.P_CENTERS['e_' .. k] then\n                        card.edition.key = 'e_' .. k\n                        break\n                    end\n                end\n            end\n            local ed_center = card.edition.key and G.P_CENTERS[card.edition.key]\n            if ed_center and ed_center.card_limit_key then\n                return ed_center:card_limit_key(card)\n            end\n        end\n        return nil\n    end",
                );
            }
            let _ = fs::write(&game_obj_lua, patched);
        }
    }
    let poker_hand_toml = smods_dir.join("lovely").join("poker_hand.toml");
    if poker_hand_toml.exists() {
        if let Ok(content) = fs::read_to_string(&poker_hand_toml) {
            if content.contains("payload = '''G.GAME.hands[text].played_this_ante = G.GAME.hands[text].played_this_ante + 1'''") {
                let patched = content.replace(
                    "payload = '''G.GAME.hands[text].played_this_ante = G.GAME.hands[text].played_this_ante + 1'''",
                    "payload = '''G.GAME.hands[text].played_this_ante = (G.GAME.hands[text].played_this_ante or 0) + 1'''",
                );
                let _ = fs::write(&poker_hand_toml, patched);
            }
        }
    }
    let fixes_toml = smods_dir.join("lovely").join("fixes.toml");
    if fixes_toml.exists() {
        if let Ok(content) = fs::read_to_string(&fixes_toml) {
            let mut patched = content;
            if patched.contains("payload = \"local cfg = (card and card.ability) or _c['config']\"") {
                patched = patched.replace(
                    "payload = \"local cfg = (card and card.ability) or _c['config']\"",
                    "payload = \"local cfg = (card and card.ability and _c['config'] and SMODS.merge_defaults(copy_table(card.ability), _c['config'])) or (card and card.ability) or _c['config']\"",
                );
            }
            let _ = fs::write(&fixes_toml, patched);
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
        let candidates = [
            data_dir.join("balatro").join("liblovely.so"),
            data_dir.join("Balatro").join("liblovely.so"),
            data_dir.join("love").join("Balatro").join("liblovely.so"),
            data_dir.join("love").join("balatro").join("liblovely.so"),
            data_dir.join("love-11").join("liblovely.so"),
        ];
        for candidate in candidates {
            if candidate.exists() {
                return Some(candidate);
            }
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
