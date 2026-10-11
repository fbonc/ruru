use iced::{Task, window as iced_window};
use ruru_core::{Input as CoreInput, Output as CoreOutput, SettingsChange};
use ruru_types::{ProviderId, Target};
use ruru_ui::main_window;
use ruru_ui::overlay;
use ruru_ui::{OverlayMode, OverlayOutput};

use super::style::{ANSWERING_HEIGHT, PROMPTING_HEIGHT, WINDOW_WIDTH};
use super::windows::{configure_window_for_active_space, show_overlay};
use super::{App, Input};

impl App {
    pub(super) fn update(&mut self, input: Input) -> Task<Input> {
        match input {
            Input::WindowOpened(id) if id == self.overlay.id => {
                configure_window_for_active_space(id)
            }
            #[cfg(target_os = "macos")]
            Input::WindowOpened(id) if id == self.main_window.id => {
                // AppKit installs its Apple event handlers during launch.
                Task::run(ruru_platform::install_reopen_handler(), |_| {
                    Input::ShowMainWindow
                })
            }
            Input::WindowOpened(_) => Task::none(),
            Input::WindowCloseRequested(id) if id == self.main_window.id => {
                self.update(Input::MainWindow(main_window::Input::DismissRequested))
            }
            Input::WindowCloseRequested(id) if id == self.overlay.id => {
                self.update(Input::Overlay(overlay::Input::DismissRequested))
            }
            Input::WindowCloseRequested(_) => Task::none(),
            #[cfg(target_os = "macos")]
            Input::ShowMainWindow => {
                let was_visible = self.main_window.ui.is_visible();
                let _ = self.main_window.ui.update(main_window::Input::Show);
                if was_visible {
                    iced_window::minimize(self.main_window.id, false)
                        .chain(iced_window::gain_focus(self.main_window.id))
                } else {
                    iced_window::set_mode(self.main_window.id, iced_window::Mode::Windowed)
                        .chain(iced_window::gain_focus(self.main_window.id))
                }
            }
            Input::DragWindow => iced_window::drag(self.overlay.id),
            Input::MainWindow(input) => match self.main_window.ui.update(input) {
                Some(output) => self.handle_main_window_output(output),
                None => Task::none(),
            },
            Input::Core(output) => match output {
                CoreOutput::ShowRequested { focused_window } => {
                    self.advance_provider_request_id();
                    let _ = self.overlay.ui.update(overlay::Input::Show);
                    show_overlay(self.overlay.id, focused_window)
                }
                CoreOutput::CaptureCompleted => {
                    let _ = self.overlay.ui.update(overlay::Input::CaptureCompleted);
                    Task::none()
                }
                CoreOutput::TargetCaptured(target) => {
                    let _ = self.overlay.ui.update(overlay::Input::SetTarget(target));
                    Task::none()
                }
                CoreOutput::RegionSelectionFinished(result) => {
                    self.selecting_region = false;
                    match result {
                        Ok(Some(image)) => {
                            let _ = self
                                .overlay
                                .ui
                                .update(overlay::Input::SetTarget(Target::Image(image)));
                        }
                        Ok(None) => {}
                        Err(error) => {
                            eprintln!("region selection failed: {error}");
                            let _ = self.overlay.ui.update(overlay::Input::CaptureFailed);
                        }
                    }
                    iced_window::set_mode(self.overlay.id, iced_window::Mode::Windowed)
                        .chain(iced_window::gain_focus(self.overlay.id))
                }
                CoreOutput::AnswerChunk { request_id, chunk } => {
                    if self.current_provider_request_id == request_id {
                        let _ = self.overlay.ui.update(overlay::Input::AppendAnswer(chunk));
                    }
                    Task::none()
                }
                CoreOutput::AnswerCompleted { request_id } => {
                    if self.current_provider_request_id == request_id {
                        self.advance_provider_request_id();
                        let _ = self.overlay.ui.update(overlay::Input::FinishAnswer);
                    }
                    Task::none()
                }
                CoreOutput::ProviderRequestFailed { request_id, error } => {
                    if self.current_provider_request_id == request_id {
                        self.advance_provider_request_id();
                        eprintln!("provider request failed: {error}");
                        let _ = self.overlay.ui.update(overlay::Input::FailAnswer(error));
                    }
                    Task::none()
                }
                CoreOutput::RequestFailed(error) => {
                    eprintln!("core request failed: {error}");
                    let _ = self.overlay.ui.update(overlay::Input::FailAnswer(error));
                    Task::none()
                }
                CoreOutput::SettingsUpdated(update) => {
                    self.advance_provider_request_id();
                    let _ = self
                        .overlay
                        .ui
                        .update(overlay::Input::ProviderRequestAbortRequested);
                    match update.change {
                        SettingsChange::KeySaved => self.apply_settings_input(
                            main_window::SettingsInput::KeySaved(update.provider.clone()),
                        ),
                        SettingsChange::KeyRemoved => self.apply_settings_input(
                            main_window::SettingsInput::KeyRemoved(update.provider.clone()),
                        ),
                        SettingsChange::ModelsChanged => {}
                    }
                    self.apply_settings_input(main_window::SettingsInput::SelectedModelsChanged(
                        update.provider,
                        update.models,
                    ));
                    self.apply_settings_input(main_window::SettingsInput::ClearError);
                    self.overlay
                        .ui
                        .update(overlay::Input::ModelsChanged(update.available_models));
                    Task::none()
                }
                CoreOutput::ModelsDiscovered { provider, result } => {
                    match result {
                        Ok(models) => self.apply_settings_input(
                            main_window::SettingsInput::DiscoveryFinished(provider, models),
                        ),
                        Err(error) => self.apply_settings_input(
                            main_window::SettingsInput::ProviderFailed(provider, error),
                        ),
                    }
                    Task::none()
                }
                CoreOutput::SettingsFailed { provider, error } => {
                    if let Some(provider) = provider {
                        self.apply_settings_input(main_window::SettingsInput::ProviderFailed(
                            provider, error,
                        ));
                    } else {
                        self.apply_settings_input(main_window::SettingsInput::Failed(error));
                    }
                    Task::none()
                }
            },
            Input::Overlay(input) => match self.overlay.ui.update(input) {
                Some(OverlayOutput::Submitted { prompt, model }) => {
                    let request_id = self.advance_provider_request_id();
                    if let Err(error) = self.to_core.unbounded_send(CoreInput::Submit {
                        request_id,
                        prompt,
                        model,
                    }) {
                        self.advance_provider_request_id();
                        let _ = self.overlay.ui.update(overlay::Input::FailAnswer(format!(
                            "failed to start provider request: {error}"
                        )));
                        return Task::none();
                    }
                    iced_window::resize(
                        self.overlay.id,
                        iced::Size::new(WINDOW_WIDTH, ANSWERING_HEIGHT),
                    )
                }
                Some(OverlayOutput::ModelSelected(model)) => {
                    if let Err(error) = self.to_core.unbounded_send(CoreInput::SelectModel(model)) {
                        eprintln!("failed to select model: {error}");
                    }
                    Task::none()
                }
                Some(OverlayOutput::TargetRemoved) => {
                    if let Err(error) = self.to_core.unbounded_send(CoreInput::RemoveTarget) {
                        eprintln!("failed to remove target from core: {error}");
                    }
                    Task::none()
                }
                Some(OverlayOutput::CaptureRegionRequested) => {
                    if self.selecting_region {
                        return Task::none();
                    }
                    self.selecting_region = true;
                    iced_window::set_mode(self.overlay.id, iced_window::Mode::Hidden)
                        .chain(Task::done(Input::BeginRegionSelection))
                }
                Some(OverlayOutput::ProviderRequestAbortRequested) => {
                    self.abort_provider_request();
                    Task::none()
                }
                Some(OverlayOutput::ProviderRequestRetryRequested) => {
                    let request_id = self.advance_provider_request_id();
                    if let Err(error) = self
                        .to_core
                        .unbounded_send(CoreInput::RetryProviderRequest { request_id })
                    {
                        self.advance_provider_request_id();
                        eprintln!("failed to retry provider request: {error}");
                        let _ = self.overlay.ui.update(overlay::Input::FailAnswer(format!(
                            "failed to retry provider request: {error}"
                        )));
                    }
                    Task::none()
                }
                Some(OverlayOutput::CopyAnswerRequested(answer)) => iced::clipboard::write(answer),
                Some(OverlayOutput::LinkClicked(uri)) => {
                    println!("link clicked: {uri}");
                    Task::none()
                }
                Some(OverlayOutput::ModeChanged(mode)) => match mode {
                    OverlayMode::Prompting => {
                        self.abort_provider_request();
                        iced_window::resize(
                            self.overlay.id,
                            iced::Size::new(WINDOW_WIDTH, PROMPTING_HEIGHT),
                        )
                    }
                    OverlayMode::Answering => iced_window::resize(
                        self.overlay.id,
                        iced::Size::new(WINDOW_WIDTH, ANSWERING_HEIGHT),
                    ),
                },
                Some(OverlayOutput::Dismissed) => {
                    self.abort_provider_request();
                    iced_window::set_mode(self.overlay.id, iced_window::Mode::Hidden)
                }
                None => Task::none(),
            },
            Input::CheckPromptInputFocus(id) => {
                if id == self.overlay.id
                    && self.overlay.ui.is_visible()
                    && self.overlay.ui.mode() == OverlayMode::Prompting
                {
                    self.overlay
                        .ui
                        .check_prompt_input_focus()
                        .map(Input::Overlay)
                } else {
                    Task::none()
                }
            }
            Input::BeginRegionSelection => {
                if let Err(error) = self.to_core.unbounded_send(CoreInput::SelectRegion) {
                    self.selecting_region = false;
                    eprintln!("region selection failed to start: {error}");
                    let _ = self.overlay.ui.update(overlay::Input::CaptureFailed);
                    iced_window::set_mode(self.overlay.id, iced_window::Mode::Windowed)
                        .chain(iced_window::gain_focus(self.overlay.id))
                } else {
                    Task::none()
                }
            }
        }
    }

