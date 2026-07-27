use std::collections::HashMap;

use argon2::password_hash::PasswordHash;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    accounts::{AccountIdentityInput, AccountInput, AccountRepository, ProxyInput},
    cleanup::{
        CleanupRepository, CleanupRuleInput, MailSettings, RuleAction, RuleCondition, RuleMatchMode,
    },
    db::{
        Database,
        entities::{
            cleanup_rule, mail_account, mail_setting, notification_setting, user, user_identity,
        },
    },
    error::AppError,
    notifications::NotificationSettings,
    preferences::{MailPreferences, PreferencesRepository, SignatureInput},
    security::{CredentialVault, decrypt_archive, encrypt_archive},
};

use super::{PublicUser, Role, UserRepository};

const ARCHIVE_FORMAT: &str = "meowmail-migration";
const ARCHIVE_VERSION: u32 = 1;
const MAX_ARCHIVE_ENCODED_SIZE: usize = 14 * 1024 * 1024;
const MAX_ARCHIVE_PLAINTEXT_SIZE: usize = 10 * 1024 * 1024;
const MAX_ARCHIVE_USERS: usize = 500;
const MAX_ACCOUNTS_PER_USER: usize = 200;
const MAX_RULES_PER_USER: usize = 2_000;
const MAX_SIGNATURES_PER_USER: usize = 200;
const MAX_IDENTITIES_PER_USER: usize = 32;
const MAX_AVATAR_SIZE: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MigrationScope {
    Mine,
    AllUsers,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationSections {
    pub profile: bool,
    pub mail_accounts: bool,
    pub notifications: bool,
    pub cleanup: bool,
    #[serde(default)]
    pub preferences: bool,
}

impl MigrationSections {
    fn any(&self) -> bool {
        self.profile || self.mail_accounts || self.notifications || self.cleanup || self.preferences
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub passphrase: String,
    pub scope: MigrationScope,
    pub sections: MigrationSections,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationArchive {
    pub format: String,
    pub version: u32,
    pub scope: MigrationScope,
    pub sections: MigrationSections,
    pub encrypted_data: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub passphrase: String,
    pub sections: MigrationSections,
    pub archive: MigrationArchive,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub users_imported: u32,
    pub accounts_imported: u32,
    pub rules_imported: u32,
    pub signatures_imported: u32,
    pub preferences_imported: u32,
    pub conflicts: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct ArchivePayload {
    format: String,
    version: u32,
    scope: MigrationScope,
    sections: MigrationSections,
    users: Vec<ArchiveUser>,
}

#[derive(Serialize, Deserialize)]
struct ArchiveUser {
    #[serde(default)]
    auth: Option<ArchiveAuth>,
    #[serde(default)]
    profile: Option<ArchiveProfile>,
    #[serde(default)]
    mail_accounts: Option<Vec<AccountInput>>,
    #[serde(default)]
    notifications: Option<NotificationSettings>,
    #[serde(default)]
    mail_settings: Option<MailSettings>,
    #[serde(default)]
    cleanup_rules: Option<Vec<ArchiveCleanupRule>>,
    #[serde(default)]
    preferences: Option<ArchivePreferences>,
}

#[derive(Serialize, Deserialize)]
struct ArchiveAuth {
    username: String,
    email: Option<String>,
    role: String,
    password_hash: Option<String>,
    pin_hash: Option<String>,
    identities: Vec<ArchiveIdentity>,
}

#[derive(Serialize, Deserialize)]
struct ArchiveIdentity {
    issuer: String,
    subject: String,
}

#[derive(Serialize, Deserialize)]
struct ArchiveProfile {
    nickname: String,
    avatar_mime: Option<String>,
    avatar_data: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ArchiveCleanupRule {
    account_email: Option<String>,
    name: String,
    sender_contains: Option<String>,
    subject_contains: Option<String>,
    body_contains: Option<String>,
    older_than_days: Option<u32>,
    delete_from_server: bool,
    enabled: bool,
    #[serde(default)]
    match_mode: RuleMatchMode,
    #[serde(default)]
    conditions: Vec<RuleCondition>,
    #[serde(default)]
    actions: Vec<RuleAction>,
    #[serde(default)]
    position: i32,
    #[serde(default)]
    stop_processing: bool,
}

#[derive(Serialize, Deserialize)]
struct ArchivePreferences {
    mail: MailPreferences,
    signatures: Vec<ArchiveSignature>,
    account_identities: Vec<ArchiveAccountIdentity>,
}

#[derive(Serialize, Deserialize)]
struct ArchiveSignature {
    name: String,
    body_text: String,
}

#[derive(Serialize, Deserialize)]
struct ArchiveAccountIdentity {
    account_email: String,
    display_name: String,
    signature_name: Option<String>,
    is_default: bool,
}

#[derive(Clone)]
pub struct MigrationService {
    db: Database,
    vault: CredentialVault,
}

impl MigrationService {
    pub fn new(db: Database, vault: CredentialVault) -> Self {
        Self { db, vault }
    }

    pub async fn export(
        &self,
        current: &PublicUser,
        request: ExportRequest,
    ) -> Result<MigrationArchive, AppError> {
        validate_passphrase(&request.passphrase)?;
        validate_sections(&request.sections)?;
        if request.scope == MigrationScope::AllUsers && current.role != Role::Admin {
            return Err(AppError::Forbidden);
        }
        let models = if request.scope == MigrationScope::AllUsers {
            user::Entity::find()
                .order_by_asc(user::Column::CreatedAt)
                .all(self.db.connection())
                .await?
        } else {
            vec![
                UserRepository::new(self.db.clone())
                    .get_model(current.id)
                    .await?,
            ]
        };
        let mut users = Vec::with_capacity(models.len());
        for model in models {
            users.push(
                self.export_user(
                    &model,
                    &request.sections,
                    request.scope == MigrationScope::AllUsers,
                )
                .await?,
            );
        }
        let plaintext = serde_json::to_vec(&ArchivePayload {
            format: ARCHIVE_FORMAT.into(),
            version: ARCHIVE_VERSION,
            scope: request.scope,
            sections: request.sections.clone(),
            users,
        })
        .map_err(AppError::internal)?;
        if plaintext.len() > MAX_ARCHIVE_PLAINTEXT_SIZE {
            return Err(AppError::Validation(
                "selected migration data is too large to export".into(),
            ));
        }
        Ok(MigrationArchive {
            format: ARCHIVE_FORMAT.into(),
            version: ARCHIVE_VERSION,
            scope: request.scope,
            sections: request.sections,
            encrypted_data: encrypt_archive(&request.passphrase, &plaintext)
                .map_err(AppError::internal)?,
        })
    }

    pub async fn import(
        &self,
        current: &PublicUser,
        request: ImportRequest,
    ) -> Result<ImportReport, AppError> {
        validate_passphrase(&request.passphrase)?;
        validate_sections(&request.sections)?;
        if request.archive.format != ARCHIVE_FORMAT || request.archive.version != ARCHIVE_VERSION {
            return Err(AppError::Validation(
                "migration archive format is unsupported".into(),
            ));
        }
        if request.archive.scope == MigrationScope::AllUsers && current.role != Role::Admin {
            return Err(AppError::Forbidden);
        }
        validate_section_subset(&request.sections, &request.archive.sections)?;
        if request.archive.encrypted_data.len() > MAX_ARCHIVE_ENCODED_SIZE {
            return Err(AppError::Validation(
                "migration archive is too large".into(),
            ));
        }
        let plaintext = decrypt_archive(&request.passphrase, &request.archive.encrypted_data)
            .map_err(|_| AppError::Validation("migration archive could not be decrypted".into()))?;
        if plaintext.len() > MAX_ARCHIVE_PLAINTEXT_SIZE {
            return Err(AppError::Validation(
                "migration archive is too large".into(),
            ));
        }
        let payload: ArchivePayload = serde_json::from_slice(&plaintext)
            .map_err(|_| AppError::Validation("migration archive contents are invalid".into()))?;
        if payload.format != request.archive.format
            || payload.version != request.archive.version
            || payload.scope != request.archive.scope
            || payload.sections != request.archive.sections
        {
            return Err(AppError::Validation(
                "migration archive metadata does not match its encrypted contents".into(),
            ));
        }
        if payload.users.is_empty() || payload.users.len() > MAX_ARCHIVE_USERS {
            return Err(AppError::Validation(
                "migration archive user count is invalid".into(),
            ));
        }
        for archived in &payload.users {
            validate_archive_user(archived, payload.scope, &payload.sections)?;
        }
        let mut report = ImportReport {
            users_imported: 0,
            accounts_imported: 0,
            rules_imported: 0,
            signatures_imported: 0,
            preferences_imported: 0,
            conflicts: Vec::new(),
        };
        if request.archive.scope == MigrationScope::Mine {
            if payload.users.len() != 1 {
                return Err(AppError::Validation(
                    "personal migration archive must contain one user".into(),
                ));
            }
            self.import_user(
                current.id,
                payload.users.into_iter().next().expect("checked one user"),
                &request.sections,
                false,
                None,
                &mut report,
            )
            .await?;
            report.users_imported = 1;
            return Ok(report);
        }

        for archived in payload.users {
            let Some(auth) = archived.auth.as_ref() else {
                report
                    .conflicts
                    .push("user without authentication metadata was skipped".into());
                continue;
            };
            match self.resolve_all_user(auth).await? {
                Ok(user_id) => {
                    self.import_user(
                        user_id,
                        archived,
                        &request.sections,
                        true,
                        Some(current.id),
                        &mut report,
                    )
                    .await?;
                    report.users_imported += 1;
                }
                Err(conflict) => report.conflicts.push(conflict),
            }
        }
        Ok(report)
    }

    async fn export_user(
        &self,
        model: &user::Model,
        sections: &MigrationSections,
        include_auth: bool,
    ) -> Result<ArchiveUser, AppError> {
        let user_id = Uuid::parse_str(&model.id).map_err(AppError::internal)?;
        let auth = if include_auth {
            Some(ArchiveAuth {
                username: model.username.clone(),
                email: model.email.clone(),
                role: model.role.clone(),
                password_hash: model.password_hash.clone(),
                pin_hash: model.pin_hash.clone(),
                identities: user_identity::Entity::find()
                    .filter(user_identity::Column::UserId.eq(&model.id))
                    .all(self.db.connection())
                    .await?
                    .into_iter()
                    .map(|identity| ArchiveIdentity {
                        issuer: identity.issuer,
                        subject: identity.subject,
                    })
                    .collect(),
            })
        } else {
            None
        };
        let profile = sections.profile.then(|| ArchiveProfile {
            nickname: model.nickname.clone(),
            avatar_mime: model.avatar_mime.clone(),
            avatar_data: model.avatar_data.as_ref().map(|data| STANDARD.encode(data)),
        });
        let account_models = mail_account::Entity::find()
            .filter(mail_account::Column::UserId.eq(&model.id))
            .order_by_asc(mail_account::Column::CreatedAt)
            .all(self.db.connection())
            .await?;
        let mut account_emails = HashMap::new();
        let mail_accounts = if sections.mail_accounts {
            let mut inputs = Vec::with_capacity(account_models.len());
            for account in &account_models {
                account_emails.insert(account.id.clone(), account.email.clone());
                inputs.push(AccountInput {
                    display_name: account.display_name.clone(),
                    email: account.email.clone(),
                    username: account.username.clone(),
                    password: Some(
                        self.vault
                            .open(&account.password_cipher)
                            .map_err(AppError::internal)?
                            .expose_secret()
                            .to_owned(),
                    ),
                    imap: crate::accounts::ServerConfig {
                        host: account.imap_host.clone(),
                        port: account.imap_port as u16,
                        security: crate::accounts::ConnectionSecurity::parse(
                            &account.imap_security,
                        )?,
                    },
                    smtp: crate::accounts::ServerConfig {
                        host: account.smtp_host.clone(),
                        port: account.smtp_port as u16,
                        security: crate::accounts::ConnectionSecurity::parse(
                            &account.smtp_security,
                        )?,
                    },
                    proxy: ProxyInput {
                        kind: crate::accounts::ProxyKind::parse(&account.proxy_kind)?,
                        host: account.proxy_host.clone(),
                        port: account.proxy_port.map(|value| value as u16),
                        username: account.proxy_username.clone(),
                        password: account
                            .proxy_password_cipher
                            .as_deref()
                            .map(|value| {
                                self.vault
                                    .open(value)
                                    .map(|secret| secret.expose_secret().to_owned())
                                    .map_err(AppError::internal)
                            })
                            .transpose()?,
                    },
                    is_default: account.is_default,
                });
            }
            Some(inputs)
        } else {
            for account in &account_models {
                account_emails.insert(account.id.clone(), account.email.clone());
            }
            None
        };
        let notifications = if sections.notifications {
            notification_setting::Entity::find_by_id(model.id.clone())
                .one(self.db.connection())
                .await?
                .map(|settings| NotificationSettings {
                    enabled: settings.enabled,
                    message_template: settings.message_template,
                    command_template: settings.command_template,
                    http_url: settings.http_url,
                })
        } else {
            None
        };
        let (mail_settings, cleanup_rules) = if sections.cleanup {
            let settings = CleanupRepository::new(self.db.clone())
                .settings(user_id)
                .await?;
            let rules = CleanupRepository::new(self.db.clone())
                .list(user_id)
                .await?
                .into_iter()
                .map(|rule| ArchiveCleanupRule {
                    account_email: rule
                        .account_id
                        .and_then(|id| account_emails.get(&id.to_string()).cloned()),
                    name: rule.name,
                    sender_contains: rule.sender_contains,
                    subject_contains: rule.subject_contains,
                    body_contains: rule.body_contains,
                    older_than_days: rule.older_than_days,
                    delete_from_server: rule.delete_from_server,
                    enabled: rule.enabled,
                    match_mode: rule.match_mode,
                    conditions: rule.conditions,
                    actions: rule.actions,
                    position: rule.position,
                    stop_processing: rule.stop_processing,
                })
                .collect();
            (Some(settings), Some(rules))
        } else {
            (None, None)
        };
        let preferences = if sections.preferences {
            let repository = PreferencesRepository::new(self.db.clone());
            let signatures = repository.list_signatures(user_id).await?;
            let signature_names = signatures
                .iter()
                .map(|signature| (signature.id.to_string(), signature.name.clone()))
                .collect::<HashMap<_, _>>();
            Some(ArchivePreferences {
                mail: repository.mail(user_id).await?,
                signatures: signatures
                    .into_iter()
                    .map(|signature| ArchiveSignature {
                        name: signature.name,
                        body_text: signature.body_text,
                    })
                    .collect(),
                account_identities: account_models
                    .iter()
                    .map(|account| ArchiveAccountIdentity {
                        account_email: account.email.clone(),
                        display_name: account.display_name.clone(),
                        signature_name: account
                            .signature_id
                            .as_ref()
                            .and_then(|id| signature_names.get(id).cloned()),
                        is_default: account.is_default,
                    })
                    .collect(),
            })
        } else {
            None
        };
        Ok(ArchiveUser {
            auth,
            profile,
            mail_accounts,
            notifications,
            mail_settings,
            cleanup_rules,
            preferences,
        })
    }

    async fn resolve_all_user(&self, auth: &ArchiveAuth) -> Result<Result<Uuid, String>, AppError> {
        validate_archived_auth(auth)?;
        let by_username = user::Entity::find()
            .filter(user::Column::Username.eq(auth.username.to_ascii_lowercase()))
            .one(self.db.connection())
            .await?;
        let mut identity_owner = None;
        for identity in &auth.identities {
            if let Some(existing) = user_identity::Entity::find()
                .filter(user_identity::Column::Issuer.eq(&identity.issuer))
                .filter(user_identity::Column::Subject.eq(&identity.subject))
                .one(self.db.connection())
                .await?
            {
                if identity_owner
                    .as_ref()
                    .is_some_and(|owner: &String| owner != &existing.user_id)
                {
                    return Ok(Err(format!(
                        "OIDC identities for {} belong to different users",
                        auth.username
                    )));
                }
                identity_owner = Some(existing.user_id);
            }
        }
        if let (Some(username), Some(identity)) = (&by_username, &identity_owner)
            && username.id != *identity
        {
            return Ok(Err(format!(
                "username and OIDC identity conflict for {}",
                auth.username
            )));
        }
        if let Some(existing) = by_username {
            return Ok(Ok(
                Uuid::parse_str(&existing.id).map_err(AppError::internal)?
            ));
        }
        if let Some(identity) = identity_owner {
            return Ok(Ok(Uuid::parse_str(&identity).map_err(AppError::internal)?));
        }
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        user::ActiveModel {
            id: Set(id.to_string()),
            username: Set(auth.username.to_ascii_lowercase()),
            nickname: Set(auth.username.clone()),
            email: Set(auth.email.clone()),
            role: Set(auth.role.clone()),
            password_hash: Set(auth.password_hash.clone()),
            pin_hash: Set(auth.pin_hash.clone()),
            avatar_mime: Set(None),
            avatar_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            last_login_at: Set(None),
        }
        .insert(self.db.connection())
        .await?;
        notification_setting::ActiveModel {
            user_id: Set(id.to_string()),
            enabled: Set(false),
            message_template: Set("[{account}] {sender}: {subject}".into()),
            command_template: Set(None),
            http_url: Set(None),
            updated_at: Set(now),
        }
        .insert(self.db.connection())
        .await?;
        mail_setting::ActiveModel {
            user_id: Set(id.to_string()),
            keep_local_after_server_delete: Set(true),
            sync_fetch_limit: Set(Some(50)),
            updated_at: Set(now),
        }
        .insert(self.db.connection())
        .await?;
        Ok(Ok(id))
    }

    async fn import_user(
        &self,
        user_id: Uuid,
        archived: ArchiveUser,
        sections: &MigrationSections,
        include_auth: bool,
        protected_admin_id: Option<Uuid>,
        report: &mut ImportReport,
    ) -> Result<(), AppError> {
        if include_auth && let Some(auth) = archived.auth.as_ref() {
            validate_archived_auth(auth)?;
            let mut active = UserRepository::new(self.db.clone())
                .get_model(user_id)
                .await?
                .into_active_model();
            active.email = Set(auth.email.clone());
            active.role = Set(if protected_admin_id == Some(user_id) {
                Role::Admin.as_str().into()
            } else {
                auth.role.clone()
            });
            active.password_hash = Set(auth.password_hash.clone());
            active.pin_hash = Set(auth.pin_hash.clone());
            active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
            active.update(self.db.connection()).await?;
            for identity in &auth.identities {
                let exists = user_identity::Entity::find()
                    .filter(user_identity::Column::Issuer.eq(&identity.issuer))
                    .filter(user_identity::Column::Subject.eq(&identity.subject))
                    .one(self.db.connection())
                    .await?;
                if let Some(existing) = exists {
                    if existing.user_id != user_id.to_string() {
                        report
                            .conflicts
                            .push(format!("OIDC identity conflict for {}", auth.username));
                    }
                } else {
                    let now = OffsetDateTime::now_utc().unix_timestamp();
                    user_identity::ActiveModel {
                        id: Set(Uuid::new_v4().to_string()),
                        user_id: Set(user_id.to_string()),
                        issuer: Set(identity.issuer.clone()),
                        subject: Set(identity.subject.clone()),
                        created_at: Set(now),
                        last_login_at: Set(now),
                    }
                    .insert(self.db.connection())
                    .await?;
                }
            }
        }
        if sections.profile
            && let Some(profile) = archived.profile
        {
            let avatar = profile
                .avatar_data
                .map(|value| STANDARD.decode(value).map_err(AppError::internal))
                .transpose()?;
            if avatar.as_ref().is_some_and(|data| data.len() > 512 * 1024) {
                return Err(AppError::Validation("imported avatar is too large".into()));
            }
            let mut active = UserRepository::new(self.db.clone())
                .get_model(user_id)
                .await?
                .into_active_model();
            active.nickname = Set(super::repository::clean_nickname(&profile.nickname)?);
            active.avatar_mime = Set(profile.avatar_mime);
            active.avatar_data = Set(avatar);
            active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
            active.update(self.db.connection()).await?;
        }
        let accounts = AccountRepository::new(self.db.clone(), self.vault.clone());
        if sections.mail_accounts
            && let Some(imported) = archived.mail_accounts
        {
            for mut input in imported {
                input.validate(true)?;
                let existing = accounts
                    .list(user_id)
                    .await?
                    .into_iter()
                    .find(|account| account.email.eq_ignore_ascii_case(&input.email));
                if let Some(existing) = existing {
                    accounts.update(user_id, existing.id, input).await?;
                } else {
                    accounts.create(user_id, input).await?;
                }
                report.accounts_imported += 1;
            }
        }
        if sections.notifications
            && let Some(settings) = archived.notifications
        {
            crate::notifications::validate_settings(&settings)?;
            let model = notification_setting::Entity::find_by_id(user_id.to_string())
                .one(self.db.connection())
                .await?
                .ok_or_else(|| AppError::internal(anyhow::anyhow!("settings are missing")))?;
            let mut active = model.into_active_model();
            active.enabled = Set(settings.enabled);
            active.message_template = Set(settings.message_template);
            active.command_template = Set(settings.command_template);
            active.http_url = Set(settings.http_url);
            active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
            active.update(self.db.connection()).await?;
        }
        if sections.cleanup {
            if let Some(settings) = archived.mail_settings {
                CleanupRepository::new(self.db.clone())
                    .update_settings(user_id, settings)
                    .await?;
            }
            if let Some(rules) = archived.cleanup_rules {
                cleanup_rule::Entity::delete_many()
                    .filter(cleanup_rule::Column::UserId.eq(user_id.to_string()))
                    .exec(self.db.connection())
                    .await?;
                let account_map = accounts
                    .list(user_id)
                    .await?
                    .into_iter()
                    .map(|account| (account.email.to_ascii_lowercase(), account.id))
                    .collect::<HashMap<_, _>>();
                for rule in rules {
                    let account_id = rule
                        .account_email
                        .as_ref()
                        .and_then(|email| account_map.get(&email.to_ascii_lowercase()).copied());
                    if rule.account_email.is_some() && account_id.is_none() {
                        report.conflicts.push(format!(
                            "cleanup rule '{}' references a missing mail account",
                            rule.name
                        ));
                        continue;
                    }
                    CleanupRepository::new(self.db.clone())
                        .create(
                            user_id,
                            CleanupRuleInput {
                                account_id,
                                name: rule.name,
                                match_mode: rule.match_mode,
                                conditions: rule.conditions,
                                actions: rule.actions,
                                position: Some(rule.position),
                                stop_processing: rule.stop_processing,
                                sender_contains: rule.sender_contains,
                                subject_contains: rule.subject_contains,
                                body_contains: rule.body_contains,
                                older_than_days: rule.older_than_days,
                                delete_from_server: rule.delete_from_server,
                                enabled: rule.enabled,
                            },
                        )
                        .await?;
                    report.rules_imported += 1;
                }
            }
        }
        if sections.preferences
            && let Some(preferences) = archived.preferences
        {
            let repository = PreferencesRepository::new(self.db.clone());
            repository.update_mail(user_id, preferences.mail).await?;
            let mut signature_map = HashMap::new();
            let existing_signatures = repository.list_signatures(user_id).await?;
            for archived_signature in preferences.signatures {
                let input = SignatureInput {
                    name: archived_signature.name.clone(),
                    body_text: archived_signature.body_text,
                };
                let signature = if let Some(existing) = existing_signatures
                    .iter()
                    .find(|item| item.name.eq_ignore_ascii_case(&archived_signature.name))
                {
                    repository
                        .update_signature(user_id, existing.id, input)
                        .await?
                } else {
                    repository.create_signature(user_id, input).await?
                };
                signature_map.insert(signature.name.to_ascii_lowercase(), signature.id);
                report.signatures_imported += 1;
            }
            let accounts = AccountRepository::new(self.db.clone(), self.vault.clone());
            let account_map = accounts
                .list(user_id)
                .await?
                .into_iter()
                .map(|account| (account.email.to_ascii_lowercase(), account))
                .collect::<HashMap<_, _>>();
            for identity in preferences.account_identities {
                let Some(account) = account_map.get(&identity.account_email.to_ascii_lowercase())
                else {
                    report.conflicts.push(format!(
                        "mail identity references a missing account: {}",
                        identity.account_email
                    ));
                    continue;
                };
                let signature_id = identity
                    .signature_name
                    .as_ref()
                    .and_then(|name| signature_map.get(&name.to_ascii_lowercase()).copied());
                if identity.signature_name.is_some() && signature_id.is_none() {
                    report.conflicts.push(format!(
                        "mail identity references a missing signature: {}",
                        identity.account_email
                    ));
                    continue;
                }
                accounts
                    .update_identity(
                        user_id,
                        account.id,
                        AccountIdentityInput {
                            display_name: identity.display_name,
                            signature_id,
                            is_default: identity.is_default,
                        },
                    )
                    .await?;
            }
            report.preferences_imported += 1;
        }
        Ok(())
    }
}

fn validate_passphrase(value: &str) -> Result<(), AppError> {
    if !(8..=1024).contains(&value.chars().count()) || value.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "migration passphrase must contain 8-1024 non-control characters".into(),
        ));
    }
    Ok(())
}

fn validate_sections(value: &MigrationSections) -> Result<(), AppError> {
    if value.any() {
        Ok(())
    } else {
        Err(AppError::Validation(
            "select at least one migration section".into(),
        ))
    }
}

fn validate_section_subset(
    requested: &MigrationSections,
    available: &MigrationSections,
) -> Result<(), AppError> {
    if (requested.profile && !available.profile)
        || (requested.mail_accounts && !available.mail_accounts)
        || (requested.notifications && !available.notifications)
        || (requested.cleanup && !available.cleanup)
        || (requested.preferences && !available.preferences)
    {
        return Err(AppError::Validation(
            "selected migration sections are not present in the archive".into(),
        ));
    }
    Ok(())
}

fn validate_archive_user(
    archived: &ArchiveUser,
    scope: MigrationScope,
    sections: &MigrationSections,
) -> Result<(), AppError> {
    match scope {
        MigrationScope::Mine if archived.auth.is_some() => {
            return Err(AppError::Validation(
                "personal migration archives cannot contain authentication metadata".into(),
            ));
        }
        MigrationScope::AllUsers => {
            let auth = archived.auth.as_ref().ok_or_else(|| {
                AppError::Validation(
                    "all-user migration archives require authentication metadata".into(),
                )
            })?;
            validate_archived_auth(auth)?;
        }
        MigrationScope::Mine => {}
    }
    if (!sections.profile && archived.profile.is_some())
        || (!sections.mail_accounts && archived.mail_accounts.is_some())
        || (!sections.notifications && archived.notifications.is_some())
        || (!sections.cleanup
            && (archived.mail_settings.is_some() || archived.cleanup_rules.is_some()))
        || (!sections.preferences && archived.preferences.is_some())
    {
        return Err(AppError::Validation(
            "migration archive contains data outside its declared sections".into(),
        ));
    }
    if let Some(profile) = archived.profile.as_ref() {
        super::repository::clean_nickname(&profile.nickname)?;
        validate_archived_avatar(profile)?;
    }
    if let Some(accounts) = archived.mail_accounts.as_ref() {
        if accounts.len() > MAX_ACCOUNTS_PER_USER {
            return Err(AppError::Validation(
                "migration archive contains too many mail accounts".into(),
            ));
        }
        for account in accounts {
            let mut account = account.clone();
            account.validate(true)?;
        }
    }
    if let Some(settings) = archived.notifications.as_ref() {
        crate::notifications::validate_settings(settings)?;
    }
    if let Some(settings) = archived.mail_settings.as_ref() {
        settings.validate()?;
    }
    if let Some(rules) = archived.cleanup_rules.as_ref() {
        if rules.len() > MAX_RULES_PER_USER {
            return Err(AppError::Validation(
                "migration archive contains too many cleanup rules".into(),
            ));
        }
        for rule in rules {
            if rule
                .account_email
                .as_ref()
                .is_some_and(|email| email.len() > 254 || email.chars().any(char::is_control))
            {
                return Err(AppError::Validation(
                    "imported cleanup account reference is invalid".into(),
                ));
            }
            let mut input = CleanupRuleInput {
                account_id: None,
                name: rule.name.clone(),
                match_mode: rule.match_mode,
                conditions: rule.conditions.clone(),
                actions: rule.actions.clone(),
                position: Some(rule.position),
                stop_processing: rule.stop_processing,
                sender_contains: rule.sender_contains.clone(),
                subject_contains: rule.subject_contains.clone(),
                body_contains: rule.body_contains.clone(),
                older_than_days: rule.older_than_days,
                delete_from_server: rule.delete_from_server,
                enabled: rule.enabled,
            };
            input.normalize()?;
        }
    }
    if let Some(preferences) = archived.preferences.as_ref() {
        let mut mail = preferences.mail.clone();
        mail.normalize()?;
        if preferences.signatures.len() > MAX_SIGNATURES_PER_USER
            || preferences.account_identities.len() > MAX_ACCOUNTS_PER_USER
        {
            return Err(AppError::Validation(
                "migration archive contains too many mail preferences".into(),
            ));
        }
        let mut names = std::collections::HashSet::new();
        for signature in &preferences.signatures {
            let mut input = SignatureInput {
                name: signature.name.clone(),
                body_text: signature.body_text.clone(),
            };
            input.normalize()?;
            if !names.insert(input.name.to_ascii_lowercase()) {
                return Err(AppError::Validation(
                    "migration archive contains duplicate signature names".into(),
                ));
            }
        }
        for identity in &preferences.account_identities {
            if identity.account_email.len() > 254
                || !identity.account_email.contains('@')
                || identity.account_email.chars().any(char::is_control)
                || identity.display_name.is_empty()
                || identity.display_name.chars().count() > 80
                || identity.display_name.chars().any(char::is_control)
                || identity.signature_name.as_ref().is_some_and(|name| {
                    name.is_empty()
                        || name.chars().count() > 120
                        || name.chars().any(char::is_control)
                })
            {
                return Err(AppError::Validation(
                    "imported mail identity is invalid".into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_archived_avatar(profile: &ArchiveProfile) -> Result<(), AppError> {
    match (&profile.avatar_mime, &profile.avatar_data) {
        (None, None) => Ok(()),
        (Some(mime), Some(encoded)) => {
            let data = STANDARD
                .decode(encoded)
                .map_err(|_| AppError::Validation("imported avatar data is invalid".into()))?;
            if data.is_empty() || data.len() > MAX_AVATAR_SIZE {
                return Err(AppError::Validation("imported avatar is too large".into()));
            }
            if detect_avatar_mime(&data) != Some(mime.as_str()) {
                return Err(AppError::Validation(
                    "imported avatar content type does not match its data".into(),
                ));
            }
            Ok(())
        }
        _ => Err(AppError::Validation(
            "imported avatar metadata is incomplete".into(),
        )),
    }
}

fn detect_avatar_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if data.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn validate_archived_auth(auth: &ArchiveAuth) -> Result<(), AppError> {
    if auth.username.is_empty()
        || auth.username.chars().count() > 128
        || auth.username.chars().any(char::is_control)
    {
        return Err(AppError::Validation("imported username is invalid".into()));
    }
    if !matches!(auth.role.as_str(), "admin" | "user") {
        return Err(AppError::Validation("imported role is invalid".into()));
    }
    if auth.email.as_ref().is_some_and(|email| {
        email.is_empty()
            || email.len() > 254
            || email.chars().any(char::is_control)
            || !email.contains('@')
    }) {
        return Err(AppError::Validation("imported email is invalid".into()));
    }
    if auth.identities.len() > MAX_IDENTITIES_PER_USER {
        return Err(AppError::Validation(
            "imported user has too many OIDC identities".into(),
        ));
    }
    for hash in [auth.password_hash.as_deref(), auth.pin_hash.as_deref()]
        .into_iter()
        .flatten()
    {
        if !hash.starts_with("$argon2id$") {
            return Err(AppError::Validation(
                "imported authentication hash must use Argon2id".into(),
            ));
        }
        PasswordHash::new(hash)
            .map_err(|_| AppError::Validation("imported authentication hash is invalid".into()))?;
    }
    for identity in &auth.identities {
        let issuer = url::Url::parse(&identity.issuer)
            .map_err(|_| AppError::Validation("imported OIDC issuer is invalid".into()))?;
        let http_loopback = issuer.scheme() == "http"
            && issuer
                .host_str()
                .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
        if identity.issuer.len() > 2048
            || identity.subject.is_empty()
            || identity.subject.len() > 1024
            || (issuer.scheme() != "https" && !http_loopback)
            || issuer.host_str().is_none()
            || !issuer.username().is_empty()
            || issuer.password().is_some()
        {
            return Err(AppError::Validation(
                "imported OIDC identity is invalid".into(),
            ));
        }
    }
    Ok(())
}
