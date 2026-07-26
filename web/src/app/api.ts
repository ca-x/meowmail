import type {
  AccountInput,
  AuthConfig,
  CleanupRule,
  CleanupRuleInput,
  ImportReport,
  MailAccount,
  MailSettings,
  MessageDetail,
  MessageSummary,
  MigrationArchive,
  MigrationScope,
  MigrationSections,
  NotificationSettings,
  PublicUser,
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
  updateProfile: (nickname: string) =>
    request<PublicUser>("/api/v1/users/me", {
      method: "PATCH",
      body: JSON.stringify({ nickname }),
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
  deleteAccount: (id: string) => request<void>(`/api/v1/accounts/${id}`, { method: "DELETE" }),
  testAccount: (input: AccountInput) =>
    request<{ imap: boolean; smtp: boolean }>("/api/v1/accounts/test", {
      method: "POST",
      body: JSON.stringify(input),
    }),
  testSavedAccount: (id: string) =>
    request<{ imap: boolean; smtp: boolean }>(`/api/v1/accounts/${id}/test`, { method: "POST" }),
  syncAccount: (id: string) =>
    request<{ inserted: number; syncedAt: number }>(`/api/v1/accounts/${id}/sync`, {
      method: "POST",
    }),
  messages: (params: URLSearchParams) =>
    request<MessageSummary[]>(`/api/v1/messages?${params.toString()}`),
  message: (id: string) => request<MessageDetail>(`/api/v1/messages/${id}`),
  updateMessage: (id: string, update: { isRead?: boolean; isStarred?: boolean }) =>
    request<MessageSummary>(`/api/v1/messages/${id}`, {
      method: "PATCH",
      body: JSON.stringify(update),
    }),
  sendMessage: (input: {
    accountId: string
    to: string[]
    cc: string[]
    bcc: string[]
    subject: string
    textBody: string
  }) => request<void>("/api/v1/messages/send", { method: "POST", body: JSON.stringify(input) }),
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
