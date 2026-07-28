import type {
  AccountInput,
  AiProvider,
  AiProviderInput,
  AiTextResponse,
  AutoLabelResult,
  AutoLabelRule,
  AutoLabelRuleFeed,
  AutoLabelRuleInput,
  AutoLabelSubscription,
  AutoLabelSubscriptionInput,
  AutoLabelSubscriptionSyncResult,
  AuthConfig,
  Calendar,
  CalendarAccount,
  CalendarAccountInput,
  CalendarDayInfo,
  CalendarEvent,
  CalendarPreferences,
  CalendarUpdate,
  CleanupRule,
  CleanupRuleInput,
  ComposeMessageInput,
  ConnectionTestResponse,
  Contact,
  ContactInput,
  DraftInput,
  EmailDraft,
  ImportReport,
  Label,
  LabelInput,
  LocalCalendarEvent,
  LocalCalendarEventInput,
  MailAccount,
  MailPreferences,
  MailSettings,
  McpSettings,
  GeneratedMcpToken,
  MessageDetail,
  MessageSummary,
  MigrationArchive,
  MigrationScope,
  MigrationSections,
  NotificationSettings,
  PublicUser,
  Signature,
  SignatureInput,
  SessionResponse,
} from "./types"

let csrfToken = ""

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
  ) {
    super(message)
  }
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const method = (init.method || "GET").toUpperCase()
  const headers = new Headers(init.headers)
  if (init.body && !headers.has("content-type")) headers.set("content-type", "application/json")
  if (!new Set(["GET", "HEAD", "OPTIONS"]).has(method) && csrfToken) {
    headers.set("x-csrf-token", csrfToken)
  }
  const response = await fetch(path, { ...init, headers, credentials: "same-origin" })
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as
      | { error?: { code?: string; message?: string } }
      | null
    throw new ApiError(
      response.status,
      body?.error?.code || "REQUEST_FAILED",
      body?.error?.message || `Request failed (${response.status})`,
    )
  }
  if (response.status === 204) return undefined as T
  return (await response.json()) as T
}

