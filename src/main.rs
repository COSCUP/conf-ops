use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::{middleware, Router};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use conf_ops::api::middleware::auth::auth_middleware;
use conf_ops::api::routes::{
    accounts, auth, contacts, health, member_tags, members, organizations, projects,
    task_templates, tasks, todos,
};
use conf_ops::app_state::AppState;
use conf_ops::config::AppConfig;
use conf_ops::db;
use conf_ops::events::EventBus;
use conf_ops::modules::auth::jwt::JwtConfig;
use conf_ops::modules::auth::passkey::build_webauthn;
use conf_ops::modules::auth::service::AuthService;
use conf_ops::modules::core::contact::service::ContactService;
use conf_ops::modules::core::member::service::MemberService;
use conf_ops::modules::core::member_tag::service::MemberTagService;
use conf_ops::modules::core::organization::service::OrganizationService;
use conf_ops::modules::core::permission::service::PermissionService;
use conf_ops::modules::core::project::service::ProjectService;
use conf_ops::modules::core::task::service::TaskService;
use conf_ops::modules::core::task_template::service::TaskTemplateService;
use conf_ops::modules::core::todo::service::TodoService;
use conf_ops::modules::email::smtp::SmtpEmailService;

fn build_app_state(config: &AppConfig, pool: sqlx::PgPool) -> AppState {
    let event_bus = EventBus::default();
    let jwt_config = JwtConfig {
        secret: config.jwt_secret.clone(),
        issuer: config.jwt_issuer.clone(),
        access_token_expiry_secs: config.jwt_access_expiry_secs,
        refresh_token_expiry_secs: config.jwt_refresh_expiry_secs,
    };

    let email_service =
        Arc::new(SmtpEmailService::new(config).expect("Failed to create email service"));

    let webauthn = build_webauthn(config).expect("Failed to build WebAuthn");

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
        email_service,
        config.frontend_url.clone(),
    ));

    let project_service = Arc::new(ProjectService::new(pool.clone(), event_bus.clone()));

    let member_service = Arc::new(MemberService::new(pool.clone(), event_bus.clone()));

    let member_tag_service = Arc::new(MemberTagService::new(pool.clone(), event_bus.clone()));

    let contact_service = Arc::new(ContactService::new(pool.clone(), event_bus.clone()));

    let permission_service = Arc::new(PermissionService::new(
        pool.clone(),
        &event_bus,
        config.authz_cache_ttl_secs,
    ));

    let task_template_service = Arc::new(TaskTemplateService::new(pool.clone(), event_bus.clone()));

    let task_service = Arc::new(TaskService::new(pool.clone(), event_bus.clone()));

    let todo_service = Arc::new(TodoService::new(pool.clone(), event_bus.clone()));

    AppState {
        pool,
        event_bus,
        jwt_config,
        app_base_url: config.app_base_url.clone(),
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
        .nest("/{orgId}/contacts", contact_routes)
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
}

fn project_routes() -> Router<AppState> {
    let member_routes = Router::new()
        .route("/", get(members::list_members))
        .route("/invite", post(members::invite_member))
        .route(
            "/{memberId}",
            get(members::get_member)
                .put(members::update_member)
                .delete(members::delete_member),
        );

    let member_tag_routes = Router::new()
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
        );

    let task_template_routes = Router::new()
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
        );

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
        .nest("/{projectId}/members", member_routes)
        .nest("/{projectId}/member-tags", member_tag_routes)
        .nest("/{projectId}/task-templates", task_template_routes)
        .nest("/{projectId}/tasks", task_routes())
}

fn build_router(state: AppState) -> Router {
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
        );

    let api_v1 = Router::new()
        .nest("/auth", auth_routes())
        .nest("/accounts", account_routes)
        .nest("/organizations", org_routes())
        .nest("/projects", project_routes());

    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .nest("/api/v1", api_v1)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
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
    let app = build_router(state);

    let addr: SocketAddr = format!("{}:{}", config.app_host, config.app_port)
        .parse()
        .expect("Invalid address");

    tracing::info!("Starting server on {addr}");

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app).await.expect("Server error");
}
