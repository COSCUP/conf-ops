use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::{middleware, Router};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use conf_ops::api::middleware::auth::auth_middleware;
use conf_ops::api::middleware::cors::build_cors_layer;
use conf_ops::api::middleware::rate_limit::{ai_rate_limit, auth_rate_limit, global_rate_limit};
use conf_ops::api::middleware::security_headers::security_headers;
use conf_ops::api::routes::ws::WsTokenStore;
use conf_ops::api::routes::{
    accounts, ai_suggestions, api_keys, audit, auth, contacts, conversations, data_entries,
    data_external, email_inbound, email_threads, files, health, member_tags, members, memories,
    notifications, organizations, projects, task_templates, tasks, todos, tools, webhooks, ws,
};
use conf_ops::app_state::AppState;
use conf_ops::config::AppConfig;
use conf_ops::db;
use conf_ops::events::EventBus;
use conf_ops::modules::ai::context::ContextAssembler;
use conf_ops::modules::ai::decision::DecisionService;
use conf_ops::modules::ai::llm_client::{GeminiClient, LlmProvider, MockLlmProvider};
use conf_ops::modules::ai::memory::service::MemoryService;
use conf_ops::modules::ai::pipeline::PipelineWorker;
use conf_ops::modules::ai::placeholder::PlaceholderResolver;
use conf_ops::modules::ai::privacy::PrivacyEngine;
use conf_ops::modules::ai::trigger::TriggerRouter;
use conf_ops::modules::audit::service::AuditService;
use conf_ops::modules::auth::jwt::JwtConfig;
use conf_ops::modules::auth::passkey::build_webauthn;
use conf_ops::modules::auth::service::AuthService;
use conf_ops::modules::conversation::awareness::AwarenessManager;
use conf_ops::modules::conversation::crdt::CrdtManager;
use conf_ops::modules::conversation::service::ConversationService;
use conf_ops::modules::conversation::ws_manager::WsManager;
use conf_ops::modules::core::api_key::service::ApiKeyService;
use conf_ops::modules::core::contact::service::ContactService;
use conf_ops::modules::core::data_sheet::service::DataSheetService;
use conf_ops::modules::core::member::service::MemberService;
use conf_ops::modules::core::member_tag::service::MemberTagService;
use conf_ops::modules::core::organization::service::OrganizationService;
use conf_ops::modules::core::permission::service::PermissionService;
use conf_ops::modules::core::project::service::ProjectService;
use conf_ops::modules::core::task::service::TaskService;
use conf_ops::modules::core::task_template::service::TaskTemplateService;
use conf_ops::modules::core::todo::service::TodoService;
use conf_ops::modules::core::webhook::service::WebhookService;
use conf_ops::modules::email::inbound::InboundEmailService;
use conf_ops::modules::email::service::EmailOutboundService;
use conf_ops::modules::email::smtp::SmtpEmailService;
use conf_ops::modules::notifications::service::NotificationService;
use conf_ops::modules::notifications::web_push::WebPushSender;
use conf_ops::modules::storage::local::LocalStorageBackend;
use conf_ops::modules::storage::service::{FileService, StorageConfig};
use conf_ops::modules::tools::service::ToolService;

fn build_file_service(
    config: &AppConfig,
    pool: sqlx::PgPool,
    event_bus: EventBus,
) -> Arc<FileService> {
    let storage_backend = Arc::new(LocalStorageBackend::new(&config.storage_base_path));
    let storage_config = StorageConfig {
        max_image_size: config.storage_max_image_size,
        max_document_size: config.storage_max_document_size,
        max_file_size: config.storage_max_file_size,
        cleanup_grace_period_secs: config.storage_cleanup_grace_period_secs,
    };
    Arc::new(FileService::new(
        pool,
        event_bus,
        storage_backend,
        storage_config,
    ))
}

