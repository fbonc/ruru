use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use ruru_types::ProviderId;
use std::sync::Arc;

use super::state::SettingsState;
use super::{Settings, SettingsChange};
use crate::Output;
use crate::providers::Providers;

impl Settings {
    pub(crate) fn discover(self: &Arc<Self>, id: ProviderId) -> BoxStream<'static, Output> {
        let request = {
            let mut state = self.state.lock().expect("settings mutex poisoned");
            self.check_edit(&state, &id).and_then(|()| {
                let key = self
                    .credentials
                    .get(&id)
                    .map_err(|error| error.to_string())?
                    .ok_or("Save an API key first")?;
                let discovery_id = state.latest_discovery_id.entry(id.clone()).or_default();
                *discovery_id = discovery_id.wrapping_add(1);
                Ok((key, *discovery_id))
            })
        };
        let (key, discovery_id) = match request {
            Ok(request) => request,
            Err(error) => {
                return stream::once(async move {
                    Output::SettingsFailed {
                        provider: Some(id),
                        error,
                    }
                })
                .boxed();
            }
        };
        let settings = Arc::clone(self);
        stream::once(async move {
            let result = Providers::discover_models(id.clone(), key.clone()).await;
            settings.finish_discovery(id, discovery_id, key, result)
        })
        .flat_map(stream::iter)
        .boxed()
    }
    pub(super) fn finish_discovery(
        &self,
        id: ProviderId,
        discovery_id: u64,
        key: String,
        result: Result<Vec<String>, String>,
    ) -> Vec<Output> {
        let mut state = self.state.lock().expect("settings mutex poisoned");
        if state.latest_discovery_id.get(&id) != Some(&discovery_id) {
            return Vec::new();
        }
        let Ok(models) = result else {
            return vec![Output::ModelsDiscovered {
                provider: id,
                result,
            }];
        };
        let mut outputs = Vec::new();
        let mut next = state.config.clone();
        let settings = next.providers.entry(id.clone()).or_default();
        let before = settings.models.clone();
        settings.models.retain(|model| models.contains(model));
        if settings.models != before {
            match self.persist_provider(
                &mut state,
                id.clone(),
                next,
                Some(key),
                SettingsChange::ModelsChanged,
            ) {
                Ok(update) => {
                    outputs.push(Output::SettingsUpdated(update));
                }
                Err(error) => outputs.push(Output::SettingsFailed {
                    provider: Some(id.clone()),
                    error,
                }),
            }
        }
        outputs.push(Output::ModelsDiscovered {
            provider: id,
            result: Ok(models),
        });
        outputs
    }
    pub(super) fn invalidate_discovery(&self, state: &mut SettingsState, id: &ProviderId) {
        let discovery_id = state.latest_discovery_id.entry(id.clone()).or_default();
        *discovery_id = discovery_id.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{TestConfig, TestCredentials, test_settings};
    use ruru_types::ModelSelection;
    use ruru_types::ProviderId;
    use std::sync::Arc;

    #[test]
    fn stale_discovery_cannot_remove_a_selected_model() {
        let settings = test_settings(
            Arc::new(TestConfig::default()),
            Arc::new(TestCredentials::default()),
        );
        let id = ProviderId::new("openai");
        let model = ModelSelection::new(id.clone(), "gpt-test");
        settings.save_key(id.clone(), "test-key".into()).unwrap();
        settings
            .set_model(id.clone(), model.model.clone(), true)
            .unwrap();
        settings
            .state
            .lock()
            .unwrap()
            .latest_discovery_id
            .insert(id.clone(), 2);
        let outputs = settings.finish_discovery(id, 1, "old-key".into(), Ok(Vec::new()));

        assert!(outputs.is_empty());
        assert_eq!(settings.snapshot().available_models, vec![model]);
    }
}
