use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use ruru_types::ProviderId;
use std::sync::Arc;

use crate::state::Core;
use crate::{Input, Output};

impl Core {
    pub(crate) fn handle_input(self: &Arc<Self>, input: Input) -> BoxStream<'static, Output> {
        match input {
            Input::Submit {
                request_id,
                prompt,
                model,
            } => {
                let Some(model) = model
                    .filter(|model| self.settings.providers.configured_models().contains(model))
                else {
                    self.forget_provider_request();
                    return stream::once(async move {
                        Output::ProviderRequestFailed {
                            request_id,
                            error: "no available model selected".into(),
                        }
                    })
                    .boxed();
                };
                self.start_provider_stream(request_id, prompt, model)
            }
            Input::AbortProviderRequest => {
                self.abort_provider_request();
                stream::empty().boxed()
            }
            Input::RetryProviderRequest { request_id } => self.retry_provider_stream(request_id),
            Input::RemoveTarget => {
                self.remove_target();
                stream::empty().boxed()
            }
            Input::SelectRegion => {
                let core = Arc::clone(self);
                stream::once(async move {
                    let result = tokio::task::spawn_blocking(move || core.select_region())
                        .await
                        .map_err(|error| format!("region selection task failed: {error}"))
                        .and_then(|result| result);

                    Output::RegionSelectionFinished(result)
                })
                .boxed()
            }
            Input::DiscoverModels(id) => self.discover_models(id),
            Input::SaveKey(id, key) => match self.settings.save_key(id.clone(), key) {
                Ok(update) => {
                    self.forget_provider_request();
                    stream::iter([Output::SettingsUpdated(update)])
                        .chain(self.discover_models(id))
                        .boxed()
                }
                Err(error) => settings_error(Some(id), error),
            },
            Input::RemoveKey(id) => match self.settings.remove_key(id.clone()) {
                Ok(update) => {
                    self.forget_provider_request();
                    stream::iter([Output::SettingsUpdated(update)]).boxed()
                }
                Err(error) => settings_error(Some(id), error),
            },
            Input::SetModel {
                provider,
                model,
                enabled,
            } => match self.settings.set_model(provider.clone(), model, enabled) {
                Ok(update) => {
                    self.forget_provider_request();
                    stream::iter([Output::SettingsUpdated(update)]).boxed()
                }
                Err(error) => settings_error(Some(provider), error),
            },
            Input::SelectModel(model) => match self.settings.select_model(model) {
                Ok(()) => stream::empty().boxed(),
                Err(error) => settings_error(None, error),
            },
        }
    }

    fn discover_models(self: &Arc<Self>, id: ProviderId) -> BoxStream<'static, Output> {
        let core = Arc::clone(self);
        self.settings
            .discover(id)
            .map(move |output| {
                if matches!(&output, Output::SettingsUpdated(_)) {
                    core.forget_provider_request();
                }
                output
            })
            .boxed()
    }
}

fn settings_error(
    provider: Option<ProviderId>,
    error: String,
) -> futures_util::stream::BoxStream<'static, Output> {
    stream::iter([Output::SettingsFailed { provider, error }]).boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Providers;
    use crate::tests::{RecordingProvider, StubCapturer, test_core};
    use ruru_provider::MockProvider;
    use ruru_types::ModelSelection;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn core_with_request() -> Arc<Core> {
        let providers = Providers::new();
        let id = ProviderId::new("openai");
        providers.insert(id.clone(), Arc::new(MockProvider::default()));
        let core = Arc::new(test_core(
            providers,
            Arc::from(ruru_capture::new_capturer()),
        ));
        drop(core.start_provider_stream(1, None, ModelSelection::new(id, "mock")));
        core
    }

    #[test]
    fn missing_model_failure_keeps_the_app_request_id() {
        let providers = Providers::new();
        let provider = Arc::new(RecordingProvider {
            requested_models: Mutex::new(Vec::new()),
            aborted: AtomicBool::new(false),
            retried: AtomicBool::new(false),
        });
        let provider_id = ProviderId::new("recording");
        providers.insert(provider_id.clone(), provider.clone());
        let core = Arc::new(test_core(providers, Arc::new(StubCapturer::default())));
        let _previous =
            core.start_provider_stream(41, None, ModelSelection::new(provider_id, "first"));
        let mut outputs = core.handle_input(Input::Submit {
            request_id: 42,
            prompt: None,
            model: None,
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("test runtime should start");

        let output = runtime
            .block_on(outputs.next())
            .expect("missing model should produce an error");

        assert!(matches!(
            output,
            Output::ProviderRequestFailed { request_id: 42, .. }
        ));
        assert!(provider.aborted.load(Ordering::SeqCst));
    }

    #[test]
    fn aborting_stops_the_active_provider_request() {
        let providers = Providers::new();
        let provider = Arc::new(RecordingProvider {
            requested_models: Mutex::new(Vec::new()),
            aborted: AtomicBool::new(false),
            retried: AtomicBool::new(false),
        });
        let provider_id = ProviderId::new("recording");
        providers.insert(provider_id.clone(), provider.clone());
        let core = Arc::new(test_core(providers, Arc::new(StubCapturer::default())));
        let _outputs = core.start_provider_stream(
            1,
            Some("explain".into()),
            ModelSelection::new(provider_id, "first"),
        );

        let _outputs = core.handle_input(Input::AbortProviderRequest);

        assert!(provider.aborted.load(Ordering::SeqCst));
    }

    #[test]
    fn retrying_uses_the_previous_provider_request() {
        let providers = Providers::new();
        let provider = Arc::new(RecordingProvider {
            requested_models: Mutex::new(Vec::new()),
            aborted: AtomicBool::new(false),
            retried: AtomicBool::new(false),
        });
        let provider_id = ProviderId::new("recording");
        providers.insert(provider_id.clone(), provider.clone());
        let core = Arc::new(test_core(providers, Arc::new(StubCapturer::default())));
        let _outputs = core.start_provider_stream(
            1,
            Some("explain".into()),
            ModelSelection::new(provider_id, "first"),
        );
        core.abort_provider_request();

        let _outputs = core.handle_input(Input::RetryProviderRequest { request_id: 2 });

        assert!(provider.retried.load(Ordering::SeqCst));
    }

    #[test]
    fn removing_a_key_cancels_the_request_before_outputs_are_polled() {
        let core = core_with_request();
        let mut outputs = core.handle_input(Input::RemoveKey(ProviderId::new("openai")));

        {
            let state = core.state.lock().unwrap();
            assert!(state.provider.is_none());
            assert!(state.provider_abort.is_none());
        }
        assert!(core.settings.providers.configured_models().is_empty());
        let output = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(outputs.next());
        assert!(matches!(output, Some(Output::SettingsUpdated(_))));
    }

    #[test]
    fn a_failed_settings_change_keeps_the_active_request() {
        let core = core_with_request();
        let id = ProviderId::new("unknown");
        let mut outputs = core.handle_input(Input::RemoveKey(id.clone()));

        assert!(core.state.lock().unwrap().provider_abort.is_some());
        let output = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(outputs.next());
        assert!(matches!(output,
            Some(Output::SettingsFailed { provider: Some(provider), .. }) if provider == id
        ));
        assert!(core.state.lock().unwrap().provider_abort.is_some());
    }
}
