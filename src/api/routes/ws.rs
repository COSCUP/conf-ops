use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::api::middleware::auth::AuthUser;
use crate::app_state::AppState;
use crate::modules::auth::repository::AccountRepository;
use crate::modules::conversation::awareness::{AwarenessManager, AwarenessState};
use crate::modules::conversation::crdt::CrdtManager;
use crate::modules::conversation::repository::MessageRepository;
use crate::modules::conversation::ws_manager::{
    AwarenessEntry, AwarenessStatePayload, ClientWsMessage, ServerWsMessage, WsManager,
};
use crate::modules::core::member::repository::MemberRepository;

/// Bundled context for a WebSocket connection handler.
struct WsConnectionContext {
    task_id: Uuid,
    member_id: Uuid,
    display_name: String,
    sender: Arc<broadcast::Sender<ServerWsMessage>>,
    receiver: broadcast::Receiver<ServerWsMessage>,
    pool: sqlx::PgPool,
    crdt_manager: Arc<CrdtManager>,
    awareness_manager: Arc<AwarenessManager>,
    ws_manager: Arc<WsManager>,
    heartbeat_interval: Duration,
    idle_timeout: Duration,
}

/// Shared references for handling individual client messages.
struct ClientMessageContext<'a> {
    task_id: Uuid,
    member_id: Uuid,
    display_name: &'a str,
    pool: &'a sqlx::PgPool,
    crdt_manager: &'a Arc<CrdtManager>,
    awareness_manager: &'a Arc<AwarenessManager>,
    sender: &'a Arc<broadcast::Sender<ServerWsMessage>>,
}

/// One-time WebSocket token entry.
struct WsTokenEntry {
    member_id: Uuid,
    display_name: String,
    task_id: Uuid,
    created_at: std::time::Instant,
}

/// Store for one-time WebSocket tokens.
pub struct WsTokenStore {
    tokens: DashMap<String, WsTokenEntry>,
}

impl WsTokenStore {
    pub fn new() -> Self {
        Self {
            tokens: DashMap::new(),
        }
    }

    /// Insert a token. Returns the generated token string.
    pub fn insert(&self, member_id: Uuid, display_name: String, task_id: Uuid) -> String {
        let token = format!("{}-{}", Uuid::now_v7(), Uuid::now_v7());
        self.tokens.insert(
            token.clone(),
            WsTokenEntry {
                member_id,
                display_name,
                task_id,
                created_at: std::time::Instant::now(),
            },
        );
        token
    }

    /// Consume a token (one-time use). Returns `None` if expired or not found.
    fn consume(&self, token: &str, expected_task_id: Uuid) -> Option<WsTokenEntry> {
        let (_, entry) = self.tokens.remove(token)?;
        // 30-second expiry
        if entry.created_at.elapsed() > Duration::from_secs(30) {
            return None;
        }
        if entry.task_id != expected_task_id {
            return None;
        }
        Some(entry)
    }

    /// Clean up expired tokens (call periodically).
    pub fn cleanup_expired(&self) {
        self.tokens
            .retain(|_, entry| entry.created_at.elapsed() <= Duration::from_secs(30));
    }
}

impl Default for WsTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

// ── Request / Response types ────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WsTokenResponse {
    token: String,
}

#[derive(Deserialize)]
pub struct WsQueryParams {
    token: String,
}

// ── Handlers ────────────────────────────────────────────────────

/// `POST /{taskId}/conversation/ws-token`
///
/// Generates a one-time WebSocket token (requires auth).
///
/// # Errors
///
/// Returns `ProblemDetails` on auth failure or database errors.
pub async fn create_ws_token(
    State(state): State<AppState>,
    Path((project_id, task_id)): Path<(Uuid, Uuid)>,
    user: AuthUser,
) -> Result<impl IntoResponse, ProblemDetails> {
    // Verify project membership
    let member =
        MemberRepository::get_by_project_and_account(&state.pool, project_id, user.account_id)
            .await
            .map_err(|_| ProblemDetails::new(StatusCode::FORBIDDEN, "Not a project member"))?
            .ok_or_else(|| ProblemDetails::new(StatusCode::FORBIDDEN, "Not a project member"))?;

    // Get account name for awareness display
    let account = AccountRepository::get_by_id(&state.pool, user.account_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {e}");
            ProblemDetails::internal_server_error()
        })?;

    let token = state
        .ws_token_store
        .insert(member.id, account.name, task_id);

    Ok((StatusCode::CREATED, Json(WsTokenResponse { token })))
}

