use axum::http::{header, HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;

use crate::{
    errors::GatewayError,
    sdk::{
        providers::base::{ProviderRequest, Transformation},
        routing::Deployment,
    },
};

// Forward a handful of Codex headers so upstream logs stay correlated.
const FORWARDED_HEADERS: &[&str] = &[
    "accept",
    "originator",
    "session-id",
    "thread-id",
    "x-client-request-id",
    "x-codex-beta-features",
    "x-codex-turn-metadata",
    "x-codex-window-id",
];

pub trait BaseOpenAiChatCompletionsTransformation: Send + Sync + 'static {
    fn validate_environment(
        &self,
        deployment: &Deployment,
        inbound_headers: &HeaderMap,
    ) -> Result<HeaderMap, GatewayError>;

    fn upstream_request_id_header(&self) -> &'static str {
        "x-request-id"
    }

    fn map_chat_completions_params(
        &self,
        mut body: Value,
        deployment: &Deployment,
    ) -> Result<Value, GatewayError> {
        if body.get("model").and_then(Value::as_str) != Some(deployment.upstream_model.as_str()) {
            body["model"] = Value::String(deployment.upstream_model.clone());
        }
        Ok(body)
    }

    fn transform_chat_completions_request(
        &self,
        body: Value,
        deployment: &Deployment,
        inbound_headers: &HeaderMap,
    ) -> Result<ProviderRequest, GatewayError> {
        let body = self.map_chat_completions_params(body, deployment)?;
        let stream = body.get("stream").and_then(Value::as_bool).unwrap_or(false);
        let headers = self.validate_environment(deployment, inbound_headers)?;

        Ok(ProviderRequest {
            body: serde_json::to_vec(&body)?,
            headers,
            stream,
        })
    }

    fn transform_chat_completions_response_headers(
        &self,
        upstream: &HeaderMap,
        stream: bool,
    ) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let content_type = if stream {
            HeaderValue::from_static("text/event-stream")
        } else {
            upstream
                .get(header::CONTENT_TYPE)
                .cloned()
                .unwrap_or_else(|| HeaderValue::from_static("application/json"))
        };
        headers.insert(header::CONTENT_TYPE, content_type);
        if let Some(request_id) = upstream.get(self.upstream_request_id_header()).cloned() {
            headers.insert("request-id", request_id);
        }
        headers
    }
}

#[derive(Debug, Default, Clone)]
pub struct OpenAiChatCompletionsTransformation;

impl BaseOpenAiChatCompletionsTransformation for OpenAiChatCompletionsTransformation {
    fn validate_environment(
        &self,
        deployment: &Deployment,
        inbound_headers: &HeaderMap,
    ) -> Result<HeaderMap, GatewayError> {
        let mut headers = HeaderMap::new();
        let bearer = format!("Bearer {}", deployment.api_key);
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&bearer)
                .map_err(|_| GatewayError::InvalidConfig("invalid api_key".to_owned()))?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        for name in FORWARDED_HEADERS {
            if let Some(value) = inbound_headers.get(*name) {
                if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) {
                    headers.insert(header_name, value.clone());
                }
            }
        }
        Ok(headers)
    }
}

impl Transformation for OpenAiChatCompletionsTransformation {
    fn transform_request(
        &self,
        body: Value,
        deployment: &Deployment,
        inbound_headers: &HeaderMap,
    ) -> Result<ProviderRequest, GatewayError> {
        BaseOpenAiChatCompletionsTransformation::transform_chat_completions_request(
            self,
            body,
            deployment,
            inbound_headers,
        )
    }

    fn transform_response_headers(&self, upstream: &HeaderMap, stream: bool) -> HeaderMap {
        BaseOpenAiChatCompletionsTransformation::transform_chat_completions_response_headers(
            self, upstream, stream,
        )
    }

    fn chat_completions_url(&self, deployment: &Deployment) -> String {
        deployment.chat_completions_url()
    }
}
