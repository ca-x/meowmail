mod api;
mod model;
mod repository;

pub use api::routes;
pub use model::{
    AfterAction, ComposeFontFamily, ListDensity, MailPreferences, ReadingMode, Signature,
    SignatureInput, SubjectPrefixLanguage,
};
pub use repository::PreferencesRepository;
