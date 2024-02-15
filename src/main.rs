use modules::base;
use rocket::fairing::AdHoc;
use rocket::fs::FileServer;
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
    smtp_host: String,
    smtp_port: u16,
    smtp_user: String,
    smtp_password: String,
    smtp_from: String,
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(MainDb::init())
        .attach(AdHoc::config::<AppConfig>())
        .mount("/", FileServer::from("public/"))
        .attach(base::stage())
}
