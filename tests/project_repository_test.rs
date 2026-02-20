mod common;

use conf_ops::id::generate_id;
use conf_ops::modules::core::project::error::ProjectError;
use conf_ops::modules::core::project::models::ProjectStatus;
use conf_ops::modules::core::project::repository::ProjectRepository;

#[tokio::test]
async fn create_and_get_project() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let project_id = generate_id();
    let project = ProjectRepository::create(
        &ctx.pool,
        project_id,
        org_id,
        "Test Project",
        Some("Desc"),
        None,
        owner_id,
    )
    .await
    .expect("should create project");

    assert_eq!(project.name, "Test Project");
    assert_eq!(project.status, ProjectStatus::Preparing);
    assert!(project.source_project_id.is_none());

    let fetched = ProjectRepository::get_by_id(&ctx.pool, project_id)
        .await
        .expect("should get project");
    assert_eq!(fetched.name, "Test Project");
}

#[tokio::test]
async fn update_project() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let project_id = generate_id();
    ProjectRepository::create(
        &ctx.pool, project_id, org_id, "Original", None, None, owner_id,
    )
    .await
    .expect("should create project");

    let updated = ProjectRepository::update(
        &ctx.pool,
        project_id,
        Some("Updated"),
        Some(Some("New desc")),
    )
    .await
    .expect("should update project");

    assert_eq!(updated.name, "Updated");
    assert_eq!(updated.description.as_deref(), Some("New desc"));
}

#[tokio::test]
async fn soft_delete_project() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let project_id = generate_id();
    ProjectRepository::create(
        &ctx.pool,
        project_id,
        org_id,
        "To Delete",
        None,
        None,
        owner_id,
    )
    .await
    .expect("should create project");

    ProjectRepository::soft_delete(&ctx.pool, project_id)
        .await
        .expect("should soft delete");

    let result = ProjectRepository::get_by_id(&ctx.pool, project_id).await;
    assert!(matches!(result, Err(ProjectError::NotFound)));
}

#[tokio::test]
async fn update_status() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let project_id = generate_id();
    ProjectRepository::create(
        &ctx.pool,
        project_id,
        org_id,
        "Status Test",
        None,
        None,
        owner_id,
    )
    .await
    .expect("should create project");

    let updated = ProjectRepository::update_status(&ctx.pool, project_id, ProjectStatus::Active)
        .await
        .expect("should update status");
    assert_eq!(updated.status, ProjectStatus::Active);
}

#[tokio::test]
async fn list_by_org_with_filter() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let p1 = generate_id();
    ProjectRepository::create(&ctx.pool, p1, org_id, "P1", None, None, owner_id)
        .await
        .expect("should create p1");

    let p2 = generate_id();
    ProjectRepository::create(&ctx.pool, p2, org_id, "P2", None, None, owner_id)
        .await
        .expect("should create p2");
    ProjectRepository::update_status(&ctx.pool, p2, ProjectStatus::Active)
        .await
        .expect("should update p2 status");

    let all = ProjectRepository::list_by_org(&ctx.pool, org_id, None)
        .await
        .expect("should list all");
    assert_eq!(all.len(), 2);

    let preparing =
        ProjectRepository::list_by_org(&ctx.pool, org_id, Some(ProjectStatus::Preparing))
            .await
            .expect("should list preparing");
    assert_eq!(preparing.len(), 1);
}

#[tokio::test]
async fn count_not_archived() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let p1 = generate_id();
    ProjectRepository::create(&ctx.pool, p1, org_id, "P1", None, None, owner_id)
        .await
        .expect("should create p1");

    let count = ProjectRepository::count_not_archived_by_org(&ctx.pool, org_id)
        .await
        .expect("should count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn get_organization_id() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let project_id = generate_id();
    ProjectRepository::create(&ctx.pool, project_id, org_id, "Test", None, None, owner_id)
        .await
        .expect("should create project");

    let fetched_org_id = ProjectRepository::get_organization_id(&ctx.pool, project_id)
        .await
        .expect("should get org id");
    assert_eq!(fetched_org_id, org_id);
}

#[tokio::test]
async fn update_permission_settings() {
    let ctx = common::TestContext::new().await;
    let (owner_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(owner_id).await;

    let project_id = generate_id();
    ProjectRepository::create(&ctx.pool, project_id, org_id, "Test", None, None, owner_id)
        .await
        .expect("should create project");

    let settings = serde_json::json!({"custom_role": true});
    let updated = ProjectRepository::update_permission_settings(&ctx.pool, project_id, &settings)
        .await
        .expect("should update settings");
    assert_eq!(updated.permission_settings, settings);
}

#[tokio::test]
async fn status_transitions() {
    assert!(ProjectStatus::Preparing.can_transition_to(ProjectStatus::Active));
    assert!(ProjectStatus::Active.can_transition_to(ProjectStatus::Completed));
    assert!(ProjectStatus::Completed.can_transition_to(ProjectStatus::Archived));
    assert!(ProjectStatus::Active.can_transition_to(ProjectStatus::Archived));
    assert!(ProjectStatus::Preparing.can_transition_to(ProjectStatus::Archived));

    assert!(!ProjectStatus::Completed.can_transition_to(ProjectStatus::Preparing));
    assert!(!ProjectStatus::Archived.can_transition_to(ProjectStatus::Active));
    assert!(!ProjectStatus::Preparing.can_transition_to(ProjectStatus::Completed));
}
