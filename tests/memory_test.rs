mod common;

use conf_ops::id::generate_id;
use conf_ops::modules::ai::memory::error::MemoryError;
use conf_ops::modules::ai::memory::models::{
    CreateLibraryDocumentParams, CreateMemoryParams, MemorySource, ScopeType,
    UpdateLibraryDocumentParams, UpdateMemoryParams,
};
use conf_ops::modules::ai::memory::service::MemoryService;
use conf_ops::modules::core::member::models::MemberRole;
use conf_ops::modules::core::task::repository::{CreateTaskParams, TaskRepository};

// ── Helpers ─────────────────────────────────────────────────

/// Build a full task hierarchy: account, org, project, `member_tag`, `task_template`, task.
/// Returns (`account_id`, `org_id`, `project_id`, `tag_id`, `template_id`, `task_id`).
async fn build_task_chain(
    ctx: &common::TestContext,
) -> (
    uuid::Uuid,
    uuid::Uuid,
    uuid::Uuid,
    uuid::Uuid,
    uuid::Uuid,
    uuid::Uuid,
) {
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "Test Tag").await;
    let template_id = ctx
        .create_test_task_template(project_id, "Test Template", account_id)
        .await;
    ctx.link_tag_to_template(template_id, tag_id).await;
    let member_id = ctx
        .create_test_member(project_id, account_id, MemberRole::Owner)
        .await;
    ctx.assign_tag_to_member(tag_id, member_id, project_id)
        .await;

    let task_id = generate_id();
    TaskRepository::create(
        &ctx.pool,
        &CreateTaskParams {
            id: task_id,
            project_id,
            task_template_id: template_id,
            owner_tag_id: tag_id,
            name: "Test Task",
            description: None,
            created_by: account_id,
        },
    )
    .await
    .expect("should create test task");

    (account_id, org_id, project_id, tag_id, template_id, task_id)
}

fn make_service(ctx: &common::TestContext) -> MemoryService {
    use conf_ops::events::EventBus;
    MemoryService::new(ctx.pool.clone(), EventBus::default())
}

// ── Memory CRUD — account scope ──────────────────────────────

#[tokio::test]
async fn memory_crud_account_scope() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let svc = make_service(&ctx);

    let memory_id = generate_id();
    let params = CreateMemoryParams {
        id: memory_id,
        scope_type: ScopeType::Account,
        scope_id: account_id,
        content: "Account-level memory".to_string(),
        library_ref: None,
        source: MemorySource::Manual,
        created_by: account_id,
    };

    let created = svc
        .create_memory(&params)
        .await
        .expect("should create memory");
    assert_eq!(created.id, memory_id);
    assert_eq!(created.scope_type, "account");
    assert_eq!(created.scope_id, account_id);
    assert_eq!(created.content, "Account-level memory");
    assert_eq!(created.source, "manual");
    assert!(created.deleted_at.is_none());

    // Get by ID
    let fetched = svc.get_memory(memory_id).await.expect("should get memory");
    assert_eq!(fetched.content, "Account-level memory");

    // List by scope
    let list = svc
        .list_memories("account", account_id, None, 10)
        .await
        .expect("should list memories");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, memory_id);

    // Update
    let update_params = UpdateMemoryParams {
        content: "Updated account memory".to_string(),
        library_ref: None,
    };
    let updated = svc
        .update_memory(memory_id, &update_params, account_id)
        .await
        .expect("should update memory");
    assert_eq!(updated.content, "Updated account memory");

    // Soft delete
    svc.delete_memory(memory_id)
        .await
        .expect("should delete memory");

    let result = svc.get_memory(memory_id).await;
    assert!(matches!(result, Err(MemoryError::NotFound)));

    // List after delete should be empty
    let list = svc
        .list_memories("account", account_id, None, 10)
        .await
        .expect("should list memories after delete");
    assert!(list.is_empty());
}

// ── Memory CRUD — organization scope ─────────────────────────

