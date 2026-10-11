use ruru_types::ProviderId;

use super::super::Output;
use super::{Input, ProviderView, Settings};

impl Settings {
    fn provider_mut(&mut self, id: &ProviderId) -> Option<&mut ProviderView> {
        self.providers
            .iter_mut()
            .find(|provider| &provider.id == id)
    }
    pub fn update(&mut self, input: Input) -> Option<Output> {
        match input {
            Input::KeySaved(id) => {
                self.key_saved(&id);
                None
            }
            Input::KeyRemoved(id) => {
                self.key_removed(&id);
                None
            }
            Input::SelectedModelsChanged(id, models) => {
                self.set_selected_models(&id, models);
                None
            }
            Input::DiscoveryStarted(id) => {
                self.start_discovery(&id);
                None
            }
            Input::DiscoveryFinished(id, models) => {
                self.finish_discovery(&id, models);
                None
            }
            Input::ProviderFailed(id, error) => {
                self.set_error(&id, error);
                None
            }
            Input::Failed(error) => {
                self.set_global_error(error);
                None
            }
            Input::ClearError => {
                self.clear_error();
                None
            }
            Input::DiscardDraft => {
                self.discard_draft();
                None
            }
            Input::SectionSelected(section) => Some(Output::JumpToSection(section)),
            Input::ProviderSelected(id) => {
                if self
                    .providers
                    .iter()
                    .any(|provider| provider.id == id && !provider.credential_present)
                {
                    self.active_provider = Some(id);
                }
                None
            }
            Input::ProviderRowToggled(id) => {
                if self.active_provider.as_ref() == Some(&id) {
                    self.active_provider = None;
                } else if self
                    .providers
                    .iter()
                    .any(|provider| provider.id == id && provider.credential_present)
                {
                    self.active_provider = Some(id.clone());
                    return Some(Output::DiscoverModels(id));
                }
                None
            }
            Input::ApiKeyChanged(id, value) => {
                if let Some(provider) = self.provider_mut(&id) {
                    provider.api_key_draft = value;
                    provider.error = None;
                }
                None
            }
            Input::SaveKeyRequested(id) => self.provider_mut(&id).and_then(|provider| {
                let key = provider.api_key_draft.trim();
                (!key.is_empty()).then(|| Output::SaveKey(id, key.to_owned()))
            }),
            Input::RemoveKeyRequested(id) => Some(Output::RemoveKey(id)),
            Input::RefreshModelsRequested(id) => Some(Output::DiscoverModels(id)),
            Input::ModelToggled(id, model, enabled) => Some(Output::SetModel {
                provider: id,
                model,
                enabled,
            }),
        }
    }
    fn key_saved(&mut self, id: &ProviderId) {
        if let Some(provider) = self.provider_mut(id) {
            provider.credential_present = true;
            provider.api_key_draft.clear();
            provider.error = None;
        }
    }
    fn key_removed(&mut self, id: &ProviderId) {
        if let Some(provider) = self.provider_mut(id) {
            provider.credential_present = false;
            provider.api_key_draft.clear();
            provider.discovered_models.clear();
            provider.discovering = false;
            provider.error = None;
        }
        if self.active_provider.as_ref() == Some(id) {
            self.active_provider = None;
        }
    }
    fn set_selected_models(&mut self, id: &ProviderId, models: Vec<String>) {
        if let Some(provider) = self.provider_mut(id) {
            if provider.discovered_models.is_empty() {
                provider.discovered_models = models.clone();
            }
            provider.selected_models = models;
        }
    }
    fn start_discovery(&mut self, id: &ProviderId) {
        if let Some(provider) = self.provider_mut(id) {
            provider.discovering = true;
            provider.error = None;
        }
    }
    fn finish_discovery(&mut self, id: &ProviderId, models: Vec<String>) {
        if let Some(provider) = self.provider_mut(id) {
            provider.discovered_models = models;
            provider.discovering = false;
            provider.error = None;
        }
    }
    fn set_error(&mut self, id: &ProviderId, error: String) {
        if let Some(provider) = self.provider_mut(id) {
            provider.discovering = false;
            provider.error = Some(error);
        }
    }
    fn set_global_error(&mut self, error: String) {
        self.error = Some(error);
    }
    fn clear_error(&mut self) {
        self.error = None;
    }
    fn discard_draft(&mut self) {
        for provider in &mut self.providers {
            provider.api_key_draft.clear();
        }
        self.active_provider = None;
    }
}

#[cfg(test)]
mod tests {
    use super::super::Section;
    use super::*;

    #[test]
    fn navigation_requests_scroll_to_provider_section() {
        let mut settings = Settings::default();

        assert!(matches!(
            settings.update(Input::SectionSelected(Section::Providers)),
            Some(Output::JumpToSection(Section::Providers))
        ));
    }

