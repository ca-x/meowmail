use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use time::OffsetDateTime;
use uuid::Uuid;
use zeroize::Zeroize;

use crate::{
    db::{Database, entities::mcp_token},
    error::AppError,
};

use super::{GeneratedMcpToken, McpAccess, McpSettings};

const TOKEN_PREFIX: &str = "mmcp";

#[derive(Clone)]
pub struct McpRepository {
    db: Database,
}

impl McpRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn settings(&self, user_id: Uuid) -> Result<McpSettings, AppError> {
        let model = mcp_token::Entity::find_by_id(user_id.to_string())
            .one(self.db.connection())
            .await?;
        Ok(match model {
            Some(model) => McpSettings {
                has_token: true,
                allow_delete: model.allow_delete,
                created_at: Some(model.created_at),
                last_used_at: model.last_used_at,
                endpoint: "/mcp",
            },
            None => McpSettings {
                has_token: false,
                allow_delete: false,
                created_at: None,
                last_used_at: None,
                endpoint: "/mcp",
            },
        })
    }

    pub async fn generate(&self, user_id: Uuid) -> Result<GeneratedMcpToken, AppError> {
        let token_id = Uuid::new_v4();
        let mut secret = [0_u8; 32];
        rand::rng().fill_bytes(&mut secret);
        let mut encoded_secret = URL_SAFE_NO_PAD.encode(secret);
        secret.zeroize();
        let plaintext = format!("{TOKEN_PREFIX}_{}_{encoded_secret}", token_id.simple());
        let digest = Sha256::digest(encoded_secret.as_bytes()).to_vec();
        encoded_secret.zeroize();
        let now = OffsetDateTime::now_utc().unix_timestamp();

        if let Some(model) = mcp_token::Entity::find_by_id(user_id.to_string())
            .one(self.db.connection())
            .await?
        {
            let mut active = model.into_active_model();
            active.token_id = Set(token_id.to_string());
            active.token_digest = Set(digest);
            active.allow_delete = Set(false);
            active.created_at = Set(now);
            active.updated_at = Set(now);
            active.last_used_at = Set(None);
            active.update(self.db.connection()).await?;
        } else {
            mcp_token::ActiveModel {
                user_id: Set(user_id.to_string()),
                token_id: Set(token_id.to_string()),
                token_digest: Set(digest),
                allow_delete: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
                last_used_at: Set(None),
            }
            .insert(self.db.connection())
            .await?;
        }

        tracing::info!(%user_id, %token_id, "MCP token generated");
        Ok(GeneratedMcpToken {
            settings: McpSettings {
                has_token: true,
                allow_delete: false,
                created_at: Some(now),
                last_used_at: None,
                endpoint: "/mcp",
            },
            token: plaintext,
        })
    }

    pub async fn set_allow_delete(
        &self,
        user_id: Uuid,
        allow_delete: bool,
    ) -> Result<McpSettings, AppError> {
        let model = mcp_token::Entity::find_by_id(user_id.to_string())
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        let mut active = model.into_active_model();
        active.allow_delete = Set(allow_delete);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        active.update(self.db.connection()).await?;
        tracing::info!(%user_id, allow_delete, "MCP deletion permission updated");
        self.settings(user_id).await
    }

    pub async fn revoke(&self, user_id: Uuid) -> Result<(), AppError> {
        mcp_token::Entity::delete_by_id(user_id.to_string())
            .exec(self.db.connection())
            .await?;
        tracing::info!(%user_id, "MCP token revoked");
        Ok(())
    }

    pub async fn authenticate(&self, plaintext: &str) -> Result<McpAccess, AppError> {
        let (token_id, secret) = parse_token(plaintext)?;
        let model = mcp_token::Entity::find()
            .filter(mcp_token::Column::TokenId.eq(token_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::Unauthorized)?;
        let supplied = Sha256::digest(secret.as_bytes());
        if model.token_digest.len() != supplied.len()
            || model
                .token_digest
                .as_slice()
                .ct_eq(supplied.as_slice())
                .unwrap_u8()
                != 1
        {
            return Err(AppError::Unauthorized);
        }
        let user_id = Uuid::parse_str(&model.user_id).map_err(AppError::internal)?;
        let allow_delete = model.allow_delete;
        let now = OffsetDateTime::now_utc().unix_timestamp();
        if model
            .last_used_at
            .is_none_or(|last_used| now - last_used >= 60)
        {
            let mut active = model.into_active_model();
            active.last_used_at = Set(Some(now));
            active.update(self.db.connection()).await?;
        }
        Ok(McpAccess {
            user_id,
            token_id,
            allow_delete,
        })
    }

    pub async fn refresh(&self, access: &McpAccess) -> Result<McpAccess, AppError> {
        let model = mcp_token::Entity::find()
            .filter(mcp_token::Column::UserId.eq(access.user_id.to_string()))
            .filter(mcp_token::Column::TokenId.eq(access.token_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::Unauthorized)?;
        Ok(McpAccess {
            user_id: access.user_id,
            token_id: access.token_id,
            allow_delete: model.allow_delete,
        })
    }
}

fn parse_token(value: &str) -> Result<(Uuid, &str), AppError> {
    if value.len() > 160 {
        return Err(AppError::Unauthorized);
    }
    let mut parts = value.splitn(3, '_');
    let prefix = parts.next();
    let token_id = parts.next();
    let secret = parts.next();
    if prefix != Some(TOKEN_PREFIX) {
        return Err(AppError::Unauthorized);
    }
    let token_id = Uuid::parse_str(token_id.ok_or(AppError::Unauthorized)?)
        .map_err(|_| AppError::Unauthorized)?;
    let secret = secret.ok_or(AppError::Unauthorized)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(secret)
        .map_err(|_| AppError::Unauthorized)?;
    if decoded.len() != 32 {
        return Err(AppError::Unauthorized);
    }
    Ok((token_id, secret))
}
