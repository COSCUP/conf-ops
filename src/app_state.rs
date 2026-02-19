use sqlx::PgPool;

use crate::events::EventBus;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub event_bus: EventBus,
}
