mod api;
mod model;
mod repository;
mod service;

pub use api::routes;
pub use model::{
    AiApiType, AiProvider, AiProviderInput, AiProviderKind, AiTextRequest, AiTextResponse,
    AutoLabelResult, AutoLabelRule, AutoLabelRuleFeed, AutoLabelRuleInput, AutoLabelSubscription,
    AutoLabelSubscriptionInput, AutoLabelSubscriptionSyncResult, Label, LabelInput,
};
pub use repository::AiRepository;
pub use service::{AiService, AutoLabelSubscriptionService};
