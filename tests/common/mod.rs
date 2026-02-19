use conf_ops::app_state::AppState;
use conf_ops::events::EventBus;
use conf_ops::id::generate_id;
use conf_ops::modules::auth::jwt::{issue_access_token, JwtConfig};
use conf_ops::modules::auth::repository::AccountRepository;
use postgresql_embedded::PostgreSQL;
use sqlx::PgPool;
use uuid::Uuid;

pub struct TestContext {
    pub pool: PgPool,
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

        Self { pool, _pg: pg }
    }

    pub fn test_jwt_config() -> JwtConfig {
        JwtConfig {
            secret: "test-secret-key-at-least-256-bits-long-for-hs256".to_string(),
            issuer: "conf-ops-test".to_string(),
            access_token_expiry_secs: 900,
            refresh_token_expiry_secs: 604_800,
        }
    }

    pub fn app_state(&self) -> AppState {
        AppState {
            pool: self.pool.clone(),
            event_bus: EventBus::default(),
            jwt_config: Self::test_jwt_config(),
            app_base_url: "http://localhost:8080".to_string(),
        }
    }

    pub fn issue_test_token(&self, account_id: Uuid) -> String {
        issue_access_token(&Self::test_jwt_config(), account_id).expect("should issue test token")
    }

    pub async fn create_test_account(&self) -> (Uuid, String) {
        let id = generate_id();
        let email = format!("test-{}@example.com", id);
        AccountRepository::create(&self.pool, id, &email, "Test User")
            .await
            .expect("should create test account");
        (id, email)
    }
}
