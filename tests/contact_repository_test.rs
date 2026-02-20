mod common;

use conf_ops::id::generate_id;
use conf_ops::modules::core::contact::error::ContactError;
use conf_ops::modules::core::contact::repository::ContactRepository;

#[tokio::test]
async fn create_and_get_contact() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let contact_id = generate_id();
    let contact =
        ContactRepository::create(&ctx.pool, contact_id, org_id, "Alice", "alice@example.com")
            .await
            .unwrap();

    assert_eq!(contact.id, contact_id);
    assert_eq!(contact.organization_id, org_id);
    assert_eq!(contact.name, "Alice");
    assert_eq!(contact.email, "alice@example.com");
    assert!(contact.merged_into_id.is_none());
    assert!(contact.deleted_at.is_none());

    let fetched = ContactRepository::get_by_id(&ctx.pool, contact_id)
        .await
        .unwrap();
    assert_eq!(fetched.id, contact_id);
    assert_eq!(fetched.name, "Alice");
}

#[tokio::test]
async fn get_nonexistent_contact_returns_not_found() {
    let ctx = common::TestContext::new().await;
    let result = ContactRepository::get_by_id(&ctx.pool, generate_id()).await;
    assert!(matches!(result, Err(ContactError::NotFound)));
}

#[tokio::test]
async fn update_contact() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let contact_id = ctx
        .create_test_contact(org_id, "Bob", "bob@example.com")
        .await;

    let updated =
        ContactRepository::update(&ctx.pool, contact_id, Some("Robert"), Some("robert@ex.com"))
            .await
            .unwrap();

    assert_eq!(updated.name, "Robert");
    assert_eq!(updated.email, "robert@ex.com");
}

#[tokio::test]
async fn update_contact_partial() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let contact_id = ctx
        .create_test_contact(org_id, "Charlie", "charlie@example.com")
        .await;

    let updated = ContactRepository::update(&ctx.pool, contact_id, Some("Charles"), None)
        .await
        .unwrap();

    assert_eq!(updated.name, "Charles");
    assert_eq!(updated.email, "charlie@example.com");
}

#[tokio::test]
async fn soft_delete_contact() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let contact_id = ctx
        .create_test_contact(org_id, "Dave", "dave@example.com")
        .await;

    ContactRepository::soft_delete(&ctx.pool, contact_id)
        .await
        .unwrap();

    let result = ContactRepository::get_by_id(&ctx.pool, contact_id).await;
    assert!(matches!(result, Err(ContactError::NotFound)));
}

#[tokio::test]
async fn soft_delete_nonexistent_returns_not_found() {
    let ctx = common::TestContext::new().await;
    let result = ContactRepository::soft_delete(&ctx.pool, generate_id()).await;
    assert!(matches!(result, Err(ContactError::NotFound)));
}

#[tokio::test]
async fn list_by_org_returns_active_non_merged() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    ctx.create_test_contact(org_id, "Alice", "alice@example.com")
        .await;
    ctx.create_test_contact(org_id, "Bob", "bob@example.com")
        .await;
    let deleted_id = ctx
        .create_test_contact(org_id, "Deleted", "deleted@example.com")
        .await;
    ContactRepository::soft_delete(&ctx.pool, deleted_id)
        .await
        .unwrap();

    let contacts = ContactRepository::list_by_org(&ctx.pool, org_id, None)
        .await
        .unwrap();
    assert_eq!(contacts.len(), 2);
}

#[tokio::test]
async fn list_by_org_excludes_merged() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let target_id = ctx
        .create_test_contact(org_id, "Target", "target@example.com")
        .await;
    let source_id = ctx
        .create_test_contact(org_id, "Source", "source@example.com")
        .await;

    ContactRepository::set_merged_into(&ctx.pool, source_id, target_id)
        .await
        .unwrap();

    let contacts = ContactRepository::list_by_org(&ctx.pool, org_id, None)
        .await
        .unwrap();
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].id, target_id);
}

#[tokio::test]
async fn search_ilike_on_name_and_email() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    ctx.create_test_contact(org_id, "Alice Smith", "alice@example.com")
        .await;
    ctx.create_test_contact(org_id, "Bob Jones", "bob@example.com")
        .await;

    let by_name = ContactRepository::list_by_org(&ctx.pool, org_id, Some("alice"))
        .await
        .unwrap();
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name[0].name, "Alice Smith");

    let by_email = ContactRepository::list_by_org(&ctx.pool, org_id, Some("bob@"))
        .await
        .unwrap();
    assert_eq!(by_email.len(), 1);
    assert_eq!(by_email[0].name, "Bob Jones");

    let no_match = ContactRepository::list_by_org(&ctx.pool, org_id, Some("zzz"))
        .await
        .unwrap();
    assert!(no_match.is_empty());
}

#[tokio::test]
async fn set_merged_into_and_follow_chain() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let a = ctx.create_test_contact(org_id, "A", "a@example.com").await;
    let b = ctx.create_test_contact(org_id, "B", "b@example.com").await;
    let c = ctx.create_test_contact(org_id, "C", "c@example.com").await;

    ContactRepository::set_merged_into(&ctx.pool, a, b)
        .await
        .unwrap();
    ContactRepository::set_merged_into(&ctx.pool, b, c)
        .await
        .unwrap();

    let target = ContactRepository::get_merge_chain_target(&ctx.pool, a)
        .await
        .unwrap();
    assert_eq!(target, c);
}

#[tokio::test]
async fn get_merge_chain_target_no_merge() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let id = ctx
        .create_test_contact(org_id, "Standalone", "standalone@example.com")
        .await;

    let target = ContactRepository::get_merge_chain_target(&ctx.pool, id)
        .await
        .unwrap();
    assert_eq!(target, id);
}
