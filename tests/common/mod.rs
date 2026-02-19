use postgresql_embedded::PostgreSQL;
use sqlx::PgPool;

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
}
