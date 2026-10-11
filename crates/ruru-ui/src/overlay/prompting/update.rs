use super::random_placeholder;
use super::{Input, Prompting};

impl Prompting {
    pub fn update(&mut self, input: Input) {
        match input {
            Input::Reset => self.reset(),
            Input::InputChanged(value) => self.prompt_value = value,
            Input::InputFocusChanged(focused) => self.input_focused = focused,
            Input::CaptureRegionRequested => self.capture_failed = false,
            Input::CaptureFailed => self.capture_failed = true,
            Input::ModelSelected(_) => {}
            Input::SubmitRequested => {}
        }
    }
    fn reset(&mut self) {
        self.prompt_value.clear();
        self.capture_failed = false;
        self.placeholder = random_placeholder();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_updates_the_local_value() {
        let mut prompting = Prompting::new();

        prompting.update(Input::InputChanged("what is this?".into()));

        assert_eq!(prompting.prompt_value(), "what is this?");
    }

    #[test]
    fn submit_does_not_mutate_local_state() {
        let mut prompting = Prompting::new();
        prompting.update(Input::InputChanged("keep me".into()));

        prompting.update(Input::SubmitRequested);

        assert_eq!(prompting.prompt_value(), "keep me");
    }

    #[test]
    fn reset_clears_the_prompt_and_capture_failure() {
        let mut prompting = Prompting::new();
        prompting.update(Input::InputChanged("old prompt".into()));
        prompting.update(Input::CaptureFailed);

        prompting.reset();

        assert!(prompting.prompt_value().is_empty());
        assert!(!prompting.capture_failed);
    }
}