#[tokio::test]
async fn memory_crud_organization_scope() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let svc = make_service(&ctx);

    let memory_id = generate_id();
    let created = svc
        .create_memory(&CreateMemoryParams {
            id: memory_id,
            scope_type: ScopeType::Organization,
            scope_id: org_id,
            content: "Org-level memory".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await
        .expect("should create org memory");

    assert_eq!(created.scope_type, "organization");
    assert_eq!(created.scope_id, org_id);

    let list = svc
        .list_memories("organization", org_id, None, 10)
        .await
        .expect("should list org memories");
    assert_eq!(list.len(), 1);

    svc.delete_memory(memory_id)
        .await
        .expect("should delete org memory");
    assert!(svc
        .list_memories("organization", org_id, None, 10)
        .await
        .unwrap()
        .is_empty());
}

// ── Memory CRUD — project scope ───────────────────────────────

#[tokio::test]
async fn memory_crud_project_scope() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let svc = make_service(&ctx);

    let memory_id = generate_id();
    let created = svc
        .create_memory(&CreateMemoryParams {
            id: memory_id,
            scope_type: ScopeType::Project,
            scope_id: project_id,
            content: "Project-level memory".to_string(),
            library_ref: None,
            source: MemorySource::AutoExtracted,
            created_by: account_id,
        })
        .await
        .expect("should create project memory");

    assert_eq!(created.scope_type, "project");
    assert_eq!(created.source, "auto_extracted");

    let fetched = svc
        .get_memory(memory_id)
        .await
        .expect("should get project memory");
    assert_eq!(fetched.content, "Project-level memory");

    let list = svc
        .list_memories("project", project_id, None, 10)
        .await
        .expect("should list project memories");
    assert_eq!(list.len(), 1);

    svc.delete_memory(memory_id)
        .await
        .expect("should delete project memory");
}

// ── Memory CRUD — member_tag scope ────────────────────────────

