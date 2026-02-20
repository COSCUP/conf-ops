mod common;

use conf_ops::modules::core::member::models::MemberRole;
use conf_ops::modules::core::member_tag::repository::MemberTagRepository;

#[tokio::test]
async fn create_and_get_tag() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;

    let tag_id = conf_ops::id::generate_id();
    let tag = MemberTagRepository::create(
        &ctx.pool,
        tag_id,
        project_id,
        "speakers",
        Some("Speaker tag"),
    )
    .await
    .unwrap();

    assert_eq!(tag.id, tag_id);
    assert_eq!(tag.project_id, project_id);
    assert_eq!(tag.name, "speakers");
    assert_eq!(tag.description.as_deref(), Some("Speaker tag"));
    assert!(tag.deleted_at.is_none());

    let fetched = MemberTagRepository::get_by_id(&ctx.pool, tag_id)
        .await
        .unwrap();
    assert_eq!(fetched.id, tag_id);
    assert_eq!(fetched.name, "speakers");
}

#[tokio::test]
async fn update_tag() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "old-name").await;

    let updated =
        MemberTagRepository::update(&ctx.pool, tag_id, Some("new-name"), Some(Some("new desc")))
            .await
            .unwrap();

    assert_eq!(updated.name, "new-name");
    assert_eq!(updated.description.as_deref(), Some("new desc"));
}

#[tokio::test]
async fn soft_delete_hides_tag() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "to-delete").await;

    MemberTagRepository::soft_delete(&ctx.pool, tag_id)
        .await
        .unwrap();

    let result = MemberTagRepository::get_by_id(&ctx.pool, tag_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn list_by_project_with_counts() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;

    let tag_id = ctx.create_test_tag(project_id, "tag1").await;

    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let contact_id = ctx
        .create_test_contact(org_id, "Contact", "contact@test.com")
        .await;
    ctx.assign_tag_to_contact(tag_id, contact_id, project_id)
        .await;

    let items = MemberTagRepository::list_by_project(&ctx.pool, project_id)
        .await
        .unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "tag1");
    assert_eq!(items[0].member_count, 1);
    assert_eq!(items[0].contact_count, 1);
}

#[tokio::test]
async fn assignment_crud() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "test-tag").await;

    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Member)
        .await;
    let assignment_id = ctx
        .assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let members = MemberTagRepository::list_assigned_members(&ctx.pool, tag_id)
        .await
        .unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].member_id, member_id);

    MemberTagRepository::delete_assignment(&ctx.pool, assignment_id)
        .await
        .unwrap();

    let members = MemberTagRepository::list_assigned_members(&ctx.pool, tag_id)
        .await
        .unwrap();
    assert!(members.is_empty());
}

#[tokio::test]
async fn contact_assignment() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "test-tag").await;

    let contact_id = ctx
        .create_test_contact(org_id, "Test Contact", "contact@example.com")
        .await;
    ctx.assign_tag_to_contact(tag_id, contact_id, project_id)
        .await;

    let contacts = MemberTagRepository::list_assigned_contacts(&ctx.pool, tag_id)
        .await
        .unwrap();
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].contact_id, contact_id);
    assert_eq!(contacts[0].name, "Test Contact");
}

#[tokio::test]
async fn duplicate_member_assignment_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "test-tag").await;

    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Member)
        .await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let result = MemberTagRepository::create_assignment(
        &ctx.pool,
        conf_ops::id::generate_id(),
        tag_id,
        Some(member_id),
        None,
        project_id,
    )
    .await;

    assert!(result.is_err());
    assert!(
        matches!(
            result.unwrap_err(),
            conf_ops::modules::core::member_tag::error::MemberTagError::AssignmentAlreadyExists
        ),
        "Expected AssignmentAlreadyExists"
    );
}

#[tokio::test]
async fn duplicate_contact_assignment_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "test-tag").await;

    let contact_id = ctx
        .create_test_contact(org_id, "Contact", "c@test.com")
        .await;
    ctx.assign_tag_to_contact(tag_id, contact_id, project_id)
        .await;

    let result = MemberTagRepository::create_assignment(
        &ctx.pool,
        conf_ops::id::generate_id(),
        tag_id,
        None,
        Some(contact_id),
        project_id,
    )
    .await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        conf_ops::modules::core::member_tag::error::MemberTagError::AssignmentAlreadyExists
    ));
}

#[tokio::test]
async fn both_null_assignment_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "test-tag").await;

    let result = MemberTagRepository::create_assignment(
        &ctx.pool,
        conf_ops::id::generate_id(),
        tag_id,
        None,
        None,
        project_id,
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn list_tags_for_member() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;

    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Member)
        .await;

    let tag1_id = ctx.create_test_tag(project_id, "tag-a").await;
    let tag2_id = ctx.create_test_tag(project_id, "tag-b").await;

    ctx.assign_tag_to_member(tag1_id, member_id, project_id)
        .await;
    ctx.assign_tag_to_member(tag2_id, member_id, project_id)
        .await;

    let tags = MemberTagRepository::list_tags_for_member(&ctx.pool, member_id)
        .await
        .unwrap();
    assert_eq!(tags.len(), 2);
}

#[tokio::test]
async fn list_tags_for_members_batch() {
    let ctx = common::TestContext::new().await;
    let (account1, _) = ctx.create_test_account().await;
    let (account2, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account1).await;
    let project_id = ctx.create_test_project(org_id, account1).await;

    let member1 = ctx
        .create_test_member(project_id, account1, MemberRole::Owner)
        .await;
    let member2 = ctx
        .create_test_member(project_id, account2, MemberRole::Member)
        .await;

    let tag_id = ctx.create_test_tag(project_id, "shared-tag").await;
    ctx.assign_tag_to_member(tag_id, member1, project_id).await;
    ctx.assign_tag_to_member(tag_id, member2, project_id).await;

    let pairs = MemberTagRepository::list_tags_for_members_batch(&ctx.pool, &[member1, member2])
        .await
        .unwrap();

    assert_eq!(pairs.len(), 2);
    assert!(pairs.iter().any(|(mid, _)| *mid == member1));
    assert!(pairs.iter().any(|(mid, _)| *mid == member2));
}

#[tokio::test]
async fn update_external_task_creation() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "ext-tag").await;

    let settings = serde_json::json!([{"type": "github", "repo": "org/repo"}]);
    let updated = MemberTagRepository::update_external_task_creation(&ctx.pool, tag_id, &settings)
        .await
        .unwrap();

    assert_eq!(updated.external_task_creation, settings);
}
