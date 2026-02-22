#![allow(dead_code)]
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use async_trait::async_trait;
use conf_ops::api::routes::ws::WsTokenStore;
use conf_ops::app_state::AppState;
use conf_ops::config::AppConfig;
use conf_ops::events::EventBus;
use conf_ops::id::generate_id;
use conf_ops::modules::ai::decision::DecisionService;
use conf_ops::modules::ai::placeholder::PlaceholderResolver;
use conf_ops::modules::ai::privacy::PrivacyEngine;
use conf_ops::modules::auth::jwt::{issue_access_token, JwtConfig};
use conf_ops::modules::auth::passkey::build_webauthn;
use conf_ops::modules::auth::repository::AccountRepository;
use conf_ops::modules::auth::service::AuthService;
use conf_ops::modules::conversation::awareness::AwarenessManager;
use conf_ops::modules::conversation::crdt::CrdtManager;
use conf_ops::modules::conversation::service::ConversationService;
use conf_ops::modules::conversation::ws_manager::WsManager;
use conf_ops::modules::core::contact::repository::ContactRepository;
use conf_ops::modules::core::contact::service::ContactService;
use conf_ops::modules::core::data_sheet::service::DataSheetService;
use conf_ops::modules::core::member::models::MemberRole;
use conf_ops::modules::core::member::repository::MemberRepository;
use conf_ops::modules::core::member::service::MemberService;
use conf_ops::modules::core::member_tag::repository::MemberTagRepository;
use conf_ops::modules::core::member_tag::service::MemberTagService;
use conf_ops::modules::core::organization::models::OrgRole;
use conf_ops::modules::core::organization::repository::{
    OrgMemberRepository, OrganizationRepository,
};
use conf_ops::modules::core::organization::service::OrganizationService;
use conf_ops::modules::core::permission::service::PermissionService;
use conf_ops::modules::core::project::service::ProjectService;
use conf_ops::modules::core::task::service::TaskService;
use conf_ops::modules::core::task_template::repository::TaskTemplateRepository;
use conf_ops::modules::core::task_template::service::TaskTemplateService;
use conf_ops::modules::core::todo::service::TodoService;
use conf_ops::modules::email::error::EmailError;
use conf_ops::modules::email::inbound::InboundEmailService;
use conf_ops::modules::email::service::EmailOutboundService;
use conf_ops::modules::email::{EmailHeaders, EmailService};
use conf_ops::modules::storage::local::LocalStorageBackend;
use conf_ops::modules::storage::service::{FileService, StorageConfig};
use conf_ops::modules::tools::service::ToolService;
use postgresql_embedded::PostgreSQL;
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct MockEmailService {
    pub sent: Mutex<Vec<(String, String, String)>>,
    pub sent_with_headers: Mutex<Vec<(String, String, String, EmailHeaders)>>,
}

impl MockEmailService {
    pub fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            sent_with_headers: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl EmailService for MockEmailService {
    async fn send(&self, to: &str, subject: &str, html_body: &str) -> Result<(), EmailError> {
        self.sent
            .lock()
            .await
            .push((to.to_string(), subject.to_string(), html_body.to_string()));
        Ok(())
    }

    async fn send_with_headers(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
        headers: &EmailHeaders,
    ) -> Result<(), EmailError> {
        self.sent_with_headers.lock().await.push((
            to.to_string(),
            subject.to_string(),
            html_body.to_string(),
            headers.clone(),
        ));
        Ok(())
    }
}

pub struct FailingEmailService;

#[async_trait]
impl EmailService for FailingEmailService {
    async fn send(&self, _to: &str, _subject: &str, _html_body: &str) -> Result<(), EmailError> {
        Err(EmailError::SmtpError("Simulated SMTP failure".to_string()))
    }

