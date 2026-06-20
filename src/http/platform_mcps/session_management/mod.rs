use std::sync::Arc;

use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    db::managed_agents::{messages, runtime_events, sessions},
    errors::GatewayError,
    proxy::state::AppState,
};

use super::{required_str, PLATFORM_SESSION_MCP_ID, SEND_PLATFORM_SESSION_MESSAGE_MCP_ID};

pub fn read_tool_def() -> Value {
    json!({
        "name": PLATFORM_SESSION_MCP_ID,
        "description": "Read persisted platform session messages by session_id.",
        "inputSchema": {
            "type": "object",
            "properties": { "session_id": { "type": "string" } },
            "required": ["session_id"]
        }
    })
}

pub fn send_tool_def() -> Value {
    json!({
        "name": SEND_PLATFORM_SESSION_MESSAGE_MCP_ID,
        "description": "Send a user message into a platform session by session_id and resume the target agent run.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "session_id": { "type": "string" },
                "text": { "type": "string" },
                "model_id": {
                    "type": "string",
                    "description": "Optional model ID for non-runtime harness sessions."
                }
            },
            "required": ["session_id", "text"]
        }
    })
}

pub async fn read_platform_session(pool: &PgPool, arguments: Value) -> Result<Value, GatewayError> {
    let session_id = required_str(&arguments, "session_id")?;
    let session = sessions::repository::get(pool, session_id)
        .await?
        .ok_or_else(|| GatewayError::NotFound("session not found".to_owned()))?;
    let rows = messages::repository::list(pool, session_id).await?;
    let runtime_events = runtime_events::repository::list(pool, session_id).await?;
    let transcript = platform_transcript(&rows, &runtime_events);
    Ok(json!({
        "session": session,
        "messages": rows.into_iter().map(|row| {
            json!({
                "id": row.id,
                "seq": row.seq,
                "info": serde_json::from_str::<Value>(&row.info_json).unwrap_or(Value::String(row.info_json)),
                "parts": serde_json::from_str::<Value>(&row.parts_json).unwrap_or(Value::String(row.parts_json))
            })
        }).collect::<Vec<_>>(),
        "runtime_events": runtime_events,
        "transcript": transcript
    }))
}

pub async fn send_platform_session_message(
    state: Arc<AppState>,
    pool: PgPool,
    arguments: Value,
) -> Result<Value, GatewayError> {
    let session_id = required_str(&arguments, "session_id")?.to_owned();
    let text = required_str(&arguments, "text")?.to_owned();
    let model = model_id(&arguments);
    crate::http::sessions::enqueue_prompt_text(state, pool.clone(), &session_id, text, model)
        .await?;

    let session = sessions::repository::get(&pool, &session_id)
        .await?
        .ok_or_else(|| GatewayError::NotFound("session not found".to_owned()))?;
    Ok(json!({
        "session_id": session.id,
        "status": session.status,
        "runtime": session.runtime,
        "provider_run_id": session.provider_run_id
    }))
}

fn model_id(arguments: &Value) -> String {
    arguments
        .get("model_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("claude-sonnet-4-6")
        .to_owned()
}

pub(super) fn platform_transcript(
    rows: &[crate::db::managed_agents::messages::schema::SessionMessageRow],
    runtime_events: &[Value],
) -> Vec<Value> {
    let mut transcript = Vec::new();
    for row in rows {
        let info = serde_json::from_str::<Value>(&row.info_json).unwrap_or(Value::Null);
        let parts = serde_json::from_str::<Value>(&row.parts_json).unwrap_or(Value::Null);
        let role = info
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let text = text_from_parts(&parts);
        if text.trim().is_empty() {
            continue;
        }
        transcript.push(json!({
            "role": role,
            "text": text,
            "source": "session_messages",
            "seq": row.seq
        }));
    }

    let mut assistant_by_key: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut assistant_chunks: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    let mut next_runtime_seq = 1usize;

    for event in runtime_events {
        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match event_type {
            "user.message" => {
                let text = text_from_content(event.get("content"));
                if text.trim().is_empty() {
                    continue;
                }
                transcript.push(json!({
                    "role": "user",
                    "text": text,
                    "source": "runtime_events",
                    "seq": next_runtime_seq
                }));
                next_runtime_seq += 1;
            }
            "agent.message" => {
                let text = text_from_content(event.get("content"));
                if text.trim().is_empty() {
                    continue;
                }
                let key = event
                    .get("messageID")
                    .or_else(|| event.get("message_id"))
                    .or_else(|| event.get("partID"))
                    .or_else(|| event.get("part_id"))
                    .or_else(|| event.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                if key.is_empty() {
                    transcript.push(json!({
                        "role": "assistant",
                        "text": text,
                        "source": "runtime_events",
                        "seq": next_runtime_seq
                    }));
                    next_runtime_seq += 1;
                    continue;
                }

                let seen_chunks = assistant_chunks.entry(key.clone()).or_default();
                if !seen_chunks.insert(text.clone()) {
                    continue;
                }

                if let Some(index) = assistant_by_key.get(&key).copied() {
                    if let Some(existing) = transcript.get_mut(index) {
                        let current = existing
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        existing["text"] = Value::String(format!("{current}{text}"));
                    }
                    continue;
                }

                assistant_by_key.insert(key, transcript.len());
                transcript.push(json!({
                    "role": "assistant",
                    "text": text,
                    "source": "runtime_events",
                    "seq": next_runtime_seq
                }));
                next_runtime_seq += 1;
            }
            _ => {}
        }
    }

    transcript
}

fn text_from_parts(parts: &Value) -> String {
    parts
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("")
}

fn text_from_content(content: Option<&Value>) -> String {
    let Some(content) = content else {
        return String::new();
    };
    match content {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(text) => Some(text.clone()),
                Value::Object(map) => map
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| {
                        map.get("content")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    }),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}
