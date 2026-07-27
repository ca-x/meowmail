mod api;
mod drafts;
mod model;
mod protocol;
mod rate_limit;
mod repository;
mod tools;

pub use api::routes;
pub use drafts::{DraftRepository, EmailDraft, EmailDraftStatus, StoredDraft};
pub use model::{GeneratedMcpToken, McpAccess, McpSettings};
pub use rate_limit::McpRateLimiter;
pub use repository::McpRepository;

use axum::{Router, middleware, routing::post};
use tower_http::limit::RequestBodyLimitLayer;

use crate::AppState;

pub fn protocol_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/mcp", post(protocol::handle))
        .route_layer(middleware::from_fn_with_state(
            state,
            protocol::authenticate,
        ))
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
}
