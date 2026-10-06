use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, text, text_input, toggler,
};
use iced::{
    Alignment, Color, Element, Length, Task,
};
use std::path::PathBuf;

use crate::config::{load_config, save_config, AppConfig};
use crate::installer::download_and_install_lovely;
use crate::launcher::{find_lovely_lib, launch_game, LaunchConfig};
use crate::mods::{scan_mods, toggle_mod, ModInfo};
use crate::scanner::{find_game_executable, find_love_binary, get_default_mods_dir};

// Balatro Aesthetic Colors
pub const BG_COLOR: Color = Color::from_rgb(0.08, 0.09, 0.11);
pub const CARD_BG: Color = Color::from_rgb(0.13, 0.15, 0.19);
pub const ACCENT_RED: Color = Color::from_rgb(0.92, 0.28, 0.25);
pub const ACCENT_BLUE: Color = Color::from_rgb(0.0, 0.65, 0.95);
pub const ACCENT_YELLOW: Color = Color::from_rgb(0.98, 0.77, 0.19);
pub const TEXT_MUTED: Color = Color::from_rgb(0.65, 0.68, 0.73);
pub const SUCCESS_GREEN: Color = Color::from_rgb(0.24, 0.78, 0.45);
#[derive(Debug, Clone)]
pub enum Message {
    RefreshData,
    ToggleMod(usize),
    SetModdedMode(bool),
    LaunchGame,
    InstallLovely,
    LovelyInstallResult(Result<PathBuf, String>),
    GamePathInputChanged(String),
    SaveGamePath,
}

pub struct LauncherApp {
    pub config: AppConfig,
    pub love_binary: Option<PathBuf>,
    pub game_path: Option<PathBuf>,
    pub game_path_input: String,
    pub mods_dir: PathBuf,
    pub mods: Vec<ModInfo>,
    pub lovely_lib: Option<PathBuf>,
    pub status_message: String,
    pub is_installing_lovely: bool,
}

