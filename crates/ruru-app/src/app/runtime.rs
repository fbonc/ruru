use iced::{Task, Theme, window as iced_window};
use ruru_ui::Overlay as OverlayUi;
use ruru_ui::main_window::{self, MainWindow as MainWindowUi};

use super::state::WindowHost;
use super::{App, Input, style, windows};

pub(crate) fn run() -> iced::Result {
    iced::daemon(boot, App::update, App::view)
        .subscription(App::subscription)
        .theme(Theme::Dark)
        .title("ruru")
        .style(style::application)
        .run()
}

fn boot() -> (App, Task<Input>) {
    let (to_core, core_outputs, settings) = ruru_core::start();
    let provider_views = settings
        .providers
        .iter()
        .map(|provider| {
            main_window::ProviderView::new(
                provider.id.clone(),
                provider.name,
                provider.has_key,
                provider.models.clone(),
            )
        })
        .collect();
    let mut main_ui = MainWindowUi::new().with_providers(provider_views);
    if let Some(error) = settings.load_error {
        main_ui.update(main_window::Input::Settings(
            main_window::SettingsInput::Failed(format!("Cannot edit settings: {error}")),
        ));
    }
    let overlay_ui = OverlayUi::new()
        .with_selected_model(settings.selected_model)
        .with_models(settings.available_models);
    let (main, open_main) = iced_window::open(windows::main_settings());
    let (overlay, open_overlay) = iced_window::open(windows::overlay_settings());

    let tasks = vec![
        open_main.map(Input::WindowOpened),
        open_overlay.map(Input::WindowOpened),
        Task::run(core_outputs, Input::Core),
    ];

    (
        App {
            main_window: WindowHost {
                id: main,
                ui: main_ui,
            },
            overlay: WindowHost {
                id: overlay,
                ui: overlay_ui,
            },
            to_core,
            current_provider_request_id: 0,
            selecting_region: false,
        },
        Task::batch(tasks),
    )
}
