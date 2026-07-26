export type ConnectionSecurity = "tls" | "starttls"
export type ProxyKind = "direct" | "http" | "socks5"

export interface ServerConfig {
  host: string
  port: number
  security: ConnectionSecurity
}

export interface PublicProxyConfig {
  kind: ProxyKind
  host?: string | null
  port?: number | null
  username?: string | null
  hasPassword: boolean
}

export interface MailAccount {
  id: string
  displayName: string
  email: string
  username: string
  imap: ServerConfig
  smtp: ServerConfig
  proxy: PublicProxyConfig
  isDefault: boolean
  lastSyncedAt?: number | null
  createdAt: number
  updatedAt: number
  hasPassword: boolean
}

export interface AccountInput {
  displayName: string
  email: string
  username: string
  password?: string
  imap: ServerConfig
  smtp: ServerConfig
  proxy: {
    kind: ProxyKind
    host?: string
    port?: number
    username?: string
    password?: string
  }
  isDefault: boolean
}

export interface MessageSummary {
  id: string
  accountId: string
  folder: string
  uid: number
  senderName?: string | null
  senderEmail: string
  subject: string
  preview: string
  receivedAt: number
  isRead: boolean
  isStarred: boolean
  attachmentCount: number
}

export interface MessageDetail extends MessageSummary {
  messageId?: string | null
  recipients: string[]
  bodyText: string
  bodyHtml?: string | null
}

export interface NotificationSettings {
  enabled: boolean
  messageTemplate: string
  commandTemplate?: string | null
  httpUrl?: string | null
}

export type UserRole = "admin" | "user"

export interface PublicUser {
  id: string
  username: string
  nickname: string
  email?: string | null
  role: UserRole
  hasPassword: boolean
  hasPin: boolean
  hasAvatar: boolean
  updatedAt: number
}

export interface AuthConfig {
  localEnabled: boolean
  oidcEnabled: boolean
}

export interface MailSettings {
  keepLocalAfterServerDelete: boolean
}

export interface CleanupRuleInput {
  accountId?: string | null
  name: string
  senderContains?: string | null
  subjectContains?: string | null
  bodyContains?: string | null
  olderThanDays?: number | null
  deleteFromServer: boolean
  enabled: boolean
}

export interface CleanupRule extends CleanupRuleInput {
  id: string
  createdAt: number
  updatedAt: number
}

export type MigrationScope = "mine" | "allUsers"

export interface MigrationSections {
  profile: boolean
  mailAccounts: boolean
  notifications: boolean
  cleanup: boolean
}

export interface MigrationArchive {
  format: string
  version: number
  scope: MigrationScope
  sections: MigrationSections
  encryptedData: string
}

export interface ImportReport {
  usersImported: number
  accountsImported: number
  rulesImported: number
  conflicts: string[]
}

export interface SessionResponse {
  authenticated: boolean
  locked: boolean
  csrfToken: string
  version: string
  user: PublicUser
}
