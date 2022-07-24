use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use juniper::http::GraphQLBatchResponse;

/// A wrapper around [`GraphQLBatchResponse`] that implements [`IntoResponse`]
/// so it can be returned from axum handlers.
pub struct JuniperResponse(pub GraphQLBatchResponse);

impl IntoResponse for JuniperResponse {
    fn into_response(self) -> Response {
        if !self.0.is_ok() {
            return (StatusCode::BAD_REQUEST, Json(self.0)).into_response();
        }

        Json(self.0).into_response()
    }
}
