use serde_json::Value;

use crate::db::managed_agents::registry::schema::ManagedAgentRow;

pub(super) fn optional_str<'a>(arguments: &'a Value, field: &str) -> Option<&'a str> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn allowed_dm_user_ids(arguments: &Value) -> Vec<String> {
    let values = arguments
        .get("allowed_dm_user_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .chain(
            arguments
                .get("allowed_dm_user_ids")
                .and_then(Value::as_str)
                .into_iter()
                .flat_map(|value| value.split([',', '\n', ' ', '\t'])),
        );
    let mut ids = Vec::new();
    for value in values {
        let id = value
            .trim()
            .trim_start_matches("<@")
            .trim_end_matches('>')
            .trim();
        if id.is_empty() || ids.iter().any(|existing| existing == id) {
            continue;
        }
        ids.push(id.to_owned());
    }
    ids
}

pub(super) fn child_allowed_dm_user_ids(child: &ManagedAgentRow) -> Vec<String> {
    child
        .config
        .get("slack")
        .and_then(|slack| slack.get("allowed_dm_user_ids"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