fn build_core_services(
    config: &AppConfig,
    pool: &sqlx::PgPool,
    event_bus: &EventBus,
    email_service: &Arc<dyn conf_ops::modules::email::EmailService>,
) -> CoreServices {
    let webauthn = build_webauthn(config).expect("Failed to build WebAuthn");
    let jwt_config = JwtConfig {
        secret: config.jwt_secret.clone(),
        issuer: config.jwt_issuer.clone(),
        access_token_expiry_secs: config.jwt_access_expiry_secs,
        refresh_token_expiry_secs: config.jwt_refresh_expiry_secs,
    };
    let auth_service = Arc::new(AuthService::new(
        pool.clone(),
        jwt_config.clone(),
        email_service.clone(),
        webauthn,
        event_bus.clone(),
        config,
    ));
    let org_service = Arc::new(OrganizationService::new(
        pool.clone(),
        event_bus.clone(),
        email_service.clone(),
        config.frontend_url.clone(),
    ));
    CoreServices {
        jwt_config,
        auth_service,
        org_service,
        project_service: Arc::new(ProjectService::new(pool.clone(), event_bus.clone())),
        member_service: Arc::new(MemberService::new(pool.clone(), event_bus.clone())),
        member_tag_service: Arc::new(MemberTagService::new(pool.clone(), event_bus.clone())),
        contact_service: Arc::new(ContactService::new(pool.clone(), event_bus.clone())),
        permission_service: Arc::new(PermissionService::new(
            pool.clone(),
            event_bus,
            config.authz_cache_ttl_secs,
        )),
        task_template_service: Arc::new(TaskTemplateService::new(pool.clone(), event_bus.clone())),
        task_service: Arc::new(TaskService::new(pool.clone(), event_bus.clone(), 120)),
        todo_service: Arc::new(TodoService::new(pool.clone(), event_bus.clone())),
        data_sheet_service: Arc::new(DataSheetService::new(pool.clone(), event_bus.clone())),
    }
}

struct CoreServices {
    jwt_config: JwtConfig,
    auth_service: Arc<AuthService>,
    org_service: Arc<OrganizationService>,
    project_service: Arc<ProjectService>,
    member_service: Arc<MemberService>,
    member_tag_service: Arc<MemberTagService>,
    contact_service: Arc<ContactService>,
    permission_service: Arc<PermissionService>,
    task_template_service: Arc<TaskTemplateService>,
    task_service: Arc<TaskService>,
    todo_service: Arc<TodoService>,
    data_sheet_service: Arc<DataSheetService>,
}

fn build_app_state(config: &AppConfig, pool: sqlx::PgPool) -> AppState {
    let event_bus = EventBus::default();
    let email_service: Arc<dyn conf_ops::modules::email::EmailService> =
        Arc::new(SmtpEmailService::new(config).expect("Failed to create email service"));

    let core = build_core_services(config, &pool, &event_bus, &email_service);
    let file_service = build_file_service(config, pool.clone(), event_bus.clone());

    let crdt_manager = Arc::new(CrdtManager::new(pool.clone()));
    let conversation_service = Arc::new(ConversationService::new(
        pool.clone(),
        event_bus.clone(),
        crdt_manager,
        Arc::clone(&file_service),
    ));
    let email_outbound_service = Arc::new(EmailOutboundService::new(
        pool.clone(),
        event_bus.clone(),
        email_service.clone(),
        config.email_domain.clone(),
    ));
    let inbound_email_service = Arc::new(InboundEmailService::new(
        pool.clone(),
        event_bus.clone(),
        Arc::clone(&file_service),
    ));
    let memory_service = Arc::new(MemoryService::new(pool.clone(), event_bus.clone()));
    let privacy_engine = Arc::new(PrivacyEngine::new(pool.clone()));
    let placeholder_resolver = Arc::new(PlaceholderResolver::new(pool.clone()));
    let decision_service = Arc::new(DecisionService::new(
        pool.clone(),
        event_bus.clone(),
        Arc::clone(&placeholder_resolver),
    ));
    let tool_service = Arc::new(ToolService::new(&pool, event_bus.clone()));
    let web_push_sender = Arc::new(WebPushSender::from_env());
    let notification_service = Arc::new(NotificationService::new(
        pool.clone(),
        event_bus.clone(),
        web_push_sender,
        email_service,
    ));
    let webhook_service = Arc::new(WebhookService::new(pool.clone(), event_bus.clone()));
    let api_key_service = Arc::new(ApiKeyService::new(pool.clone()));
    let audit_service = Arc::new(AuditService::new(pool.clone(), event_bus.clone()));

    AppState {
        pool,
        event_bus,
        jwt_config: core.jwt_config,
        app_base_url: config.app_base_url.clone(),
        auth_service: core.auth_service,
        org_service: core.org_service,
        member_service: core.member_service,
        member_tag_service: core.member_tag_service,
        contact_service: core.contact_service,
        project_service: core.project_service,
        permission_service: core.permission_service,
        task_template_service: core.task_template_service,
        task_service: core.task_service,
        todo_service: core.todo_service,
        data_sheet_service: core.data_sheet_service,
        file_service,
        conversation_service,
        email_outbound_service,
        inbound_email_service,
        email_inbound_api_key: config.email_inbound_api_key.clone(),
        ws_token_store: Arc::new(WsTokenStore::new()),
        ws_manager: Arc::new(WsManager::new(config.crdt_ws_max_connections)),
        awareness_manager: Arc::new(AwarenessManager::new()),
        crdt_ws_heartbeat_interval_secs: config.crdt_ws_heartbeat_interval_secs,
        crdt_ws_idle_timeout_secs: config.crdt_ws_idle_timeout_secs,
        memory_service,
        decision_service,
        placeholder_resolver,
        privacy_engine,
        tool_service,
        notification_service,
        webhook_service,
        api_key_service,
        audit_service,
    }
}

fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/magic-link/request", post(auth::request_magic_link))
        .route("/magic-link/verify", get(auth::verify_magic_link))
        .route("/refresh", post(auth::refresh))
        .route("/logout", post(auth::logout))
        .route(
            "/passkey/register/begin",
            post(auth::passkey_register_begin),
        )
        .route(
            "/passkey/register/complete",
            post(auth::passkey_register_complete),
        )
        .route("/passkey/login/begin", post(auth::passkey_login_begin))
        .route(
            "/passkey/login/complete",
            post(auth::passkey_login_complete),
        )
        .route("/passkeys", get(accounts::list_passkeys))
        .route("/passkeys/{id}", delete(accounts::delete_passkey))
}

fn org_routes() -> Router<AppState> {
    let contact_routes = Router::new()
        .route(
            "/",
            post(contacts::create_contact).get(contacts::list_contacts),
        )
        .route("/merge", post(contacts::merge_contacts))
        .route(
            "/{contactId}",
            get(contacts::get_contact)
                .put(contacts::update_contact)
                .delete(contacts::delete_contact),
        );

    Router::new()
        .route(
            "/",
            post(organizations::create_organization).get(organizations::list_organizations),
        )
        .route(
            "/{orgId}",
            get(organizations::get_organization)
                .put(organizations::update_organization)
                .delete(organizations::delete_organization),
        )
        .route("/{orgId}/members", get(organizations::list_members))
        .route(
            "/{orgId}/members/invite",
            post(organizations::invite_member),
        )
        .route(
            "/{orgId}/members/{memberId}",
            put(organizations::update_member_role).delete(organizations::remove_member),
        )
        .nest(
            "/{orgId}/projects",
            Router::new()
                .route(
                    "/",
                    post(projects::create_project).get(projects::list_projects),
                )
                .route("/copy", post(projects::copy_project)),
        )
        .route("/{orgId}/tool-configs", get(tools::list_org_tool_configs))
        .nest("/{orgId}/contacts", contact_routes)
        .route("/{orgId}/audit-logs", get(audit::list_org_audit_logs))
}

fn task_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(tasks::list_tasks).post(tasks::create_task))
        .route(
            "/{taskId}",
            get(tasks::get_task)
                .put(tasks::update_task)
                .delete(tasks::delete_task),
        )
        .route("/{taskId}/status", put(tasks::update_task_status))
        .route("/{taskId}/participants", get(tasks::get_task_participants))
        .route(
            "/{taskId}/todos",
            get(todos::list_todos).post(todos::create_todo),
        )
        .route(
            "/{taskId}/todos/{todoId}",
            get(todos::get_todo)
                .put(todos::update_todo)
                .delete(todos::delete_todo),
        )
        .route(
            "/{taskId}/todos/{todoId}/status",
            put(todos::update_todo_status),
        )
        .route(
            "/{taskId}/todos/{todoId}/assignees",
            post(todos::add_assignee),
        )
        .route(
            "/{taskId}/todos/{todoId}/assignees/{memberId}",
            delete(todos::remove_assignee),
        )
        .route(
            "/{taskId}/todos/{todoId}/linked-task",
            put(todos::link_task).delete(todos::unlink_task),
        )
        .route("/{taskId}/data-entries", get(data_entries::list_entries))
        .route(
            "/{taskId}/data-entries/{schemaId}",
            get(data_entries::get_entry)
                .put(data_entries::upsert_entry)
                .delete(data_entries::delete_entry),
        )
        .route(
            "/{taskId}/data-entries/{schemaId}/share",
            post(data_entries::share_data),
        )
        .route(
            "/{taskId}/conversation",
            get(conversations::get_conversation),
        )
        .route(
            "/{taskId}/conversation/messages",
            post(conversations::send_message),
        )
        .route(
            "/{taskId}/conversation/last-seen",
            get(conversations::get_last_seen).put(conversations::update_last_seen),
        )
        .route(
            "/{taskId}/conversation/last-seen-position",
            get(conversations::get_last_seen_position)
                .put(conversations::update_last_seen_position),
        )
        .route("/{taskId}/conversation/ws-token", post(ws::create_ws_token))
        .route(
            "/{taskId}/email-threads",
            get(email_threads::list_email_threads).post(email_threads::create_email_thread),
        )
        .route(
            "/{taskId}/email-threads/{threadId}",
            axum::routing::patch(email_threads::update_email_thread)
                .delete(email_threads::delete_email_thread),
        )
        .route(
            "/{taskId}/email-threads/{threadId}/messages",
            get(email_threads::list_thread_messages).post(email_threads::send_thread_email),
        )
        .route(
            "/{taskId}/suggestions",
            get(ai_suggestions::list_suggestions),
        )
        .route(
            "/{taskId}/suggestions/request",
            post(ai_suggestions::request_suggestion).layer(middleware::from_fn(ai_rate_limit)),
        )
        .route(
            "/{taskId}/suggestions/{groupId}",
            get(ai_suggestions::get_suggestion_group),
        )
        .route(
            "/{taskId}/suggestions/{groupId}/suggestions/{suggestionId}/decide",
            post(ai_suggestions::decide_suggestion),
        )
        .route(
            "/{taskId}/ai/resolve-placeholders",
            post(ai_suggestions::resolve_placeholders),
        )
}

