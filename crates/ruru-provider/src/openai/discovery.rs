use crate::ProviderError;

use super::OpenAiProvider;

impl OpenAiProvider {
    pub(super) async fn discover_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let mut models = self
            .fetch_model_ids()
            .await?
            .into_iter()
            .filter(|id| supports_text_and_images(id))
            .collect::<Vec<_>>();
        models.sort_unstable();
        models.dedup();
        if models.is_empty() {
            return Err(ProviderError::new(
                "OpenAI returned no compatible text-and-image models",
            ));
        }
        Ok(models)
    }
}

fn supports_text_and_images(id: &str) -> bool {
    // The models endpoint omits input modalities, so use supported model families.
    let supported_family = [
        "gpt-4o",
        "gpt-4.1",
        "gpt-4-turbo",
        "gpt-5",
        "gpt-6",
        "o1",
        "o3",
        "o4",
    ]
    .iter()
    .any(|family| {
        id.strip_prefix(family).is_some_and(|suffix| {
            suffix.is_empty() || suffix.starts_with('-') || suffix.starts_with('.')
        })
    });
    let unsupported_variant = [
        "audio",
        "realtime",
        "transcribe",
        "tts",
        "image",
        "search",
        "codex",
        "instruct",
        "moderation",
        "computer",
        "deep-research",
        "chat",
    ]
    .iter()
    .any(|variant| id.contains(variant));

    supported_family
        && !unsupported_variant
        && !id.starts_with("o1-mini")
        && !id.starts_with("o1-preview")
        && !id.starts_with("o3-mini")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_text_and_image_response_models() {
        for id in [
            "gpt-4o",
            "gpt-4.1-mini",
            "gpt-5.6-terra",
            "gpt-6.1-sol",
            "o1",
            "o3",
            "o4-mini",
        ] {
            assert!(supports_text_and_images(id), "{id}");
        }
        for id in [
            "gpt-3.5-turbo",
            "gpt-4o-audio-preview",
            "gpt-4o-mini-search-preview",
            "gpt-5-codex",
            "gpt-5-chat-latest",
            "gpt-image-1",
            "o1-mini",
            "o1-preview",
            "o3-mini",
            "text-embedding-3-large",
        ] {
            assert!(!supports_text_and_images(id), "{id}");
        }
    }
}
