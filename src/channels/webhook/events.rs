use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde_json::Value;
use tracing::warn;

use crate::{
    db::managed_agents::{registry::schema::ManagedAgentRow, sessions},
    errors::GatewayError,
    http::sessions::{create_runtime_session_for_agent_without_prompt, enqueue_prompt_text},
    proxy::state::AppState,
};

use super::{
    auth::verify_webhook_secret,
    config::{agent_runtime, load_agent, load_webhook_secret, webhook_config},
    metadata::{request_id, session_metadata, session_title},
    repository::{self, EventClaim},
    types::WebhookAcceptedResponse,
};

pub(crate) async fn events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<WebhookAcceptedResponse>), GatewayError> {
    let pool = state
        .db
        .as_ref()
        .ok_or(GatewayError::MissingDatabase)?
        .clone();
    let agent = load_agent(&pool, &agent_id).await?;
    let config = webhook_config(&agent)?;
    let secret = load_webhook_secret(&state, &agent.id, &config).await?;
    verify_webhook_secret(&headers, &secret)?;

    let request_id = request_id(&headers);
    let (session_id, claim_id) = match claimed_session(
        state.clone(),
        &pool,
        &agent,
        &headers,
        &agent_id,
        &request_id,
    )
    .await?
    {
        ClaimedWebhookSession::Ready {
            session_id,
            claim_id,
        } => (session_id, claim_id),
        ClaimedWebhookSession::Response(response) => return Ok(response),
    };
    let prompt = webhook_prompt(&payload)?;
    let model = agent.model.clone();
    if let Err(error) = enqueue_prompt_text(state, pool.clone(), &session_id, prompt, model).await {
        cleanup_event_session(&pool, &agent.id, &request_id, &session_id).await;
        return Err(error);
    }
    if !repository::complete_event(&pool, &agent.id, &request_id, &claim_id).await? {
        return webhook_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "processing",
            agent_id,
            String::new(),
            request_id,
        );
    }

    webhook_response(
        StatusCode::ACCEPTED,
        "accepted",
        agent_id,
        session_id,
        request_id,
    )
}

enum ClaimedWebhookSession {
    Ready {
        session_id: String,
        claim_id: String,
    },
    Response((StatusCode, Json<WebhookAcceptedResponse>)),
}

async fn claimed_session(
    state: Arc<AppState>,
    pool: &sqlx::PgPool,
    agent: &ManagedAgentRow,
    headers: &HeaderMap,
    response_agent_id: &str,
    request_id: &str,
) -> Result<ClaimedWebhookSession, GatewayError> {
    let claim = repository::claim_event(pool, &agent.id, request_id).await?;
    match claim {
        EventClaim::Completed { session_id } => {
            Ok(ClaimedWebhookSession::Response(webhook_response(
                StatusCode::ACCEPTED,
                "duplicate",
                response_agent_id.to_owned(),
                session_id,
                request_id.to_owned(),
            )?))
        }
        EventClaim::InProgress => Ok(ClaimedWebhookSession::Response(webhook_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "processing",
            response_agent_id.to_owned(),
            String::new(),
            request_id.to_owned(),
        )?)),
        EventClaim::Claimed {
            claim_id,
            session_id,
        } => {
            let context = ClaimedSessionContext {
                state,
                pool,
                agent,
                headers,
                response_agent_id,
                request_id,
            };
            prepare_claimed_session(context, claim_id, session_id).await
        }
    }
}

struct ClaimedSessionContext<'a> {
    state: Arc<AppState>,
    pool: &'a sqlx::PgPool,
    agent: &'a ManagedAgentRow,
    headers: &'a HeaderMap,
    response_agent_id: &'a str,
    request_id: &'a str,
}

async fn prepare_claimed_session(
    context: ClaimedSessionContext<'_>,
    claim_id: String,
    session_id: Option<String>,
) -> Result<ClaimedWebhookSession, GatewayError> {
    let Some(session_id) = claim_session(
        context.state.clone(),
        context.pool,
        context.agent,
        context.headers,
        context.request_id,
        &claim_id,
        session_id,
    )
    .await?
    else {
        return processing_claim_response(context.response_agent_id, context.request_id);
    };

    if repository::begin_event_enqueue(
        context.pool,
        &context.agent.id,
        context.request_id,
        &claim_id,
        &session_id,
    )
    .await?
    {
        Ok(ClaimedWebhookSession::Ready {
            session_id,
            claim_id,
        })
    } else {
        processing_claim_response(context.response_agent_id, context.request_id)
    }
}

async fn claim_session(
    state: Arc<AppState>,
    pool: &sqlx::PgPool,
    agent: &ManagedAgentRow,
    headers: &HeaderMap,
    request_id: &str,
    claim_id: &str,
    session_id: Option<String>,
) -> Result<Option<String>, GatewayError> {
    if let Some(session_id) = session_id {
        return Ok(Some(session_id));
    }
    let session_id = create_session(state, pool, agent, headers, request_id).await?;
    match repository::attach_event_session(pool, &agent.id, request_id, claim_id, &session_id).await
    {
        Ok(true) => Ok(Some(session_id)),
        Ok(false) => {
            cleanup_created_session(pool, &session_id).await;
            Ok(None)
        }
        Err(error) => {
            cleanup_event_session(pool, &agent.id, request_id, &session_id).await;
            Err(error)
        }
    }
}

fn processing_claim_response(
    response_agent_id: &str,
    request_id: &str,
) -> Result<ClaimedWebhookSession, GatewayError> {
    Ok(ClaimedWebhookSession::Response(webhook_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "processing",
        response_agent_id.to_owned(),
        String::new(),
        request_id.to_owned(),
    )?))
}

async fn create_session(
    state: Arc<AppState>,
    pool: &sqlx::PgPool,
    agent: &ManagedAgentRow,
    headers: &HeaderMap,
    request_id: &str,
) -> Result<String, GatewayError> {
    match create_runtime_session_for_agent_without_prompt(
        state,
        pool,
        agent.id.clone(),
        agent_runtime(agent),
        session_title(request_id),
        session_metadata(headers, request_id),
    )
    .await
    {
        Ok(session_id) => Ok(session_id),
        Err(error) => {
            let _ = repository::delete_event(pool, &agent.id, request_id).await;
            Err(error)
        }
    }
}

fn webhook_response(
    status_code: StatusCode,
    status: &'static str,
    agent_id: String,
    session_id: String,
    request_id: String,
) -> Result<(StatusCode, Json<WebhookAcceptedResponse>), GatewayError> {
    Ok((
        status_code,
        Json(WebhookAcceptedResponse {
            status,
            agent_id,
            session_id,
            request_id,
        }),
    ))
}

async fn cleanup_event_session(
    pool: &sqlx::PgPool,
    agent_id: &str,
    request_id: &str,
    session_id: &str,
) {
    let _ = repository::delete_event(pool, agent_id, request_id).await;
    cleanup_created_session(pool, session_id).await;
}

async fn cleanup_created_session(pool: &sqlx::PgPool, session_id: &str) {
    if let Err(error) = sessions::repository::delete(pool, session_id).await {
        warn!("webhook session cleanup failed: {error}");
    }
}

fn webhook_prompt(payload: &Value) -> Result<String, GatewayError> {
    Ok(serde_json::to_string_pretty(payload)?)
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
