use sqlx::PgPool;

use crate::{
    db::managed_agents::{id, now_ms},
    errors::GatewayError,
};

const EVENT_PROCESSING_TIMEOUT_MS: i64 = 5 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EventClaim {
    Claimed {
        claim_id: String,
        session_id: Option<String>,
    },
    Completed {
        session_id: String,
    },
    InProgress,
}

pub(crate) async fn claim_event(
    pool: &PgPool,
    agent_id: &str,
    event_id: &str,
) -> Result<EventClaim, GatewayError> {
    let mut tx = pool.begin().await.map_err(GatewayError::Database)?;
    let now = now_ms();
    let claim_id = id("whclaim");
    let claim = if insert_event(tx.as_mut(), agent_id, event_id, &claim_id, now).await? {
        EventClaim::Claimed {
            claim_id,
            session_id: None,
        }
    } else {
        claim_existing_event(tx.as_mut(), agent_id, event_id, &claim_id, now).await?
    };
    tx.commit().await.map_err(GatewayError::Database)?;
    Ok(claim)
}

async fn claim_existing_event(
    conn: &mut sqlx::PgConnection,
    agent_id: &str,
    event_id: &str,
    claim_id: &str,
    now: i64,
) -> Result<EventClaim, GatewayError> {
    let (session_id, status, updated_at) = sqlx::query_as::<_, (Option<String>, String, i64)>(
        r#"
        SELECT session_id, status, updated_at
        FROM "LiteLLM_ManagedAgentWebhookEventsTable"
        WHERE agent_id = $1 AND event_id = $2
        FOR UPDATE
        "#,
    )
    .bind(agent_id)
    .bind(event_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(GatewayError::Database)?;

    if status == "completed" {
        if let Some(session_id) = session_id {
            return Ok(EventClaim::Completed { session_id });
        }
    }
    if let Some(session_id) = session_id.as_deref() {
        if session_has_user_message(conn, session_id).await? {
            update_event_status(conn, agent_id, event_id, "completed", now).await?;
            return Ok(EventClaim::Completed {
                session_id: session_id.to_owned(),
            });
        }
    }
    if now - updated_at < EVENT_PROCESSING_TIMEOUT_MS {
        return Ok(EventClaim::InProgress);
    }
    if let Some(session_id) = session_id.as_deref() {
        update_event_claim(conn, agent_id, event_id, claim_id, "session_created", now).await?;
        return Ok(EventClaim::Claimed {
            claim_id: claim_id.to_owned(),
            session_id: Some(session_id.to_owned()),
        });
    }
    update_event_claim(conn, agent_id, event_id, claim_id, "processing", now).await?;
    Ok(EventClaim::Claimed {
        claim_id: claim_id.to_owned(),
        session_id: None,
    })
}

async fn insert_event(
    conn: &mut sqlx::PgConnection,
    agent_id: &str,
    event_id: &str,
    claim_id: &str,
    now: i64,
) -> Result<bool, GatewayError> {
    let result = sqlx::query(
        r#"
        INSERT INTO "LiteLLM_ManagedAgentWebhookEventsTable"
          (agent_id, event_id, claim_id, status, created_at, updated_at)
        VALUES ($1, $2, $3, 'processing', $4, $4)
        ON CONFLICT (agent_id, event_id) DO NOTHING
        "#,
    )
    .bind(agent_id)
    .bind(event_id)
    .bind(claim_id)
    .bind(now)
    .execute(conn)
    .await
    .map_err(GatewayError::Database)?;
    Ok(result.rows_affected() == 1)
}

async fn update_event_claim(
    conn: &mut sqlx::PgConnection,
    agent_id: &str,
    event_id: &str,
    claim_id: &str,
    status: &str,
    now: i64,
) -> Result<(), GatewayError> {
    sqlx::query(
        r#"
        UPDATE "LiteLLM_ManagedAgentWebhookEventsTable"
        SET claim_id = $3, status = $4, updated_at = $5
        WHERE agent_id = $1 AND event_id = $2
        "#,
    )
    .bind(agent_id)
    .bind(event_id)
    .bind(claim_id)
    .bind(status)
    .bind(now)
    .execute(conn)
    .await
    .map_err(GatewayError::Database)?;
    Ok(())
}

async fn update_event_status(
    conn: &mut sqlx::PgConnection,
    agent_id: &str,
    event_id: &str,
    status: &str,
    now: i64,
) -> Result<(), GatewayError> {
    sqlx::query(
        r#"
        UPDATE "LiteLLM_ManagedAgentWebhookEventsTable"
        SET status = $3, updated_at = $4
        WHERE agent_id = $1 AND event_id = $2
        "#,
    )
    .bind(agent_id)
    .bind(event_id)
    .bind(status)
    .bind(now)
    .execute(conn)
    .await
    .map_err(GatewayError::Database)?;
    Ok(())
}

pub(crate) async fn attach_event_session(
    pool: &PgPool,
    agent_id: &str,
    event_id: &str,
    claim_id: &str,
    session_id: &str,
) -> Result<bool, GatewayError> {
    let result = sqlx::query(
        r#"
        UPDATE "LiteLLM_ManagedAgentWebhookEventsTable"
        SET session_id = $4, status = 'session_created', updated_at = $5
        WHERE agent_id = $1
          AND event_id = $2
          AND claim_id = $3
          AND status = 'processing'
          AND session_id IS NULL
        "#,
    )
    .bind(agent_id)
    .bind(event_id)
    .bind(claim_id)
    .bind(session_id)
    .bind(now_ms())
    .execute(pool)
    .await
    .map_err(GatewayError::Database)?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn begin_event_enqueue(
    pool: &PgPool,
    agent_id: &str,
    event_id: &str,
    claim_id: &str,
    session_id: &str,
) -> Result<bool, GatewayError> {
    let result = sqlx::query(
        r#"
        UPDATE "LiteLLM_ManagedAgentWebhookEventsTable"
        SET status = 'enqueuing', updated_at = $5
        WHERE agent_id = $1
          AND event_id = $2
          AND claim_id = $3
          AND session_id = $4
          AND status = 'session_created'
        "#,
    )
    .bind(agent_id)
    .bind(event_id)
    .bind(claim_id)
    .bind(session_id)
    .bind(now_ms())
    .execute(pool)
    .await
    .map_err(GatewayError::Database)?;
    Ok(result.rows_affected() == 1)
}

async fn session_has_user_message(
    conn: &mut sqlx::PgConnection,
    session_id: &str,
) -> Result<bool, GatewayError> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM "LiteLLM_ManagedAgentSessionMessagesTable"
          WHERE session_id = $1
            AND info_json::jsonb->>'role' = 'user'
        )
        "#,
    )
    .bind(session_id)
    .fetch_one(conn)
    .await
    .map_err(GatewayError::Database)
}

pub(crate) async fn complete_event(
    pool: &PgPool,
    agent_id: &str,
    event_id: &str,
    claim_id: &str,
) -> Result<bool, GatewayError> {
    let result = sqlx::query(
        r#"
        UPDATE "LiteLLM_ManagedAgentWebhookEventsTable"
        SET status = 'completed', updated_at = $4
        WHERE agent_id = $1
          AND event_id = $2
          AND claim_id = $3
          AND status = 'enqueuing'
        "#,
    )
    .bind(agent_id)
    .bind(event_id)
    .bind(claim_id)
    .bind(now_ms())
    .execute(pool)
    .await
    .map_err(GatewayError::Database)?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn delete_event(
    pool: &PgPool,
    agent_id: &str,
    event_id: &str,
) -> Result<(), GatewayError> {
    sqlx::query(
        r#"
        DELETE FROM "LiteLLM_ManagedAgentWebhookEventsTable"
        WHERE agent_id = $1 AND event_id = $2
        "#,
    )
    .bind(agent_id)
    .bind(event_id)
    .execute(pool)
    .await
    .map_err(GatewayError::Database)?;
    Ok(())
}
