use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, svg, text, text_input, toggler,
};
use iced::{
    time, Alignment, Border, Color, Element, Length, Shadow, Subscription, Task, Vector,
};
use std::path::PathBuf;
use std::time::Duration;

use crate::config::{load_config, save_config, AppConfig};
use crate::installer::download_and_install_lovely;
use crate::launcher::{find_lovely_lib, launch_game, LaunchConfig};
use crate::mods::{scan_mods, toggle_mod, ModInfo};
use crate::scanner::{find_game_executable, find_love_binary, get_default_mods_dir};

// Embedded SVG icons (vectors scale with pixel-perfect precision)
const ICON_CARD_SVG: &[u8] = include_bytes!("../assets/icons/card.svg");
const ICON_PUZZLE_SVG: &[u8] = include_bytes!("../assets/icons/puzzle.svg");
const ICON_GEAR_SVG: &[u8] = include_bytes!("../assets/icons/gear.svg");
const ICON_REFRESH_SVG: &[u8] = include_bytes!("../assets/icons/refresh.svg");
const ICON_FOLDER_SVG: &[u8] = include_bytes!("../assets/icons/folder.svg");
const ICON_PLAY_SVG: &[u8] = include_bytes!("../assets/icons/play.svg");
const ICON_DOWNLOAD_SVG: &[u8] = include_bytes!("../assets/icons/download.svg");

// =========================================================================
// APPLE HUMAN INTERFACE GUIDELINES DESIGN SYSTEM (macOS Sequoia Dark / Pro)
// =========================================================================
// Materials & Layers:
pub const COLOR_WINDOW_BG: Color = Color::from_rgb(0.102, 0.106, 0.114); // #1a1b1d macOS window background
pub const COLOR_SIDEBAR_BG: Color = Color::from_rgb(0.125, 0.129, 0.137); // #202123 Translucent sidebar tone
pub const COLOR_TOOLBAR_BG: Color = Color::from_rgb(0.122, 0.125, 0.133); // #1f2022 Window header / titlebar
pub const COLOR_CARD_BG: Color = Color::from_rgb(0.153, 0.157, 0.169); // #27282b Grouped cell surface
pub const COLOR_CARD_BORDER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.06); // Hairline separator
pub const ACCENT_APPLE_BLUE: Color = Color::from_rgb(0.039, 0.518, 1.0); // #0a84ff macOS Primary Accent
pub const ACCENT_APPLE_GREEN: Color = Color::from_rgb(0.196, 0.843, 0.294); // #32d74b macOS System Green
pub const ACCENT_APPLE_RED: Color = Color::from_rgb(1.0, 0.271, 0.227); // #ff453a macOS System Red
pub const ACCENT_BALATRO_GOLD: Color = Color::from_rgb(1.0, 0.839, 0.039); // #ffd60a Balatro Edition Gold

