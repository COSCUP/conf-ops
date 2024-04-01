// copy from https://github.com/rwf2/Rocket/blob/master/contrib/db_pools/lib/src/database.rs#L197
use std::ops::{Deref, DerefMut};

use rocket::{http::Status, request::{FromRequest, Outcome}, Ignite, Request, Rocket, Sentinel};
use rocket_db_pools::{Database, Pool};


pub struct Connection<D: Database>(<D::Pool as Pool>::Connection);

impl<D: Database> Connection<D> {
    pub async fn get (db: &D) -> Result<Self, <D::Pool as Pool>::Error> {
        match db.get().await {
            Ok(conn) => Ok(Connection(conn)),
            Err(e) => Err(e),
        }
    }
    pub fn into_inner(self) -> <D::Pool as Pool>::Connection {
        self.0
    }
}

#[rocket::async_trait]
impl<'r, D: Database> FromRequest<'r> for Connection<D> {
    type Error = Option<<D::Pool as Pool>::Error>;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match D::fetch(req.rocket()) {
            Some(db) => match db.get().await {
                Ok(conn) => Outcome::Success(Connection(conn)),
                Err(e) => Outcome::Error((Status::ServiceUnavailable, Some(e))),
            },
            None => Outcome::Error((Status::InternalServerError, None)),
        }
    }
}

impl<D: Database> Sentinel for Connection<D> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        D::fetch(rocket).is_none()
    }
}

impl<D: Database> Deref for Connection<D> {
    type Target = <D::Pool as Pool>::Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Database> DerefMut for Connection<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
