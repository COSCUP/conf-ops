use crate::models::role::Role;
use crate::models::user::User;
use crate::DbConn;

use self::forms::models::TicketFormAnswer;
use self::forms::FormSchema;
use self::models::{Ticket, TicketFlow, TicketSchema, TicketSchemaFlow};
use self::reviews::models::{TicketReview, TicketSchemaReview};

use super::EnabledFeature;

pub mod api;
pub mod forms;
pub mod models;
pub mod reviews;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum TicketSchemaFlowValue {
    Form(FormSchema),
    Review(TicketSchemaReview),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TicketSchemaFlowItem {
    #[serde(flatten)]
    schema: TicketSchemaFlow,
    module: TicketSchemaFlowValue,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum TicketFlowValue {
    None,
    Form(TicketFormAnswer),
    Review(TicketReview),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum TicketFlowOperator {
    User(User),
    Role(Role),
    None,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TicketFlowItem {
    #[serde(flatten)]
    flow: TicketFlow,
    module: TicketFlowValue,
    operator: TicketFlowOperator,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TicketFlowStatus {
    schema: TicketSchemaFlowItem,
    flow: Option<TicketFlowItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TicketStatus {
    InProgress,
    Pending,
    Finished,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TicketWithStatus {
    #[serde(flatten)]
    pub ticket: Ticket,
    pub status: TicketStatus,
}

pub async fn get_enabled_features_by_user(conn: &mut DbConn, user: &User) -> Vec<EnabledFeature> {
    let mut features = vec![];

    let tickets = Ticket::get_tickets_by_user(conn, user)
        .await
        .unwrap_or(vec![]);

    let pending_tickets = tickets
        .iter()
        .filter(|t| match t.status {
            TicketStatus::Pending => true,
            _ => false,
        })
        .collect::<Vec<_>>();

    let in_progress_tickets = tickets
        .iter()
        .filter(|t| match t.status {
            TicketStatus::InProgress => true,
            _ => false,
        })
        .collect::<Vec<_>>();

    if tickets.is_empty() {
        features.push(EnabledFeature::Ticket(0, 0));
    } else {
        features.push(EnabledFeature::Ticket(
            pending_tickets.len(),
            in_progress_tickets.len(),
        ));
    }

    let manager_tickets = TicketSchema::get_manager_schemas(conn, user)
        .await
        .unwrap_or(vec![]);

    if !manager_tickets.is_empty() {
        features.push(EnabledFeature::TicketManage(0, 0));
    }

    features
}
