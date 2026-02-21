use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::error::ProblemDetails;

/// Get template data (external API — not yet implemented).
///
/// This endpoint will be available for external integrations in Phase 11
/// with API Key authentication. Currently returns 501 Not Implemented.
///
/// # Errors
///
/// Always returns `ProblemDetails` with 501 status.
#[utoipa::path(
    get,
    path = "/external/v1/projects/{projectId}/task-templates/{templateId}/data",
    responses(
        (status = 501, body = ProblemDetails),
    ),
    params(
        ("projectId" = Uuid, Path,),
        ("templateId" = Uuid, Path,),
    ),
    tag = "external-data",
)]
pub async fn get_template_data(
    Path((_project_id, _template_id)): Path<(Uuid, Uuid)>,
) -> (StatusCode, Json<ProblemDetails>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(
            ProblemDetails::new(StatusCode::NOT_IMPLEMENTED, "Not Implemented")
                .with_detail("External data API is not yet available. Planned for Phase 11."),
        ),
    )
}
