use std::path::{Path, PathBuf};

use rocket::fairing::AdHoc;
use rocket::fs::{FileServer, NamedFile};
use rocket::http::Method;
use rocket::http::Header;
use rocket::{Build, Rocket};
use rocket_db_pools::diesel::MysqlPool;
use rocket_db_pools::{Database, Connection};

#[cfg(not(debug_assertions))]
use diesel_migrations::{EmbeddedMigrations, embed_migrations};

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde;

mod error;
mod models;
mod modules;
mod schema;
mod utils;

#[derive(Database)]
#[database("main_db")]
pub struct MainDb(MysqlPool);

pub type DbConn = Connection<MainDb>;

#[derive(Deserialize)]
pub struct AppConfig {
    secret_key: String,
    smtp_url: String,
    email_from: String,
}

pub struct DataFolder(pub std::path::PathBuf);

impl DataFolder {
    pub fn base_path(&self) -> std::path::PathBuf {
        self.0.clone()
    }

    pub fn image_path(&self, image_name: &str) -> std::path::PathBuf {
        self.0.join(Path::new("images")).join(image_name)
    }

    pub fn file_path(&self, file_name: &str) -> std::path::PathBuf {
        self.0.join(Path::new("files")).join(file_name)
    }
}

#[get("/<path..>", rank = 1)]
async fn get_default_page(
    path: PathBuf
) -> Option<NamedFile> {
    if path.starts_with("api/") {
        return None;
    }
    NamedFile::open("public/index.html").await.ok()
}

#[cfg(not(debug_assertions))]
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[cfg(debug_assertions)]
async fn run_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    println!("Not running migrations in debug mode");
    rocket
}

#[cfg(not(debug_assertions))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SimpleDatabaseConfig {
    url: String
}

#[cfg(not(debug_assertions))]
async fn run_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    use diesel_migrations::MigrationHarness;
    use rocket_db_pools::diesel::{AsyncMysqlConnection, async_connection_wrapper::AsyncConnectionWrapper};

    println!("Running migrations in release mode");

    let database_config = rocket.figment()
        .focus("databases.main_db")
        .extract::<SimpleDatabaseConfig>()
        .expect("Failed to extract database config");

    tokio::task::spawn_blocking(move || {
        let mut conn = <AsyncConnectionWrapper::<AsyncMysqlConnection> as diesel::Connection>::establish(&database_config.url)
            .expect("Failed to establish connection");

        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
    }).await.expect("Failed to run blocking task");

    rocket
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(AdHoc::config::<AppConfig>())
        .attach(MainDb::init())
        .attach(AdHoc::on_ignite("run migrations", run_migrations))
        .attach(AdHoc::on_ignite("Data Folder", |rocket| async {
            let data_folder_path = std::env::current_dir().unwrap().join(Path::new("app-data"));
            let data_folder = DataFolder(data_folder_path);
            tokio::fs::create_dir_all(&data_folder.base_path())
                .await
                .expect("Failed to create data folder");
            tokio::fs::create_dir_all(&data_folder.image_path(""))
                .await
                .expect("Failed to create image folder");
            tokio::fs::create_dir_all(&data_folder.file_path(""))
                .await
                .expect("Failed to create file folder");
            rocket.manage(data_folder)
        }))
        .attach(modules::stage())
        .mount("/", FileServer::from("public/").rank(0))
        .mount("/", routes![get_default_page])
        .attach(AdHoc::on_response("cache header", |req, res| Box::pin(async move {
            if req.method() != Method::Get || res.status().code >= 400 {
                return
            }

            if req.uri().path().starts_with("/api/") {
                res.set_header(Header::new("Cache-Control", "max-age=0, no-store"));
                return
            }
            if req.uri().path() == "/" || req.uri().path().starts_with("/index.html") {
                res.set_header(Header::new("Cache-Control", "max-age=0, no-store"));
                return
            }
            if req.uri().path().starts_with("/assets/") {
                res.set_header(Header::new("Cache-Control", "public, max-age=604800, immutable"));
                return
            }
            res.set_header(Header::new("Cache-Control", "max-age=600"));
        })))
}
