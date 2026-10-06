mod catalog;
mod config;
mod installer;
mod launcher;
mod mods;
mod scanner;
mod ui;

use ui::LauncherApp;

fn main() -> iced::Result {
    launcher::ensure_desktop_integration();
    iced::application(
        LauncherApp::title,
        LauncherApp::update,
        LauncherApp::view,
    )
    .subscription(LauncherApp::subscription)
    .window_size((850.0, 620.0))
    .theme(|_| iced::Theme::Dark)
    .run_with(LauncherApp::new)
}
