mod api;
mod drafts_api;
mod outgoing;
mod repository;
mod scheduled;

pub use api::routes;
pub use drafts_api::routes as draft_routes;
pub use outgoing::{AutomaticMessageKind, ComposeInput, ThreadingHeaders, send_outgoing};
pub use repository::{
    AttachmentContent, MessageAttachment, MessageDetail, MessageFilter, MessageInsertResult,
    MessageRepository, MessageSummary, NewMessage,
};
pub use scheduled::spawn_runner as spawn_scheduled_draft_runner;
