use std::sync::{Arc, Mutex};

use futures_channel::mpsc;
use ruru_capture::Capturer;
use ruru_types::{ImageCapture, ModelSelection, ProviderId, Target, WindowBounds};

use crate::settings::Settings;
use crate::state::{Core, State};

mod capture;
mod handle_input;
mod hotkey;
mod providers;
mod requests;
mod runtime;
mod settings;
mod state;

pub use runtime::start;
pub use state::Capture;

pub type ProviderRequestId = u64;
pub use settings::{ProviderSettingsInfo, SettingsChange, SettingsSnapshot, SettingsUpdate};

pub type Sender = mpsc::UnboundedSender<Input>;

#[derive(Clone)]
pub enum Input {
    Submit {
        request_id: ProviderRequestId,
        prompt: Option<String>,
        model: Option<ModelSelection>,
    },
    AbortProviderRequest,
    RetryProviderRequest {
        request_id: ProviderRequestId,
    },
    RemoveTarget,
    SelectRegion,
    DiscoverModels(ProviderId),
    SaveKey(ProviderId, String),
    RemoveKey(ProviderId),
    SetModel {
        provider: ProviderId,
        model: String,
        enabled: bool,
    },
    SelectModel(ModelSelection),
}

#[derive(Debug, Clone)]
pub enum Output {
    ShowRequested {
        focused_window: Option<WindowBounds>,
    },
    CaptureCompleted,
    TargetCaptured(Target),
    RegionSelectionFinished(Result<Option<ImageCapture>, String>),
    AnswerChunk {
        request_id: ProviderRequestId,
        chunk: String,
    },
    AnswerCompleted {
        request_id: ProviderRequestId,
    },
    ProviderRequestFailed {
        request_id: ProviderRequestId,
        error: String,
    },
    RequestFailed(String),
    SettingsUpdated(SettingsUpdate),
    ModelsDiscovered {
        provider: ProviderId,
        result: Result<Vec<String>, String>,
    },
    SettingsFailed {
        provider: Option<ProviderId>,
        error: String,
    },
}

impl Core {
    pub(crate) fn new(settings: Arc<Settings>, capturer: Arc<dyn Capturer>) -> Self {
        Self {
            state: Mutex::new(State::default()),
            settings,
            capturer,
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{StreamExt, stream};
    use ruru_capture::Capturer;
    use ruru_config::{AppConfig, CredentialStore};
    use ruru_provider::{MockProvider, Provider, ProviderError, ProviderRequest, ProviderStream};
    use ruru_types::{
        ContextCapture, ImageCapture, Provenance, ProviderId, TextCapture, TextCaptureMethod,
        WindowBounds,
    };
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::Capture;
    use crate::providers::Providers;
    use crate::settings::Settings;
    use crate::state::{Core, State};

    impl State {
        pub(crate) fn current_capture(&self) -> Option<&Capture> {
            self.current_capture.as_ref()
        }
    }

    struct EmptyCredentials;

    impl CredentialStore for EmptyCredentials {
        fn get(&self, _: &ProviderId) -> ruru_config::Result<Option<String>> {
            Ok(None)
        }
        fn set(&self, _: &ProviderId, _: &str) -> ruru_config::Result<()> {
            Ok(())
        }
        fn remove(&self, _: &ProviderId) -> ruru_config::Result<()> {
            Ok(())
        }
    }

    pub(crate) fn test_core(providers: Providers, capturer: Arc<dyn Capturer>) -> Core {
        let mut settings = Settings::new(
            Ok(AppConfig::default()),
            Arc::new(EmptyCredentials),
            Arc::new(|_| Ok(())),
        );
        settings.providers = providers;
        Core::new(Arc::new(settings), capturer)
    }

    #[derive(Default)]
    pub(super) struct StubCapturer {
        pub(super) context_completed: Option<Arc<AtomicBool>>,
    }

    pub(super) struct RecordingProvider {
        pub(super) requested_models: Mutex<Vec<String>>,
        pub(super) aborted: AtomicBool,
        pub(super) retried: AtomicBool,
    }

    impl Provider for RecordingProvider {
        fn configured_models(&self) -> Vec<String> {
            vec!["first".into(), "second".into()]
        }

        fn discover_models(
            &self,
        ) -> futures_util::future::BoxFuture<'_, Result<Vec<String>, ProviderError>> {
            Box::pin(async { Ok(self.configured_models()) })
        }

        fn stream(&self, model: &str, _request: ProviderRequest) -> ProviderStream {
            self.requested_models
                .lock()
                .expect("recorded models mutex poisoned")
                .push(model.to_owned());
            stream::empty().boxed()
        }

        fn abort_request(&self) {
            self.aborted.store(true, Ordering::SeqCst);
        }

        fn retry_request(&self) -> Option<ProviderStream> {
            self.retried.store(true, Ordering::SeqCst);
            Some(stream::empty().boxed())
        }
    }

    impl Capturer for StubCapturer {
        fn capture_text(&self) -> ruru_capture::Result<TextCapture> {
            Ok(TextCapture {
                text: "selected text".into(),
                method: TextCaptureMethod::Accessibility,
            })
        }

        fn begin_context_capture(
            &self,
        ) -> ruru_capture::Result<ruru_capture::PendingContextCapture> {
            let context_completed = self.context_completed.clone();
            Ok(ruru_capture::PendingContextCapture::new(move || {
                if let Some(context_completed) = context_completed {
                    context_completed.store(true, Ordering::SeqCst);
                }
                Ok(ContextCapture {
                    image: ImageCapture {
                        png: b"context".to_vec(),
                        region: None,
                    },
                    accessibility_text: Some("surrounding context".into()),
                })
            }))
        }

        fn select_region(&self) -> ruru_capture::Result<Option<ImageCapture>> {
            Ok(Some(ImageCapture {
                png: b"png".to_vec(),
                region: None,
            }))
        }

        fn capture_provenance(&self) -> ruru_capture::Result<Provenance> {
            Ok(Provenance {
                app_name: "Example".into(),
                window_title: "Document".into(),
                path: Some("/tmp/example".into()),
            })
        }

        fn capture_window_bounds(&self) -> ruru_capture::Result<WindowBounds> {
            Ok(WindowBounds {
                x: 100.0,
                y: 200.0,
                w: 1200.0,
                h: 800.0,
            })
        }
    }

    pub(super) fn test_providers() -> Providers {
        let providers = Providers::new();
        providers.insert(ProviderId::new("mock"), Arc::new(MockProvider::default()));
        providers
    }

    pub(super) fn image() -> ImageCapture {
        ImageCapture {
            png: b"png".to_vec(),
            region: None,
        }
    }

    pub(super) fn context() -> ContextCapture {
        ContextCapture {
            image: image(),
            accessibility_text: Some("surrounding context".into()),
        }
    }
}