    async fn send_with_headers(
        &self,
        _to: &str,
        _subject: &str,
        _html_body: &str,
        _headers: &EmailHeaders,
    ) -> Result<(), EmailError> {
        Err(EmailError::SmtpError("Simulated SMTP failure".to_string()))
    }
}

pub struct TestContext {
    pub pool: PgPool,
    pub email_service: Arc<MockEmailService>,
    pub storage_dir: std::path::PathBuf,
    _pg: PostgreSQL,
    _temp_dir: tempfile::TempDir,
}

impl TestContext {
    pub async fn new() -> Self {
        let mut pg = PostgreSQL::default();
        pg.setup().await.expect("Failed to setup PostgreSQL");
        pg.start().await.expect("Failed to start PostgreSQL");

        let db_name = format!("test_{}", uuid::Uuid::now_v7().simple());
        pg.create_database(&db_name)
            .await
            .expect("Failed to create test database");

        let settings = pg.settings();
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            settings.username, settings.password, settings.host, settings.port, db_name,
        );

        let pool = PgPool::connect(&url)
            .await
            .expect("Failed to connect to test database");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        let email_service = Arc::new(MockEmailService::new());
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let storage_dir = temp_dir.path().join("storage");
        std::fs::create_dir_all(&storage_dir).expect("Failed to create storage dir");

        Self {
            pool,
            email_service,
            storage_dir,
            _pg: pg,
            _temp_dir: temp_dir,
        }
    }

    pub fn test_jwt_config() -> JwtConfig {
        JwtConfig {
            secret: "test-secret-key-at-least-256-bits-long-for-hs256".to_string(),
            issuer: "conf-ops-test".to_string(),
            access_token_expiry_secs: 900,
            refresh_token_expiry_secs: 604_800,
        }
    }

    pub fn test_app_config() -> AppConfig {
        AppConfig {
            database_url: String::new(),
            database_max_connections: 5,
            database_min_connections: 1,
            app_host: "127.0.0.1".to_string(),
            app_port: 8080,
            app_log_level: "debug".to_string(),
            app_base_url: "http://localhost:8080".to_string(),
            jwt_secret: "test-secret-key-at-least-256-bits-long-for-hs256".to_string(),
            jwt_issuer: "conf-ops-test".to_string(),
            jwt_access_expiry_secs: 900,
            jwt_refresh_expiry_secs: 604_800,
            webauthn_rp_id: "localhost".to_string(),
            webauthn_rp_origin: "http://localhost:3000".to_string(),
            webauthn_rp_name: "Conf-Ops Test".to_string(),
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "noreply@conf-ops.dev".to_string(),
            frontend_url: "http://localhost:3000".to_string(),
            authz_cache_ttl_secs: 300,
            storage_base_path: "./storage".to_string(),
            storage_max_image_size: 10 * 1024 * 1024,
            storage_max_document_size: 50 * 1024 * 1024,
            storage_max_file_size: 20 * 1024 * 1024,
            storage_cleanup_grace_period_secs: 604_800,
            email_inbound_api_key: Some("test-api-key".to_string()),
            email_domain: "conf-ops.dev".to_string(),
            crdt_ws_max_connections: 50,
            crdt_ws_heartbeat_interval_secs: 30,
            crdt_ws_idle_timeout_secs: 300,
            gemini_api_key: None,
            gemini_model: "gemini-2.5-flash".to_string(),
        }
    }

