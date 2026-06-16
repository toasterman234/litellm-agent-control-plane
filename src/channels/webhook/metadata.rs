use axum::http::HeaderMap;
use serde_json::{json, Value};

pub(super) fn session_title(request_id: &str) -> String {
    truncate_title(&format!("Webhook {request_id}"))
}

pub(super) fn truncate_title(value: &str) -> String {
    const MAX_TITLE_LEN: usize = 120;
    let trimmed = value.trim();
    if trimmed.chars().count() <= MAX_TITLE_LEN {
        return trimmed.to_owned();
    }
    let truncated = trimmed.chars().take(MAX_TITLE_LEN - 3).collect::<String>();
    format!("{truncated}...")
}

pub(super) fn session_metadata(headers: &HeaderMap, request_id: &str) -> Value {
    json!({
        "source": "webhook",
        "request_id": request_id,
        "content_type": header_value(headers, "content-type"),
        "user_agent": header_value(headers, "user-agent"),
        "webhook_event_id": first_header_value(headers, &[
            "x-zendesk-webhook-id",
            "x-zendesk-event-id",
            "x-github-delivery",
        ]),
    })
}

pub(super) fn request_id(headers: &HeaderMap) -> String {
    first_header_value(
        headers,
        &[
            "x-zendesk-webhook-id",
            "x-zendesk-event-id",
            "x-github-delivery",
            "x-request-id",
        ],
    )
    .unwrap_or_else(|| format!("webhook_{}", uuid::Uuid::new_v4().simple()))
}

fn first_header_value(headers: &HeaderMap, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| header_value(headers, name))
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            let value = value.trim();
            (!value.is_empty()).then_some(value)
        })
        .map(str::to_owned)
}
