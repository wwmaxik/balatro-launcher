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

// Embedded SVG icons (vectors scale perfectly, crisp rendering)
const ICON_CARD_SVG: &[u8] = include_bytes!("../assets/icons/card.svg");
const ICON_PUZZLE_SVG: &[u8] = include_bytes!("../assets/icons/puzzle.svg");
const ICON_GEAR_SVG: &[u8] = include_bytes!("../assets/icons/gear.svg");
const ICON_REFRESH_SVG: &[u8] = include_bytes!("../assets/icons/refresh.svg");
const ICON_FOLDER_SVG: &[u8] = include_bytes!("../assets/icons/folder.svg");

// ==========================================
// APPLE HIG DESIGN SYSTEM & BALATRO PALETTE
// ==========================================
pub const COLOR_WINDOW_BG: Color = Color::from_rgb(0.094, 0.098, 0.106); // #18191b (macOS Dark Canvas)
pub const COLOR_SIDEBAR_BG: Color = Color::from_rgb(0.118, 0.122, 0.133); // #1e1f22 (Sidebar Material)
pub const COLOR_CARD_BG: Color = Color::from_rgb(0.145, 0.153, 0.165); // #25272a (Card Surface)
pub const COLOR_CARD_BORDER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08); // Subtle separator

// Typography & Labels:
pub const COLOR_LABEL_PRIMARY: Color = Color::from_rgb(0.96, 0.96, 0.97); // 100% Label
pub const COLOR_LABEL_SECONDARY: Color = Color::from_rgb(0.68, 0.70, 0.74); // Secondary Label
pub const COLOR_LABEL_TERTIARY: Color = Color::from_rgb(0.48, 0.50, 0.54); // Tertiary / Caption

