mod api;
mod model;
mod repository;

pub use api::routes;
pub use model::{CleanupRule, CleanupRuleInput, MailSettings};
pub use repository::CleanupRepository;
