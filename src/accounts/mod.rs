mod api;
mod model;
mod repository;

pub use api::routes;
pub use model::{
    AccountIdentityInput, AccountInput, AccountSecrets, ConnectionSecurity, MailAccount,
    ProxyConfig, ProxyInput, ProxyKind, PublicProxyConfig, ServerConfig,
};
pub use repository::AccountRepository;
