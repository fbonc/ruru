use crate::{Provider, ProviderError, ProviderRequest, ProviderStream};
use eventsource_stream::Eventsource;
use futures_util::future::{Abortable, BoxFuture};
use futures_util::{StreamExt, stream};
use std::sync::Arc;
use std::time::Duration;

use super::OpenAiProvider;
use super::protocol::{ModelsResponse, ResponsesRequest, api_error, parse_event};

impl Provider for OpenAiProvider {
    fn configured_models(&self) -> Vec<String> {
        self.models.clone()
    }

    fn discover_models(&self) -> BoxFuture<'_, Result<Vec<String>, ProviderError>> {
        Box::pin(self.discover_supported_models())
    }

    fn stream(&self, model: &str, request: ProviderRequest) -> ProviderStream {
        *self
            .last_request
            .lock()
            .expect("OpenAI last request mutex poisoned") =
            Some((model.to_owned(), request.clone()));
        let client = self.client.clone();
        let api_key = Arc::clone(&self.api_key);
        let endpoint = Arc::clone(&self.endpoint);
        let request = ResponsesRequest::new(model, request);

        let stream = stream::once(async move {
            match Self::open_stream(client, api_key, endpoint, request).await {
                Ok(stream) => stream,
                Err(error) => stream::once(async move { Err(error) }).boxed(),
            }
        })
        .flatten();

        Abortable::new(stream, self.request_abort.register()).boxed()
    }

    fn abort_request(&self) {
        self.request_abort.abort();
    }

    fn retry_request(&self) -> Option<ProviderStream> {
        let request = self
            .last_request
            .lock()
            .expect("OpenAI last request mutex poisoned")
            .clone();

        request.map(|(model, request)| self.stream(&model, request))
    }
}

impl OpenAiProvider {
    pub(super) async fn fetch_model_ids(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .client
            .get(self.models_endpoint.as_ref())
            .bearer_auth(self.api_key.as_ref())
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|error| {
                ProviderError::new(format!("OpenAI model discovery failed: {error}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.map_err(|error| {
                ProviderError::new(format!("failed to read OpenAI error response: {error}"))
            })?;
            return Err(api_error(status, &body));
        }

        let response = response.json::<ModelsResponse>().await.map_err(|error| {
            ProviderError::new(format!("invalid OpenAI models response: {error}"))
        })?;
        Ok(response.into_ids())
    }