fn member_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(members::list_members))
        .route("/invite", post(members::invite_member))
        .route(
            "/{memberId}",
            get(members::get_member)
                .put(members::update_member)
                .delete(members::delete_member),
        )
}

fn member_tag_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(member_tags::list_tags).post(member_tags::create_tag),
        )
        .route(
            "/{tagId}",
            get(member_tags::get_tag)
                .put(member_tags::update_tag)
                .delete(member_tags::delete_tag),
        )
        .route("/{tagId}/assign", post(member_tags::assign_tag))
        .route(
            "/{tagId}/assignments/{assignmentId}",
            delete(member_tags::remove_assignment),
        )
        .route(
            "/{tagId}/external-task-creation",
            put(member_tags::update_external_task_creation),
        )
}

fn task_template_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(task_templates::list_templates).post(task_templates::create_template),
        )
        .route(
            "/{templateId}",
            get(task_templates::get_template)
                .put(task_templates::update_template)
                .delete(task_templates::delete_template),
        )
        .route(
            "/{templateId}/tags",
            get(task_templates::list_template_tags).post(task_templates::link_tag),
        )
        .route(
            "/{templateId}/tags/{memberTagId}",
            delete(task_templates::unlink_tag),
        )
        .route(
            "/{templateId}/todo-templates",
            get(task_templates::list_todo_templates).post(task_templates::create_todo_template),
        )
        .route(
            "/{templateId}/todo-templates/reorder",
            put(task_templates::reorder_todo_templates),
        )
        .route(
            "/{templateId}/todo-templates/{todoTemplateId}",
            get(task_templates::get_todo_template)
                .put(task_templates::update_todo_template)
                .delete(task_templates::delete_todo_template),
        )
        .route(
            "/{templateId}/data-schemas",
            get(task_templates::list_data_schemas).post(task_templates::create_data_schema),
        )
        .route(
            "/{templateId}/data-schemas/{schemaId}",
            get(task_templates::get_data_schema)
                .put(task_templates::update_data_schema)
                .delete(task_templates::delete_data_schema),
        )
}

fn webhook_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(webhooks::list_webhooks).post(webhooks::create_webhook),
        )
        .route(
            "/{webhookId}",
            get(webhooks::get_webhook)
                .put(webhooks::update_webhook)
                .delete(webhooks::delete_webhook),
        )
        .route("/{webhookId}/test", post(webhooks::test_webhook))
        .route("/{webhookId}/logs", get(webhooks::list_webhook_logs))
}

fn api_key_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(api_keys::list_api_keys).post(api_keys::create_api_key),
        )
        .route("/{keyId}", delete(api_keys::delete_api_key))
}