    fn abort_provider_request(&mut self) {
        self.advance_provider_request_id();
        if let Err(error) = self.to_core.unbounded_send(CoreInput::AbortProviderRequest) {
            eprintln!("failed to abort provider request: {error}");
        }
    }

    fn advance_provider_request_id(&mut self) -> ruru_core::ProviderRequestId {
        self.current_provider_request_id = self.current_provider_request_id.wrapping_add(1);
        self.current_provider_request_id
    }

    fn handle_main_window_output(&mut self, output: main_window::Output) -> Task<Input> {
        match output {
            main_window::Output::Dismissed => {
                iced_window::set_mode(self.main_window.id, iced_window::Mode::Hidden)
            }
            main_window::Output::JumpToSection(section) => {
                self.main_window.ui.jump_to_section(section)
            }
            main_window::Output::DiscoverModels(id) => {
                self.apply_settings_input(main_window::SettingsInput::DiscoveryStarted(id.clone()));
                self.send_settings_command(&id, CoreInput::DiscoverModels(id.clone()))
            }
            main_window::Output::SaveKey(id, key) => {
                self.apply_settings_input(main_window::SettingsInput::DiscoveryStarted(id.clone()));
                self.send_settings_command(&id, CoreInput::SaveKey(id.clone(), key))
            }
            main_window::Output::RemoveKey(id) => {
                self.send_settings_command(&id, CoreInput::RemoveKey(id.clone()))
            }
            main_window::Output::SetModel {
                provider,
                model,
                enabled,
            } => self.send_settings_command(
                &provider,
                CoreInput::SetModel {
                    provider: provider.clone(),
                    model,
                    enabled,
                },
            ),
        }
    }

    fn send_settings_command(&mut self, provider: &ProviderId, input: CoreInput) -> Task<Input> {
        if let Err(error) = self.to_core.unbounded_send(input) {
            self.apply_settings_input(main_window::SettingsInput::ProviderFailed(
                provider.clone(),
                error.to_string(),
            ));
        }
        Task::none()
    }

    fn apply_settings_input(&mut self, input: main_window::SettingsInput) {
        self.main_window
            .ui
            .update(main_window::Input::Settings(input));
    }
}
