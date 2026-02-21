mod common;

use common::TestContext;
use conf_ops::id::generate_id;
use conf_ops::modules::conversation::ws_manager::ClientWsMessage;
use conf_ops::modules::core::member::models::MemberRole;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite;

struct TaskSetup {
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
    member_id: uuid::Uuid,
}

async fn setup_task(ctx: &TestContext) -> TaskSetup {
    let (account_id, _email) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "ws-tag").await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;
    let template_id = ctx
        .create_test_task_template(project_id, "WS Template", account_id)
        .await;
    ctx.link_tag_to_template(template_id, tag_id).await;

    let token = common::TestContext::issue_test_token(account_id);
    let app = common::build_app(ctx);
    let (status, body) = common::json_request(
        app,
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks"),
        &token,
        Some(serde_json::json!({
            "taskTemplateId": template_id,
            "ownerTagId": tag_id,
            "name": "WS Test Task"
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    let task_id = body["id"]
        .as_str()
        .expect("task id")
        .parse::<uuid::Uuid>()
        .expect("parse uuid");

    TaskSetup {
        project_id,
        task_id,
        member_id,
    }
}

/// Create a second member in the same project for multi-client tests.
async fn create_second_member(ctx: &TestContext, project_id: uuid::Uuid) -> uuid::Uuid {
    let (account_id2, _) = ctx.create_test_account().await;
    let org_id =
        conf_ops::modules::core::project::repository::ProjectRepository::get_organization_id(
            &ctx.pool, project_id,
        )
        .await
        .expect("get org_id");
    let org_member_id = generate_id();
    conf_ops::modules::core::organization::repository::OrgMemberRepository::create(
        &ctx.pool,
        org_member_id,
        org_id,
        account_id2,
        conf_ops::modules::core::organization::models::OrgRole::OrgMember,
    )
    .await
    .expect("create org member");
    ctx.create_test_member(project_id, account_id2, MemberRole::Member)
        .await
}

/// Connect a WS client to the test server.
async fn connect_ws(
    addr: std::net::SocketAddr,
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
    ws_token: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let url = format!(
        "ws://{addr}/api/v1/projects/{project_id}/tasks/{task_id}/conversation/ws?token={ws_token}"
    );
    let (stream, _response) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("WS connect");
    stream
}

/// Read the next text message from a WS stream with a timeout, skipping Ping/Pong frames.
async fn recv_text(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> serde_json::Value {
    loop {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
            .await
            .expect("timeout")
            .expect("stream ended")
            .expect("ws error");
        match msg {
            tungstenite::Message::Text(t) => {
                return serde_json::from_str(&t).expect("parse json");
            }
            tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_) => continue,
            other => panic!("expected text, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn ws_connect_and_receive_awareness() {
    let ctx = TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let (addr, state) = common::start_test_server(&ctx).await;

    let ws_token =
        state
            .ws_token_store
            .insert(setup.member_id, "Test User".to_string(), setup.task_id);
    let mut ws = connect_ws(addr, setup.project_id, setup.task_id, &ws_token).await;

    let parsed = recv_text(&mut ws).await;
    assert_eq!(parsed["type"], "AwarenessChange");
    let entries = parsed["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 1, "should have one awareness entry");
    assert!(entries[0]["member_id"].is_string());
    assert!(entries[0]["display_name"].is_string());

    ws.close(None).await.ok();
}

#[tokio::test]
async fn ws_invalid_token_rejected() {
    let ctx = TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let (addr, _state) = common::start_test_server(&ctx).await;

    let url = format!(
        "ws://{addr}/api/v1/projects/{}/tasks/{}/conversation/ws?token=invalid-token",
        setup.project_id, setup.task_id
    );
    let result = tokio_tungstenite::connect_async(&url).await;
    assert!(result.is_err(), "connection with invalid token should fail");
}

#[tokio::test]
async fn ws_token_is_one_time_use() {
    let ctx = TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let (addr, state) = common::start_test_server(&ctx).await;

    let ws_token =
        state
            .ws_token_store
            .insert(setup.member_id, "Test User".to_string(), setup.task_id);

    // First connection should succeed
    let mut ws1 = connect_ws(addr, setup.project_id, setup.task_id, &ws_token).await;
    ws1.close(None).await.ok();

    // Second connection with same token should fail (consumed)
    let url = format!(
        "ws://{addr}/api/v1/projects/{}/tasks/{}/conversation/ws?token={ws_token}",
        setup.project_id, setup.task_id
    );
    let result = tokio_tungstenite::connect_async(&url).await;
    assert!(result.is_err(), "reusing a consumed token should fail");
}

#[tokio::test]
async fn ws_two_clients_awareness_broadcast() {
    let ctx = TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let (addr, state) = common::start_test_server(&ctx).await;

    // Client 1
    let ws_token1 =
        state
            .ws_token_store
            .insert(setup.member_id, "User 1".to_string(), setup.task_id);
    let mut ws1 = connect_ws(addr, setup.project_id, setup.task_id, &ws_token1).await;
    let _ = recv_text(&mut ws1).await; // drain initial awareness

    // Client 2
    let member_id2 = create_second_member(&ctx, setup.project_id).await;
    let ws_token2 = state
        .ws_token_store
        .insert(member_id2, "User 2".to_string(), setup.task_id);
    let mut ws2 = connect_ws(addr, setup.project_id, setup.task_id, &ws_token2).await;

    // Client 1 should receive awareness update with 2 entries
    let parsed = recv_text(&mut ws1).await;
    assert_eq!(parsed["type"], "AwarenessChange");
    let entries = parsed["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 2, "should have two awareness entries");

    ws1.close(None).await.ok();
    ws2.close(None).await.ok();
}

#[tokio::test]
async fn ws_crdt_sync_between_clients() {
    let ctx = TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let (addr, state) = common::start_test_server(&ctx).await;

    // Client 1
    let ws_token1 =
        state
            .ws_token_store
            .insert(setup.member_id, "User 1".to_string(), setup.task_id);
    let mut ws1 = connect_ws(addr, setup.project_id, setup.task_id, &ws_token1).await;
    let _ = recv_text(&mut ws1).await; // drain initial awareness

    // Client 2
    let member_id2 = create_second_member(&ctx, setup.project_id).await;
    let ws_token2 = state
        .ws_token_store
        .insert(member_id2, "User 2".to_string(), setup.task_id);
    let mut ws2 = connect_ws(addr, setup.project_id, setup.task_id, &ws_token2).await;

    // Drain awareness messages from both
    let _ = recv_text(&mut ws1).await;
    let _ = recv_text(&mut ws2).await;

    // Small delay to let awareness settle
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Client 1 sends a CRDT update
    let doc = yrs::Doc::new();
    let array = doc.get_or_insert_array("test-data");
    {
        use yrs::{Array, Transact};
        let mut txn = doc.transact_mut();
        array.push_back(&mut txn, "hello-from-client1");
    }
    let update = {
        use yrs::{ReadTxn, Transact};
        doc.transact()
            .encode_state_as_update_v1(&yrs::StateVector::default())
    };

    let sync_msg = ClientWsMessage::SyncStep2 { update };
    ws1.send(tungstenite::Message::Binary(
        serde_json::to_vec(&sync_msg).expect("serialize").into(),
    ))
    .await
    .expect("send sync");

    // Client 2 should receive a PeerUpdate (skip any remaining awareness messages)
    let parsed = loop {
        let msg = recv_text(&mut ws2).await;
        if msg["type"] == "PeerUpdate" {
            break msg;
        }
    };
    assert!(parsed["update"].is_array(), "update should be a byte array");

    ws1.close(None).await.ok();
    ws2.close(None).await.ok();
}

#[tokio::test]
async fn ws_awareness_typing_state() {
    let ctx = TestContext::new().await;
    let setup = setup_task(&ctx).await;
    let (addr, state) = common::start_test_server(&ctx).await;

    // Client 1
    let ws_token1 =
        state
            .ws_token_store
            .insert(setup.member_id, "User 1".to_string(), setup.task_id);
    let mut ws1 = connect_ws(addr, setup.project_id, setup.task_id, &ws_token1).await;
    let _ = recv_text(&mut ws1).await;

    // Client 2
    let member_id2 = create_second_member(&ctx, setup.project_id).await;
    let ws_token2 = state
        .ws_token_store
        .insert(member_id2, "User 2".to_string(), setup.task_id);
    let mut ws2 = connect_ws(addr, setup.project_id, setup.task_id, &ws_token2).await;

    // Drain awareness
    let _ = recv_text(&mut ws1).await;
    let _ = recv_text(&mut ws2).await;

    // Client 1 sends typing awareness
    let awareness_msg = serde_json::json!({
        "type": "AwarenessUpdate",
        "state": { "is_typing": true, "cursor_position": 42 }
    });
    ws1.send(tungstenite::Message::Binary(
        serde_json::to_vec(&awareness_msg)
            .expect("serialize")
            .into(),
    ))
    .await
    .expect("send awareness");

    // Client 2 should receive AwarenessChange with typing=true
    let parsed = recv_text(&mut ws2).await;
    assert_eq!(parsed["type"], "AwarenessChange");
    let entries = parsed["entries"].as_array().expect("entries");
    let typing_entry = entries
        .iter()
        .find(|e| e["is_typing"].as_bool() == Some(true));
    assert!(
        typing_entry.is_some(),
        "should have an entry with isTyping=true"
    );

    ws1.close(None).await.ok();
    ws2.close(None).await.ok();
}

/// Start a test server with a custom idle timeout (in seconds).
async fn start_test_server_with_idle_timeout(
    ctx: &TestContext,
    idle_timeout_secs: u64,
) -> (std::net::SocketAddr, conf_ops::app_state::AppState) {
    let mut state = ctx.app_state();
    state.crdt_ws_idle_timeout_secs = idle_timeout_secs;
    // Use a very short heartbeat so timeout is checked frequently
    state.crdt_ws_heartbeat_interval_secs = 1;

    use axum::routing::get;
    use conf_ops::api::routes::ws;

    let ws_route: axum::Router = axum::Router::new()
        .route(
            "/api/v1/projects/{projectId}/tasks/{taskId}/conversation/ws",
            get(ws::ws_upgrade),
        )
        .with_state(state.clone());

    let app = common::build_app(ctx).merge(ws_route);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind to ephemeral port");
    let addr = listener.local_addr().expect("get local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    (addr, state)
}

#[tokio::test]
async fn ws_idle_timeout_disconnects_client() {
    let ctx = TestContext::new().await;
    let setup = setup_task(&ctx).await;

    // Start server with 2-second idle timeout and 1-second heartbeat
    let (addr, state) = start_test_server_with_idle_timeout(&ctx, 2).await;

    let ws_token =
        state
            .ws_token_store
            .insert(setup.member_id, "User 1".to_string(), setup.task_id);
    let mut ws = connect_ws(addr, setup.project_id, setup.task_id, &ws_token).await;

    // Drain the initial awareness message
    let _ = recv_text(&mut ws).await;

    // Do NOT send any messages — let the connection go idle.
    // The server heartbeat (1s) checks if last_activity > idle_timeout (2s).
    // After ~3s, the server should close the connection.
    // We read messages in a loop, expecting Pings initially, then a Close/disconnect.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut got_disconnect = false;

    while tokio::time::Instant::now() < deadline {
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), ws.next()).await;
        match result {
            Ok(Some(Ok(tungstenite::Message::Ping(_)))) => {
                // Server heartbeat ping — keep waiting
                continue;
            }
            Ok(Some(Ok(tungstenite::Message::Close(_)))) => {
                got_disconnect = true;
                break;
            }
            Ok(None) => {
                // Stream ended (server closed)
                got_disconnect = true;
                break;
            }
            Ok(Some(Err(_))) => {
                // Connection error (server closed)
                got_disconnect = true;
                break;
            }
            Err(_) => {
                // Timeout reading — should not happen within deadline
                break;
            }
            Ok(Some(Ok(_))) => {
                // Other message (e.g., Pong, Text) — keep waiting
                continue;
            }
        }
    }

    assert!(
        got_disconnect,
        "Server should have disconnected the idle client"
    );
}