    pub fn app_state(&self) -> AppState {
        let jwt_config = Self::test_jwt_config();
        let app_config = Self::test_app_config();

        let webauthn = build_webauthn(&app_config).expect("should build webauthn");

        let event_bus = EventBus::default();

        let auth_service = Arc::new(AuthService::new(
            self.pool.clone(),
            jwt_config.clone(),
            self.email_service.clone(),
            webauthn,
            event_bus.clone(),
            &app_config,
        ));

        let org_service = Arc::new(OrganizationService::new(
            self.pool.clone(),
            event_bus.clone(),
            self.email_service.clone(),
            "http://localhost:3000".to_string(),
        ));

        let project_service = Arc::new(ProjectService::new(self.pool.clone(), event_bus.clone()));

        let member_service = Arc::new(MemberService::new(self.pool.clone(), event_bus.clone()));

        let member_tag_service =
            Arc::new(MemberTagService::new(self.pool.clone(), event_bus.clone()));

        let contact_service = Arc::new(ContactService::new(self.pool.clone(), event_bus.clone()));

        let permission_service =
            Arc::new(PermissionService::new(self.pool.clone(), &event_bus, 300));

        let task_template_service = Arc::new(TaskTemplateService::new(
            self.pool.clone(),
            event_bus.clone(),
        ));

        let task_service = Arc::new(TaskService::new(self.pool.clone(), event_bus.clone(), 120));

        let todo_service = Arc::new(TodoService::new(self.pool.clone(), event_bus.clone()));

        let data_sheet_service =
            Arc::new(DataSheetService::new(self.pool.clone(), event_bus.clone()));

        let storage_backend = Arc::new(LocalStorageBackend::new(&self.storage_dir));
        let file_service = Arc::new(FileService::new(
            self.pool.clone(),
            event_bus.clone(),
            storage_backend,
            StorageConfig::default(),
        ));

        let crdt_manager = Arc::new(CrdtManager::new(self.pool.clone()));
        let conversation_service = Arc::new(ConversationService::new(
            self.pool.clone(),
            event_bus.clone(),
            crdt_manager,
            Arc::clone(&file_service),
        ));

        let email_outbound_service = Arc::new(EmailOutboundService::new(
            self.pool.clone(),
            event_bus.clone(),
            self.email_service.clone(),
            "conf-ops.dev".to_string(),
        ));

        let inbound_email_service = Arc::new(InboundEmailService::new(
            self.pool.clone(),
            event_bus.clone(),
            Arc::clone(&file_service),
        ));

        let memory_service = Arc::new(conf_ops::modules::ai::memory::service::MemoryService::new(
            self.pool.clone(),
            event_bus.clone(),
        ));

        let privacy_engine = Arc::new(PrivacyEngine::new(self.pool.clone()));
        let placeholder_resolver = Arc::new(PlaceholderResolver::new(self.pool.clone()));
        let decision_service = Arc::new(DecisionService::new(
            self.pool.clone(),
            event_bus.clone(),
            Arc::clone(&placeholder_resolver),
        ));

        let tool_service = Arc::new(ToolService::new(&self.pool, event_bus.clone()));

        let ws_token_store = Arc::new(WsTokenStore::new());
        let ws_manager = Arc::new(WsManager::new(50));
        let awareness_manager = Arc::new(AwarenessManager::new());

        AppState {
            pool: self.pool.clone(),
            event_bus,
            jwt_config,
            app_base_url: "http://localhost:8080".to_string(),
            auth_service,
            org_service,
            member_service,
            member_tag_service,
            contact_service,
            project_service,
            permission_service,
            task_template_service,
            task_service,
            todo_service,
            data_sheet_service,
            file_service,
            conversation_service,
            email_outbound_service,
            inbound_email_service,
            email_inbound_api_key: Some("test-api-key".to_string()),
            ws_token_store,
            ws_manager,
            awareness_manager,
            crdt_ws_heartbeat_interval_secs: 30,
            crdt_ws_idle_timeout_secs: 300,
            memory_service,
            decision_service,
            placeholder_resolver,
            privacy_engine,
            tool_service,
        }
    }

    pub fn issue_test_token(account_id: Uuid) -> String {
        issue_access_token(&Self::test_jwt_config(), account_id).expect("should issue test token")
    }

    pub async fn create_test_account(&self) -> (Uuid, String) {
        let id = generate_id();
        let email = format!("test-{id}@example.com");
        AccountRepository::create(&self.pool, id, &email, "Test User")
            .await
            .expect("should create test account");
        (id, email)
    }

    pub async fn create_test_org(&self, owner_id: Uuid) -> Uuid {
        let org_id = generate_id();
        OrganizationRepository::create(
            &self.pool,
            org_id,
            "Test Organization",
            Some("Test org description"),
            None,
            owner_id,
        )
        .await
        .expect("should create test organization");

        let member_id = generate_id();
        OrgMemberRepository::create(&self.pool, member_id, org_id, owner_id, OrgRole::OrgOwner)
            .await
            .expect("should create org owner member");

        org_id
    }

    pub async fn create_test_project(&self, org_id: Uuid, created_by: Uuid) -> Uuid {
        let project_id = generate_id();
        conf_ops::modules::core::project::repository::ProjectRepository::create(
            &self.pool,
            project_id,
            org_id,
            "Test Project",
            Some("Test project description"),
            None,
            created_by,
        )
        .await
        .expect("should create test project");

        project_id
    }

