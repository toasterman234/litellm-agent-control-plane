use sqlx::PgPool;

use crate::{db::managed_agents::now_ms, errors::GatewayError};

use super::schema::GoogleChatSpaceSessionRow;

pub async fn get(
    pool: &PgPool,
    agent_id: &str,
    conversation_key: &str,
) -> Result<Option<GoogleChatSpaceSessionRow>, GatewayError> {
    sqlx::query_as::<_, GoogleChatSpaceSessionRow>(
        r#"
        SELECT *
        FROM "LiteLLM_ManagedAgentGoogleChatSpaceSessionsTable"
        WHERE agent_id = $1 AND conversation_key = $2
        "#,
    )
    .bind(agent_id)
    .bind(conversation_key)
    .fetch_optional(pool)
    .await
    .map_err(GatewayError::Database)
}

pub struct UpsertSessionInput<'a> {
    pub agent_id: &'a str,
    pub conversation_key: &'a str,
    pub session_id: &'a str,
    pub space_name: &'a str,
    pub thread_name: Option<&'a str>,
}

pub async fn upsert(
    pool: &PgPool,
    input: UpsertSessionInput<'_>,
) -> Result<GoogleChatSpaceSessionRow, GatewayError> {
    let now = now_ms();
    sqlx::query_as::<_, GoogleChatSpaceSessionRow>(
        r#"
        INSERT INTO "LiteLLM_ManagedAgentGoogleChatSpaceSessionsTable"
          (agent_id, conversation_key, session_id, space_name, thread_name, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $6)
        ON CONFLICT (agent_id, conversation_key) DO UPDATE SET
          space_name = EXCLUDED.space_name,
          thread_name = EXCLUDED.thread_name,
          updated_at = EXCLUDED.updated_at
        RETURNING *
        "#,
    )
    .bind(input.agent_id)
    .bind(input.conversation_key)
    .bind(input.session_id)
    .bind(input.space_name)
    .bind(input.thread_name)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(GatewayError::Database)
}

pub async fn record_event(
    pool: &PgPool,
    agent_id: &str,
    event_id: &str,
) -> Result<bool, GatewayError> {
    let result = sqlx::query(
        r#"
        INSERT INTO "LiteLLM_ManagedAgentGoogleChatEventsTable" (agent_id, event_id, created_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (agent_id, event_id) DO NOTHING
        "#,
    )
    .bind(agent_id)
    .bind(event_id)
    .bind(now_ms())
    .execute(pool)
    .await
    .map_err(GatewayError::Database)?;
    Ok(result.rows_affected() == 1)
}