#[tokio::test]
async fn memory_crud_member_tag_scope() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let tag_id = ctx.create_test_tag(project_id, "Speaker Team").await;
    let svc = make_service(&ctx);

    let memory_id = generate_id();
    let created = svc
        .create_memory(&CreateMemoryParams {
            id: memory_id,
            scope_type: ScopeType::MemberTag,
            scope_id: tag_id,
            content: "Tag-level memory".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await
        .expect("should create tag memory");

    assert_eq!(created.scope_type, "member_tag");
    assert_eq!(created.scope_id, tag_id);

    let list = svc
        .list_memories("member_tag", tag_id, None, 10)
        .await
        .expect("should list tag memories");
    assert_eq!(list.len(), 1);

    svc.delete_memory(memory_id)
        .await
        .expect("should delete tag memory");
}

// ── Memory CRUD — task_template scope ────────────────────────

#[tokio::test]
async fn memory_crud_task_template_scope() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let template_id = ctx
        .create_test_task_template(project_id, "Sponsorship Template", account_id)
        .await;
    let svc = make_service(&ctx);

    let memory_id = generate_id();
    let created = svc
        .create_memory(&CreateMemoryParams {
            id: memory_id,
            scope_type: ScopeType::TaskTemplate,
            scope_id: template_id,
            content: "Template-level memory".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await
        .expect("should create template memory");

    assert_eq!(created.scope_type, "task_template");
    assert_eq!(created.scope_id, template_id);

    let list = svc
        .list_memories("task_template", template_id, None, 10)
        .await
        .expect("should list template memories");
    assert_eq!(list.len(), 1);

    svc.delete_memory(memory_id)
        .await
        .expect("should delete template memory");
}

// ── Memory CRUD — task scope ──────────────────────────────────

#[tokio::test]
async fn memory_crud_task_scope() {
    let ctx = common::TestContext::new().await;
    let (account_id, _org, _proj, _tag, _tmpl, task_id) = build_task_chain(&ctx).await;
    let svc = make_service(&ctx);

    let memory_id = generate_id();
    let created = svc
        .create_memory(&CreateMemoryParams {
            id: memory_id,
            scope_type: ScopeType::Task,
            scope_id: task_id,
            content: "Task-level memory".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await
        .expect("should create task memory");

    assert_eq!(created.scope_type, "task");
    assert_eq!(created.scope_id, task_id);

    let fetched = svc
        .get_memory(memory_id)
        .await
        .expect("should get task memory");
    assert_eq!(fetched.content, "Task-level memory");

    let update = UpdateMemoryParams {
        content: "Updated task memory".to_string(),
        library_ref: None,
    };
    let updated = svc
        .update_memory(memory_id, &update, account_id)
        .await
        .expect("should update task memory");
    assert_eq!(updated.content, "Updated task memory");

    let list = svc
        .list_memories("task", task_id, None, 10)
        .await
        .expect("should list task memories");
    assert_eq!(list.len(), 1);

    svc.delete_memory(memory_id)
        .await
        .expect("should delete task memory");

    let result = svc.get_memory(memory_id).await;
    assert!(matches!(result, Err(MemoryError::NotFound)));
}

// ── Inheritance chain (6-layer merge) ─────────────────────────

#[tokio::test]
async fn inheritance_chain_collects_all_layers() {
    let ctx = common::TestContext::new().await;
    let (account_id, org_id, project_id, tag_id, template_id, task_id) =
        build_task_chain(&ctx).await;
    let svc = make_service(&ctx);

    // Create one memory at each scope level
    for (scope_type, scope_id, content) in [
        (ScopeType::Account, account_id, "account memory"),
        (ScopeType::Organization, org_id, "org memory"),
        (ScopeType::Project, project_id, "project memory"),
        (ScopeType::MemberTag, tag_id, "tag memory"),
        (ScopeType::TaskTemplate, template_id, "template memory"),
        (ScopeType::Task, task_id, "task memory"),
    ] {
        svc.create_memory(&CreateMemoryParams {
            id: generate_id(),
            scope_type,
            scope_id,
            content: content.to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await
        .expect("should create memory at each scope level");
    }

    let inherited = svc
        .get_inherited_memories(task_id, account_id)
        .await
        .expect("should collect inherited memories");

    assert_eq!(inherited.task_id, task_id);
    // All 6 scope layers should be represented
    assert_eq!(
        inherited.memories.len(),
        6,
        "expected 6 inherited memories, got: {}",
        inherited.memories.len()
    );

    // Verify sort order: task first (sort_order=0), then template (1), tag (2),
    // project (3), org (4), account (5)
    let scope_types: Vec<&str> = inherited
        .memories
        .iter()
        .map(|m| m.scope_type.as_str())
        .collect();
    assert_eq!(scope_types[0], "task");
    assert_eq!(scope_types[1], "task_template");
    assert_eq!(scope_types[2], "member_tag");
    assert_eq!(scope_types[3], "project");
    assert_eq!(scope_types[4], "organization");
    assert_eq!(scope_types[5], "account");
}

#[tokio::test]
async fn inheritance_chain_partial_layers() {
    let ctx = common::TestContext::new().await;
    let (account_id, _org_id, project_id, _tag_id, _template_id, task_id) =
        build_task_chain(&ctx).await;
    let svc = make_service(&ctx);

    // Only create memories at project and task level
    svc.create_memory(&CreateMemoryParams {
        id: generate_id(),
        scope_type: ScopeType::Project,
        scope_id: project_id,
        content: "project memory".to_string(),
        library_ref: None,
        source: MemorySource::Manual,
        created_by: account_id,
    })
    .await
    .expect("should create project memory");

    svc.create_memory(&CreateMemoryParams {
        id: generate_id(),
        scope_type: ScopeType::Task,
        scope_id: task_id,
        content: "task memory".to_string(),
        library_ref: None,
        source: MemorySource::Manual,
        created_by: account_id,
    })
    .await
    .expect("should create task memory");

    let inherited = svc
        .get_inherited_memories(task_id, account_id)
        .await
        .expect("should collect inherited memories");

    assert_eq!(inherited.memories.len(), 2);
    assert_eq!(inherited.memories[0].scope_type, "task");
    assert_eq!(inherited.memories[1].scope_type, "project");
}

// ── Library Document CRUD with version control ────────────────

#[tokio::test]
async fn library_document_crud_with_version_control() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let svc = make_service(&ctx);

    let doc_id = generate_id();
    let params = CreateLibraryDocumentParams {
        id: doc_id,
        scope_type: ScopeType::Organization,
        scope_id: org_id,
        title: "Sponsorship Guidelines".to_string(),
        content: "Initial content about sponsorship".to_string(),
        created_by: account_id,
    };

    // Create
    let created = svc
        .create_library_document(&params)
        .await
        .expect("should create library document");

    assert_eq!(created.id, doc_id);
    assert_eq!(created.title, "Sponsorship Guidelines");
    assert_eq!(created.content, "Initial content about sponsorship");
    assert_eq!(created.scope_type, "organization");
    assert_eq!(created.scope_id, org_id);
    assert!(created.deleted_at.is_none());

    // Get by ID
    let fetched = svc
        .get_library_document(doc_id)
        .await
        .expect("should get library document");
    assert_eq!(fetched.title, "Sponsorship Guidelines");

    // List by scope
    let list = svc
        .list_library_documents("organization", org_id, None, 10)
        .await
        .expect("should list library documents");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, doc_id);

    // Initial version recorded on create
    let versions_after_create = svc
        .list_library_document_versions(doc_id)
        .await
        .expect("should list versions after create");
    assert_eq!(versions_after_create.len(), 1);
    assert_eq!(
        versions_after_create[0].content,
        "Initial content about sponsorship"
    );

    // Update — creates a new version
    let update_params = UpdateLibraryDocumentParams {
        title: Some("Updated Sponsorship Guidelines".to_string()),
        content: "Revised content about sponsorship".to_string(),
    };
    let updated = svc
        .update_library_document(doc_id, &update_params, account_id)
        .await
        .expect("should update library document");
    assert_eq!(updated.title, "Updated Sponsorship Guidelines");
    assert_eq!(updated.content, "Revised content about sponsorship");

    // Two versions now (create + update)
    let versions_after_update = svc
        .list_library_document_versions(doc_id)
        .await
        .expect("should list versions after update");
    assert_eq!(versions_after_update.len(), 2);
    // Most recent version first
    assert_eq!(
        versions_after_update[0].content,
        "Revised content about sponsorship"
    );
    assert_eq!(versions_after_update[0].changed_by, account_id);

    // Update content only (no title change)
    let update_params2 = UpdateLibraryDocumentParams {
        title: None,
        content: "Third version content".to_string(),
    };
    let updated2 = svc
        .update_library_document(doc_id, &update_params2, account_id)
        .await
        .expect("should update library document again");
    assert_eq!(updated2.title, "Updated Sponsorship Guidelines");
    assert_eq!(updated2.content, "Third version content");

    let versions_final = svc
        .list_library_document_versions(doc_id)
        .await
        .expect("should list final versions");
    assert_eq!(versions_final.len(), 3);

    // Soft delete
    svc.delete_library_document(doc_id)
        .await
        .expect("should delete library document");

    let result = svc.get_library_document(doc_id).await;
    assert!(matches!(result, Err(MemoryError::LibraryDocumentNotFound)));

    // List after delete should be empty
    let list_after = svc
        .list_library_documents("organization", org_id, None, 10)
        .await
        .expect("should list after delete");
    assert!(list_after.is_empty());
}

// ── Library Document across different scopes ──────────────────

#[tokio::test]
async fn library_document_project_scope() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let project_id = ctx.create_test_project(org_id, account_id).await;
    let svc = make_service(&ctx);

    let doc_id = generate_id();
    let created = svc
        .create_library_document(&CreateLibraryDocumentParams {
            id: doc_id,
            scope_type: ScopeType::Project,
            scope_id: project_id,
            title: "Project Runbook".to_string(),
            content: "Project-specific guidelines".to_string(),
            created_by: account_id,
        })
        .await
        .expect("should create project-scoped library document");

    assert_eq!(created.scope_type, "project");
    assert_eq!(created.scope_id, project_id);

    let list = svc
        .list_library_documents("project", project_id, None, 10)
        .await
        .expect("should list project library documents");
    assert_eq!(list.len(), 1);
}

// ── Memory with library_ref ───────────────────────────────────

#[tokio::test]
async fn memory_with_library_ref() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let svc = make_service(&ctx);

    // Create a library document to reference
    let doc_id = generate_id();
    svc.create_library_document(&CreateLibraryDocumentParams {
        id: doc_id,
        scope_type: ScopeType::Organization,
        scope_id: org_id,
        title: "Reference Document".to_string(),
        content: "Reference content".to_string(),
        created_by: account_id,
    })
    .await
    .expect("should create library document");

    // Create memory referencing the library document
    let memory_id = generate_id();
    let created = svc
        .create_memory(&CreateMemoryParams {
            id: memory_id,
            scope_type: ScopeType::Organization,
            scope_id: org_id,
            content: "Memory with library ref".to_string(),
            library_ref: Some(doc_id),
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await
        .expect("should create memory with library_ref");

    assert_eq!(created.library_ref, Some(doc_id));

    // Update memory with library_ref
    let updated = svc
        .update_memory(
            memory_id,
            &UpdateMemoryParams {
                content: "Updated content".to_string(),
                library_ref: Some(doc_id),
            },
            account_id,
        )
        .await
        .expect("should update memory with library_ref");
    assert_eq!(updated.library_ref, Some(doc_id));
}

// ── Memory version history ────────────────────────────────────

#[tokio::test]
async fn memory_version_history() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let svc = make_service(&ctx);

    let memory_id = generate_id();
    svc.create_memory(&CreateMemoryParams {
        id: memory_id,
        scope_type: ScopeType::Organization,
        scope_id: org_id,
        content: "Version 1".to_string(),
        library_ref: None,
        source: MemorySource::Manual,
        created_by: account_id,
    })
    .await
    .expect("should create memory");

    // Initial create should record version 1
    let versions_v1 = svc
        .list_memory_versions(memory_id)
        .await
        .expect("should list versions");
    assert_eq!(versions_v1.len(), 1);
    assert_eq!(versions_v1[0].content, "Version 1");
    assert_eq!(versions_v1[0].changed_by, account_id);

    // Update to version 2
    svc.update_memory(
        memory_id,
        &UpdateMemoryParams {
            content: "Version 2".to_string(),
            library_ref: None,
        },
        account_id,
    )
    .await
    .expect("should update memory to v2");

    // Update to version 3
    svc.update_memory(
        memory_id,
        &UpdateMemoryParams {
            content: "Version 3".to_string(),
            library_ref: None,
        },
        account_id,
    )
    .await
    .expect("should update memory to v3");

    let versions_final = svc
        .list_memory_versions(memory_id)
        .await
        .expect("should list final versions");
    assert_eq!(versions_final.len(), 3);
    // Most recent first
    assert_eq!(versions_final[0].content, "Version 3");
    assert_eq!(versions_final[1].content, "Version 2");
    assert_eq!(versions_final[2].content, "Version 1");

    // All versions reference the same memory_id
    for v in &versions_final {
        assert_eq!(v.memory_id, memory_id);
    }
}

// ── Cache hit and invalidation ────────────────────────────────

#[tokio::test]
async fn cache_hit_on_second_call() {
    let ctx = common::TestContext::new().await;
    let (account_id, _org, _proj, _tag, _tmpl, task_id) = build_task_chain(&ctx).await;
    let svc = make_service(&ctx);

    // Create a memory at task level
    svc.create_memory(&CreateMemoryParams {
        id: generate_id(),
        scope_type: ScopeType::Task,
        scope_id: task_id,
        content: "cached task memory".to_string(),
        library_ref: None,
        source: MemorySource::Manual,
        created_by: account_id,
    })
    .await
    .expect("should create task memory");

    // First call — populates cache
    let first = svc
        .get_inherited_memories(task_id, account_id)
        .await
        .expect("first call should succeed");
    assert_eq!(first.memories.len(), 1);

    // Second call — should hit cache and return same result
    let second = svc
        .get_inherited_memories(task_id, account_id)
        .await
        .expect("second call should succeed");
    assert_eq!(second.memories.len(), 1);
    assert_eq!(second.memories[0].content, "cached task memory");
}

#[tokio::test]
async fn cache_invalidated_after_memory_update() {
    let ctx = common::TestContext::new().await;
    let (account_id, _org, _proj, _tag, _tmpl, task_id) = build_task_chain(&ctx).await;
    let svc = make_service(&ctx);

    let memory_id = generate_id();
    svc.create_memory(&CreateMemoryParams {
        id: memory_id,
        scope_type: ScopeType::Task,
        scope_id: task_id,
        content: "original content".to_string(),
        library_ref: None,
        source: MemorySource::Manual,
        created_by: account_id,
    })
    .await
    .expect("should create task memory");

    // Populate cache
    let cached = svc
        .get_inherited_memories(task_id, account_id)
        .await
        .expect("should get inherited memories");
    assert_eq!(cached.memories[0].content, "original content");

    // Update the memory — this publishes a MemoryUpserted event which triggers invalidation
    svc.update_memory(
        memory_id,
        &UpdateMemoryParams {
            content: "updated content".to_string(),
            library_ref: None,
        },
        account_id,
    )
    .await
    .expect("should update memory");

    // After update, the service's internal event bus publishes MemoryUpserted.
    // The cache is invalidated inline; re-querying should return fresh data.
    let fresh = svc
        .get_inherited_memories(task_id, account_id)
        .await
        .expect("should get fresh inherited memories");
    assert_eq!(fresh.memories[0].content, "updated content");
}

// ── CTE performance ──────────────────────────────────────────

#[tokio::test]
async fn inherited_memories_cte_query_under_5ms() {
    let ctx = common::TestContext::new().await;
    let (account_id, org_id, project_id, tag_id, template_id, task_id) =
        build_task_chain(&ctx).await;
    let svc = make_service(&ctx);

    // Seed a memory at each scope level to exercise the full CTE
    for (scope_type, scope_id) in [
        (ScopeType::Account, account_id),
        (ScopeType::Organization, org_id),
        (ScopeType::Project, project_id),
        (ScopeType::MemberTag, tag_id),
        (ScopeType::TaskTemplate, template_id),
        (ScopeType::Task, task_id),
    ] {
        svc.create_memory(&CreateMemoryParams {
            id: generate_id(),
            scope_type,
            scope_id,
            content: format!("perf test memory for {scope_id}"),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await
        .expect("should create memory for perf test");
    }

    // Warm-up run (connection pool, query plan cache, etc.)
    let _ = svc
        .get_inherited_memories(task_id, account_id)
        .await
        .expect("warm-up should succeed");

    // Re-create service to get a fresh cache so we measure the actual DB query
    let svc = make_service(&ctx);

    let start = std::time::Instant::now();
    let result = svc
        .get_inherited_memories(task_id, account_id)
        .await
        .expect("CTE query should succeed");
    let elapsed = start.elapsed();

    assert!(
        !result.memories.is_empty(),
        "should return inherited memories"
    );
    assert!(
        elapsed.as_millis() < 5,
        "CTE query took {}ms, expected < 5ms",
        elapsed.as_millis()
    );
}

// ── Scope validation — invalid scope_id ──────────────────────

#[tokio::test]
async fn create_memory_with_invalid_account_scope_id_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let svc = make_service(&ctx);
    let nonexistent_id = generate_id();

    let result = svc
        .create_memory(&CreateMemoryParams {
            id: generate_id(),
            scope_type: ScopeType::Account,
            scope_id: nonexistent_id,
            content: "should fail".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await;

    assert!(
        matches!(result, Err(MemoryError::InvalidScopeId { .. })),
        "expected InvalidScopeId, got: {result:?}"
    );
}

#[tokio::test]
async fn create_memory_with_invalid_org_scope_id_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let svc = make_service(&ctx);
    let nonexistent_id = generate_id();

    let result = svc
        .create_memory(&CreateMemoryParams {
            id: generate_id(),
            scope_type: ScopeType::Organization,
            scope_id: nonexistent_id,
            content: "should fail".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidScopeId { .. })));
}

#[tokio::test]
async fn create_memory_with_invalid_project_scope_id_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let svc = make_service(&ctx);
    let nonexistent_id = generate_id();

    let result = svc
        .create_memory(&CreateMemoryParams {
            id: generate_id(),
            scope_type: ScopeType::Project,
            scope_id: nonexistent_id,
            content: "should fail".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidScopeId { .. })));
}

#[tokio::test]
async fn create_memory_with_invalid_member_tag_scope_id_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let svc = make_service(&ctx);
    let nonexistent_id = generate_id();

    let result = svc
        .create_memory(&CreateMemoryParams {
            id: generate_id(),
            scope_type: ScopeType::MemberTag,
            scope_id: nonexistent_id,
            content: "should fail".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidScopeId { .. })));
}

