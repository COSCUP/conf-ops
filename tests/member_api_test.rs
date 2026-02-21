mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use conf_ops::modules::core::member::models::MemberRole;
use conf_ops::modules::core::organization::models::OrgRole;
use conf_ops::modules::core::organization::repository::OrgMemberRepository;

#[tokio::test]
async fn invite_member_returns_201() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let (invitee_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let org_id = ctx.create_test_org(owner_id).await;
    // Add invitee to org
    OrgMemberRepository::create(
        &ctx.pool,
        conf_ops::id::generate_id(),
        org_id,
        invitee_id,
        OrgRole::OrgMember,
    )
    .await
    .unwrap();

    let project_id = ctx.create_test_project(org_id, owner_id).await;

    let app = common::build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/members/invite"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "accountId": invitee_id,
                        "role": "member"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["accountId"], invitee_id.to_string());
    assert_eq!(json["role"], "member");
}

#[tokio::test]
async fn invite_non_org_member_returns_400() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let (outsider_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;

    let app = common::build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/members/invite"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "accountId": outsider_id,
                        "role": "member"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_members_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    ctx.create_test_member(project_id, owner_id, MemberRole::Owner)
        .await;

    let app = common::build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/projects/{project_id}/members"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["members"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn get_member_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let member_id = ctx
        .create_test_member(project_id, owner_id, MemberRole::Owner)
        .await;

    let app = common::build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/projects/{project_id}/members/{member_id}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], member_id.to_string());
}

#[tokio::test]
async fn update_member_role_returns_200() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let (member_account, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let member_id = ctx
        .create_test_member(project_id, member_account, MemberRole::Member)
        .await;

    let app = common::build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/projects/{project_id}/members/{member_id}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "role": "tag_admin"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["role"], "tag_admin");
}

#[tokio::test]
async fn delete_member_returns_204() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let (member_account, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let member_id = ctx
        .create_test_member(project_id, member_account, MemberRole::Member)
        .await;

    let app = common::build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/projects/{project_id}/members/{member_id}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn last_owner_removal_returns_409() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let token = common::TestContext::issue_test_token(owner_id);

    let org_id = ctx.create_test_org(owner_id).await;
    let project_id = ctx.create_test_project(org_id, owner_id).await;
    let member_id = ctx
        .create_test_member(project_id, owner_id, MemberRole::Owner)
        .await;

    let app = common::build_app(&ctx);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/projects/{project_id}/members/{member_id}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}
