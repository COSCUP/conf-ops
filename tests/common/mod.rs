#![allow(dead_code)]
use std::sync::Arc;

use async_trait::async_trait;
use conf_ops::app_state::AppState;
use conf_ops::config::AppConfig;
use conf_ops::events::EventBus;
use conf_ops::id::generate_id;
use conf_ops::modules::auth::jwt::{issue_access_token, JwtConfig};
use conf_ops::modules::auth::passkey::build_webauthn;
use conf_ops::modules::auth::repository::AccountRepository;
use conf_ops::modules::auth::service::AuthService;
use conf_ops::modules::core::contact::repository::ContactRepository;
use conf_ops::modules::core::contact::service::ContactService;
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
use conf_ops::modules::core::task_template::repository::TaskTemplateRepository;
use conf_ops::modules::core::task_template::service::TaskTemplateService;
use conf_ops::modules::email::EmailService;
use postgresql_embedded::PostgreSQL;
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct MockEmailService {
    pub sent: Mutex<Vec<(String, String, String)>>,
}

impl MockEmailService {
    pub fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl EmailService for MockEmailService {
    async fn send(&self, to: &str, subject: &str, html_body: &str) -> Result<(), String> {
        self.sent
            .lock()
            .await
            .push((to.to_string(), subject.to_string(), html_body.to_string()));
        Ok(())
    }
}

pub struct TestContext {
    pub pool: PgPool,
    pub email_service: Arc<MockEmailService>,
    _pg: PostgreSQL,
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

        Self {
            pool,
            email_service,
            _pg: pg,
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
        }
    }

    #[allow(clippy::unused_self)]
    pub fn issue_test_token(&self, account_id: Uuid) -> String {
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
