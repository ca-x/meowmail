mod api;
mod outgoing;
mod repository;

pub use api::routes;
pub use outgoing::{AutomaticMessageKind, ComposeInput, ThreadingHeaders, send_outgoing};
pub use repository::{
    AttachmentContent, MessageAttachment, MessageDetail, MessageFilter, MessageInsertResult,
    MessageRepository, MessageSummary, NewMessage,
};
