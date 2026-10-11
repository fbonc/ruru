use futures_util::future::{AbortHandle, AbortRegistration, Abortable, ready};
use futures_util::stream::BoxStream;
use futures_util::{StreamExt, stream};
use ruru_provider::{Provider, ProviderError, ProviderOutput, ProviderRequest, ProviderStream};
use ruru_types::ModelSelection;
use std::sync::Arc;

use crate::state::{Core, State};
use crate::{Output, ProviderRequestId};

enum ProviderResponseState {
    Streaming(ProviderStream),
    Finished,
}
impl Core {
    pub(super) fn start_provider_stream(
        self: &Arc<Self>,
        output_request_id: ProviderRequestId,
        prompt: Option<String>,
        model: ModelSelection,
    ) -> BoxStream<'static, Output> {
        let provider = self.settings.providers.resolve(&model);
        let (request, request_id, abort_registration) = self
            .state
            .lock()
            .expect("core state mutex poisoned")
            .start_provider_request(prompt, provider.clone());
        let provider_stream = match provider {
            Some(provider) => provider.stream(&model.model, request),
            None => stream::once(async move {
                Err(ProviderError::new(format!(
                    "provider `{}` is not configured",
                    model.provider
                )))
            })
            .boxed(),
        };
        self.provider_outputs(
            provider_stream,
            request_id,
            output_request_id,
            abort_registration,
        )
    }
    pub(super) fn retry_provider_stream(
        self: &Arc<Self>,
        output_request_id: ProviderRequestId,
    ) -> BoxStream<'static, Output> {
        let (provider, request_id, abort_registration) = self
            .state
            .lock()
            .expect("core state mutex poisoned")
            .start_provider_retry();
        let provider_stream = provider
            .and_then(|provider| provider.retry_request())
            .unwrap_or_else(|| {
                stream::once(async {
                    Err(ProviderError::new(
                        "no provider request is available to retry",
                    ))
                })
                .boxed()
            });

        self.provider_outputs(
            provider_stream,
            request_id,
            output_request_id,
            abort_registration,
        )
    }
    pub(super) fn provider_outputs(
        self: &Arc<Self>,
        provider_stream: ProviderStream,
        request_id: u64,
        output_request_id: ProviderRequestId,
        abort_registration: AbortRegistration,
    ) -> BoxStream<'static, Output> {
        let outputs = stream::unfold(
            ProviderResponseState::Streaming(provider_stream),
            move |response| async move {
                match response {
                    ProviderResponseState::Streaming(mut stream) => match stream.next().await {
                        Some(Ok(ProviderOutput::TextDelta(chunk))) => Some((
                            Output::AnswerChunk {
                                request_id: output_request_id,
                                chunk,
                            },
                            ProviderResponseState::Streaming(stream),
                        )),
                        Some(Err(error)) => Some((
                            Output::ProviderRequestFailed {
                                request_id: output_request_id,
                                error: error.to_string(),
                            },
                            ProviderResponseState::Finished,
                        )),
                        None => Some((
                            Output::AnswerCompleted {
                                request_id: output_request_id,
                            },
                            ProviderResponseState::Finished,
                        )),
                    },
                    ProviderResponseState::Finished => None,
                }
            },
        );
        let core = Arc::clone(self);

        Abortable::new(outputs, abort_registration)
            .filter_map(move |output| {
                let accepted = core.accept_provider_output(request_id, &output);
                ready(accepted.then_some(output))
            })
            .boxed()
    }
    pub(super) fn abort_provider_request(&self) {
        self.state
            .lock()
            .expect("core state mutex poisoned")
            .abort_provider_request();
    }
    pub(crate) fn forget_provider_request(&self) {
        self.state
            .lock()
            .expect("core state mutex poisoned")
            .forget_provider_request();
    }
    pub(super) fn accept_provider_output(&self, request_id: u64, output: &Output) -> bool {
        self.state
            .lock()
            .expect("core state mutex poisoned")
            .accept_provider_output(request_id, output)
    }
}