fn project_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/{projectId}",
            get(projects::get_project)
                .put(projects::update_project)
                .delete(projects::delete_project),
        )
        .route("/{projectId}/status", put(projects::update_project_status))
        .route(
            "/{projectId}/permission-settings",
            get(projects::get_permission_settings).put(projects::update_permission_settings),
        )
        .nest("/{projectId}/members", member_routes())
        .nest("/{projectId}/member-tags", member_tag_routes())
        .nest("/{projectId}/task-templates", task_template_routes())
        .nest("/{projectId}/tasks", task_routes())
        .route(
            "/{projectId}/unassigned-inbox",
            get(email_inbound::list_unassigned_inbox),
        )
        .route(
            "/{projectId}/unassigned-inbox/{emailId}/assign",
            post(email_inbound::assign_unassigned_email),
        )
        .route(
            "/{projectId}/task-templates/{templateId}/data-sheets/{schemaId}",
            get(data_entries::get_aggregated_sheet),
        )
        .merge(tool_routes())
        .nest("/{projectId}/webhooks", webhook_routes())
        .nest("/{projectId}/api-keys", api_key_routes())
        .route(
            "/{projectId}/audit-logs",
            get(audit::list_project_audit_logs),
        )
}

fn tool_routes() -> Router<AppState> {
    Router::new()
        .route("/{projectId}/tools", get(tools::list_tools))
        .route(
            "/{projectId}/tools/{toolName}",
            get(tools::get_tool_details),
        )
        .route(
            "/{projectId}/tools/{toolName}/execute",
            post(tools::execute_tool),
        )
        .route(
            "/{projectId}/tool-configs",
            get(tools::list_project_tool_configs).post(tools::create_project_tool_config),
        )
        .route(
            "/{projectId}/tool-configs/{configId}",
            put(tools::update_project_tool_config).delete(tools::delete_project_tool_config),
        )
}

fn external_v1_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{projectId}/task-templates",
            get(data_external::list_external_templates),
        )
        .route(
            "/projects/{projectId}/task-templates/{templateId}/data",
            get(data_external::get_template_data),
        )
        .route(
            "/projects/{projectId}/tasks/{taskId}/data",
            get(data_external::get_task_data).put(data_external::update_task_data),
        )
}

fn build_router(state: AppState, config: &AppConfig) -> Router {
    let account_routes = Router::new()
        .route("/me", get(accounts::get_me).patch(accounts::update_me))
        .route(
            "/me/profile",
            get(accounts::get_profile).put(accounts::update_profile),
        )
        .route(
            "/me/notification-preferences",
            get(accounts::get_notification_preferences)
                .put(accounts::update_notification_preferences),
        )
        .route("/me/todos", get(todos::list_my_todos));

    let memory_routes = Router::new()
        .route(
            "/",
            get(memories::list_memories).post(memories::create_memory),
        )
        .route(
            "/{memoryId}",
            get(memories::get_memory)
                .put(memories::update_memory)
                .delete(memories::delete_memory),
        )
        .route("/{memoryId}/versions", get(memories::list_memory_versions));

    let library_document_routes = Router::new()
        .route(
            "/",
            get(memories::list_library_documents).post(memories::create_library_document),
        )
        .route(
            "/{documentId}",
            get(memories::get_library_document)
                .put(memories::update_library_document)
                .delete(memories::delete_library_document),
        )
        .route(
            "/{documentId}/versions",
            get(memories::list_library_document_versions),
        );

    let notification_routes = Router::new()
        .route("/", get(notifications::list_notifications))
        .route("/unread-count", get(notifications::get_unread_count))
        .route("/{notificationId}/read", put(notifications::mark_as_read))
        .route("/read-all", put(notifications::mark_all_as_read))
        .route(
            "/web-push/subscribe",
            post(notifications::subscribe_web_push),
        )
        .route(
            "/web-push/subscriptions/{endpoint}",
            delete(notifications::unsubscribe_web_push),
        );

    let api_v1 = Router::new()
        .nest(
            "/auth",
            auth_routes().layer(middleware::from_fn(auth_rate_limit)),
        )
        .nest("/accounts", account_routes)
        .nest("/organizations", org_routes())
        .nest("/projects", project_routes())
        .nest("/notifications", notification_routes)
        .nest("/files", files::file_routes())
        .nest("/memories", memory_routes)
        .nest("/library-documents", library_document_routes);

    // WS upgrade route must be outside auth middleware
    let ws_route = Router::new().route(
        "/api/v1/projects/{projectId}/tasks/{taskId}/conversation/ws",
        get(ws::ws_upgrade),
    );

    // Email inbound webhook must be outside auth middleware (uses API key auth)
    let email_inbound_route = Router::new().route(
        "/api/v1/email/inbound",
        post(email_inbound::receive_inbound_email),
    );

    let cors_layer = build_cors_layer(&config.cors_origins);

    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .nest("/api/v1", api_v1)
        .nest("/external/v1", external_v1_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .merge(ws_route)
        .merge(email_inbound_route)
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(global_rate_limit))
        .layer(cors_layer)
        .with_state(state)
}

