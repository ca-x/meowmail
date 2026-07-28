export type ConnectionSecurity = "tls" | "starttls"
export type ProxyKind = "direct" | "http" | "socks5"
export type AiProviderKind = "openai" | "claude" | "gemini"
export type AiApiType = "chat" | "responses" | "messages" | "generateContent"

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
  signatureId?: string | null
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
  imap: Omit<ServerConfig, "port"> & { port: number | null }
  smtp: Omit<ServerConfig, "port"> & { port: number | null }
  proxy: {
    kind: ProxyKind
    host?: string
    port?: number | null
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
  threadKey: string
  preview: string
  receivedAt: number
  isRead: boolean
  isStarred: boolean
  attachmentCount: number
  rawSize: number
  isPromotional: boolean
}

export interface MailAttachment {
  id: string
  filename: string
  contentType: string
  size: number
  available: boolean
}

export interface MessageDetail extends MessageSummary {
  messageId?: string | null
  replyToEmail?: string | null
  references: string[]
  recipients: string[]
  ccRecipients: string[]
  bodyText: string
  bodyHtml?: string | null
  attachments: MailAttachment[]
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
  aiEnabled: boolean
  updatedAt: number
}

export interface AuthConfig {
  localEnabled: boolean
  oidcEnabled: boolean
}

export interface MailSettings {
  keepLocalAfterServerDelete: boolean
  syncFetchLimit: number | null
}

export type ReadingMode = "list" | "preview"
export type ListDensity = "default" | "compact"
export type AfterAction = "nextMessage" | "messageList"
export type SubjectPrefixLanguage = "chinese" | "english"
export type ComposeFontFamily = "default" | "serif" | "monospace"

export interface MailPreferences {
  readingMode: ReadingMode
  listDensity: ListDensity
  conversationMode: boolean
  aggregatePromotions: boolean
  showSummary: boolean
  showMessageSize: boolean
  showAttachmentPreview: boolean
  afterAction: AfterAction
  plainTextReading: boolean
  attachOriginalOnReply: boolean
  subjectPrefixLanguage: SubjectPrefixLanguage
  emptySubjectFromBody: boolean
  composeFontFamily: ComposeFontFamily
  composeFontSize: number
  composeFontColor: string
  autoForwardEnabled: boolean
  autoForwardAddress?: string | null
  autoReplyEnabled: boolean
  autoReplySubject: string
  autoReplyText: string
  autoReplyStartAt?: number | null
  autoReplyEndAt?: number | null
  autoReplyAccountIds: string[]
  autoReplyContactsOnly: boolean
}

export interface SignatureInput {
  name: string
  bodyText: string
}

export interface Signature extends SignatureInput {
  id: string
  createdAt: number
  updatedAt: number
}

export interface ContactInput {
  displayName: string
  email: string
  notes?: string
}

export interface Contact {
  id: string
  displayName: string
  email: string
  notes: string
  searchAliases: string[]
  createdAt: number
  updatedAt: number
}

export type EmailDraftStatus = "draft" | "sending" | "ambiguous" | "sent"

export interface EmailDraft {
  id: string
  accountId: string
  replyToMessageId?: string | null
  to: string[]
  cc: string[]
  bcc: string[]
  subject: string
  textBody: string
  htmlBody?: string | null
  attachments: ComposeAttachmentInput[]
  signatureId?: string | null
  applySignature: boolean
  scheduledAt?: number | null
  lastError?: string | null
  status: EmailDraftStatus
  createdAt: number
  updatedAt: number
}

export interface ComposeAttachmentInput {
  filename: string
  contentType: string
  contentBase64: string
  size: number
}

export interface ComposeMessageInput {
  accountId: string
  to: string[]
  cc: string[]
  bcc: string[]
  subject: string
  textBody: string
  htmlBody?: string | null
  attachments?: ComposeAttachmentInput[]
  signatureId?: string | null
  applySignature?: boolean
}

export interface DraftInput extends ComposeMessageInput {
  scheduledAt?: number | null
}

export interface AiProvider {
  id: string
  name: string
  providerKind: AiProviderKind
  apiType: AiApiType
  model: string
  baseUrl?: string | null
  proxy: PublicProxyConfig
  isDefault: boolean
  enabled: boolean
  hasApiKey: boolean
  createdAt: number
  updatedAt: number
}

export interface AiProviderInput {
  name: string
  providerKind: AiProviderKind
  apiType: AiApiType
  model: string
  baseUrl?: string | null
  apiKey?: string | null
  proxy: {
    kind: ProxyKind
    host?: string | null
    port?: number | null
    username?: string | null
    password?: string | null
  }
  isDefault: boolean
  enabled: boolean
}

export interface AiTextResponse {
  text: string
}

export interface LabelInput {
  name: string
  color: string
  isAuto: boolean
}

export interface Label extends LabelInput {
  id: string
  createdAt: number
  updatedAt: number
}

export interface AutoLabelRuleInput {
  accountId?: string | null
  providerId?: string | null
  name: string
  labelIds: string[]
  instructions: string
  enabled: boolean
  applyAutomatically: boolean
}

export interface AutoLabelRule extends AutoLabelRuleInput {
  id: string
  sourceSubscriptionId?: string | null
  createdAt: number
  updatedAt: number
}

export interface AutoLabelSubscriptionInput {
  name: string
  url: string
  enabled: boolean
}