impl State {
    pub(crate) fn start_provider_request(
        &mut self,
        prompt: Option<String>,
        provider: Option<Arc<dyn Provider>>,
    ) -> (ProviderRequest, u64, AbortRegistration) {
        self.abort_provider_request();
        let request = ProviderRequest {
            prompt,
            target: self
                .current_capture
                .as_ref()
                .and_then(|capture| capture.target.clone()),
            context: self
                .current_capture
                .as_ref()
                .and_then(|capture| capture.context.clone()),
        };
        let (abort, registration) = AbortHandle::new_pair();
        self.provider_abort = Some(abort);
        self.provider = provider;
        (request, self.provider_request_id, registration)
    }
    pub(crate) fn start_provider_retry(
        &mut self,
    ) -> (Option<Arc<dyn Provider>>, u64, AbortRegistration) {
        self.abort_provider_request();
        let (abort, registration) = AbortHandle::new_pair();
        self.provider_abort = Some(abort);
        (
            self.provider.clone(),
            self.provider_request_id,
            registration,
        )
    }
    pub(crate) fn abort_provider_request(&mut self) {
        self.provider_request_id = self.provider_request_id.wrapping_add(1);
        let request_was_active = if let Some(abort) = self.provider_abort.take() {
            abort.abort();
            true
        } else {
            false
        };
        if request_was_active && let Some(provider) = &self.provider {
            provider.abort_request();
        }
    }
    pub(crate) fn forget_provider_request(&mut self) {
        self.abort_provider_request();
        self.provider = None;
    }
    pub(crate) fn accept_provider_output(&mut self, request_id: u64, output: &Output) -> bool {
        if self.provider_request_id != request_id || self.provider_abort.is_none() {
            return false;
        }

        if matches!(
            output,
            Output::AnswerCompleted { .. } | Output::ProviderRequestFailed { .. }
        ) {
            self.provider_abort = None;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Capture;
    use crate::providers::Providers;
    use crate::tests::{RecordingProvider, StubCapturer, context, test_core, test_providers};
    use ruru_provider::MockProvider;
    use ruru_types::{
        ContextCapture, ImageCapture, Provenance, ProviderId, Target, TextCapture,
        TextCaptureMethod,
    };
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    #[test]
    fn routes_each_request_using_its_model_selection() {
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
            ModelSelection::new(provider_id, "second"),
        );

        assert_eq!(
            *provider
                .requested_models
                .lock()
                .expect("recorded models mutex poisoned"),
            vec!["second"]
        );
    }

    #[test]
    fn provider_outputs_keep_the_app_request_id() {
        let core = Arc::new(test_core(
            test_providers(),
            Arc::new(StubCapturer::default()),
        ));
        let mut outputs = core.start_provider_stream(
            42,
            None,
            ModelSelection::new(ProviderId::new("mock"), "mock"),
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("test runtime should start");

        let output = runtime
            .block_on(outputs.next())
            .expect("provider should produce output");

        assert!(matches!(output, Output::AnswerChunk { request_id: 42, .. }));
    }

    #[test]
    fn provider_request_uses_prompt_target_and_context() {
        let target = Target::Text(TextCapture {
            text: "selected text".into(),
            method: TextCaptureMethod::Accessibility,
        });
        let context = context();
        let mut state = State::default();
        let capture_id = state.start_capture();
        state.commit_capture(
            capture_id,
            Capture {
                target: Some(target),
                context: Some(context),
                provenance: Some(Provenance {
                    app_name: "Example".into(),
                    window_title: "Document".into(),
                    path: Some("/tmp/example".into()),
                }),
                elapsed_ms: 12,
            },
        );

        let (request, _, _) = state.start_provider_request(Some("explain this".into()), None);

        assert_eq!(request.prompt.as_deref(), Some("explain this"));
        assert!(matches!(
            request.target,
            Some(Target::Text(TextCapture { ref text, .. })) if text == "selected text"
        ));
        assert!(matches!(
            request.context,
            Some(ContextCapture { image: ImageCapture { ref png, .. }, .. }) if png == b"png"
        ));
    }

    #[test]
    fn removing_target_preserves_context_for_provider_request() {
        let context = context();
        let mut state = State::default();
        let capture_id = state.start_capture();
        state.commit_capture(
            capture_id,
            Capture {
                target: Some(Target::Text(TextCapture {
                    text: "selected text".into(),
                    method: TextCaptureMethod::Accessibility,
                })),
                context: Some(context),
                provenance: None,
                elapsed_ms: 0,
            },
        );

        state.remove_target();
        let (request, _, _) =
            state.start_provider_request(Some("explain the context".into()), None);

        assert!(request.target.is_none());
        assert_eq!(request.prompt.as_deref(), Some("explain the context"));
        assert!(matches!(
            request.context,
            Some(ContextCapture { image: ImageCapture { ref png, .. }, .. }) if png == b"png"
        ));
    }

    #[test]
    fn newer_provider_request_aborts_the_previous_request() {
        let mut state = State::default();
        let (_, first_id, first_registration) = state.start_provider_request(None, None);
        let first_abort = first_registration.handle();

        let (_, second_id, _) = state.start_provider_request(None, None);

        assert!(first_abort.is_aborted());
        assert_ne!(first_id, second_id);
        assert!(!state.accept_provider_output(
            first_id,
            &Output::AnswerChunk {
                request_id: 1,
                chunk: "stale".into(),
            }
        ));
        assert!(state.accept_provider_output(
            second_id,
            &Output::AnswerChunk {
                request_id: 2,
                chunk: "current".into(),
            }
        ));
    }

    #[test]
    fn forgetting_a_provider_request_prevents_retry() {
        let mut state = State::default();
        state.start_provider_request(None, Some(Arc::new(MockProvider::new(Duration::ZERO))));

        state.forget_provider_request();

        let (provider, _, _) = state.start_provider_retry();
        assert!(provider.is_none());
    }

    #[test]
    fn terminal_provider_output_closes_the_active_request() {
        let mut state = State::default();
        let (_, request_id, _) = state.start_provider_request(None, None);

        assert!(
            state.accept_provider_output(request_id, &Output::AnswerCompleted { request_id: 1 })
        );
        assert!(!state.accept_provider_output(
            request_id,
            &Output::AnswerChunk {
                request_id: 1,
                chunk: "late".into(),
            }
        ));
    }
}
