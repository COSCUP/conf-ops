use std::path::{Path, PathBuf};

use rocket::fairing::AdHoc;
use rocket::fs::{FileServer, NamedFile};
use rocket::http::Method;
use rocket::http::Header;
use rocket_db_pools::diesel::MysqlPool;
use rocket_db_pools::{Connection, Database};

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
async fn get_default_page(path: PathBuf) -> Option<NamedFile> {
    if path.starts_with("api/") {
        return None;
    }
    NamedFile::open("public/index.html").await.ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(AdHoc::config::<AppConfig>())
        .attach(MainDb::init())
        .attach(AdHoc::try_on_ignite("Data Folder", |rocket| async {
            let data_folder_path = std::env::current_dir().unwrap().join(Path::new("app-data"));
            let data_folder = DataFolder(data_folder_path);
            tokio::fs::create_dir_all(&data_folder.base_path())
                .await
                .unwrap();
            tokio::fs::create_dir_all(&data_folder.image_path(""))
                .await
                .unwrap();
            tokio::fs::create_dir_all(&data_folder.file_path(""))
                .await
                .unwrap();
            Ok(rocket.manage(data_folder))
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
