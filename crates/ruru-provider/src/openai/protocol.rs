use crate::{ANSWER_INSTRUCTION, ProviderError, ProviderOutput, ProviderRequest};
use base64::Engine;
use ruru_types::{ImageCapture, Target};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize)]
pub(super) struct ResponsesRequest {
    model: String,
    instructions: &'static str,
    input: Vec<InputMessage>,
    stream: bool,
    store: bool,
}

impl ResponsesRequest {
    pub(super) fn new(model: &str, request: ProviderRequest) -> Self {
        let mut content = Vec::new();

        if let Some(context) = request.context {
            content.push(InputContent::Text {
                text: "Surrounding context image:".into(),
            });
            content.push(InputContent::from_image(context.image));
            if let Some(text) = context.accessibility_text {
                content.push(InputContent::Text {
                    text: format!("Supplemental accessibility context:\n{text}"),
                });
            }
        }

        if let Some(target) = request.target {
            match target {
                Target::Text(target) => content.push(InputContent::Text {
                    text: format!("Target:\n{}", target.text),
                }),
                Target::Image(image) => {
                    content.push(InputContent::Text {
                        text: "Target image:".into(),
                    });
                    content.push(InputContent::from_image(image));
                }
            }
        }

        content.push(InputContent::Text {
            text: format!("Question:\n{}", request.prompt.as_deref().unwrap_or("")),
        });

        Self {
            model: model.into(),
            instructions: ANSWER_INSTRUCTION,
            input: vec![InputMessage {
                role: "user",
                content,
            }],
            stream: true,
            store: false,
        }
    }
}

#[derive(Serialize)]
struct InputMessage {
    role: &'static str,
    content: Vec<InputContent>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum InputContent {
    #[serde(rename = "input_text")]
    Text { text: String },
    #[serde(rename = "input_image")]
    Image {
        image_url: String,
        detail: &'static str,
    },
}

impl InputContent {
    fn from_image(image: ImageCapture) -> Self {
        let encoded = base64::engine::general_purpose::STANDARD.encode(image.png);
        Self::Image {
            image_url: format!("data:image/png;base64,{encoded}"),
            detail: "auto",
        }
    }
}

#[derive(Deserialize)]
pub(super) struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

impl ModelsResponse {
    pub(super) fn into_ids(self) -> Vec<String> {
        self.data.into_iter().map(|model| model.id).collect()
    }
}

pub(super) fn parse_event(data: &str) -> Option<Result<ProviderOutput, ProviderError>> {
    if data.is_empty() || data == "[DONE]" {
        return None;
    }

    let event = match serde_json::from_str::<Value>(data) {
        Ok(event) => event,
        Err(error) => {
            return Some(Err(ProviderError::new(format!(
                "invalid OpenAI response event: {error}"
            ))));
        }
    };

    match event.get("type").and_then(Value::as_str) {
        Some("response.output_text.delta" | "response.refusal.delta") => {
            match event.get("delta").and_then(Value::as_str) {
                Some(delta) => Some(Ok(ProviderOutput::TextDelta(delta.into()))),
                None => Some(Err(ProviderError::new(
                    "OpenAI text delta did not contain text",
                ))),
            }
        }
        Some("error") => Some(Err(ProviderError::new(
            event
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("OpenAI returned an unknown streaming error"),
        ))),
        Some("response.failed") => Some(Err(ProviderError::new(
            event
                .pointer("/response/error/message")
                .and_then(Value::as_str)
                .unwrap_or("OpenAI response failed"),
        ))),
        _ => None,
    }
}

pub(super) fn api_error(status: reqwest::StatusCode, body: &str) -> ProviderError {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|body| {
            body.pointer("/error/message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown API error".into());

    ProviderError::new(format!("OpenAI API request failed ({status}): {message}"))
}

#[cfg(test)]
mod tests {
    use super::super::tests::request;
    use super::*;
    use ruru_types::{TextCapture, TextCaptureMethod};

    #[test]
    fn builds_request_with_image_context_for_responses_api() {
        let request = ResponsesRequest::new(
            "gpt-test",
            request(Some(Target::Text(TextCapture {
                text: "selected text".into(),
                method: TextCaptureMethod::Accessibility,
            }))),
        );
        let json = serde_json::to_value(request).expect("serialize request");

        assert_eq!(json["model"], "gpt-test");
        assert_eq!(json["stream"], true);
        assert_eq!(json["store"], false);
        assert_eq!(json["input"][0]["role"], "user");
        assert_eq!(
            json["input"][0]["content"][0]["text"],
            "Surrounding context image:"
        );
        assert_eq!(json["input"][0]["content"][1]["type"], "input_image");
        assert_eq!(
            json["input"][0]["content"][2]["text"],
            "Supplemental accessibility context:\nsurrounding text"
        );
        assert_eq!(
            json["input"][0]["content"][3]["text"],
            "Target:\nselected text"
        );
        assert_eq!(
            json["input"][0]["content"][4]["text"],
            "Question:\nWhat does this mean?"
        );
    }

    #[test]
    fn encodes_png_target_as_an_input_image() {
        let request = ResponsesRequest::new(
            "gpt-test",
            request(Some(Target::Image(ImageCapture {
                png: b"png".to_vec(),
                region: None,
            }))),
        );
        let json = serde_json::to_value(request).expect("serialize request");
        let image = &json["input"][0]["content"][4];

        assert_eq!(image["type"], "input_image");
        assert_eq!(image["image_url"], "data:image/png;base64,cG5n");
        assert_eq!(image["detail"], "auto");
    }

    #[test]
    fn parses_text_and_error_events() {
        assert_eq!(
            parse_event(r#"{"type":"response.output_text.delta","delta":"hello"}"#),
            Some(Ok(ProviderOutput::TextDelta("hello".into())))
        );
        assert_eq!(
            parse_event(r#"{"type":"error","message":"rate limited"}"#),
            Some(Err(ProviderError::new("rate limited")))
        );
        assert_eq!(
            parse_event(r#"{"type":"response.refusal.delta","delta":"I cannot"}"#),
            Some(Ok(ProviderOutput::TextDelta("I cannot".into())))
        );
        assert_eq!(parse_event(r#"{"type":"response.completed"}"#), None);
    }
}
