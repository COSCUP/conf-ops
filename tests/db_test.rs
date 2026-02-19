mod common;

use common::TestContext;

#[tokio::test]
async fn test_database_connection_and_migration() {
    let ctx = TestContext::new().await;

    let row: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&ctx.pool)
        .await
        .expect("Should execute query");

    assert_eq!(row.0, 1);
}

#[tokio::test]
async fn test_uuid_extension_available() {
    let ctx = TestContext::new().await;

    let row: (uuid::Uuid,) = sqlx::query_as("SELECT uuid_generate_v4()")
        .fetch_one(&ctx.pool)
        .await
        .expect("uuid-ossp extension should be available");

    assert_eq!(row.0.get_version_num(), 4);
}