    pub async fn create_test_contact(&self, org_id: Uuid, name: &str, email: &str) -> Uuid {
        let contact_id = generate_id();
        ContactRepository::create(&self.pool, contact_id, org_id, name, email)
            .await
            .expect("should create test contact");
        contact_id
    }

    pub async fn create_test_member(
        &self,
        project_id: Uuid,
        account_id: Uuid,
        role: MemberRole,
    ) -> Uuid {
        let member_id = generate_id();
        MemberRepository::create(&self.pool, member_id, project_id, account_id, role)
            .await
            .expect("should create test member");
        member_id
    }

    pub async fn create_test_tag(&self, project_id: Uuid, name: &str) -> Uuid {
        let tag_id = generate_id();
        MemberTagRepository::create(&self.pool, tag_id, project_id, name, None)
            .await
            .expect("should create test tag");
        tag_id
    }

    pub async fn assign_tag_to_member(
        &self,
        tag_id: Uuid,
        member_id: Uuid,
        project_id: Uuid,
    ) -> Uuid {
        let assignment_id = generate_id();
        MemberTagRepository::create_assignment(
            &self.pool,
            assignment_id,
            tag_id,
            Some(member_id),
            None,
            project_id,
        )
        .await
        .expect("should assign tag to member");
        assignment_id
    }

    pub async fn create_test_task_template(
        &self,
        project_id: Uuid,
        name: &str,
        created_by: Uuid,
    ) -> Uuid {
        let template_id = generate_id();
        TaskTemplateRepository::create(&self.pool, template_id, project_id, name, None, created_by)
            .await
            .expect("should create test task template");
        template_id
    }

    pub async fn link_tag_to_template(&self, task_template_id: Uuid, member_tag_id: Uuid) -> Uuid {
        let id = generate_id();
        TaskTemplateRepository::link_tag(&self.pool, id, task_template_id, member_tag_id)
            .await
            .expect("should link tag to template");
        id
    }

    pub async fn create_test_data_schema(
        &self,
        task_template_id: Uuid,
        name: &str,
        fields: &serde_json::Value,
    ) -> Uuid {
        let schema_id = generate_id();
        TaskTemplateRepository::create_data_schema(
            &self.pool,
            schema_id,
            task_template_id,
            name,
            fields,
        )
        .await
        .expect("should create test data schema");
        schema_id
    }

    pub async fn assign_tag_to_contact(
        &self,
        tag_id: Uuid,
        contact_id: Uuid,
        project_id: Uuid,
    ) -> Uuid {
        let assignment_id = generate_id();
        MemberTagRepository::create_assignment(
            &self.pool,
            assignment_id,
            tag_id,
            None,
            Some(contact_id),
            project_id,
        )
        .await
        .expect("should assign tag to contact");
        assignment_id
    }
}

// ── Shared test router ──────────────────────────────────────────

pub fn build_app(ctx: &TestContext) -> Router {
    let state = ctx.app_state();
    assemble_router(state)
}