fn spawn_background_tasks(state: &AppState, config: &AppConfig) {
    // Spawn periodic CRDT compaction background task (every 10 minutes, threshold: 100 ops)
    let compaction_crdt_manager = Arc::clone(state.conversation_service.crdt_manager());
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
        loop {
            interval.tick().await;
            match compaction_crdt_manager.compact_if_needed(100).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("CRDT compaction: compacted {n} document(s)"),
                Err(e) => tracing::warn!("CRDT compaction error: {e}"),
            }
        }
    });

    // Spawn periodic email retry background task (every 60 seconds)
    let retry_service = Arc::clone(&state.email_outbound_service);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            match retry_service.retry_failed_emails().await {
                Ok(0) => {}
                Ok(n) => tracing::info!("Email retry: retried {n} message(s)"),
                Err(e) => tracing::warn!("Email retry error: {e}"),
            }
        }
    });

    // Spawn memory cache invalidation listener
    state
        .memory_service
        .start_cache_invalidation(&state.event_bus);

    // Start AI pipeline: TriggerRouter + PipelineWorker
    TriggerRouter::start(state.pool.clone(), &state.event_bus);

    let llm_provider: Arc<dyn LlmProvider> = if let Some(ref api_key) = config.gemini_api_key {
        Arc::new(GeminiClient::new(
            api_key.clone(),
            config.gemini_model.clone(),
        ))
    } else {
        tracing::warn!("GEMINI_API_KEY not set, using MockLlmProvider for AI pipeline");
        Arc::new(MockLlmProvider::new())
    };

    let context_assembler = Arc::new(ContextAssembler::new(
        state.pool.clone(),
        Arc::clone(&state.memory_service),
        Arc::clone(&state.privacy_engine),
    ));

    let pipeline_worker = Arc::new(PipelineWorker::new(
        state.pool.clone(),
        state.event_bus.clone(),
        context_assembler,
        llm_provider,
    ));
    pipeline_worker.start();

    // Start notification event listener
    state.notification_service.start_event_listener();

    // Start reminder scheduler
    let reminder_scheduler = Arc::new(
        conf_ops::modules::notifications::scheduler::ReminderScheduler::new(Arc::clone(
            &state.notification_service,
        )),
    );
    reminder_scheduler.start();

    // Spawn periodic due date and stale todo scanner (every hour)
    let scanner_pool = state.pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            match conf_ops::modules::notifications::scheduler::scan_due_date_reminders(
                &scanner_pool,
            )
            .await
            {
                Ok(0) => {}
                Ok(n) => tracing::info!("Due date scanner: created {n} reminder(s)"),
                Err(e) => tracing::warn!("Due date scanner error: {e}"),
            }
            match conf_ops::modules::notifications::scheduler::scan_stale_todos(&scanner_pool, 7)
                .await
            {
                Ok(0) => {}
                Ok(n) => tracing::info!("Stale todo scanner: created {n} reminder(s)"),
                Err(e) => tracing::warn!("Stale todo scanner error: {e}"),
            }
        }
    });

    // Start webhook event listener
    state.webhook_service.start_event_listener();

    // Spawn periodic webhook retry task (every 60 seconds)
    let webhook_retry_service = Arc::clone(&state.webhook_service);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            match webhook_retry_service.retry_failed_events().await {
                Ok(0) => {}
                Ok(n) => tracing::info!("Webhook retry: retried {n} event(s)"),
                Err(e) => tracing::warn!("Webhook retry error: {e}"),
            }
        }
    });

    // Start audit event listener
    state.audit_service.start_event_listener();

    // Start audit partition manager
    state.audit_service.start_partition_manager();
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env().expect("Failed to load configuration");

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.app_log_level))
        .init();

    let pool = db::create_pool(&config)
        .await
        .expect("Failed to create database pool");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let state = build_app_state(&config, pool);

    spawn_background_tasks(&state, &config);

    let app = build_router(state, &config);

    let addr: SocketAddr = format!("{}:{}", config.app_host, config.app_port)
        .parse()
        .expect("Invalid address");

    tracing::info!("Starting server on {addr}");

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app).await.expect("Server error");
}
