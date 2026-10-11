use super::super::latex;
use super::{Answering, Input};

impl Answering {
    pub fn update(&mut self, input: Input) {
        match input {
            Input::AppendToken(token) => self.push_token(&token),
            Input::Finished => self.finish(),
            Input::Failed(error) => self.fail(error),
            Input::Reset => self.reset(),
            Input::LoadingTick => self.loading_frame = self.loading_frame.wrapping_add(1) % 24,
            Input::LinkClicked(_) => {}
        }
    }
    fn push_token(&mut self, token: &str) {
        self.answer.push_str(token);
        self.markdown = latex::parse(&self.answer);
    }
    fn finish(&mut self) {
        self.done = true;
    }
    fn fail(&mut self, error: String) {
        self.error = Some(error);
    }
    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::markdown;

    impl Answering {
        pub fn is_done(&self) -> bool {
            self.done
        }
    }

    #[test]
    fn streamed_tokens_update_the_source_and_markdown() {
        let mut answering = Answering::default();

        answering.push_token("# Head");
        answering.push_token("ing\n\nBody");

        assert_eq!(answering.answer(), "# Heading\n\nBody");
        assert!(matches!(
            answering.markdown.items().first(),
            Some(markdown::Item::Heading(..))
        ));
        assert!(matches!(
            answering.markdown.items().get(1),
            Some(markdown::Item::Paragraph(..))
        ));
    }

    #[test]
    fn reset_clears_the_previous_response() {
        let mut answering = Answering::default();
        answering.push_token("old answer");
        answering.finish();
        answering.fail("old error".into());

        answering.reset();

        assert!(answering.answer().is_empty());
        assert!(answering.markdown.items().is_empty());
        assert!(!answering.is_done());
        assert!(answering.error().is_none());
        assert!(answering.is_waiting());
    }

    #[test]
    fn waiting_ends_when_the_first_token_arrives() {
        let mut answering = Answering::default();
        assert!(answering.is_waiting());

        answering.push_token("first");

        assert!(!answering.is_waiting());
    }
}
