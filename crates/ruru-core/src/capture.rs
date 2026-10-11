use ruru_types::{ImageCapture, Target, WindowBounds};
use std::time::Instant;

use crate::Capture;
use crate::state::{Core, State};

pub(crate) struct CaptureOutcome {
    pub(crate) failure: Option<String>,
}

impl Core {
    pub(crate) fn capture(
        &self,
        on_capture_started: impl FnOnce(Option<WindowBounds>, Option<Target>),
    ) -> CaptureOutcome {
        let capture_id = self
            .state
            .lock()
            .expect("core state mutex poisoned")
            .start_capture();
        let started = Instant::now();
        let focused_window = self.capturer.capture_window_bounds().ok();
        let provenance = self.capturer.capture_provenance().ok();
        let (target, failure) = match self.capturer.capture_text() {
            Ok(capture) => (Some(Target::Text(capture)), None),
            Err(error) => (None, Some(format!("failed to capture text: {error:?}"))),
        };
        let pending_context = self.capturer.begin_context_capture().ok();
        on_capture_started(focused_window, target.clone());
        let context = pending_context.and_then(|context| context.complete().ok());
        let elapsed_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
        let capture = Capture {
            target: target.clone(),
            context,
            provenance,
            elapsed_ms,
        };

        self.state
            .lock()
            .expect("core state mutex poisoned")
            .commit_capture(capture_id, capture);

        CaptureOutcome { failure }
    }
    pub(super) fn select_region(&self) -> Result<Option<ImageCapture>, String> {
        let capture_id = self
            .state
            .lock()
            .expect("core state mutex poisoned")
            .capture_id;
        let result = self
            .capturer
            .select_region()
            .map_err(|error| error.to_string())?;

        let Some(image) = result else {
            return Ok(None);
        };
        let applied = self
            .state
            .lock()
            .expect("core state mutex poisoned")
            .apply_region(capture_id, image.clone());

        Ok(applied.then_some(image))
    }
    pub(super) fn remove_target(&self) {
        self.state
            .lock()
            .expect("core state mutex poisoned")
            .remove_target();
    }
}

impl State {
    pub(crate) fn start_capture(&mut self) -> u64 {
        self.forget_provider_request();
        self.capture_id = self.capture_id.wrapping_add(1);
        self.current_capture = None;
        self.capture_id
    }
    pub(crate) fn commit_capture(&mut self, capture_id: u64, capture: Capture) {
        if self.capture_id == capture_id {
            self.current_capture = Some(capture);
        }
    }
    pub(crate) fn apply_region(&mut self, capture_id: u64, image: ImageCapture) -> bool {
        if self.capture_id != capture_id {
            return false;
        }

        let Some(capture) = &mut self.current_capture else {
            return false;
        };

        capture.target = Some(Target::Image(image));
        true
    }
    pub(crate) fn remove_target(&mut self) {
        if let Some(capture) = &mut self.current_capture {
            capture.target = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Output;
    use crate::tests::{StubCapturer, context, image, test_core, test_providers};
    use ruru_types::{ContextCapture, TextCapture, TextCaptureMethod};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn capture_orchestrates_and_commits_capture_data() {
        let core = test_core(test_providers(), Arc::new(StubCapturer::default()));

        let mut initial_bounds = None;
        let mut initial_target = None;
        let outcome = core.capture(|bounds, target| {
            initial_bounds = bounds;
            initial_target = target;
        });

        assert!(outcome.failure.is_none());
        assert!(matches!(
            initial_target,
            Some(Target::Text(TextCapture { ref text, .. })) if text == "selected text"
        ));
        assert_eq!(
            initial_bounds,
            Some(WindowBounds {
                x: 100.0,
                y: 200.0,
                w: 1200.0,
                h: 800.0,
            })
        );

        let state = core.state.lock().expect("core state mutex poisoned");
        let capture = state.current_capture().expect("capture committed");
        assert!(matches!(
            capture.context,
            Some(ContextCapture {
                image: ImageCapture { ref png, .. },
                ..
            }) if png == b"context"
        ));
        assert_eq!(
            capture
                .provenance
                .as_ref()
                .map(|value| value.app_name.as_str()),
            Some("Example")
        );
    }

    #[test]
    fn requests_the_overlay_before_completing_context_capture() {
        let context_completed = Arc::new(AtomicBool::new(false));
        let core = test_core(
            test_providers(),
            Arc::new(StubCapturer {
                context_completed: Some(Arc::clone(&context_completed)),
            }),
        );

        core.capture(|_, _| assert!(!context_completed.load(Ordering::SeqCst)));

        assert!(context_completed.load(Ordering::SeqCst));
    }

    #[test]
    fn region_selection_replaces_the_committed_target() {
        let core = test_core(test_providers(), Arc::new(StubCapturer::default()));
        core.capture(|_, _| {});

        let selected = core.select_region().expect("region capture succeeds");

        assert!(selected.is_some());
        let state = core.state.lock().expect("core state mutex poisoned");
        assert!(matches!(
            state
                .current_capture()
                .and_then(|capture| capture.target.as_ref()),
            Some(Target::Image(_))
        ));
    }

    #[test]
    fn beginning_a_capture_discards_the_previous_capture() {
        let mut state = State::default();
        let first_id = state.start_capture();
        state.commit_capture(
            first_id,
            Capture {
                target: Some(Target::Image(image())),
                context: None,
                provenance: None,
                elapsed_ms: 0,
            },
        );

        state.start_capture();

        assert!(state.current_capture.is_none());
    }

    #[test]
    fn stale_region_capture_does_not_replace_the_current_target() {
        let mut state = State::default();
        let stale_id = state.start_capture();
        let current_id = state.start_capture();
        state.commit_capture(
            current_id,
            Capture {
                target: Some(Target::Text(TextCapture {
                    text: "current target".into(),
                    method: TextCaptureMethod::Accessibility,
                })),
                context: None,
                provenance: None,
                elapsed_ms: 0,
            },
        );

        assert!(!state.apply_region(stale_id, image()));
        assert!(matches!(
            state.current_capture.and_then(|capture| capture.target),
            Some(Target::Text(TextCapture { ref text, .. })) if text == "current target"
        ));
    }

    #[test]
    fn region_capture_replaces_text_target_and_preserves_context() {
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

        assert!(state.apply_region(capture_id, image()));
        let (request, _, _) = state.start_provider_request(None, None);

        assert!(matches!(request.target, Some(Target::Image(_))));
        assert!(matches!(
            request.context,
            Some(ContextCapture { image: ImageCapture { ref png, .. }, .. }) if png == b"png"
        ));
    }

    #[test]
    fn beginning_a_capture_aborts_the_active_provider_request() {
        let mut state = State::default();
        let (_, request_id, registration) = state.start_provider_request(None, None);
        let abort = registration.handle();

        state.start_capture();

        assert!(abort.is_aborted());
        assert!(!state.accept_provider_output(
            request_id,
            &Output::AnswerChunk {
                request_id: 1,
                chunk: "stale".into(),
            }
        ));
    }
}
