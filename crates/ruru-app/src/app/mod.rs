use iced::window as iced_window;
use ruru_core::Output as CoreOutput;
use ruru_ui::{main_window, overlay};
use state::App;

mod runtime;
mod state;
mod style;
mod subscription;
mod update;
mod view;
mod windows;

pub(super) use runtime::run;

#[derive(Debug, Clone)]
enum Input {
    WindowOpened(iced_window::Id),
    WindowCloseRequested(iced_window::Id),
    #[cfg(target_os = "macos")]
    ShowMainWindow,
    DragWindow,
    Core(CoreOutput),
    MainWindow(main_window::Input),
    Overlay(overlay::Input),
    CheckPromptInputFocus(iced_window::Id),
    BeginRegionSelection,
}
