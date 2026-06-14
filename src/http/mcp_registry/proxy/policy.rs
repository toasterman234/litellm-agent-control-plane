use std::collections::HashSet;

use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, HeaderValue, StatusCode},
    response::Response,
};
use serde_json::{json, Value};

#[derive(Debug, Default)]
pub(super) struct McpRequest {
    id: Option<Value>,
    method: Option<String>,
    tool_name: Option<String>,
}

pub(super) fn parse_mcp_request(body: &[u8]) -> McpRequest {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return McpRequest::default();
    };
    let Some(obj) = value.as_object() else {
        return McpRequest::default();
    };
    let method = obj.get("method").and_then(Value::as_str).map(str::to_owned);
    let tool_name = (method.as_deref() == Some("tools/call"))
        .then(|| {
            obj.get("params")
                .and_then(|params| params.get("name"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .flatten();
    McpRequest {
        id: obj.get("id").cloned(),
        method,
        tool_name,
    }
}

pub(super) fn allowed_tools(value: &Value) -> HashSet<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(super) fn reject_disallowed_call(
    request: &McpRequest,
    allowed_tools: &HashSet<String>,
) -> Option<Response> {
    let tool_name = request.tool_name.as_deref()?;
    if tool_is_allowed(tool_name, allowed_tools) {
        return None;
    }
    Some(mcp_error_response(
        request.id.clone(),
        "Tool is not allowed for this MCP server",
    ))
}

pub(super) fn should_filter_tools_list(
    request: &McpRequest,
    status: StatusCode,
    allowed_tools: &HashSet<String>,
) -> bool {
    request.method.as_deref() == Some("tools/list")
        && status.is_success()
        && !allowed_tools.is_empty()
}

pub(super) fn filter_tools_list_payload(
    text: &str,
    content_type: &str,
    allowed_tools: &HashSet<String>,
) -> String {
    if content_type.contains("event-stream") || text.starts_with("data:") {
        return filter_event_stream_tools(text, allowed_tools);
    }
    let Ok(mut value) = serde_json::from_str::<Value>(text) else {
        return text.to_owned();
    };
    filter_tools_in_value(&mut value, allowed_tools);
    value.to_string()
}

fn filter_event_stream_tools(text: &str, allowed_tools: &HashSet<String>) -> String {
    text.lines()
        .map(|line| {
            let Some(data) = line.strip_prefix("data:") else {
                return line.to_owned();
            };
            let Ok(mut value) = serde_json::from_str::<Value>(data.trim()) else {
                return line.to_owned();
            };
            filter_tools_in_value(&mut value, allowed_tools);
            format!("data: {value}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn filter_tools_in_value(value: &mut Value, allowed_tools: &HashSet<String>) {
    if let Some(tools) = value
        .pointer_mut("/result/tools")
        .and_then(Value::as_array_mut)
    {
        retain_allowed_tools(tools, allowed_tools);
    }
    if let Some(tools) = value.get_mut("tools").and_then(Value::as_array_mut) {
        retain_allowed_tools(tools, allowed_tools);
    }
}

fn retain_allowed_tools(tools: &mut Vec<Value>, allowed_tools: &HashSet<String>) {
    tools.retain(|tool| {
        tool.get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| tool_is_allowed(name, allowed_tools))
    });
}

fn tool_is_allowed(tool_name: &str, allowed_tools: &HashSet<String>) -> bool {
    allowed_tools.is_empty() || allowed_tools.contains(tool_name)
}

fn mcp_error_response(id: Option<Value>, message: &str) -> Response {
    let body = json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": {
            "code": -32602,
            "message": message,
        },
    });
    let mut response = Response::new(Body::from(body.to_string()));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
}