impl LauncherApp {
    pub fn new() -> (Self, Task<Message>) {
        let config = load_config();
        let love_binary = config.love_binary.clone().or_else(find_love_binary);
        let game_path = find_game_executable(config.game_path.as_deref());
        let mods_dir = config
            .custom_mods_dir
            .clone()
            .unwrap_or_else(get_default_mods_dir);
        let mods = scan_mods(&mods_dir);
        let lovely_lib = find_lovely_lib(game_path.as_deref().and_then(|p| p.parent()));

        let initial_input = game_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let status_message = if love_binary.is_none() {
            "Внимание: LÖVE не найден в системе (установи: sudo apt install love)".to_string()
        } else if game_path.is_none() {
            "Balatro не найден автоматически. Укажи путь вручную ниже.".to_string()
        } else {
            "Готов к запуску!".to_string()
        };

        (
            Self {
                config,
                love_binary,
                game_path,
                game_path_input: initial_input,
                mods_dir,
                mods,
                lovely_lib,
                status_message,
                is_installing_lovely: false,
            },
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        "Balatro Linux Launcher & Mod Manager".to_string()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshData => {
                self.love_binary = self.config.love_binary.clone().or_else(find_love_binary);
                self.game_path = find_game_executable(self.config.game_path.as_deref());
                self.mods = scan_mods(&self.mods_dir);
                self.lovely_lib =
                    find_lovely_lib(self.game_path.as_deref().and_then(|p| p.parent()));
                self.status_message = "Список модов и пути обновлены.".to_string();
                Task::none()
            }
            Message::ToggleMod(index) => {
                if let Some(mod_info) = self.mods.get_mut(index) {
                    if let Ok(new_path) = toggle_mod(mod_info) {
                        mod_info.path = new_path;
                        mod_info.is_enabled = !mod_info.is_enabled;
                        mod_info.folder_name = mod_info
                            .path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();
                    }
                }
                Task::none()
            }
            Message::SetModdedMode(modded) => {
                self.config.modded_mode = modded;
                let _ = save_config(&self.config);
                Task::none()
            }
            Message::GamePathInputChanged(val) => {
                self.game_path_input = val;
                Task::none()
            }
            Message::SaveGamePath => {
                let path = PathBuf::from(&self.game_path_input);
                if path.exists() {
                    self.game_path = Some(path.clone());
                    self.config.game_path = Some(path);
                    let _ = save_config(&self.config);
                    self.lovely_lib =
                        find_lovely_lib(self.game_path.as_deref().and_then(|p| p.parent()));
                    self.status_message = "Путь к игре сохранен!".to_string();
                } else {
                    self.status_message = "Указанный путь не существует.".to_string();
                }
                Task::none()
            }
            Message::InstallLovely => {
                self.is_installing_lovely = true;
                self.status_message = "Загрузка инжектора Lovely с GitHub...".to_string();

                let target_dir = self
                    .game_path
                    .as_deref()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| self.mods_dir.parent().unwrap().to_path_buf());

                Task::perform(
                    async move {
                        download_and_install_lovely(&target_dir, |_status| ()).await
                    },
                    Message::LovelyInstallResult,
                )
            }
            Message::LovelyInstallResult(res) => {
                self.is_installing_lovely = false;
                match res {
                    Ok(path) => {
                        self.lovely_lib = Some(path);
                        self.status_message =
                            "Lovely успешно установлен! Теперь доступен запуск с модами."
                                .to_string();
                    }
                    Err(e) => {
                        self.status_message = format!("Ошибка установки Lovely: {e}");
                    }
                }
                Task::none()
            }
            Message::LaunchGame => {
                let Some(ref love_bin) = self.love_binary else {
                    self.status_message = "Ошибка: LÖVE не найден в системе!".to_string();
                    return Task::none();
                };
                let Some(ref game_target) = self.game_path else {
                    self.status_message = "Ошибка: путь к Balatro не выбран!".to_string();
                    return Task::none();
                };

                let config = LaunchConfig {
                    love_binary: love_bin.clone(),
                    game_target: game_target.clone(),
                    modded: self.config.modded_mode,
                    lovely_lib_path: self.lovely_lib.clone(),
                };

                match launch_game(&config) {
                    Ok(_) => {
                        self.status_message = format!(
                            "Игра запущена ({})!",
                            if self.config.modded_mode {
                                "С модами / Lovely"
                            } else {
                                "Оригинал / Vanilla"
                            }
                        );
                    }
                    Err(e) => {
                        self.status_message = format!("Ошибка запуска: {e}");
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // TOP HEADER
        let header_title = text("🃏 BALATRO LINUX LAUNCHER")
            .size(24)
            .color(ACCENT_YELLOW);

        let refresh_btn = button(text("🔄 Обновить").size(14))
            .padding([6, 12])
            .on_press(Message::RefreshData);

        let top_header = row![header_title, horizontal_space(), refresh_btn]
            .align_y(Alignment::Center)
            .padding(15);

        // STATUS BAR CARDS
        let love_status = if self.love_binary.is_some() {
            text("✓ LÖVE: Установлен").color(SUCCESS_GREEN)
        } else {
            text("✗ LÖVE: Не найден").color(ACCENT_RED)
        };

        let game_status = if let Some(ref p) = self.game_path {
            text(format!("✓ Игра: {}", p.file_name().unwrap().to_string_lossy()))
                .color(SUCCESS_GREEN)
        } else {
            text("✗ Игра: Не найдена").color(ACCENT_RED)
        };

        let lovely_status = if self.lovely_lib.is_some() {
            text("✓ Lovely Injector: Активен").color(SUCCESS_GREEN)
        } else {
            text("✗ Lovely: Не установлен").color(TEXT_MUTED)
        };

        let install_lovely_btn = if self.is_installing_lovely {
            button(text("Установка...").size(13)).padding([4, 8])
        } else {
            button(text("Установить Lovely").size(13))
                .padding([4, 8])
                .on_press(Message::InstallLovely)
        };

        let status_cards = container(
            row![
                love_status,
                text("|").color(TEXT_MUTED),
                game_status,
                text("|").color(TEXT_MUTED),
                lovely_status,
                install_lovely_btn,
            ]
            .spacing(15)
            .align_y(Alignment::Center),
        )
        .padding([8, 15]);

        // GAME PATH MANUAL OVERRIDE
        let path_row = row![
            text("Путь к игре:").size(13).color(TEXT_MUTED),
            text_input("Путь к Balatro.exe / Balatro.love", &self.game_path_input)
                .on_input(Message::GamePathInputChanged)
                .size(13)
                .padding(6),
            button(text("Сохранить").size(13))
                .padding([6, 12])
                .on_press(Message::SaveGamePath),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([0, 15]);

        // MODS LIST
        let mods_title = row![
            text(format!("Установленные Моды ({})", self.mods.len()))
                .size(16)
                .color(ACCENT_BLUE),
            horizontal_space(),
            text(format!("Директория: {}", self.mods_dir.display()))
                .size(12)
                .color(TEXT_MUTED),
        ]
        .padding([10, 15]);

        let mods_content: Element<'_, Message> = if self.mods.is_empty() {
            container(
                column![
                    text("В папке Mods пока нет модификаций.").color(TEXT_MUTED),
                    text(format!("Поместите моды в: {}", self.mods_dir.display()))
                        .size(12)
                        .color(TEXT_MUTED),
                ]
                .spacing(5),
            )
            .padding(20)
            .into()
        } else {
            let mut list_col = column![].spacing(8);

            for (idx, m) in self.mods.iter().enumerate() {
                let badge = if m.is_smods {
                    container(text("SMODS Core").size(11).color(ACCENT_YELLOW))
                        .padding([2, 6])
                } else {
                    container(text("MOD").size(11).color(TEXT_MUTED)).padding([2, 6])
                };

                let name_label = text(&m.name).size(15);
                let version_label = text(
                    m.version
                        .as_deref()
                        .map(|v| format!("v{v}"))
                        .unwrap_or_default(),
                )
                .size(12)
                .color(TEXT_MUTED);

                let is_on = m.is_enabled;
                let toggle = toggler(is_on)
                    .on_toggle(move |_| Message::ToggleMod(idx))
                    .size(20);

                let mod_card = container(
                    row![badge, name_label, version_label, horizontal_space(), toggle]
                        .spacing(10)
                        .align_y(Alignment::Center),
                )
                .padding([10, 15])
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(CARD_BG)),
                    border: iced::Border {
                        color: if is_on { ACCENT_BLUE } else { Color::TRANSPARENT },
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                });

                list_col = list_col.push(mod_card);
            }

            scrollable(list_col.padding([0, 15])).height(Length::Fill).into()
        };

        // BOTTOM ACTION BAR
        let mode_toggle = toggler(self.config.modded_mode)
            .label(if self.config.modded_mode {
                "Режим: Модифицированный (Lovely / Mods)"
            } else {
                "Режим: Чистый (Vanilla)"
            })
            .on_toggle(Message::SetModdedMode)
            .size(22);

        let launch_btn = button(
            text(if self.config.modded_mode {
                "▶ ИГРАТЬ (MODDED)"
            } else {
                "▶ ИГРАТЬ (VANILLA)"
            })
            .size(18),
        )
        .padding([12, 35])
        .on_press(Message::LaunchGame);

        let bottom_bar = container(
            row![mode_toggle, horizontal_space(), launch_btn]
                .align_y(Alignment::Center)
                .padding(15),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(CARD_BG)),
            ..Default::default()
        });

        // NOTIFICATION BAR
        let status_bar = container(
            text(&self.status_message)
                .size(12)
                .color(ACCENT_YELLOW),
        )
        .padding([6, 15]);

        let main_layout = column![
            top_header,
            status_cards,
            path_row,
            mods_title,
            mods_content,
            status_bar,
            bottom_bar,
        ];

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(BG_COLOR)),
                text_color: Some(Color::WHITE),
                ..Default::default()
            })
            .into()
    }
}
