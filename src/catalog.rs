use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModCategory {
    All,
    Core,
    Content,
    Jokers,
    QoL,
    Multiplayer,
}

impl ModCategory {
    pub fn title(&self) -> &'static str {
        match self {
            ModCategory::All => "Все моды",
            ModCategory::Core => "Ядро / API",
            ModCategory::Content => "Контент",
            ModCategory::Jokers => "Джокеры",
            ModCategory::QoL => "Утилиты & QoL",
            ModCategory::Multiplayer => "Мультиплеер",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMod {
    pub id: String,
    pub name: String,
    pub author: String,
    pub category: String,
    pub description: String,
    pub download_url: String,
    pub folder_name: String,
    pub version: String,
    pub requires_steamodded: bool,
    pub requires_talisman: bool,
    pub badge: Option<String>,
}

const EMBEDDED_MODS_JSON: &str = include_str!("../assets/mods_index.json");

pub fn get_curated_catalog() -> Vec<CatalogMod> {
    let mut all_mods = get_featured_mods();

    if let Ok(community_mods) = serde_json::from_str::<Vec<CatalogMod>>(EMBEDDED_MODS_JSON) {
        for cmod in community_mods {
            let already = all_mods.iter().any(|m| {
                m.folder_name.to_lowercase() == cmod.folder_name.to_lowercase()
                    || m.name.to_lowercase() == cmod.name.to_lowercase()
            });
            if !already {
                all_mods.push(cmod);
            }
        }
    }

    all_mods
}

pub fn get_featured_mods() -> Vec<CatalogMod> {
    vec![
        CatalogMod {
            id: "steamodded".to_string(),
            name: "Steamodded (SMODS)".to_string(),
            author: "Steamodded Team".to_string(),
            category: "Ядро / API".to_string(),
            description: "Главное ядро и API для модов Balatro. Необходим для работы 95% контентных модов, джокеров и колод.".to_string(),
            download_url: "https://github.com/Steamodded/smods/archive/refs/heads/main.zip".to_string(),
            folder_name: "Steamodded".to_string(),
            version: "v26.829".to_string(),
            requires_steamodded: false,
            requires_talisman: false,
            badge: Some("Обязательный".to_string()),
        },
        CatalogMod {
            id: "quantum-opt".to_string(),
            name: "QuantumOpt".to_string(),
            author: "wwmaxik".to_string(),
            category: "Утилиты & QoL".to_string(),
            description: "Высокопроизводительный оптимизатор FPS для бесконечных забегов: устраняет смертельный цикл nuGC, ускоряет рендеринг сотен карт и текста.".to_string(),
            download_url: "https://github.com/wwmaxik/QuantumOpt/archive/refs/heads/main.zip".to_string(),
            folder_name: "QuantumOpt".to_string(),
            version: "v1.3.0".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: Some("Оптимизация".to_string()),
        },
        CatalogMod {
            id: "talisman".to_string(),
            name: "Talisman".to_string(),
            author: "MathIsFun_".to_string(),
            category: "Ядро / API".to_string(),
            description: "Снимает лимиты очков (e, ee, бесконечность). Необходим для модов с экспоненциальным счетом (Cryptid и др.).".to_string(),
            download_url: "https://github.com/MathIsFun0/Talisman/archive/refs/heads/main.zip".to_string(),
            folder_name: "Talisman".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: Some("Хит".to_string()),
        },
        CatalogMod {
            id: "cryptid".to_string(),
            name: "Cryptid".to_string(),
            author: "MathIsFun_ & Cryptid Team".to_string(),
            category: "Контент".to_string(),
            description: "Самый масштабный контент-мод! Добавляет сотни безумных джокеров, экзотические колоды, новые механики и астрономические очки.".to_string(),
            download_url: "https://github.com/MathIsFun0/Cryptid/archive/refs/heads/main.zip".to_string(),
            folder_name: "Cryptid".to_string(),
            version: "v0.5.4".to_string(),
            requires_steamodded: true,
            requires_talisman: true,
            badge: Some("Топ".to_string()),
        },
        CatalogMod {
            id: "bunco".to_string(),
            name: "Bunco".to_string(),
            author: "Firch".to_string(),
            category: "Контент".to_string(),
            description: "Высококачественное ванильное расширение: десятки сбалансированных джокеров, новые боссы-блайнды, карты таро и расходники.".to_string(),
            download_url: "https://github.com/Firch/Bunco/archive/refs/heads/main.zip".to_string(),
            folder_name: "Bunco".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: Some("Популярный".to_string()),
        },
        CatalogMod {
            id: "joker-evolution".to_string(),
            name: "Joker Evolution".to_string(),
            author: "elbe".to_string(),
            category: "Джокеры".to_string(),
            description: "Система эволюции джокеров: выполняя условия в раундах, джокеры преобразуются в улучшенные эпические формы.".to_string(),
            download_url: "https://github.com/elbe/JokerEvolution/archive/refs/heads/main.zip".to_string(),
            folder_name: "JokerEvolution".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: Some("Интересный".to_string()),
        },
        CatalogMod {
            id: "multiplayer".to_string(),
            name: "Balatro Multiplayer".to_string(),
            author: "Virtualized".to_string(),
            category: "Мультиплеер".to_string(),
            description: "Реальный мультиплеер в Balatro: дуэли против других игроков, драфт и совместные лобби по сети.".to_string(),
            download_url: "https://github.com/Virtualized/Multiplayer/archive/refs/heads/main.zip".to_string(),
            folder_name: "BalatroMultiplayer".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: Some("Онлайн".to_string()),
        },
        CatalogMod {
            id: "brainstorm-rerolled".to_string(),
            name: "Brainstorm Rerolled".to_string(),
            author: "ABGamma & OceanRamen".to_string(),
            category: "Утилиты & QoL".to_string(),
            description: "Система сохранений (стейты по Z+1..5), супер-быстрый реролл магазина и умный поиск сидов прямо в игре.".to_string(),
            download_url: "https://github.com/ABGamma/Brainstorm-Rerolled/releases/latest/download/Brainstorm-Rerolled.zip".to_string(),
            folder_name: "Brainstorm-Rerolled".to_string(),
            version: "1.0.2".to_string(),
            requires_steamodded: false,
            requires_talisman: false,
            badge: None,
        },
        CatalogMod {
            id: "agarmons".to_string(),
            name: "Agarmons (Pokermon)".to_string(),
            author: "Agarpain".to_string(),
            category: "Джокеры".to_string(),
            description: "Покемоны в виде карт-джокеров! Получают опыт за сыгранные руки и эволюционируют прямо посреди партии.".to_string(),
            download_url: "https://github.com/Agarpain/Agarmons/archive/refs/heads/main.zip".to_string(),
            folder_name: "Agarmons".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: None,
        },
        CatalogMod {
            id: "flush-hotkeys".to_string(),
            name: "Flush Hotkeys".to_string(),
            author: "Agoraaa".to_string(),
            category: "Утилиты & QoL".to_string(),
            description: "Горячие клавиши для быстрого выбора флешей, стритов, сортировки карт, сброса и розыгрыша рук.".to_string(),
            download_url: "https://github.com/Agoraaa/FlushHotkeys/archive/refs/heads/main.zip".to_string(),
            folder_name: "FlushHotkeys".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: None,
        },
        CatalogMod {
            id: "six-suits".to_string(),
            name: "Six Suits".to_string(),
            author: "Aure".to_string(),
            category: "Контент".to_string(),
            description: "Добавляет 2 новые масти (Звёзды и Луны), расширяя колоду до 78 карт с новыми комбинациями рук.".to_string(),
            download_url: "https://github.com/Aure/SixSuits/archive/refs/heads/main.zip".to_string(),
            folder_name: "SixSuits".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: None,
        },
        CatalogMod {
            id: "more-fluff".to_string(),
            name: "More Fluff".to_string(),
            author: "notmario".to_string(),
            category: "Контент".to_string(),
            description: "Набор джокеров с пушистыми зверями, оригинальные арты и приятные синергии для казуальной игры.".to_string(),
            download_url: "https://github.com/notmario/MoreFluff/archive/refs/heads/main.zip".to_string(),
            folder_name: "MoreFluff".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: None,
        },
        CatalogMod {
            id: "cardsauce".to_string(),
            name: "Cardsauce".to_string(),
            author: "BarrierTrio".to_string(),
            category: "Контент".to_string(),
            description: "Забавные и безумные джокеры, колоды и испытания, вдохновленные классическими инди-играми.".to_string(),
            download_url: "https://github.com/BarrierTrio/Cardsauce/archive/refs/heads/main.zip".to_string(),
            folder_name: "Cardsauce".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: None,
        },
        CatalogMod {
            id: "mathblinds".to_string(),
            name: "Math Blinds".to_string(),
            author: "Bazinga9000".to_string(),
            category: "Контент".to_string(),
            description: "Необычные математические боссы с уникальными модификаторами очков, требующие нестандартной тактики.".to_string(),
            download_url: "https://github.com/Bazinga9000/MathBlinds/archive/refs/heads/main.zip".to_string(),
            folder_name: "MathBlinds".to_string(),
            version: "main".to_string(),
            requires_steamodded: true,
            requires_talisman: false,
            badge: None,
        },
    ]
}

/// Download a zip archive from a URL, extract it, and optionally flatten if nested
pub async fn download_and_install_mod(
    url: &str,
    target_folder_name: &str,
    mods_dir: &Path,
    on_status: impl Fn(String),
) -> Result<PathBuf, String> {
    on_status(format!("Скачивание архива {}...", target_folder_name));

    let client = reqwest::Client::builder()
        .user_agent("balatro-launcher-linux")
        .build()
        .map_err(|e| format!("Ошибка создания HTTP клиента: {e}"))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Не удалось скачать мод: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Сервер вернул статус HTTP {}", resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Ошибка чтения данных: {e}"))?;

