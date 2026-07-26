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

export interface SessionResponse {
  authenticated: boolean
  csrfToken: string
  version: string
}