export interface AutoLabelSubscription extends AutoLabelSubscriptionInput {
  id: string
  lastSyncedAt?: number | null
  lastError?: string | null
  createdAt: number
  updatedAt: number
}

export interface AutoLabelSubscriptionSyncResult {
  subscription: AutoLabelSubscription
  labelsImported: number
  rulesImported: number
  rulesSkipped: number
}

export interface AutoLabelRuleFeed {
  format: string
  version: number
  labels: Array<{ name: string; color: string; isAuto: boolean }>
  rules: Array<{
    accountEmail?: string | null
    providerName?: string | null
    name: string
    labelNames: string[]
    instructions: string
    enabled: boolean
    applyAutomatically: boolean
  }>
}

export interface AutoLabelResult {
  messageId: string
  labels: Label[]
}

export interface CalendarAccountInput {
  name: string
  baseUrl: string
  username: string
  password?: string | null
  enabled: boolean
}

export interface CalendarAccount {
  id: string
  name: string
  baseUrl: string
  username: string
  enabled: boolean
  hasPassword: boolean
  lastSyncedAt?: number | null
  lastError?: string | null
  createdAt: number
  updatedAt: number
}

export interface Calendar {
  id: string
  accountId: string
  displayName: string
  color: string
  remoteHref: string
  syncToken?: string | null
  enabled: boolean
  createdAt: number
  updatedAt: number
}

export interface CalendarUpdate {
  displayName: string
  color: string
  enabled: boolean
}

export interface CalendarEvent {
  id: string
  calendarId: string
  uid: string
  summary: string
  description: string
  location: string
  startsAt: number
  endsAt: number
  allDay: boolean
  timezone?: string | null
  createdAt: number
  updatedAt: number
}

export type CalendarFeature =
  | "lunarDate"
  | "weekday"
  | "leapYear"
  | "solarFestival"
  | "solarOtherFestival"
  | "holidayAdjustment"
  | "constellation"
  | "julianDay"
  | "ganZhiYear"
  | "ganZhiMonth"
  | "ganZhiDay"
  | "zodiacYear"
  | "zodiacMonth"
  | "zodiacDay"
  | "season"
  | "solarTerm"
  | "nearSolarTerms"
  | "lunarFestival"
  | "lunarOtherFestival"
  | "moonPhase"
  | "pengZu"
  | "dayYi"
  | "dayJi"
  | "auspiciousGods"
  | "inauspiciousSpirits"
  | "joyPosition"
  | "yangNoblePosition"
  | "yinNoblePosition"
  | "fortunePosition"
  | "wealthPosition"
  | "yearTaiSui"
  | "monthTaiSui"
  | "dayTaiSui"
  | "dayFetalGod"
  | "monthFetalGod"
  | "chong"
  | "sha"
  | "yearNaYin"
  | "monthNaYin"
  | "dayNaYin"
  | "xiu"
  | "twelveOfficer"
  | "dayGod"
  | "yearNineStar"
  | "monthNineStar"
  | "dayNineStar"
  | "xun"
  | "xunKong"
  | "shuJiu"
  | "sanFu"
  | "liuYao"
  | "wuHou"
  | "hou"
  | "dayLu"
  | "buddhistCalendar"
  | "buddhistFestivals"
  | "buddhistObservances"
  | "buddhistXiu"
  | "taoistCalendar"
  | "taoistFestivals"
  | "taoistObservances"

export interface CalendarPreferences {
  enabledFeatures: CalendarFeature[]
}

export interface CalendarDayDetail {
  feature: CalendarFeature
  values: string[]
  shortValue?: string | null
}

export interface CalendarDayInfo {
  date: string
  details: CalendarDayDetail[]
}

export interface ConnectionTestResponse {
  imap: boolean
  smtp: boolean
  imapError?: string | null
  smtpError?: string | null
}

export interface McpSettings {
  hasToken: boolean
  allowDelete: boolean
  createdAt?: number | null
  lastUsedAt?: number | null
  endpoint: string
}

export interface GeneratedMcpToken extends McpSettings {
  token: string
}

export type RuleMatchMode = "all" | "any"
export type RuleField = "sender" | "senderDomain" | "recipient" | "cc" | "recipientOrCc"
  | "subject" | "body" | "attachmentName" | "messageSize" | "receivedAt" | "ageDays" | "hasAttachment"
export type RuleOperator = "containsAny" | "containsAll" | "equals" | "notContains"
  | "greaterThan" | "lessThan" | "before" | "after" | "isTrue" | "isFalse"
export type RuleActionKind = "deleteLocal" | "deleteServer" | "markRead" | "markUnread"
  | "star" | "unstar" | "forward" | "autoReply"

export interface RuleCondition {
  field: RuleField
  operator: RuleOperator
  values: string[]
}

export interface RuleAction {
  kind: RuleActionKind
  value?: string | null
}

export interface CleanupRuleInput {
  accountId?: string | null
  name: string
  matchMode: RuleMatchMode
  conditions: RuleCondition[]
  actions: RuleAction[]
  position?: number | null
  stopProcessing: boolean
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
  preferences: boolean
  ai: boolean
  calendar: boolean
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
  signaturesImported: number
  preferencesImported: number
  aiProvidersImported: number
  labelsImported: number
  autoLabelRulesImported: number
  autoLabelSubscriptionsImported: number
  calendarAccountsImported: number
  conflicts: string[]
}

export interface SessionResponse {
  authenticated: boolean
  locked: boolean
  csrfToken: string
  version: string
  user: PublicUser
}