    pub(super) async fn open_stream(
        client: reqwest::Client,
        api_key: Arc<str>,
        endpoint: Arc<str>,
        request: ResponsesRequest,
    ) -> Result<ProviderStream, ProviderError> {
        let response = client
            .post(endpoint.as_ref())
            .bearer_auth(api_key.as_ref())
            .json(&request)
            .send()
            .await
            .map_err(|error| ProviderError::new(format!("OpenAI request failed: {error}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.map_err(|error| {
                ProviderError::new(format!("failed to read OpenAI error response: {error}"))
            })?;
            return Err(api_error(status, &body));
        }

        Ok(response
            .bytes_stream()
            .eventsource()
            .filter_map(|event| async move {
                match event {
                    Ok(event) => parse_event(&event.data),
                    Err(error) => Some(Err(ProviderError::new(format!(
                        "OpenAI response stream failed: {error}"
                    )))),
                }
            })
            .boxed())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::StreamExt;
    use tokio::net::TcpListener;

    use super::super::tests::request;
    use super::super::{OpenAiConfig, OpenAiProvider};
    use crate::{Provider, ProviderError, ProviderOutput};

    #[test]
    fn retries_the_last_request() {
        let provider = OpenAiProvider::new(OpenAiConfig {
            api_key: "test-key".into(),
            models: vec!["gpt-test".into()],
            base_url: "http://127.0.0.1:1".into(),
        });
        assert!(provider.retry_request().is_none());
        drop(provider.stream("gpt-test", request(None)));

        assert!(provider.retry_request().is_some());
        let (model, request) = provider
            .last_request
            .lock()
            .expect("OpenAI last request mutex poisoned")
            .clone()
            .expect("last request");
        assert_eq!(model, "gpt-test");
        assert_eq!(request.prompt.as_deref(), Some("What does this mean?"));
        assert!(request.context.is_some());
    }

    #[tokio::test]
    async fn streams_text_deltas_from_the_responses_endpoint() {
        let events = concat!(
            "data: {\"type\":\"response.created\"}\n\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello \"}\n\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"owl\"}\n\n",
            "data: {\"type\":\"response.completed\"}\n\n"
        );
        let (base_url, received) = serve_once(200, "text/event-stream", events).await;
        let provider = OpenAiProvider::new(OpenAiConfig {
            api_key: "test-key".into(),
            models: vec!["gpt-test".into()],
            base_url,
        });

        let output = provider
            .stream("gpt-test", request(None))
            .map(Result::unwrap)
            .map(|output| match output {
                ProviderOutput::TextDelta(delta) => delta,
            })
            .collect::<String>()
            .await;
        let received = received.await.expect("server task");

        assert_eq!(output, "hello owl");
        assert!(received.starts_with("POST /v1/responses HTTP/1.1"));
        assert!(
            received
                .to_ascii_lowercase()
                .contains("authorization: bearer test-key")
        );
    }

    #[tokio::test]
    async fn discovers_models_with_the_configured_api_key() {
        let body = r#"{"object":"list","data":[{"id":"gpt-4o"},{"id":"gpt-image-1"},{"id":"o4-mini"},{"id":"gpt-4o"}]}"#;
        let (base_url, received) = serve_once(200, "application/json", body).await;
        let provider = OpenAiProvider::new(OpenAiConfig {
            api_key: "test-key".into(),
            models: vec!["configured-fallback".into()],
            base_url,
        });

        let models = provider.discover_models().await.expect("discover models");
        let received = received.await.expect("server task");

        assert_eq!(models, vec!["gpt-4o", "o4-mini"]);
        assert!(received.starts_with("GET /v1/models HTTP/1.1"));
        assert!(
            received
                .to_ascii_lowercase()
                .contains("authorization: bearer test-key")
        );
    }

    #[tokio::test]
    async fn reports_model_discovery_errors() {
        let body = r#"{"error":{"message":"invalid API key"}}"#;
        let (base_url, received) = serve_once(401, "application/json", body).await;
        let provider = OpenAiProvider::new(OpenAiConfig {
            api_key: "bad-key".into(),
            models: vec!["configured-fallback".into()],
            base_url,
        });

        let error = provider
            .discover_models()
            .await
            .expect_err("discovery should fail");
        received.await.expect("server task");

        assert_eq!(
            error,
            ProviderError::new("OpenAI API request failed (401 Unauthorized): invalid API key")
        );
        assert_eq!(provider.configured_models(), vec!["configured-fallback"]);
    }

    #[tokio::test]
    async fn returns_api_error_messages() {
        let body = r#"{"error":{"message":"invalid API key"}}"#;
        let (base_url, received) = serve_once(401, "application/json", body).await;
        let provider = OpenAiProvider::new(OpenAiConfig {
            api_key: "bad-key".into(),
            models: vec!["gpt-test".into()],
            base_url,
        });

        let error = provider
            .stream("gpt-test", request(None))
            .next()
            .await
            .expect("provider result")
            .expect_err("request should fail");
        received.await.expect("server task");

        assert_eq!(
            error,
            ProviderError::new("OpenAI API request failed (401 Unauthorized): invalid API key")
        );
    }

    #[tokio::test]
    async fn aborts_an_active_request() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let provider = OpenAiProvider::new(OpenAiConfig {
            api_key: "test-key".into(),
            models: vec!["gpt-test".into()],
            base_url: format!("http://{address}"),
        });
        let mut stream = provider.stream("gpt-test", request(None));
        let next_output = tokio::spawn(async move { stream.next().await });
        let _connection = tokio::time::timeout(Duration::from_secs(1), listener.accept())
            .await
            .expect("request should reach test server")
            .expect("accept request");

        provider.abort_request();

        let output = tokio::time::timeout(Duration::from_secs(1), next_output)
            .await
            .expect("aborted request should stop promptly")
            .expect("stream task");
        assert!(output.is_none());
    }

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn serve_once(
        status: u16,
        content_type: &'static str,
        response_body: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];

            loop {
                let count = socket.read(&mut buffer).await.expect("read request");
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);

                let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }

            let reason = if status == 200 { "OK" } else { "Error" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                response_body.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
            String::from_utf8(request).expect("UTF-8 request")
        });

        (format!("http://{address}"), task)
    }
}