    on_status("Распаковка файлов мода...".to_string());

    let target_dir = mods_dir.join(target_folder_name);
    if target_dir.exists() {
        let _ = fs::remove_dir_all(&target_dir);
    }
    // Also remove disabled version if present
    let disabled_dir = mods_dir.join(format!("{}.disabled", target_folder_name));
    if disabled_dir.exists() {
        let _ = fs::remove_dir_all(&disabled_dir);
    }

    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Не удалось создать папку мода: {e}"))?;

    let cursor = Cursor::new(bytes);
    let mut zip = ZipArchive::new(cursor)
        .map_err(|e| format!("Неверный zip-архив: {e}"))?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let enclosed_name = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let out_path = target_dir.join(&enclosed_name);

        if file.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out_path.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    // Flatten nested directory if the zip contained a single top-level folder (e.g. repo-main/)
    if let Ok(entries) = fs::read_dir(&target_dir) {
        let items: Vec<_> = entries.filter_map(Result::ok).collect();
        if items.len() == 1 && items[0].file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let single_nested = items[0].path();
            let temp_dest = mods_dir.join(format!("__temp_{}", target_folder_name));
            if temp_dest.exists() {
                let _ = fs::remove_dir_all(&temp_dest);
            }

            // Move nested dir out
            fs::rename(&single_nested, &temp_dest)
                .map_err(|e| format!("Ошибка перемещения вложенной папки: {e}"))?;
            let _ = fs::remove_dir_all(&target_dir);
            fs::rename(&temp_dest, &target_dir)
                .map_err(|e| format!("Ошибка финализации папки мода: {e}"))?;
        }
    }

    crate::launcher::ensure_linux_nativefs_compatibility(mods_dir);

    on_status(format!("Мод {} успешно установлен!", target_folder_name));
    Ok(target_dir)
}

