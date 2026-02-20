use axum::http::StatusCode;
use uuid::Uuid;

use crate::api::error::ProblemDetails;
use crate::app_state::AppState;
use crate::modules::core::permission::service::{Action, Resource};

/// Check that the given account has permission to perform the action on the resource.
///
/// # Errors
///
/// Returns `ProblemDetails` with 403 if the user lacks permission.
pub async fn require_permission(
    state: &AppState,
    account_id: Uuid,
    resource: Resource,
    action: Action,
) -> Result<(), ProblemDetails> {
    let allowed = state
        .permission_service
        .check(account_id, &resource, action)
        .await
        .map_err(|_| ProblemDetails::internal_server_error())?;

    if !allowed {
        return Err(ProblemDetails::new(StatusCode::FORBIDDEN, "Forbidden")
            .with_detail("You do not have permission to perform this action"));
    }

    Ok(())
}