#[tokio::test]
async fn create_memory_with_invalid_task_template_scope_id_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let svc = make_service(&ctx);
    let nonexistent_id = generate_id();

    let result = svc
        .create_memory(&CreateMemoryParams {
            id: generate_id(),
            scope_type: ScopeType::TaskTemplate,
            scope_id: nonexistent_id,
            content: "should fail".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidScopeId { .. })));
}

#[tokio::test]
async fn create_memory_with_invalid_task_scope_id_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let svc = make_service(&ctx);
    let nonexistent_id = generate_id();

    let result = svc
        .create_memory(&CreateMemoryParams {
            id: generate_id(),
            scope_type: ScopeType::Task,
            scope_id: nonexistent_id,
            content: "should fail".to_string(),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidScopeId { .. })));
}

#[tokio::test]
async fn create_library_document_with_invalid_scope_id_fails() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let svc = make_service(&ctx);
    let nonexistent_id = generate_id();

    let result = svc
        .create_library_document(&CreateLibraryDocumentParams {
            id: generate_id(),
            scope_type: ScopeType::Organization,
            scope_id: nonexistent_id,
            title: "Should fail".to_string(),
            content: "Should fail".to_string(),
            created_by: account_id,
        })
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidScopeId { .. })));
}

// ── Invalid scope type string ─────────────────────────────────