/// `GET /{taskId}/conversation/ws?token={token}`
///
/// WebSocket upgrade endpoint. Must be outside auth middleware.
///
/// # Errors
///
/// Returns `ProblemDetails` on invalid token or capacity limit.
pub async fn ws_upgrade(
    State(state): State<AppState>,
    Path((_project_id, task_id)): Path<(Uuid, Uuid)>,
    Query(params): Query<WsQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ProblemDetails> {
    let entry = state
        .ws_token_store
        .consume(&params.token, task_id)
        .ok_or_else(|| ProblemDetails::new(StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

    // Join room
    let (sender, receiver) = state.ws_manager.join(task_id).ok_or_else(|| {
        ProblemDetails::new(StatusCode::SERVICE_UNAVAILABLE, "Room is at capacity")
    })?;

    let pool = state.pool.clone();
    let crdt_manager = Arc::clone(state.conversation_service.crdt_manager());
    let awareness_manager = Arc::clone(&state.awareness_manager);
    let ws_manager = Arc::clone(&state.ws_manager);
    let heartbeat_interval = Duration::from_secs(state.crdt_ws_heartbeat_interval_secs);
    let idle_timeout = Duration::from_secs(state.crdt_ws_idle_timeout_secs);

    let ctx = WsConnectionContext {
        task_id,
        member_id: entry.member_id,
        display_name: entry.display_name,
        sender,
        receiver,
        pool,
        crdt_manager,
        awareness_manager,
        ws_manager,
        heartbeat_interval,
        idle_timeout,
    };

    Ok(ws.on_upgrade(move |socket| handle_ws_connection(socket, ctx)))
}

async fn handle_ws_connection(socket: WebSocket, ctx: WsConnectionContext) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Set initial awareness
    ctx.awareness_manager.update(
        ctx.task_id,
        AwarenessState {
            member_id: ctx.member_id,
            display_name: ctx.display_name.clone(),
            color: member_color(ctx.member_id),
            cursor_position: None,
            is_typing: false,
            last_active: Utc::now(),
        },
    );

    // Broadcast awareness to room
    broadcast_awareness(&ctx.awareness_manager, &ctx.sender, ctx.task_id);

    let mut heartbeat = tokio::time::interval(ctx.heartbeat_interval);
    let mut last_activity = tokio::time::Instant::now();
    let mut receiver = ctx.receiver;

    loop {
        tokio::select! {
            // Incoming message from client
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        last_activity = tokio::time::Instant::now();
                        let msg_ctx = ClientMessageContext {
                            task_id: ctx.task_id,
                            member_id: ctx.member_id,
                            display_name: &ctx.display_name,
                            pool: &ctx.pool,
                            crdt_manager: &ctx.crdt_manager,
                            awareness_manager: &ctx.awareness_manager,
                            sender: &ctx.sender,
                        };
                        handle_client_binary(&data, &msg_ctx, &mut ws_sender).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        last_activity = tokio::time::Instant::now();
                        let _ = ws_sender.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            // Broadcast from room
            msg = receiver.recv() => {
                if let Ok(server_msg) = msg {
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        let _ = ws_sender.send(Message::Text(json.into())).await;
                    }
                }
            }
            // Heartbeat ping
            _ = heartbeat.tick() => {
                if last_activity.elapsed() > ctx.idle_timeout {
                    let _ = ws_sender.send(Message::Close(None)).await;
                    break;
                }
                let _ = ws_sender.send(Message::Ping(vec![].into())).await;
            }
        }
    }

    // Cleanup
    ctx.awareness_manager.remove(ctx.task_id, ctx.member_id);
    ctx.ws_manager.leave(ctx.task_id);
    broadcast_awareness(&ctx.awareness_manager, &ctx.sender, ctx.task_id);
}

async fn handle_client_binary(
    data: &[u8],
    ctx: &ClientMessageContext<'_>,
    ws_sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) {
    let Ok(msg) = serde_json::from_slice::<ClientWsMessage>(data) else {
        return;
    };

    match msg {
        ClientWsMessage::SyncStep1 { state_vector } => {
            if let Ok(diff) = ctx
                .crdt_manager
                .encode_state_as_update(ctx.task_id, &state_vector)
                .await
            {
                let response = ServerWsMessage::SyncDiff { update: diff };
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = ws_sender.send(Message::Text(json.into())).await;
                }
            }
        }
        ClientWsMessage::SyncStep2 { update } => {
            if let Ok(broadcast_update) = ctx
                .crdt_manager
                .apply_update(ctx.task_id, &update, ctx.member_id)
                .await
            {
                let _ = ctx.sender.send(ServerWsMessage::PeerUpdate {
                    update: broadcast_update,
                });
            }
        }
        ClientWsMessage::AwarenessUpdate { state } => {
            update_awareness_from_payload(
                ctx.awareness_manager,
                ctx.task_id,
                ctx.member_id,
                ctx.display_name,
                &state,
            );
            broadcast_awareness(ctx.awareness_manager, ctx.sender, ctx.task_id);
        }
        ClientWsMessage::WriteOperation {
            update,
            last_seen_message_id,
        } => {
            // Validate lastSeenMessageId before applying the CRDT update
            let stale = check_ws_stale(ctx.pool, ctx.task_id, last_seen_message_id).await;
            if let Some((latest_id, _count)) = stale {
                let _ = ws_sender
                    .send(Message::Text(
                        serde_json::to_string(&ServerWsMessage::WriteRejected {
                            reason: "Stale conversation".to_string(),
                            latest_message_id: latest_id,
                        })
                        .unwrap_or_default()
                        .into(),
                    ))
                    .await;
                return;
            }

            if let Ok(broadcast_update) = ctx
                .crdt_manager
                .apply_update(ctx.task_id, &update, ctx.member_id)
                .await
            {
                let _ = ctx.sender.send(ServerWsMessage::PeerUpdate {
                    update: broadcast_update,
                });
            }
        }
    }
}