/// Start a real TCP server and return (addr, `AppState`).
/// Needed for WebSocket tests where `oneshot` cannot do protocol upgrades.
pub async fn start_test_server(ctx: &TestContext) -> (std::net::SocketAddr, AppState) {
    let state = ctx.app_state();
    let app = assemble_router_with_ws(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind to ephemeral port");
    let addr = listener.local_addr().expect("get local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    (addr, state)
}

/// Assemble a router that includes the WS upgrade route (outside auth middleware).
fn assemble_router_with_ws(state: AppState) -> Router {
    use axum::routing::get;
    use conf_ops::api::routes::ws;

    let ws_route: Router = Router::new()
        .route(
            "/api/v1/projects/{projectId}/tasks/{taskId}/conversation/ws",
            get(ws::ws_upgrade),
        )
        .with_state(state.clone());

    assemble_router(state).merge(ws_route)
}

fn assemble_router(state: AppState) -> Router {
    use axum::routing::{get, post};
    use conf_ops::api::middleware::auth::auth_middleware;
    use conf_ops::api::routes::{data_entries, email_inbound, projects};

    let project_top = Router::new()
        .route(
            "/{projectId}",
            get(projects::get_project)
                .put(projects::update_project)
                .delete(projects::delete_project),
        )
        .nest("/{projectId}/members", test_member_routes())
        .nest("/{projectId}/member-tags", test_member_tag_routes())
        .nest("/{projectId}/task-templates", test_task_template_routes())
        .nest("/{projectId}/tasks", test_task_routes())
        .route(
            "/{projectId}/task-templates/{templateId}/data-sheets/{schemaId}",
            get(data_entries::get_aggregated_sheet),
        )
        .route(
            "/{projectId}/unassigned-inbox",
            get(email_inbound::list_unassigned_inbox),
        )
        .route(
            "/{projectId}/unassigned-inbox/{emailId}/assign",
            post(email_inbound::assign_unassigned_email),
        );

    // Email inbound webhook (outside auth middleware, uses API key auth)
    let email_inbound_route = Router::new()
        .route(
            "/api/v1/email/inbound",
            post(email_inbound::receive_inbound_email),
        )
        .with_state(state.clone());

    Router::new()
        .nest("/api/v1/organizations", test_org_routes())
        .nest(
            "/api/v1/organizations/{orgId}/projects",
            test_project_nested_routes(),
        )
        .nest(
            "/api/v1/organizations/{orgId}/contacts",
            test_contact_routes(),
        )
        .nest("/api/v1/projects", project_top)
        .nest("/api/v1/accounts", test_account_routes())
        .nest("/api/v1/files", conf_ops::api::routes::files::file_routes())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .merge(email_inbound_route)
        .with_state(state)
}

fn test_org_routes() -> Router<AppState> {
    use axum::routing::post;
    use conf_ops::api::routes::organizations;
    Router::new().route(
        "/",
        post(organizations::create_organization).get(organizations::list_organizations),
    )
}

fn test_project_nested_routes() -> Router<AppState> {
    use axum::routing::post;
    use conf_ops::api::routes::projects;
    Router::new().route(
        "/",
        post(projects::create_project).get(projects::list_projects),
    )
}

fn test_contact_routes() -> Router<AppState> {
    use axum::routing::{get, post};
    use conf_ops::api::routes::contacts;
    Router::new()
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
        )
}

fn test_member_routes() -> Router<AppState> {
    use axum::routing::{get, post};
    use conf_ops::api::routes::members;
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

fn test_member_tag_routes() -> Router<AppState> {
    use axum::routing::{delete, get, post, put};
    use conf_ops::api::routes::member_tags;
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

fn test_task_template_routes() -> Router<AppState> {
    use axum::routing::{delete, get, put};
    use conf_ops::api::routes::task_templates;
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

fn test_task_routes() -> Router<AppState> {
    use axum::routing::{delete, get, post, put};
    use conf_ops::api::routes::{ai_suggestions, conversations, data_entries, tasks, todos, ws};
    Router::new()
        .route("/", get(tasks::list_tasks).post(tasks::create_task))
        .route(
            "/{taskId}",
            get(tasks::get_task)
                .put(tasks::update_task)
                .delete(tasks::delete_task),
        )
        .route("/{taskId}/status", put(tasks::update_task_status))
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
        .route("/{taskId}/participants", get(tasks::get_task_participants))
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
            "/{taskId}/suggestions",
            get(ai_suggestions::list_suggestions),
        )
        .route(
            "/{taskId}/suggestions/request",
            post(ai_suggestions::request_suggestion),
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

fn test_account_routes() -> Router<AppState> {
    use axum::routing::get;
    use conf_ops::api::routes::todos;
    Router::new().route("/me/todos", get(todos::list_my_todos))
}

// ── Request helpers ─────────────────────────────────────────────

pub fn auth_header(token: &str) -> String {
    format!("Bearer {token}")
}

pub async fn body_json(resp: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

pub async fn json_request(
    app: Router,
    method: &str,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let auth = auth_header(token);
    let (req_body, content_type) = body.map_or_else(
        || (Body::empty(), None),
        |v| (Body::from(v.to_string()), Some("application/json")),
    );

    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", &auth);

    if let Some(ct) = content_type {
        builder = builder.header("Content-Type", ct);
    }

    let resp = app.oneshot(builder.body(req_body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::json!(null));
    (status, json)
}