pub const COLOR_LABEL_PRIMARY: Color = Color::from_rgb(0.98, 0.98, 0.99); // 100% Text Label
pub const COLOR_LABEL_SECONDARY: Color = Color::from_rgb(0.70, 0.72, 0.76); // Secondary Label
pub const COLOR_LABEL_TERTIARY: Color = Color::from_rgb(0.48, 0.50, 0.54); // Caption / Muted

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationTab {
    Mods,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    SelectTab(NavigationTab),
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
    pub current_tab: NavigationTab,
    pub config: AppConfig,
    pub love_binary: Option<PathBuf>,
    pub game_path: Option<PathBuf>,
    pub game_path_input: String,
    pub mods_dir: PathBuf,
    pub mods: Vec<ModInfo>,
    pub lovely_lib: Option<PathBuf>,
    pub status_message: String,
    pub is_installing_lovely: bool,
    pub animation_phase: f32,
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
            "LÖVE не найден в системе (sudo apt install love)".to_string()
        } else if game_path.is_none() {
            "Игра не обнаружена. Укажите путь в Настройках.".to_string()
        } else {
            "Все системы готовы к запуску".to_string()
        };

        (
            Self {
                current_tab: NavigationTab::Mods,
                config,
                love_binary,
                game_path,
                game_path_input: initial_input,
                mods_dir,
                mods,
                lovely_lib,
                status_message,
                is_installing_lovely: false,
                animation_phase: 0.0,
            },
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        "Balatro".to_string()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                self.animation_phase = (self.animation_phase + 0.04) % (std::f32::consts::PI * 2.0);
                Task::none()
            }
            Message::SelectTab(tab) => {
                self.current_tab = tab;
                Task::none()
            }
            Message::RefreshData => {
                self.love_binary = self.config.love_binary.clone().or_else(find_love_binary);
                self.game_path = find_game_executable(self.config.game_path.as_deref());
                self.mods = scan_mods(&self.mods_dir);
                self.lovely_lib =
                    find_lovely_lib(self.game_path.as_deref().and_then(|p| p.parent()));
                self.status_message = "Данные и список модов обновлены".to_string();
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
                    self.status_message = "Путь к исполняемому файлу сохранен".to_string();
                } else {
                    self.status_message = "Указанный путь не существует".to_string();
                }
                Task::none()
            }
            Message::InstallLovely => {
                self.is_installing_lovely = true;
                self.status_message = "Загрузка и установка Lovely Injector...".to_string();

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
                        self.status_message = "Lovely Injector готов к работе!".to_string();
                    }
                    Err(e) => {
                        self.status_message = format!("Ошибка установки Lovely: {e}");
                    }
                }
                Task::none()
            }
            Message::LaunchGame => {
                let Some(ref love_bin) = self.love_binary else {
                    self.status_message = "LÖVE не найден в системе!".to_string();
                    return Task::none();
                };
                let Some(ref game_target) = self.game_path else {
                    self.status_message = "Путь к Balatro не выбран!".to_string();
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
                            "Balatro запущена ({})",
                            if self.config.modded_mode {
                                "Режим с модами"
                            } else {
                                "Оригинальная версия"
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
        let pulse = (self.animation_phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);

        // =========================================================================
        // 1. MACOS UNIFIED TITLEBAR / WINDOW CONTROLS (Traffic lights + Title)
        // =========================================================================
        let traffic_lights = row![
            // Close (Red)
            container(text("").size(1))
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(1.0, 0.36, 0.33))),
                    border: Border { radius: 6.0.into(), ..Default::default() },
                    ..Default::default()
                }),
            // Minimize (Yellow)
            container(text("").size(1))
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(1.0, 0.75, 0.18))),
                    border: Border { radius: 6.0.into(), ..Default::default() },
                    ..Default::default()
                }),
            // Zoom (Green)
            container(text("").size(1))
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.16, 0.79, 0.28))),
                    border: Border { radius: 6.0.into(), ..Default::default() },
                    ..Default::default()
                }),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        // Sidebar Brand header integrated below traffic lights
        let brand_icon = svg(svg::Handle::from_memory(ICON_CARD_SVG))
            .width(18)
            .height(18)
            .style(|_, _| svg::Style {
                color: Some(ACCENT_APPLE_RED),
            });

        let app_brand = row![
            brand_icon,
            column![
                text("Balatro").size(14).color(COLOR_LABEL_PRIMARY),
                text("Launcher").size(11).color(COLOR_LABEL_TERTIARY),
            ]
            .spacing(1)
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let sidebar_header = column![
            traffic_lights,
            app_brand,
        ]
        .spacing(16)
        .padding([14, 18]);

        // =========================================================================
        // 2. MACOS HIG SIDEBAR ITEMS (Clean rounded selection pills)
        // =========================================================================
        let mods_count_badge = container(
            text(format!("{}", self.mods.len()))
                .size(11)
                .color(COLOR_LABEL_SECONDARY),
        )
        .padding([2, 7])
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
            border: Border { radius: 10.0.into(), ..Default::default() },
            ..Default::default()
        });

        let tab_mods_active = self.current_tab == NavigationTab::Mods;
        let puzzle_icon = svg(svg::Handle::from_memory(ICON_PUZZLE_SVG))
            .width(16)
            .height(16)
            .style(move |_, _| svg::Style {
                color: Some(if tab_mods_active {
                    ACCENT_APPLE_BLUE
                } else {
                    COLOR_LABEL_SECONDARY
                }),
            });

        let tab_mods_btn = button(
            row![
                puzzle_icon,
                text("Модификации").size(13),
                horizontal_space(),
                mods_count_badge
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(move |_, _| button::Style {
            background: Some(iced::Background::Color(if tab_mods_active {
                Color::from_rgba(0.039, 0.518, 1.0, 0.15)
            } else {
                Color::TRANSPARENT
            })),
            text_color: if tab_mods_active {
                COLOR_LABEL_PRIMARY
            } else {
                COLOR_LABEL_SECONDARY
            },
            border: Border { radius: 7.0.into(), ..Default::default() },
            ..Default::default()
        })
        .on_press(Message::SelectTab(NavigationTab::Mods));

        let tab_settings_active = self.current_tab == NavigationTab::Settings;
        let gear_icon = svg(svg::Handle::from_memory(ICON_GEAR_SVG))
            .width(16)
            .height(16)
            .style(move |_, _| svg::Style {
                color: Some(if tab_settings_active {
                    ACCENT_APPLE_BLUE
                } else {
                    COLOR_LABEL_SECONDARY
                }),
            });

        let tab_settings_btn = button(
            row![
                gear_icon,
                text("Настройки").size(13),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(move |_, _| button::Style {
            background: Some(iced::Background::Color(if tab_settings_active {
                Color::from_rgba(0.039, 0.518, 1.0, 0.15)
            } else {
                Color::TRANSPARENT
            })),
            text_color: if tab_settings_active {
                COLOR_LABEL_PRIMARY
            } else {
                COLOR_LABEL_SECONDARY
            },
            border: Border { radius: 7.0.into(), ..Default::default() },
            ..Default::default()
        })
        .on_press(Message::SelectTab(NavigationTab::Settings));

        let refresh_icon = svg(svg::Handle::from_memory(ICON_REFRESH_SVG))
            .width(13)
            .height(13)
            .style(|_, _| svg::Style {
                color: Some(COLOR_LABEL_SECONDARY),
            });

        let refresh_btn = button(
            row![
                refresh_icon,
                text("Обновить").size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([6, 12])
        .width(Length::Fill)
        .style(|_, _| button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04))),
            text_color: COLOR_LABEL_SECONDARY,
            border: Border { radius: 6.0.into(), ..Default::default() },
            ..Default::default()
        })
        .on_press(Message::RefreshData);

        let sidebar_nav = column![
            sidebar_header,
            column![tab_mods_btn, tab_settings_btn].spacing(4).padding([4, 10]),
            iced::widget::vertical_space(),
            column![refresh_btn].padding([12, 10])
        ]
        .width(Length::Fixed(215.0))
        .height(Length::Fill);

        let sidebar_container = container(sidebar_nav)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(COLOR_SIDEBAR_BG)),
                border: Border {
                    color: COLOR_CARD_BORDER,
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            });

        // =========================================================================
        // 3. TOP TOOLBAR (Apple HIG: Inline window title + macOS System Pills)
        // =========================================================================
        let love_indicator = container(
            row![
                text(if self.love_binary.is_some() { "●" } else { "○" })
                    .size(8)
                    .color(if self.love_binary.is_some() { ACCENT_APPLE_GREEN } else { ACCENT_APPLE_RED }),
                text(if self.love_binary.is_some() { "LÖVE 11.5" } else { "LÖVE не найден" })
                    .size(11)
                    .color(COLOR_LABEL_SECONDARY)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .padding([4, 10])
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
            border: Border {
                radius: 12.0.into(),
                color: COLOR_CARD_BORDER,
                width: 1.0,
            },
            ..Default::default()
        });

        let lovely_dot_color = if self.lovely_lib.is_some() {
            Color::from_rgba(
                0.039 + 0.1 * pulse,
                0.518 + 0.1 * pulse,
                1.0,
                0.7 + 0.3 * pulse,
            )
        } else {
            COLOR_LABEL_TERTIARY
        };

        let lovely_pill = container(
            row![
                text(if self.lovely_lib.is_some() { "●" } else { "○" })
                    .size(8)
                    .color(lovely_dot_color),
                text(if self.lovely_lib.is_some() { "Lovely активен" } else { "Lovely выкл" })
                    .size(11)
                    .color(COLOR_LABEL_SECONDARY)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .padding([4, 10])
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
            border: Border {
                radius: 12.0.into(),
                color: COLOR_CARD_BORDER,
                width: 1.0,
            },
            ..Default::default()
        });

        let top_header = container(
            row![
                column![
                    text(match self.current_tab {
                        NavigationTab::Mods => "Управление модификациями",
                        NavigationTab::Settings => "Настройки окружения",
                    })
                    .size(15)
                    .color(COLOR_LABEL_PRIMARY),
                    text(&self.status_message).size(11).color(COLOR_LABEL_TERTIARY),
                ]
                .spacing(2),
                horizontal_space(),
                love_indicator,
                lovely_pill,
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .padding([14, 24]),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(COLOR_TOOLBAR_BG)),
            border: Border {
                color: COLOR_CARD_BORDER,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

        // =========================================================================
        // 4. MAIN CONTENT AREA (macOS Grouped List & Card Insets)
        // =========================================================================
        let main_view: Element<'_, Message> = match self.current_tab {
            NavigationTab::Mods => {
                if self.mods.is_empty() {
                    let folder_icon = svg(svg::Handle::from_memory(ICON_FOLDER_SVG))
                        .width(44)
                        .height(44)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_LABEL_TERTIARY),
                        });

                    container(
                        column![
                            folder_icon,
                            text("Нет установленных модификаций")
                                .size(15)
                                .color(COLOR_LABEL_PRIMARY),
                            text(format!(
                                "Поместите папки с модами в:\n{}",
                                self.mods_dir.display()
                            ))
                            .size(12)
                            .color(COLOR_LABEL_SECONDARY),
                        ]
                        .spacing(10)
                        .align_x(Alignment::Center),
                    )
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
                } else {
                    let mut list = column![].spacing(8);

                    for (idx, m) in self.mods.iter().enumerate() {
                        let is_on = m.is_enabled;

                        let badge = if m.is_smods {
                            container(
                                text("SMODS")
                                    .size(9)
                                    .color(ACCENT_BALATRO_GOLD),
                            )
                            .padding([2, 5])
                            .style(|_| container::Style {
                                background: Some(iced::Background::Color(Color::from_rgba(1.0, 0.839, 0.039, 0.12))),
                                border: Border { radius: 4.0.into(), ..Default::default() },
                                ..Default::default()
                            })
                        } else {
                            container(
                                text("MOD")
                                    .size(9)
                                    .color(COLOR_LABEL_SECONDARY),
                            )
                            .padding([2, 5])
                            .style(|_| container::Style {
                                background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
                                border: Border { radius: 4.0.into(), ..Default::default() },
                                ..Default::default()
                            })
                        };

                        let name = text(&m.name).size(13).color(if is_on {
                            COLOR_LABEL_PRIMARY
                        } else {
                            COLOR_LABEL_SECONDARY
                        });

                        let version = text(
                            m.version
                                .as_deref()
                                .map(|v| format!("v{v}"))
                                .unwrap_or_default(),
                        )
                        .size(11)
                        .color(COLOR_LABEL_TERTIARY);

                        let author_desc = if let Some(ref a) = m.author {
                            format!("от {a}")
                        } else if let Some(ref d) = m.description {
                            d.chars().take(50).collect::<String>()
                        } else {
                            String::new()
                        };

                        let has_subtext = !author_desc.is_empty();
                        let author_label = text(author_desc).size(11).color(COLOR_LABEL_TERTIARY);

                        let info_col = if has_subtext {
                            column![
                                row![badge, name, version].spacing(8).align_y(Alignment::Center),
                                author_label,
                            ]
                            .spacing(2)
                        } else {
                            column![
                                row![badge, name, version].spacing(8).align_y(Alignment::Center),
                            ]
                        };

                        let toggle = toggler(is_on)
                            .on_toggle(move |_| Message::ToggleMod(idx))
                            .size(16);

                        let border_color = if is_on {
                            Color::from_rgba(
                                0.039,
                                0.518,
                                1.0,
                                0.30 + 0.20 * pulse,
                            )
                        } else {
                            COLOR_CARD_BORDER
                        };

                        let card = container(
                            row![info_col, horizontal_space(), toggle]
                                .align_y(Alignment::Center)
                                .padding([12, 16]),
                        )
                        .style(move |_| container::Style {
                            background: Some(iced::Background::Color(COLOR_CARD_BG)),
                            border: Border {
                                color: border_color,
                                width: 1.0,
                                radius: 8.0.into(),
                            },
                            shadow: Shadow {
                                color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                                offset: Vector::new(0.0, 1.0),
                                blur_radius: 2.0,
                            },
                            ..Default::default()
                        });

                        list = list.push(card);
                    }

                    scrollable(list.padding([18, 24])).height(Length::Fill).into()
                }
            }
            NavigationTab::Settings => {
                let path_label = text("Расположение игры (Balatro)").size(13).color(COLOR_LABEL_PRIMARY);
                let path_input = text_input("Путь к Balatro.exe / balatro", &self.game_path_input)
                    .on_input(Message::GamePathInputChanged)
                    .size(12)
                    .padding(8)
                    .style(|_, _| text_input::Style {
                        background: iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.25)),
                        border: Border {
                            radius: 6.0.into(),
                            color: COLOR_CARD_BORDER,
                            width: 1.0,
                        },
                        icon: COLOR_LABEL_TERTIARY,
                        placeholder: COLOR_LABEL_TERTIARY,
                        value: COLOR_LABEL_PRIMARY,
                        selection: ACCENT_APPLE_BLUE,
                    });

                let save_btn = button(text("Сохранить").size(12))
                    .padding([6, 14])
                    .style(|_, _| button::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                        text_color: COLOR_LABEL_PRIMARY,
                        border: Border { radius: 6.0.into(), ..Default::default() },
                        ..Default::default()
                    })
                    .on_press(Message::SaveGamePath);

                let path_card = container(
                    column![path_label, row![path_input, save_btn].spacing(8)].spacing(8).padding(16)
                )
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(COLOR_CARD_BG)),
                    border: Border {
                        radius: 8.0.into(),
                        color: COLOR_CARD_BORDER,
                        width: 1.0,
                    },
                    ..Default::default()
                });

                let lovely_title = text("Инжектор модификаций (Lovely)").size(13).color(COLOR_LABEL_PRIMARY);
                let lovely_desc = text(
                    "Lovely перехватывает код игры для поддержки модов и дополнительных колод."
                )
                .size(11)
                .color(COLOR_LABEL_SECONDARY);

                let dl_icon = svg(svg::Handle::from_memory(ICON_DOWNLOAD_SVG))
                    .width(13)
                    .height(13)
                    .style(|_, _| svg::Style { color: Some(Color::WHITE) });

                let lovely_action_btn = if self.is_installing_lovely {
                    button(text("Загрузка с GitHub...").size(12)).padding([6, 14])
                } else {
                    button(
                        row![
                            dl_icon,
                            text(if self.lovely_lib.is_some() {
                                "Обновить Lovely"
                            } else {
                                "Установить Lovely"
                            }).size(12)
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center)
                    )
                    .padding([6, 14])
                    .style(|_, _| button::Style {
                        background: Some(iced::Background::Color(ACCENT_APPLE_BLUE)),
                        text_color: Color::WHITE,
                        border: Border { radius: 6.0.into(), ..Default::default() },
                        ..Default::default()
                    })
                    .on_press(Message::InstallLovely)
                };

                let lovely_card = container(
                    column![
                        lovely_title,
                        lovely_desc,
                        lovely_action_btn
                    ]
                    .spacing(8)
                    .padding(16)
                )
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(COLOR_CARD_BG)),
                    border: Border {
                        radius: 8.0.into(),
                        color: COLOR_CARD_BORDER,
                        width: 1.0,
                    },
                    ..Default::default()
                });

                let settings_col = column![path_card, lovely_card].spacing(12).padding([18, 24]);
                scrollable(settings_col).height(Length::Fill).into()
            }
        };

        // =========================================================================
        // 5. MACOS SEGMENTED CONTROL / BOTTOM ACTION BAR
        // =========================================================================
        let is_modded = self.config.modded_mode;

        // Apple Segmented Control for launch mode (Vanilla vs Modded)
        let vanilla_seg = button(text("Чистый (Vanilla)").size(12))
            .padding([5, 12])
            .style(move |_, _| button::Style {
                background: Some(iced::Background::Color(if !is_modded {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.14)
                } else {
                    Color::TRANSPARENT
                })),
                text_color: if !is_modded {
                    COLOR_LABEL_PRIMARY
                } else {
                    COLOR_LABEL_TERTIARY
                },
                border: Border { radius: 5.0.into(), ..Default::default() },
                ..Default::default()
            })
            .on_press(Message::SetModdedMode(false));

        let modded_seg = button(text("С модами (Lovely)").size(12))
            .padding([5, 12])
            .style(move |_, _| button::Style {
                background: Some(iced::Background::Color(if is_modded {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.14)
                } else {
                    Color::TRANSPARENT
                })),
                text_color: if is_modded {
                    COLOR_LABEL_PRIMARY
                } else {
                    COLOR_LABEL_TERTIARY
                },
                border: Border { radius: 5.0.into(), ..Default::default() },
                ..Default::default()
            })
            .on_press(Message::SetModdedMode(true));

        let segmented_container = container(
            row![vanilla_seg, modded_seg].spacing(2).padding(2)
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.35))),
            border: Border {
                radius: 7.0.into(),
                color: COLOR_CARD_BORDER,
                width: 1.0,
            },
            ..Default::default()
        });

        // macOS Primary Prominent Button with subtle breathing glow
        let play_icon = svg(svg::Handle::from_memory(ICON_PLAY_SVG))
            .width(13)
            .height(13)
            .style(|_, _| svg::Style { color: Some(Color::WHITE) });

        let button_glow_alpha = 0.25 + 0.15 * pulse;
        let launch_btn = button(
            row![
                play_icon,
                text(if self.config.modded_mode {
                    "Играть с модами"
                } else {
                    "Играть"
                })
                .size(13),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .padding([8, 22])
        .style(move |_, _| button::Style {
            background: Some(iced::Background::Color(ACCENT_APPLE_RED)),
            text_color: Color::WHITE,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(1.0, 0.271, 0.227, button_glow_alpha),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 5.0 + 3.0 * pulse,
            },
            ..Default::default()
        })
        .on_press(Message::LaunchGame);

        let bottom_bar = container(
            row![
                row![text("Режим запуска:").size(12).color(COLOR_LABEL_SECONDARY), segmented_container]
                    .spacing(10)
                    .align_y(Alignment::Center),
                horizontal_space(),
                launch_btn,
            ]
            .align_y(Alignment::Center)
            .padding([12, 24]),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(COLOR_SIDEBAR_BG)),
            border: Border {
                color: COLOR_CARD_BORDER,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

        // CONTENT PANE (Toolbar + Body + Bottom Bar)
        let content_pane = column![top_header, main_view, bottom_bar]
            .width(Length::Fill)
            .height(Length::Fill);

        // ROOT SPLIT VIEW
        let root_split = row![sidebar_container, content_pane]
            .width(Length::Fill)
            .height(Length::Fill);

        container(root_split)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(COLOR_WINDOW_BG)),
                text_color: Some(COLOR_LABEL_PRIMARY),
                ..Default::default()
            })
            .into()
    }
}