/// Check if a conversation is stale relative to `last_seen_message_id`.
///
/// Returns `Some((latest_message_id, unseen_count))` if stale, `None` if up-to-date.
async fn check_ws_stale(
    pool: &sqlx::PgPool,
    task_id: Uuid,
    last_seen_message_id: Uuid,
) -> Option<(Uuid, i64)> {
    let latest = MessageRepository::get_latest_message_id(pool, task_id)
        .await
        .ok()?;
    let latest_id = latest?;
    if latest_id == last_seen_message_id {
        return None;
    }
    let unseen_count = MessageRepository::count_after_message(pool, task_id, last_seen_message_id)
        .await
        .unwrap_or(1);
    Some((latest_id, unseen_count))
}

fn update_awareness_from_payload(
    awareness_manager: &Arc<AwarenessManager>,
    task_id: Uuid,
    member_id: Uuid,
    display_name: &str,
    payload: &AwarenessStatePayload,
) {
    awareness_manager.update(
        task_id,
        AwarenessState {
            member_id,
            display_name: display_name.to_string(),
            color: member_color(member_id),
            cursor_position: payload.cursor_position,
            is_typing: payload.is_typing,
            last_active: Utc::now(),
        },
    );
}

fn broadcast_awareness(
    awareness_manager: &Arc<AwarenessManager>,
    sender: &Arc<broadcast::Sender<ServerWsMessage>>,
    task_id: Uuid,
) {
    let states = awareness_manager.get_all(task_id);
    let entries: Vec<AwarenessEntry> = states
        .into_iter()
        .map(|s| AwarenessEntry {
            member_id: s.member_id,
            display_name: s.display_name,
            color: s.color,
            cursor_position: s.cursor_position,
            is_typing: s.is_typing,
        })
        .collect();
    let _ = sender.send(ServerWsMessage::AwarenessChange { entries });
}

/// Generate a deterministic color from a member ID.
fn member_color(member_id: Uuid) -> String {
    let bytes = member_id.as_bytes();
    let hue = u16::from(bytes[0]) * 360 / 256;
    format!("hsl({hue}, 70%, 50%)")
}
