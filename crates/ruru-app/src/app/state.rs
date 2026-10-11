use iced::window as iced_window;
use ruru_core::Sender as CoreSender;
use ruru_ui::Overlay as OverlayUi;
use ruru_ui::main_window::MainWindow as MainWindowUi;

pub(super) struct App {
    pub(super) main_window: WindowHost<MainWindowUi>,
    pub(super) overlay: WindowHost<OverlayUi>,
    pub(super) to_core: CoreSender,
    pub(super) current_provider_request_id: ruru_core::ProviderRequestId,
    pub(super) selecting_region: bool,
}

pub(super) struct WindowHost<T> {
    pub(super) id: iced_window::Id,
    pub(super) ui: T,
}