    #[test]
    fn add_provider_opens_an_unconnected_provider() {
        let id = ProviderId::new("openai");
        let mut settings = Settings::default().with_providers(vec![ProviderView::new(
            id.clone(),
            "OpenAI",
            false,
            Vec::new(),
        )]);

        assert!(
            settings
                .update(Input::ProviderSelected(id.clone()))
                .is_none()
        );
        assert_eq!(settings.active_provider, Some(id.clone()));

        settings.update(Input::KeySaved(id.clone()));
        assert!(settings.providers[0].credential_present);
        assert_eq!(settings.active_provider, Some(id));
    }

    #[test]
    fn managing_a_connected_provider_starts_discovery() {
        let id = ProviderId::new("openai");
        let mut settings = Settings::default().with_providers(vec![ProviderView::new(
            id.clone(),
            "OpenAI",
            true,
            Vec::new(),
        )]);

        assert!(matches!(
            settings.update(Input::ProviderRowToggled(id.clone())),
            Some(Output::DiscoverModels(provider)) if provider == id
        ));
        assert_eq!(settings.active_provider, Some(id.clone()));
        assert!(settings.update(Input::ProviderRowToggled(id)).is_none());
        assert_eq!(settings.active_provider, None);
    }

    #[test]
    fn provider_key_drafts_are_independent() {
        let first = ProviderId::new("first");
        let second = ProviderId::new("second");
        let mut settings = Settings::default().with_providers(vec![
            ProviderView::new(first.clone(), "First", false, Vec::new()),
            ProviderView::new(second.clone(), "Second", false, Vec::new()),
        ]);

        settings.update(Input::ApiKeyChanged(first.clone(), "first-key".into()));
        settings.update(Input::ApiKeyChanged(second.clone(), "second-key".into()));
        assert!(
            matches!(settings.update(Input::SaveKeyRequested(first.clone())), Some(Output::SaveKey(id, key)) if id == first && key == "first-key")
        );
        assert!(
            matches!(settings.update(Input::SaveKeyRequested(second.clone())), Some(Output::SaveKey(id, key)) if id == second && key == "second-key")
        );
    }

    #[test]
    fn key_is_submitted_only_on_commit_and_is_redacted_from_debug() {
        let id = ProviderId::new("test");
        let mut settings = Settings::default().with_providers(vec![ProviderView::new(
            id.clone(),
            "Test",
            false,
            Vec::new(),
        )]);
        assert!(
            settings
                .update(Input::ApiKeyChanged(id.clone(), " sk-test ".into()))
                .is_none()
        );
        assert!(
            matches!(settings.update(Input::SaveKeyRequested(id.clone())), Some(Output::SaveKey(provider, key)) if provider == id && key == "sk-test")
        );
        assert!(!format!("{:?}", Input::ApiKeyChanged(id, "sk-test".into())).contains("sk-test"));
    }

    #[test]
    fn removing_a_key_clears_the_provider_setup() {
        let id = ProviderId::new("openai");
        let mut settings = Settings::default().with_providers(vec![ProviderView::new(
            id.clone(),
            "OpenAI",
            true,
            vec!["gpt-test".into()],
        )]);
        settings.update(Input::ProviderRowToggled(id.clone()));
        settings.update(Input::ApiKeyChanged(id.clone(), "replacement-key".into()));
        settings.update(Input::DiscoveryStarted(id.clone()));

        assert!(settings.update(Input::KeyRemoved(id)).is_none());

        let provider = &settings.providers[0];
        assert!(!provider.credential_present);
        assert!(provider.api_key_draft.is_empty());
        assert!(provider.discovered_models.is_empty());
        assert!(!provider.discovering);
        assert!(settings.active_provider.is_none());
    }

    #[test]
    fn discovery_failure_preserves_models_and_allows_retry() {
        let id = ProviderId::new("openai");
        let mut settings = Settings::default().with_providers(vec![ProviderView::new(
            id.clone(),
            "OpenAI",
            true,
            vec!["gpt-test".into()],
        )]);
        settings.update(Input::DiscoveryStarted(id.clone()));
        settings.update(Input::ProviderFailed(id.clone(), "request failed".into()));

        assert!(!settings.providers[0].discovering);
        assert_eq!(settings.providers[0].selected_models, vec!["gpt-test"]);
        assert_eq!(
            settings.providers[0].error.as_deref(),
            Some("request failed")
        );

        settings.update(Input::DiscoveryStarted(id.clone()));
        assert!(settings.providers[0].error.is_none());
        settings.update(Input::DiscoveryFinished(id, vec!["gpt-new".into()]));
        assert!(!settings.providers[0].discovering);
        assert_eq!(settings.providers[0].discovered_models, vec!["gpt-new"]);
        assert_eq!(settings.providers[0].selected_models, vec!["gpt-test"]);
    }
}
