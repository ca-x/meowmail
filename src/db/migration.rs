use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(InitialMigration)]
    }
}

#[derive(DeriveMigrationName)]
struct InitialMigration;

#[async_trait::async_trait]
impl MigrationTrait for InitialMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MailAccount::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MailAccount::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(MailAccount::DisplayName)
                            .string_len(80)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MailAccount::Email)
                            .string_len(254)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(MailAccount::Username)
                            .string_len(320)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MailAccount::PasswordCipher)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MailAccount::ImapHost)
                            .string_len(253)
                            .not_null(),
                    )
                    .col(ColumnDef::new(MailAccount::ImapPort).integer().not_null())
                    .col(
                        ColumnDef::new(MailAccount::ImapSecurity)
                            .string_len(16)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MailAccount::SmtpHost)
                            .string_len(253)
                            .not_null(),
                    )
                    .col(ColumnDef::new(MailAccount::SmtpPort).integer().not_null())
                    .col(
                        ColumnDef::new(MailAccount::SmtpSecurity)
                            .string_len(16)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MailAccount::ProxyKind)
                            .string_len(16)
                            .not_null()
                            .default("direct"),
                    )
                    .col(ColumnDef::new(MailAccount::ProxyHost).string_len(253))
                    .col(ColumnDef::new(MailAccount::ProxyPort).integer())
                    .col(ColumnDef::new(MailAccount::ProxyUsername).string_len(255))
                    .col(ColumnDef::new(MailAccount::ProxyPasswordCipher).text())
                    .col(
                        ColumnDef::new(MailAccount::IsDefault)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(MailAccount::LastSyncedAt).big_integer())
                    .col(
                        ColumnDef::new(MailAccount::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MailAccount::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Message::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Message::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Message::AccountId).string().not_null())
                    .col(ColumnDef::new(Message::Folder).string_len(160).not_null())
                    .col(ColumnDef::new(Message::Uid).big_integer().not_null())
                    .col(ColumnDef::new(Message::InternetMessageId).string_len(998))
                    .col(ColumnDef::new(Message::SenderName).string_len(320))
                    .col(
                        ColumnDef::new(Message::SenderEmail)
                            .string_len(320)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Message::RecipientsJson).text().not_null())
                    .col(ColumnDef::new(Message::Subject).text().not_null())
                    .col(ColumnDef::new(Message::Preview).text().not_null())
                    .col(ColumnDef::new(Message::BodyText).text().not_null())
                    .col(ColumnDef::new(Message::BodyHtml).text())
                    .col(ColumnDef::new(Message::ReceivedAt).big_integer().not_null())
                    .col(
                        ColumnDef::new(Message::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Message::IsStarred)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Message::AttachmentCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Message::CreatedAt).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_messages_account")
                            .from(Message::Table, Message::AccountId)
                            .to(MailAccount::Table, MailAccount::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_messages_account_folder_uid")
                    .table(Message::Table)
                    .col(Message::AccountId)
                    .col(Message::Folder)
                    .col(Message::Uid)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_messages_list")
                    .table(Message::Table)
                    .col(Message::AccountId)
                    .col(Message::Folder)
                    .col(Message::ReceivedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Preference::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Preference::Key)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Preference::Value).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(NotificationSetting::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NotificationSetting::Singleton)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(NotificationSetting::Enabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(NotificationSetting::MessageTemplate)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(NotificationSetting::CommandTemplate).text())
                    .col(ColumnDef::new(NotificationSetting::HttpUrl).text())
                    .col(
                        ColumnDef::new(NotificationSetting::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT OR IGNORE INTO notification_settings(singleton, enabled, message_template, updated_at) VALUES(1, 0, '[{account}] {sender}: {subject}', unixepoch())",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NotificationSetting::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Preference::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Message::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MailAccount::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum MailAccount {
    #[sea_orm(iden = "mail_accounts")]
    Table,
    Id,
    DisplayName,
    Email,
    Username,
    PasswordCipher,
    ImapHost,
    ImapPort,
    ImapSecurity,
    SmtpHost,
    SmtpPort,
    SmtpSecurity,
    ProxyKind,
    ProxyHost,
    ProxyPort,
    ProxyUsername,
    ProxyPasswordCipher,
    IsDefault,
    LastSyncedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Message {
    #[sea_orm(iden = "messages")]
    Table,
    Id,
    AccountId,
    Folder,
    Uid,
    #[sea_orm(iden = "message_id")]
    InternetMessageId,
    SenderName,
    SenderEmail,
    RecipientsJson,
    Subject,
    Preview,
    BodyText,
    BodyHtml,
    ReceivedAt,
    IsRead,
    IsStarred,
    AttachmentCount,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Preference {
    #[sea_orm(iden = "preferences")]
    Table,
    Key,
    Value,
}

#[derive(DeriveIden)]
enum NotificationSetting {
    #[sea_orm(iden = "notification_settings")]
    Table,
    Singleton,
    Enabled,
    MessageTemplate,
    CommandTemplate,
    HttpUrl,
    UpdatedAt,
}
