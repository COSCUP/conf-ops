#![allow(clippy::similar_names)]
mod common;

use conf_ops::id::generate_id;
use conf_ops::modules::core::organization::error::OrgError;
use conf_ops::modules::core::organization::models::OrgRole;
use conf_ops::modules::core::organization::repository::{
    OrgMemberRepository, OrganizationRepository,
};

#[tokio::test]
async fn create_and_get_organization() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;

    let org_id = generate_id();
    let org = OrganizationRepository::create(
        &ctx.pool,
        org_id,
        "Test Org",
        Some("A test org"),
        None,
        owner_id,
    )
    .await
    .expect("should create org");

    assert_eq!(org.name, "Test Org");
    assert_eq!(org.description.as_deref(), Some("A test org"));
    assert!(org.deleted_at.is_none());

    let fetched = OrganizationRepository::get_by_id(&ctx.pool, org_id)
        .await
        .expect("should get org");
    assert_eq!(fetched.name, "Test Org");
}

#[tokio::test]
async fn update_organization() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;

    let org_id = generate_id();
    OrganizationRepository::create(&ctx.pool, org_id, "Original", None, None, owner_id)
        .await
        .expect("should create org");

    let updated = OrganizationRepository::update(
        &ctx.pool,
        org_id,
        Some("Updated"),
        Some(Some("New desc")),
        None,
    )
    .await
    .expect("should update org");

    assert_eq!(updated.name, "Updated");
    assert_eq!(updated.description.as_deref(), Some("New desc"));
}

#[tokio::test]
async fn soft_delete_organization() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;

    let org_id = generate_id();
    OrganizationRepository::create(&ctx.pool, org_id, "To Delete", None, None, owner_id)
        .await
        .expect("should create org");

    OrganizationRepository::soft_delete(&ctx.pool, org_id)
        .await
        .expect("should soft delete");

    let result = OrganizationRepository::get_by_id(&ctx.pool, org_id).await;
    assert!(matches!(result, Err(OrgError::NotFound)));
}

#[tokio::test]
async fn member_crud() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let (member_id_account, _) = ctx.create_test_account().await;

    let org_id = generate_id();
    OrganizationRepository::create(&ctx.pool, org_id, "Test Org", None, None, owner_id)
        .await
        .expect("should create org");

    let member_id = generate_id();
    let member =
        OrgMemberRepository::create(&ctx.pool, member_id, org_id, owner_id, OrgRole::OrgOwner)
            .await
            .expect("should create member");
    assert_eq!(member.role, OrgRole::OrgOwner);

    let member2_id = generate_id();
    OrgMemberRepository::create(
        &ctx.pool,
        member2_id,
        org_id,
        member_id_account,
        OrgRole::OrgMember,
    )
    .await
    .expect("should create second member");

    let members = OrgMemberRepository::list_by_org(&ctx.pool, org_id)
        .await
        .expect("should list members");
    assert_eq!(members.len(), 2);

    OrgMemberRepository::update_role(&ctx.pool, member2_id, OrgRole::OrgAdmin)
        .await
        .expect("should update role");

    let updated = OrgMemberRepository::get_by_id(&ctx.pool, member2_id)
        .await
        .expect("should get member");
    assert_eq!(updated.role, OrgRole::OrgAdmin);

    OrgMemberRepository::delete(&ctx.pool, member2_id)
        .await
        .expect("should delete member");
}

#[tokio::test]
async fn duplicate_member_returns_error() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;

    let org_id = generate_id();
    OrganizationRepository::create(&ctx.pool, org_id, "Test Org", None, None, owner_id)
        .await
        .expect("should create org");

    let member_id = generate_id();
    OrgMemberRepository::create(&ctx.pool, member_id, org_id, owner_id, OrgRole::OrgOwner)
        .await
        .expect("should create member");

    let dup_id = generate_id();
    let result =
        OrgMemberRepository::create(&ctx.pool, dup_id, org_id, owner_id, OrgRole::OrgMember).await;
    assert!(matches!(result, Err(OrgError::MemberAlreadyExists)));
}

#[tokio::test]
async fn last_owner_count() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;

    let org_id = generate_id();
    OrganizationRepository::create(&ctx.pool, org_id, "Test Org", None, None, owner_id)
        .await
        .expect("should create org");

    let member_id = generate_id();
    OrgMemberRepository::create(&ctx.pool, member_id, org_id, owner_id, OrgRole::OrgOwner)
        .await
        .expect("should create owner member");

    let count = OrgMemberRepository::count_owners(&ctx.pool, org_id)
        .await
        .expect("should count owners");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn list_orgs_for_account() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;

    let org1_id = generate_id();
    OrganizationRepository::create(&ctx.pool, org1_id, "Org 1", None, None, owner_id)
        .await
        .expect("should create org 1");
    OrgMemberRepository::create(
        &ctx.pool,
        generate_id(),
        org1_id,
        owner_id,
        OrgRole::OrgOwner,
    )
    .await
    .expect("should add owner to org 1");

    let org2_id = generate_id();
    OrganizationRepository::create(&ctx.pool, org2_id, "Org 2", None, None, owner_id)
        .await
        .expect("should create org 2");
    OrgMemberRepository::create(
        &ctx.pool,
        generate_id(),
        org2_id,
        owner_id,
        OrgRole::OrgAdmin,
    )
    .await
    .expect("should add admin to org 2");

    let orgs = OrgMemberRepository::list_orgs_for_account(&ctx.pool, owner_id)
        .await
        .expect("should list orgs");
    assert_eq!(orgs.len(), 2);
}