#[tokio::test]
async fn list_memories_with_invalid_scope_type_fails() {
    let ctx = common::TestContext::new().await;
    let svc = make_service(&ctx);

    let result = svc
        .list_memories("not_a_scope", generate_id(), None, 10)
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidScopeType(_))));
}

#[tokio::test]
async fn list_library_documents_with_invalid_scope_type_fails() {
    let ctx = common::TestContext::new().await;
    let svc = make_service(&ctx);

    let result = svc
        .list_library_documents("bad_scope", generate_id(), None, 10)
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidScopeType(_))));
}

// ── get_memory / get_library_document not found ───────────────

#[tokio::test]
async fn get_nonexistent_memory_returns_not_found() {
    let ctx = common::TestContext::new().await;
    let svc = make_service(&ctx);

    let result = svc.get_memory(generate_id()).await;
    assert!(matches!(result, Err(MemoryError::NotFound)));
}

#[tokio::test]
async fn get_nonexistent_library_document_returns_not_found() {
    let ctx = common::TestContext::new().await;
    let svc = make_service(&ctx);

    let result = svc.get_library_document(generate_id()).await;
    assert!(matches!(result, Err(MemoryError::LibraryDocumentNotFound)));
}

// ── Pagination cursor ─────────────────────────────────────────

#[tokio::test]
async fn memory_list_pagination_with_cursor() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let svc = make_service(&ctx);

    // Create 5 memories
    for i in 0..5 {
        let id = generate_id();
        svc.create_memory(&CreateMemoryParams {
            id,
            scope_type: ScopeType::Organization,
            scope_id: org_id,
            content: format!("Memory {i}"),
            library_ref: None,
            source: MemorySource::Manual,
            created_by: account_id,
        })
        .await
        .expect("should create memory");
    }

    // List first 3 (no cursor)
    let first_page = svc
        .list_memories("organization", org_id, None, 3)
        .await
        .expect("should list first page");
    assert_eq!(first_page.len(), 3);

    // Use the last ID of first page as cursor
    let cursor = first_page.last().map(|m| m.id);
    let second_page = svc
        .list_memories("organization", org_id, cursor, 3)
        .await
        .expect("should list second page");
    assert_eq!(second_page.len(), 2);

    // No overlap between pages
    let first_ids: std::collections::HashSet<_> = first_page.iter().map(|m| m.id).collect();
    let second_ids: std::collections::HashSet<_> = second_page.iter().map(|m| m.id).collect();
    assert!(first_ids.is_disjoint(&second_ids));
}

