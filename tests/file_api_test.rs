mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use conf_ops::modules::core::member::models::MemberRole;

struct FileTestSetup {
    _project_id: String,
    task_id: String,
    token: String,
    _account_id: uuid::Uuid,
}

async fn setup_file_test(ctx: &common::TestContext) -> FileTestSetup {
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    let token = ctx.issue_test_token(account_id);

    let template_id = ctx
        .create_test_task_template(project_id, "Template", account_id)
        .await;
    let tag_id = ctx.create_test_tag(project_id, "Team").await;
    ctx.link_tag_to_template(template_id, tag_id).await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let app = common::build_app(ctx);
    let (status, body) = common::json_request(
        app,
        "POST",
        &format!("/api/v1/projects/{project_id}/tasks"),
        &token,
        Some(serde_json::json!({
            "taskTemplateId": template_id,
            "ownerTagId": tag_id,
            "name": "Test Task"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let task_id = body["id"].as_str().unwrap().to_string();

    FileTestSetup {
        _project_id: project_id.to_string(),
        task_id,
        token,
        _account_id: account_id,
    }
}

/// Build a multipart/form-data request body manually.
fn build_multipart_body(
    filename: &str,
    content_type: &str,
    data: &[u8],
    scope_type: &str,
    scope_id: &str,
) -> (String, Vec<u8>) {
    let boundary = "----TestBoundary12345";

    let mut body = Vec::new();

    // scopeType field
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"scopeType\"\r\n\r\n");
    body.extend_from_slice(scope_type.as_bytes());
    body.extend_from_slice(b"\r\n");

    // scopeId field
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"scopeId\"\r\n\r\n");
    body.extend_from_slice(scope_id.as_bytes());
    body.extend_from_slice(b"\r\n");

    // file field
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(data);
    body.extend_from_slice(b"\r\n");

    // End boundary
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    let content_type_header = format!("multipart/form-data; boundary={boundary}");
    (content_type_header, body)
}

async fn upload_file(
    app: axum::Router,
    token: &str,
    filename: &str,
    content_type: &str,
    data: &[u8],
    scope_type: &str,
    scope_id: &str,
) -> (StatusCode, serde_json::Value) {
    let (ct_header, body) =
        build_multipart_body(filename, content_type, data, scope_type, scope_id);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/files/upload")
        .header("Authorization", common::auth_header(token))
        .header("Content-Type", ct_header)
        .body(Body::from(body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::json!(null));
    (status, json)
}

#[tokio::test]
async fn upload_file_success() {
    let ctx = common::TestContext::new().await;
    let setup = setup_file_test(&ctx).await;
    let app = common::build_app(&ctx);

    let data = b"hello world file content";
    let (status, body) = upload_file(
        app,
        &setup.token,
        "test.txt",
        "text/plain",
        data,
        "task",
        &setup.task_id,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["filename"], "test.txt");
    assert_eq!(body["mimeType"], "text/plain");
    assert_eq!(body["size"], data.len() as i64);
    assert!(body["id"].is_string());
    assert!(body["createdAt"].is_string());
}

#[tokio::test]
async fn download_file_success() {
    let ctx = common::TestContext::new().await;
    let setup = setup_file_test(&ctx).await;
    let app = common::build_app(&ctx);

    let data = b"download test content";
    let (status, body) = upload_file(
        app.clone(),
        &setup.token,
        "download.txt",
        "text/plain",
        data,
        "task",
        &setup.task_id,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let file_id = body["id"].as_str().unwrap();

    // Download
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/files/{file_id}"))
        .header("Authorization", common::auth_header(&setup.token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(content_type, "text/plain");

    let content_disposition = resp
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_disposition.contains("download.txt"));

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], data);
}

#[tokio::test]
async fn delete_file_success() {
    let ctx = common::TestContext::new().await;
    let setup = setup_file_test(&ctx).await;
    let app = common::build_app(&ctx);

    let data = b"delete me";
    let (status, body) = upload_file(
        app.clone(),
        &setup.token,
        "delete.txt",
        "text/plain",
        data,
        "task",
        &setup.task_id,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let file_id = body["id"].as_str().unwrap();

    // Delete
    let (status, _) = common::json_request(
        app.clone(),
        "DELETE",
        &format!("/api/v1/files/{file_id}"),
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn download_after_delete_returns_404() {
    let ctx = common::TestContext::new().await;
    let setup = setup_file_test(&ctx).await;
    let app = common::build_app(&ctx);

    let data = b"to be deleted";
    let (status, body) = upload_file(
        app.clone(),
        &setup.token,
        "gone.txt",
        "text/plain",
        data,
        "task",
        &setup.task_id,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let file_id = body["id"].as_str().unwrap();

    // Delete
    let (status, _) = common::json_request(
        app.clone(),
        "DELETE",
        &format!("/api/v1/files/{file_id}"),
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Try download
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/files/{file_id}"))
        .header("Authorization", common::auth_header(&setup.token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unsupported_mime_type_returns_400() {
    let ctx = common::TestContext::new().await;
    let setup = setup_file_test(&ctx).await;
    let app = common::build_app(&ctx);

    let data = b"binary data";
    let (status, body) = upload_file(
        app,
        &setup.token,
        "archive.zip",
        "application/zip",
        data,
        "task",
        &setup.task_id,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["title"]
        .as_str()
        .unwrap_or("")
        .contains("Unsupported MIME Type"));
}

#[tokio::test]
async fn file_too_large_returns_413() {
    let ctx = common::TestContext::new().await;
    let setup = setup_file_test(&ctx).await;
    let app = common::build_app(&ctx);

    // Create data larger than 10MB image limit
    let data = vec![0u8; 10 * 1024 * 1024 + 1];
    let (status, body) = upload_file(
        app,
        &setup.token,
        "huge.png",
        "image/png",
        &data,
        "task",
        &setup.task_id,
    )
    .await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(body["title"]
        .as_str()
        .unwrap_or("")
        .contains("File Too Large"));
}

#[tokio::test]
async fn cleanup_orphaned_files_removes_old_files() {
    let ctx = common::TestContext::new().await;
    let setup = setup_file_test(&ctx).await;
    let app = common::build_app(&ctx);

    // Upload a file
    let data = b"file to be cleaned up";
    let (status, body) = upload_file(
        app.clone(),
        &setup.token,
        "orphan.txt",
        "text/plain",
        data,
        "task",
        &setup.task_id,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let file_id = body["id"].as_str().unwrap();
    let file_id_uuid: uuid::Uuid = file_id.parse().unwrap();

    // Soft-delete the file
    let (status, _) = common::json_request(
        app.clone(),
        "DELETE",
        &format!("/api/v1/files/{file_id}"),
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Simulate the file being deleted more than 7 days ago
    sqlx::query("UPDATE files SET deleted_at = NOW() - INTERVAL '8 days' WHERE id = $1")
        .bind(file_id_uuid)
        .execute(&ctx.pool)
        .await
        .expect("should update deleted_at");

    // Trigger cleanup
    let (status, body) = common::json_request(
        app.clone(),
        "POST",
        "/api/v1/files/cleanup",
        &setup.token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["removedCount"], 1);

    // Verify the file is no longer accessible
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/files/{file_id}"))
        .header("Authorization", common::auth_header(&setup.token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unauthorized_user_cannot_download() {
    let ctx = common::TestContext::new().await;
    let setup = setup_file_test(&ctx).await;
    let app = common::build_app(&ctx);

    let data = b"secret file";
    let (status, body) = upload_file(
        app.clone(),
        &setup.token,
        "secret.txt",
        "text/plain",
        data,
        "task",
        &setup.task_id,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let file_id = body["id"].as_str().unwrap();

    // Create another user without project access
    let (other_account_id, _) = ctx.create_test_account().await;
    let other_token = ctx.issue_test_token(other_account_id);

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/files/{file_id}"))
        .header("Authorization", common::auth_header(&other_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
