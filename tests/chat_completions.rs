use std::{collections::HashMap, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use litellm_rust::{
    http::routes::router,
    proxy::{
        config::{GatewayConfig, GeneralSettings, LiteLlmParams, ModelEntry},
        state::AppState,
    },
    sdk::{
        providers::{self, ProviderRegistry},
        routing::Router as ModelRouter,
    },
};
use serde_json::json;
use tower::util::ServiceExt;
use wiremock::{
    matchers::{header as header_match, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn forwards_non_streaming_gemini_chat_completions() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header_match("authorization", "Bearer sk-gemini-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_test",
            "object": "chat.completion",
            "model": "gemini-3.5-flash",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "ok" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&upstream)
        .await;

    let config = test_config(vec![chat_model(
        "gemini-3.5-flash",
        "gemini_chat/gemini-3.5-flash",
        "sk-gemini-test",
        upstream.uri(),
    )]);
    let app = router(build_state(&config));

    let response = app
        .oneshot(chat_request("gemini-3.5-flash", false))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["choices"][0]["message"]["content"], "ok");
}

#[tokio::test]
async fn forwards_streaming_mistral_chat_completions() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header_match("authorization", "Bearer sk-mistral-test"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "data: {\"id\":\"chunk_1\",\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n",
                ),
        )
        .mount(&upstream)
        .await;

    let config = test_config(vec![chat_model(
        "mistral-small-latest",
        "mistral/mistral-small-latest",
        "sk-mistral-test",
        upstream.uri(),
    )]);
    let app = router(build_state(&config));

    let response = app
        .oneshot(chat_request("mistral-small-latest", true))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert!(std::str::from_utf8(&body).unwrap().contains("\"ok\""));
}

fn chat_request(model: &str, stream: bool) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header(header::AUTHORIZATION, "Bearer sk-local")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "model": model,
                "messages": [{"role": "user", "content": "hi"}],
                "stream": stream
            })
            .to_string(),
        ))
        .unwrap()
}

fn chat_model(
    model_name: &str,
    provider_model: &str,
    api_key: &str,
    api_base: String,
) -> ModelEntry {
    ModelEntry {
        model_name: model_name.to_owned(),
        litellm_params: LiteLlmParams {
            model: provider_model.to_owned(),
            api_key: Some(api_key.to_owned()),
            api_base: Some(api_base),
            extra: Default::default(),
        },
    }
}

fn test_config(model_list: Vec<ModelEntry>) -> GatewayConfig {
    GatewayConfig {
        model_list,
        mcp_servers: Default::default(),
        general_settings: GeneralSettings {
            master_key: Some("sk-local".to_owned()),
            ..Default::default()
        },
        slack: Default::default(),
        agents: Vec::new(),
    }
}

fn build_state(config: &GatewayConfig) -> Arc<AppState> {
    let mut providers = ProviderRegistry::new();
    providers::register_all(&mut providers);
    let model_router = ModelRouter::from_config(config, &providers).unwrap();
    let http = AppState::build_http_client().unwrap();
    Arc::new(AppState::new(config.clone(), model_router, http, HashMap::new(), None).unwrap())
}
