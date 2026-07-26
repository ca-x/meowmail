import type {
  AccountInput,
  MailAccount,
  MessageDetail,
  MessageSummary,
  NotificationSettings,
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
  async session() {
    const session = await request<SessionResponse>("/api/v1/session")
    csrfToken = session.csrfToken
    return session
  },
  async login(pin: string) {
    const session = await request<SessionResponse>("/api/v1/auth/login", {
      method: "POST",
      body: JSON.stringify({ pin }),
    })
    csrfToken = session.csrfToken
    return session
  },
  async logout() {
    await request<void>("/api/v1/auth/logout", { method: "POST" })
    csrfToken = ""
  },
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
}