// Accents:
pub const ACCENT_BALATRO_GOLD: Color = Color::from_rgb(0.98, 0.76, 0.20);
pub const ACCENT_BALATRO_RED: Color = Color::from_rgb(0.92, 0.28, 0.25);
pub const ACCENT_APPLE_BLUE: Color = Color::from_rgb(0.18, 0.54, 0.96);
pub const ACCENT_SYSTEM_GREEN: Color = Color::from_rgb(0.24, 0.78, 0.45);

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
    pub animation_phase: f32, // Smooth pulsing animation counter
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
            "Система готова к запуску".to_string()
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
        "Balatro Launcher".to_string()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // 60 FPS smooth animation ticker for subtle breathing glows
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
                self.status_message = "Данные обновлены".to_string();
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
                        self.status_message = "Lovely Injector успешно активирован!".to_string();
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
        // Subtle Apple-style breathing oscillation for active accent elements
        let pulse = (self.animation_phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);

        // SIDEBAR HEADER
        let brand_icon = svg(svg::Handle::from_memory(ICON_CARD_SVG))
            .width(20)
            .height(20)
            .style(|_, _| svg::Style {
                color: Some(ACCENT_BALATRO_RED),
            });

        let app_brand = row![
            brand_icon,
            column![
                text("Balatro").size(15).color(COLOR_LABEL_PRIMARY),
                text("Launcher").size(11).color(COLOR_LABEL_TERTIARY),
            ]
            .spacing(1)
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([18, 16]);

        let mods_count_badge = container(
            text(format!("{}", self.mods.len()))
                .size(11)
                .color(COLOR_LABEL_SECONDARY),
        )
        .padding([2, 8])
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
            border: Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let tab_mods_active = self.current_tab == NavigationTab::Mods;
        let puzzle_icon = svg(svg::Handle::from_memory(ICON_PUZZLE_SVG))
            .width(16)
            .height(16)
            .style(move |_, _| svg::Style {
                color: Some(if tab_mods_active {
                    COLOR_LABEL_PRIMARY
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
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::TRANSPARENT
            })),
            text_color: if tab_mods_active {
                COLOR_LABEL_PRIMARY
            } else {
                COLOR_LABEL_SECONDARY
            },
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .on_press(Message::SelectTab(NavigationTab::Mods));

        let tab_settings_active = self.current_tab == NavigationTab::Settings;
        let gear_icon = svg(svg::Handle::from_memory(ICON_GEAR_SVG))
            .width(16)
            .height(16)
            .style(move |_, _| svg::Style {
                color: Some(if tab_settings_active {
                    COLOR_LABEL_PRIMARY
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
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::TRANSPARENT
            })),
            text_color: if tab_settings_active {
                COLOR_LABEL_PRIMARY
            } else {
                COLOR_LABEL_SECONDARY
            },
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .on_press(Message::SelectTab(NavigationTab::Settings));

        let refresh_icon = svg(svg::Handle::from_memory(ICON_REFRESH_SVG))
            .width(14)
            .height(14)
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
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
            text_color: COLOR_LABEL_SECONDARY,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .on_press(Message::RefreshData);

        let sidebar_nav = column![
            app_brand,
            column![tab_mods_btn, tab_settings_btn].spacing(4).padding([0, 10]),
            iced::widget::vertical_space(),
            column![refresh_btn].padding([12, 10])
        ]
        .width(Length::Fixed(220.0))
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

        // TOP TOOLBAR
        let love_indicator = container(
            row![
                text(if self.love_binary.is_some() { "●" } else { "○" })
                    .size(9)
                    .color(if self.love_binary.is_some() { ACCENT_SYSTEM_GREEN } else { ACCENT_BALATRO_RED }),
                text(if self.love_binary.is_some() { "LÖVE 11.5" } else { "Нет LÖVE" })
                    .size(12)
                    .color(COLOR_LABEL_SECONDARY)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .padding([5, 12])
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04))),
            border: Border {
                radius: 14.0.into(),
                color: COLOR_CARD_BORDER,
                width: 1.0,
            },
            ..Default::default()
        });

        // Glowing dot animation on Lovely pill
        let lovely_dot_color = if self.lovely_lib.is_some() {
            Color::from_rgba(
                0.18 + 0.1 * pulse,
                0.54 + 0.1 * pulse,
                0.96,
                0.7 + 0.3 * pulse,
            )
        } else {
            COLOR_LABEL_TERTIARY
        };

        let lovely_pill = container(
            row![
                text(if self.lovely_lib.is_some() { "●" } else { "○" })
                    .size(9)
                    .color(lovely_dot_color),
                text(if self.lovely_lib.is_some() { "Lovely Активен" } else { "Lovely выкл" })
                    .size(12)
                    .color(COLOR_LABEL_SECONDARY)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .padding([5, 12])
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04))),
            border: Border {
                radius: 14.0.into(),
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
                        NavigationTab::Settings => "Параметры и окружение",
                    })
                    .size(17)
                    .color(COLOR_LABEL_PRIMARY),
                    text(&self.status_message).size(11).color(COLOR_LABEL_TERTIARY),
                ]
                .spacing(3),
                horizontal_space(),
                love_indicator,
                lovely_pill,
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .padding([16, 24]),
        )
        .style(|_| container::Style {
            border: Border {
                color: COLOR_CARD_BORDER,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

        // MAIN CONTENT AREA
        let main_view: Element<'_, Message> = match self.current_tab {
            NavigationTab::Mods => {
                if self.mods.is_empty() {
                    let folder_icon = svg(svg::Handle::from_memory(ICON_FOLDER_SVG))
                        .width(42)
                        .height(42)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_LABEL_TERTIARY),
                        });

                    container(
                        column![
                            folder_icon,
                            text("Моды не найдены").size(16).color(COLOR_LABEL_PRIMARY),
                            text(format!(
                                "Поместите распакованные папки с модами в:\n{}",
                                self.mods_dir.display()
                            ))
                            .size(12)
                            .color(COLOR_LABEL_SECONDARY),
                        ]
                        .spacing(12)
                        .align_x(Alignment::Center),
                    )
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
                } else {
                    let mut list = column![].spacing(10);

                    for (idx, m) in self.mods.iter().enumerate() {
                        let is_on = m.is_enabled;

                        let badge = if m.is_smods {
                            container(
                                text("SMODS CORE")
                                    .size(10)
                                    .color(ACCENT_BALATRO_GOLD),
                            )
                            .padding([2, 6])
                            .style(|_| container::Style {
                                background: Some(iced::Background::Color(Color::from_rgba(0.98, 0.76, 0.20, 0.12))),
                                border: Border {
                                    radius: 4.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                        } else {
                            container(
                                text("MOD")
                                    .size(10)
                                    .color(COLOR_LABEL_SECONDARY),
                            )
                            .padding([2, 6])
                            .style(|_| container::Style {
                                background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
                                border: Border {
                                    radius: 4.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                        };

                        let name = text(&m.name).size(14).color(if is_on {
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
                        .size(12)
                        .color(COLOR_LABEL_TERTIARY);

                        let author_desc = if let Some(ref a) = m.author {
                            format!("от {a}")
                        } else if let Some(ref d) = m.description {
                            d.chars().take(45).collect::<String>()
                        } else {
                            String::new()
                        };

                        let has_subtext = !author_desc.is_empty();
                        let author_label = text(author_desc).size(12).color(COLOR_LABEL_TERTIARY);

                        let info_col = if has_subtext {
                            column![
                                row![badge, name, version].spacing(8).align_y(Alignment::Center),
                                author_label,
                            ]
                            .spacing(3)
                        } else {
                            column![
                                row![badge, name, version].spacing(8).align_y(Alignment::Center),
                            ]
                        };

                        let toggle = toggler(is_on)
                            .on_toggle(move |_| Message::ToggleMod(idx))
                            .size(18);

                        // Subtle breathing active border glow
                        let border_color = if is_on {
                            Color::from_rgba(
                                0.18,
                                0.54,
                                0.96,
                                0.35 + 0.25 * pulse,
                            )
                        } else {
                            COLOR_CARD_BORDER
                        };

                        let card = container(
                            row![info_col, horizontal_space(), toggle]
                                .align_y(Alignment::Center)
                                .padding([14, 18]),
                        )
                        .style(move |_| container::Style {
                            background: Some(iced::Background::Color(COLOR_CARD_BG)),
                            border: Border {
                                color: border_color,
                                width: 1.0,
                                radius: 10.0.into(),
                            },
                            shadow: Shadow {
                                color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
                                offset: Vector::new(0.0, 1.0),
                                blur_radius: 3.0,
                            },
                            ..Default::default()
                        });

                        list = list.push(card);
                    }

                    scrollable(list.padding([20, 24])).height(Length::Fill).into()
                }
            }
            NavigationTab::Settings => {
                let path_label = text("Путь к исполняемому файлу Balatro:").size(13).color(COLOR_LABEL_PRIMARY);
                let path_input = text_input("Например: /home/.../Balatro/balatro", &self.game_path_input)
                    .on_input(Message::GamePathInputChanged)
                    .size(13)
                    .padding(10)
                    .style(|_, _| text_input::Style {
                        background: iced::Background::Color(COLOR_CARD_BG),
                        border: Border {
                            radius: 8.0.into(),
                            color: COLOR_CARD_BORDER,
                            width: 1.0,
                        },
                        icon: COLOR_LABEL_TERTIARY,
                        placeholder: COLOR_LABEL_TERTIARY,
                        value: COLOR_LABEL_PRIMARY,
                        selection: ACCENT_APPLE_BLUE,
                    });

                let save_btn = button(text("Сохранить").size(13))
                    .padding([8, 16])
                    .style(|_, _| button::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                        text_color: COLOR_LABEL_PRIMARY,
                        border: Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .on_press(Message::SaveGamePath);

                let path_card = container(
                    column![path_label, row![path_input, save_btn].spacing(10)].spacing(8).padding(16)
                )
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(COLOR_CARD_BG)),
                    border: Border {
                        radius: 10.0.into(),
                        color: COLOR_CARD_BORDER,
                        width: 1.0,
                    },
                    ..Default::default()
                });

                let lovely_title = text("Инжектор модов Lovely:").size(13).color(COLOR_LABEL_PRIMARY);
                let lovely_desc = text(
                    "Lovely позволяет перехватывать и исполнять Lua-патчи модов (SMODS, кастомные карты и колоды)."
                )
                .size(12)
                .color(COLOR_LABEL_SECONDARY);

                let lovely_action_btn = if self.is_installing_lovely {
                    button(text("Загрузка с GitHub...").size(13)).padding([8, 16])
                } else {
                    button(
                        text(if self.lovely_lib.is_some() {
                            "Обновить Lovely"
                        } else {
                            "Установить Lovely"
                        })
                        .size(13),
                    )
                    .padding([8, 16])
                    .style(|_, _| button::Style {
                        background: Some(iced::Background::Color(ACCENT_APPLE_BLUE)),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
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
                        radius: 10.0.into(),
                        color: COLOR_CARD_BORDER,
                        width: 1.0,
                    },
                    ..Default::default()
                });

                let settings_col = column![path_card, lovely_card].spacing(14).padding([20, 24]);
                scrollable(settings_col).height(Length::Fill).into()
            }
        };

        // BOTTOM ACTION BAR
        let mode_toggle = toggler(self.config.modded_mode)
            .label(if self.config.modded_mode {
                "Режим: С модами (Lovely)"
            } else {
                "Режим: Оригинал (Vanilla)"
            })
            .size(18)
            .on_toggle(Message::SetModdedMode);

        // Animated breathing shadow on the primary Launch button
        let button_glow_alpha = 0.25 + 0.15 * pulse;
        let launch_btn = button(
            text(if self.config.modded_mode {
                "ИГРАТЬ (MODDED)"
            } else {
                "ИГРАТЬ"
            })
            .size(14),
        )
        .padding([10, 28])
        .style(move |_, _| button::Style {
            background: Some(iced::Background::Color(ACCENT_BALATRO_RED)),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.92, 0.28, 0.25, button_glow_alpha),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 6.0 + 4.0 * pulse,
            },
            ..Default::default()
        })
        .on_press(Message::LaunchGame);

        let bottom_bar = container(
            row![mode_toggle, horizontal_space(), launch_btn]
                .align_y(Alignment::Center)
                .padding([14, 24]),
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

        // CONTENT PANE (Header + View + Bottom Bar)
        let content_pane = column![top_header, main_view, bottom_bar]
            .width(Length::Fill)
            .height(Length::Fill);

        // ROOT TWO-PANE SPLIT
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
