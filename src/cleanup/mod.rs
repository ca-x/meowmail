mod api;
mod model;
mod repository;

pub use api::routes;
pub use model::{
    CleanupRule, CleanupRuleInput, MailSettings, RuleAction, RuleActionKind, RuleCondition,
    RuleField, RuleMatchMode, RuleOperator,
};
pub use repository::{CachedRuleOutcome, CleanupRepository, RuleOutcome};