export const api = {
  authConfig: () => request<AuthConfig>("/api/v1/auth/config"),
  async session() {
    const session = await request<SessionResponse>("/api/v1/session")
    csrfToken = session.csrfToken
    return session
  },
  async login(username: string, password: string) {
    const session = await request<SessionResponse>("/api/v1/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    })
    csrfToken = session.csrfToken
    return session
  },
  async logout() {
    await request<void>("/api/v1/auth/logout", { method: "POST" })
    csrfToken = ""
  },
  lock: () => request<SessionResponse>("/api/v1/auth/lock", { method: "POST" }),
  unlock: (pin: string) =>
    request<SessionResponse>("/api/v1/auth/unlock", {
      method: "POST",
      body: JSON.stringify({ pin }),
    }),
  setPin: (pin: string) =>
    request<PublicUser>("/api/v1/auth/pin", {
      method: "PUT",
      body: JSON.stringify({ pin }),
    }),
  removePin: () => request<PublicUser>("/api/v1/auth/pin", { method: "DELETE" }),
  profile: () => request<PublicUser>("/api/v1/users/me"),
  updateProfile: (username: string | null, nickname: string) =>
    request<PublicUser>("/api/v1/users/me", {
      method: "PATCH",
      body: JSON.stringify({ username, nickname }),
    }),
  updatePassword: (currentPassword: string | null, newPassword: string) =>
    request<PublicUser>("/api/v1/users/me/password", {
      method: "PUT",
      body: JSON.stringify({ currentPassword, newPassword }),
    }),
  updateAiAccess: (enabled: boolean) =>
    request<PublicUser>("/api/v1/users/me/ai", {
      method: "PUT",
      body: JSON.stringify({ enabled }),
    }),
  updateAutoLock: (minutes: number | null) =>
    request<PublicUser>("/api/v1/users/me/auto-lock", {
      method: "PUT",
      body: JSON.stringify({ minutes }),
    }),
  updateAvatar: (file: File) =>
    request<PublicUser>("/api/v1/users/me/avatar", {
      method: "PUT",
      headers: { "content-type": file.type },
      body: file,
    }),
  removeAvatar: () => request<PublicUser>("/api/v1/users/me/avatar", { method: "DELETE" }),
  accounts: () => request<MailAccount[]>("/api/v1/accounts"),
  createAccount: (input: AccountInput) =>
    request<MailAccount>("/api/v1/accounts", { method: "POST", body: JSON.stringify(input) }),
  updateAccount: (id: string, input: AccountInput) =>
    request<MailAccount>(`/api/v1/accounts/${id}`, {
      method: "PATCH",
      body: JSON.stringify(input),
    }),
  updateAccountIdentity: (id: string, input: { displayName: string; signatureId: string | null; isDefault: boolean }) =>
    request<MailAccount>(`/api/v1/accounts/${id}/identity`, {
      method: "PATCH",
      body: JSON.stringify(input),
    }),
  deleteAccount: (id: string) => request<void>(`/api/v1/accounts/${id}`, { method: "DELETE" }),
  testAccount: (input: AccountInput) =>
    request<ConnectionTestResponse>("/api/v1/accounts/test", {
      method: "POST",
      body: JSON.stringify(input),
    }),
  testSavedAccount: (id: string) =>
    request<ConnectionTestResponse>(`/api/v1/accounts/${id}/test`, { method: "POST" }),
  syncAccount: (id: string) =>
    request<{ inserted: number; syncedAt: number }>(`/api/v1/accounts/${id}/sync`, {
      method: "POST",
    }),
  messages: (params: URLSearchParams) =>
    request<MessageSummary[]>(`/api/v1/messages?${params.toString()}`),
  message: (id: string) => request<MessageDetail>(`/api/v1/messages/${id}`),
  messageThread: (id: string) => request<MessageDetail[]>(`/api/v1/messages/${id}/thread`),
  attachmentUrl: (messageId: string, attachmentId: string, download = false) =>
    `/api/v1/messages/${encodeURIComponent(messageId)}/attachments/${encodeURIComponent(attachmentId)}${download ? "?download=true" : ""}`,
  updateMessage: (id: string, update: { isRead?: boolean; isStarred?: boolean }) =>
    request<MessageSummary>(`/api/v1/messages/${id}`, {
      method: "PATCH",
      body: JSON.stringify(update),
    }),
  deleteMessage: (id: string) => request<void>(`/api/v1/messages/${id}`, { method: "DELETE" }),
  sendMessage: (input: ComposeMessageInput) =>
    request<void>("/api/v1/messages/send", { method: "POST", body: JSON.stringify(input) }),
  contacts: (params = new URLSearchParams({ limit: "100" })) =>
    request<Contact[]>(`/api/v1/contacts?${params.toString()}`),
  createContact: (input: ContactInput) =>
    request<Contact>("/api/v1/contacts", { method: "POST", body: JSON.stringify(input) }),
  updateContact: (id: string, input: ContactInput) =>
    request<Contact>(`/api/v1/contacts/${id}`, {
      method: "PATCH",
      body: JSON.stringify(input),
    }),
  deleteContact: (id: string) => request<void>(`/api/v1/contacts/${id}`, { method: "DELETE" }),
  drafts: () => request<EmailDraft[]>("/api/v1/drafts"),
  createDraft: (input: DraftInput) =>
    request<EmailDraft>("/api/v1/drafts", { method: "POST", body: JSON.stringify(input) }),
  updateDraft: (id: string, input: DraftInput) =>
    request<EmailDraft>(`/api/v1/drafts/${id}`, {
      method: "PATCH",
      body: JSON.stringify(input),
    }),
  deleteDraft: (id: string) => request<void>(`/api/v1/drafts/${id}`, { method: "DELETE" }),
  sendDraft: (id: string) => request<void>(`/api/v1/drafts/${id}/send`, { method: "POST" }),
  aiProviders: () => request<AiProvider[]>("/api/v1/ai/providers"),
  createAiProvider: (input: AiProviderInput) =>
    request<AiProvider>("/api/v1/ai/providers", { method: "POST", body: JSON.stringify(input) }),
  updateAiProvider: (id: string, input: AiProviderInput) =>
    request<AiProvider>(`/api/v1/ai/providers/${id}`, { method: "PATCH", body: JSON.stringify(input) }),
  deleteAiProvider: (id: string) =>
    request<void>(`/api/v1/ai/providers/${id}`, { method: "DELETE" }),
  testAiProvider: (id: string) =>
    request<AiTextResponse>(`/api/v1/ai/providers/${id}/test`, { method: "POST" }),
  translateText: (input: { providerId?: string | null; text: string; targetLanguage?: string | null }) =>
    request<AiTextResponse>("/api/v1/ai/translate", { method: "POST", body: JSON.stringify(input) }),
  polishText: (input: { providerId?: string | null; text: string; tone?: string | null }) =>
    request<AiTextResponse>("/api/v1/ai/polish", { method: "POST", body: JSON.stringify(input) }),
  labels: () => request<Label[]>("/api/v1/labels"),
  createLabel: (input: LabelInput) =>
    request<Label>("/api/v1/labels", { method: "POST", body: JSON.stringify(input) }),
  updateLabel: (id: string, input: LabelInput) =>
    request<Label>(`/api/v1/labels/${id}`, { method: "PATCH", body: JSON.stringify(input) }),
  deleteLabel: (id: string) => request<void>(`/api/v1/labels/${id}`, { method: "DELETE" }),
  autoLabelRules: () => request<AutoLabelRule[]>("/api/v1/auto-label-rules"),
  exportAutoLabelRules: () => request<AutoLabelRuleFeed>("/api/v1/auto-label-rules/export"),
  createAutoLabelRule: (input: AutoLabelRuleInput) =>
    request<AutoLabelRule>("/api/v1/auto-label-rules", { method: "POST", body: JSON.stringify(input) }),
  updateAutoLabelRule: (id: string, input: AutoLabelRuleInput) =>
    request<AutoLabelRule>(`/api/v1/auto-label-rules/${id}`, { method: "PATCH", body: JSON.stringify(input) }),
  deleteAutoLabelRule: (id: string) =>
    request<void>(`/api/v1/auto-label-rules/${id}`, { method: "DELETE" }),
  autoLabelSubscriptions: () =>
    request<AutoLabelSubscription[]>("/api/v1/auto-label-subscriptions"),
  createAutoLabelSubscription: (input: AutoLabelSubscriptionInput) =>
    request<AutoLabelSubscription>("/api/v1/auto-label-subscriptions", {
      method: "POST",
      body: JSON.stringify(input),
    }),
  updateAutoLabelSubscription: (id: string, input: AutoLabelSubscriptionInput) =>
    request<AutoLabelSubscription>(`/api/v1/auto-label-subscriptions/${id}`, {
      method: "PATCH",
      body: JSON.stringify(input),
    }),
  deleteAutoLabelSubscription: (id: string) =>
    request<void>(`/api/v1/auto-label-subscriptions/${id}`, { method: "DELETE" }),
  syncAutoLabelSubscription: (id: string) =>
    request<AutoLabelSubscriptionSyncResult>(`/api/v1/auto-label-subscriptions/${id}/sync`, {
      method: "POST",
    }),
  autoLabelMessage: (id: string, ruleId?: string | null) =>
    request<AutoLabelResult>(`/api/v1/messages/${id}/auto-label`, {
      method: "POST",
      body: JSON.stringify({ ruleId }),
    }),
  calendarAccounts: () => request<CalendarAccount[]>("/api/v1/calendar/accounts"),
  createCalendarAccount: (input: CalendarAccountInput) =>
    request<CalendarAccount>("/api/v1/calendar/accounts", { method: "POST", body: JSON.stringify(input) }),
  updateCalendarAccount: (id: string, input: CalendarAccountInput) =>
    request<CalendarAccount>(`/api/v1/calendar/accounts/${id}`, { method: "PATCH", body: JSON.stringify(input) }),
  deleteCalendarAccount: (id: string) =>
    request<void>(`/api/v1/calendar/accounts/${id}`, { method: "DELETE" }),
  discoverCalendarAccount: (id: string) =>
    request<Calendar[]>(`/api/v1/calendar/accounts/${id}/discover`, { method: "POST" }),
  syncCalendarAccount: (id: string) =>
    request<{ imported: number }>(`/api/v1/calendar/accounts/${id}/sync`, { method: "POST" }),
  calendars: () => request<Calendar[]>("/api/v1/calendars"),
  updateCalendar: (id: string, input: CalendarUpdate) =>
    request<Calendar>(`/api/v1/calendars/${id}`, { method: "PATCH", body: JSON.stringify(input) }),
  calendarPreferences: () => request<CalendarPreferences>("/api/v1/calendar/preferences"),
  updateCalendarPreferences: (preferences: CalendarPreferences) =>
    request<CalendarPreferences>("/api/v1/calendar/preferences", {
      method: "PUT",
      body: JSON.stringify(preferences),
    }),
  calendarDayInfo: (params: URLSearchParams) =>
    request<CalendarDayInfo[]>(`/api/v1/calendar/day-info?${params.toString()}`),
  calendarEvents: (params: URLSearchParams) =>
    request<CalendarEvent[]>(`/api/v1/calendar/events?${params.toString()}`),
  localCalendarEvents: (params: URLSearchParams) =>
    request<LocalCalendarEvent[]>(`/api/v1/calendar/local-events?${params.toString()}`),
  createLocalCalendarEvent: (input: LocalCalendarEventInput) =>
    request<LocalCalendarEvent>("/api/v1/calendar/local-events", { method: "POST", body: JSON.stringify(input) }),
  updateLocalCalendarEvent: (id: string, input: LocalCalendarEventInput) =>
    request<LocalCalendarEvent>(`/api/v1/calendar/local-events/${id}`, { method: "PATCH", body: JSON.stringify(input) }),
  deleteLocalCalendarEvent: (id: string) =>
    request<void>(`/api/v1/calendar/local-events/${id}`, { method: "DELETE" }),
  notificationSettings: () =>
    request<NotificationSettings>("/api/v1/notifications/settings"),
  updateNotificationSettings: (settings: NotificationSettings) =>
    request<NotificationSettings>("/api/v1/notifications/settings", {
      method: "PATCH",
      body: JSON.stringify(settings),
    }),
  testNotificationSettings: (settings: NotificationSettings) =>
    request<void>("/api/v1/notifications/test", {
      method: "POST",
      body: JSON.stringify(settings),
    }),
  mailSettings: () => request<MailSettings>("/api/v1/mail/settings"),
  updateMailSettings: (settings: MailSettings) =>
    request<MailSettings>("/api/v1/mail/settings", {
      method: "PATCH",
      body: JSON.stringify(settings),
    }),
  mailPreferences: () => request<MailPreferences>("/api/v1/preferences/mail"),
  updateMailPreferences: (preferences: MailPreferences) =>
    request<MailPreferences>("/api/v1/preferences/mail", {
      method: "PUT",
      body: JSON.stringify(preferences),
    }),
  signatures: () => request<Signature[]>("/api/v1/signatures"),
  createSignature: (input: SignatureInput) =>
    request<Signature>("/api/v1/signatures", { method: "POST", body: JSON.stringify(input) }),
  updateSignature: (id: string, input: SignatureInput) =>
    request<Signature>(`/api/v1/signatures/${id}`, {
      method: "PATCH",
      body: JSON.stringify(input),
    }),
  deleteSignature: (id: string) => request<void>(`/api/v1/signatures/${id}`, { method: "DELETE" }),
  mcpSettings: () => request<McpSettings>("/api/v1/mcp/settings"),
  generateMcpToken: () => request<GeneratedMcpToken>("/api/v1/mcp/token", { method: "POST" }),
  updateMcpSettings: (allowDelete: boolean) =>
    request<McpSettings>("/api/v1/mcp/settings", {
      method: "PATCH",
      body: JSON.stringify({ allowDelete }),
    }),
  revokeMcpToken: () => request<void>("/api/v1/mcp/token", { method: "DELETE" }),
  cleanupRules: () => request<CleanupRule[]>("/api/v1/cleanup/rules"),
  createCleanupRule: (input: CleanupRuleInput) =>
    request<CleanupRule>("/api/v1/cleanup/rules", {
      method: "POST",
      body: JSON.stringify(input),
    }),
  updateCleanupRule: (id: string, input: CleanupRuleInput) =>
    request<CleanupRule>(`/api/v1/cleanup/rules/${id}`, {
      method: "PATCH",
      body: JSON.stringify(input),
    }),
  deleteCleanupRule: (id: string) =>
    request<void>(`/api/v1/cleanup/rules/${id}`, { method: "DELETE" }),
  reorderCleanupRules: (ids: string[]) =>
    request<CleanupRule[]>("/api/v1/cleanup/rules/reorder", {
      method: "PUT",
      body: JSON.stringify({ ids }),
    }),
  exportConfiguration: (
    passphrase: string,
    scope: MigrationScope,
    sections: MigrationSections,
  ) => request<MigrationArchive>("/api/v1/users/migration/export", {
    method: "POST",
    body: JSON.stringify({ passphrase, scope, sections }),
  }),
  importConfiguration: (
    passphrase: string,
    sections: MigrationSections,
    archive: MigrationArchive,
  ) => request<ImportReport>("/api/v1/users/migration/import", {
    method: "POST",
    body: JSON.stringify({ passphrase, sections, archive }),
  }),
}