#[tokio::test]
async fn library_document_list_pagination_with_cursor() {
    let ctx = common::TestContext::new().await;
    let (account_id, _) = ctx.create_test_account().await;
    let org_id = ctx.create_test_org(account_id).await;
    let svc = make_service(&ctx);

    // Create 4 library documents
    for i in 0..4 {
        svc.create_library_document(&CreateLibraryDocumentParams {
            id: generate_id(),
            scope_type: ScopeType::Organization,
            scope_id: org_id,
            title: format!("Doc {i}"),
            content: format!("Content {i}"),
            created_by: account_id,
        })
        .await
        .expect("should create library document");
    }

    let first_page = svc
        .list_library_documents("organization", org_id, None, 2)
        .await
        .expect("should list first page");
    assert_eq!(first_page.len(), 2);

    let cursor = first_page.last().map(|d| d.id);
    let second_page = svc
        .list_library_documents("organization", org_id, cursor, 2)
        .await
        .expect("should list second page");
    assert_eq!(second_page.len(), 2);

    let first_ids: std::collections::HashSet<_> = first_page.iter().map(|d| d.id).collect();
    let second_ids: std::collections::HashSet<_> = second_page.iter().map(|d| d.id).collect();
    assert!(first_ids.is_disjoint(&second_ids));
}
