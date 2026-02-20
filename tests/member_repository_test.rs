mod common;

use conf_ops::modules::core::member::models::MemberRole;
use conf_ops::modules::core::member::repository::MemberRepository;

#[tokio::test]
async fn create_and_get_member() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;

    let member_id = conf_ops::id::generate_id();
    let member = MemberRepository::create(
        &ctx.pool,
        member_id,
        project_id,
        account_id,
        MemberRole::Owner,
    )
    .await
    .unwrap();

    assert_eq!(member.id, member_id);
    assert_eq!(member.project_id, project_id);
    assert_eq!(member.account_id, account_id);
    assert_eq!(member.role, MemberRole::Owner);
    assert!(member.deleted_at.is_none());

    let fetched = MemberRepository::get_by_id(&ctx.pool, member_id)
        .await
        .unwrap();
    assert_eq!(fetched.id, member_id);
}

#[tokio::test]
async fn unique_constraint_returns_already_exists() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;

    let id1 = conf_ops::id::generate_id();
    MemberRepository::create(&ctx.pool, id1, project_id, account_id, MemberRole::Owner)
        .await
        .unwrap();

    let id2 = conf_ops::id::generate_id();
    let result =
        MemberRepository::create(&ctx.pool, id2, project_id, account_id, MemberRole::Member).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            conf_ops::modules::core::member::error::MemberError::AlreadyExists
        ),
        "Expected AlreadyExists, got: {err:?}"
    );
}

#[tokio::test]
async fn get_by_project_and_account() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;

    let member_id = conf_ops::id::generate_id();
    MemberRepository::create(
        &ctx.pool,
        member_id,
        project_id,
        account_id,
        MemberRole::Member,
    )
    .await
    .unwrap();

    let found = MemberRepository::get_by_project_and_account(&ctx.pool, project_id, account_id)
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, member_id);

    let (other_id, _) = ctx.create_test_account().await;
    let not_found = MemberRepository::get_by_project_and_account(&ctx.pool, project_id, other_id)
        .await
        .unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn soft_delete_hides_member() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;

    let member_id = conf_ops::id::generate_id();
    MemberRepository::create(
        &ctx.pool,
        member_id,
        project_id,
        account_id,
        MemberRole::Member,
    )
    .await
    .unwrap();

    MemberRepository::soft_delete(&ctx.pool, member_id)
        .await
        .unwrap();

    let result = MemberRepository::get_by_id(&ctx.pool, member_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn count_owners() {
    let ctx = common::TestContext::new().await;
    let (account1, _) = ctx.create_test_account().await;
    let (account2, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account1).await;
    let project_id = ctx.create_test_project(org_id, account1).await;

    let id1 = conf_ops::id::generate_id();
    MemberRepository::create(&ctx.pool, id1, project_id, account1, MemberRole::Owner)
        .await
        .unwrap();

    assert_eq!(
        MemberRepository::count_owners(&ctx.pool, project_id)
            .await
            .unwrap(),
        1
    );

    let id2 = conf_ops::id::generate_id();
    MemberRepository::create(&ctx.pool, id2, project_id, account2, MemberRole::Owner)
        .await
        .unwrap();

    assert_eq!(
        MemberRepository::count_owners(&ctx.pool, project_id)
            .await
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn update_role() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;

    let member_id = conf_ops::id::generate_id();
    MemberRepository::create(
        &ctx.pool,
        member_id,
        project_id,
        account_id,
        MemberRole::Member,
    )
    .await
    .unwrap();

    let updated = MemberRepository::update_role(&ctx.pool, member_id, MemberRole::TagAdmin)
        .await
        .unwrap();
    assert_eq!(updated.role, MemberRole::TagAdmin);
}

#[tokio::test]
async fn list_by_project_with_account_info() {
    let ctx = common::TestContext::new().await;
    let (account1, _) = ctx.create_test_account().await;
    let (account2, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account1).await;
    let project_id = ctx.create_test_project(org_id, account1).await;

    let id1 = conf_ops::id::generate_id();
    MemberRepository::create(&ctx.pool, id1, project_id, account1, MemberRole::Owner)
        .await
        .unwrap();

    let id2 = conf_ops::id::generate_id();
    MemberRepository::create(&ctx.pool, id2, project_id, account2, MemberRole::Member)
        .await
        .unwrap();

    // List all
    let all = MemberRepository::list_by_project(&ctx.pool, project_id, None)
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
    // Verify JOIN fields are populated
    assert!(!all[0].name.is_empty());
    assert!(!all[0].email.is_empty());

    // List by role filter
    let owners = MemberRepository::list_by_project(&ctx.pool, project_id, Some(MemberRole::Owner))
        .await
        .unwrap();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].role, MemberRole::Owner);
}