/// Delete an installed mod
pub fn delete_mod(folder_name: &str, mods_dir: &Path) -> Result<(), String> {
    let clean = folder_name.trim_end_matches(".disabled");
    let regular = mods_dir.join(clean);
    let disabled = mods_dir.join(format!("{clean}.disabled"));

    let mut deleted = false;
    if regular.exists() {
        fs::remove_dir_all(&regular).map_err(|e| format!("Ошибка удаления: {e}"))?;
        deleted = true;
    }
    if disabled.exists() {
        fs::remove_dir_all(&disabled).map_err(|e| format!("Ошибка удаления: {e}"))?;
        deleted = true;
    }

    if deleted {
        Ok(())
    } else {
        Err("Папка мода не найдена".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_curated_catalog() {
        let catalog = get_curated_catalog();
        assert!(catalog.len() >= 400, "Should load 400+ mods from community index, got {}", catalog.len());
        let smods = catalog.iter().find(|m| m.id == "steamodded");
        assert!(smods.is_some());
        assert_eq!(smods.unwrap().folder_name, "Steamodded");
    }

    #[test]
    fn test_delete_mod() {
        let dir = tempdir().unwrap();
        let mods_dir = dir.path();
        let test_mod = mods_dir.join("TestMod");
        fs::create_dir_all(&test_mod).unwrap();
        assert!(test_mod.exists());

        let res = delete_mod("TestMod", mods_dir);
        assert!(res.is_ok());
        assert!(!test_mod.exists());

        // Test deleting disabled mod
        let test_mod_dis = mods_dir.join("TestMod2.disabled");
        fs::create_dir_all(&test_mod_dis).unwrap();
        assert!(test_mod_dis.exists());

        let res = delete_mod("TestMod2", mods_dir);
        assert!(res.is_ok());
        assert!(!test_mod_dis.exists());
    }
}
