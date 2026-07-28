import { fireEvent, render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import { defaultMailPreferences } from "../../app/mailPreferences"
import type { MailAccount } from "../../app/types"
import { Providers } from "../../app/Providers"
import { SettingsDialog } from "./SettingsDialog"

afterEach(() => vi.restoreAllMocks())

const session = {
  authenticated: true,
  locked: false,
  csrfToken: "csrf",
  version: "0.3.0",
  user: {
    id: "admin-id",
    username: "admin",
    nickname: "Admin",
    role: "admin" as const,
    hasPassword: true,
    hasPin: false,
    hasAvatar: false,
    aiEnabled: false,
    autoLockMinutes: null,
    updatedAt: 1,
  },
}

function mockSettingsLoad() {
  vi.spyOn(api, "notificationSettings").mockResolvedValue({
    enabled: false,
    messageTemplate: "[{account}] {sender}: {subject}",
    commandTemplate: "",
    httpUrl: "",
  })
  vi.spyOn(api, "mailSettings").mockResolvedValue({
    keepLocalAfterServerDelete: true,
    syncFetchLimit: 50,
  })
  vi.spyOn(api, "mcpSettings").mockResolvedValue({
    hasToken: false,
    allowDelete: false,
    endpoint: "/mcp",
  })
  vi.spyOn(api, "cleanupRules").mockResolvedValue([])
}

const account: MailAccount = {
  id: "account-1",
  displayName: "Work",
  email: "me@example.com",
  username: "me@example.com",
  imap: { host: "imap.example.com", port: 993, security: "tls" },
  smtp: { host: "smtp.example.com", port: 465, security: "tls" },
  proxy: { kind: "direct", hasPassword: false },
  isDefault: true,
  createdAt: 1,
  updatedAt: 1,
  hasPassword: true,
}

function renderSettings(accounts: MailAccount[] = [], onOpenAccounts = vi.fn(), onLoggedOut = vi.fn()) {
  render(
    <Providers>
      <SettingsDialog
        session={session}
        accounts={accounts}
        mailPreferences={defaultMailPreferences}
        onMailPreferencesChanged={vi.fn()}
        onAccountsChanged={vi.fn()}
        onSessionChanged={vi.fn()}
        onLoggedOut={onLoggedOut}
        onClose={vi.fn()}
        onOpenAccounts={onOpenAccounts}
      />
    </Providers>,
  )
}

test("configured mail accounts show a management summary instead of the first-account empty state", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  const onOpenAccounts = vi.fn()
  renderSettings([account], onOpenAccounts)
  await useEnglish(user)

  expect(screen.getByText("Configured mail accounts: 1. Add or edit them here.")).toBeInTheDocument()
  expect(screen.queryByText(/Add your first IMAP/i)).not.toBeInTheDocument()
  await user.click(screen.getByRole("button", { name: "Manage mail accounts" }))
  expect(onOpenAccounts).toHaveBeenCalledOnce()
})

test("about settings show the running application version", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  renderSettings()
  await useEnglish(user)

  await user.click(screen.getByRole("tab", { name: "About" }))

  expect(screen.getByRole("region", { name: "About Meowmail" })).toBeInTheDocument()
  expect(screen.getByText(session.version)).toBeInTheDocument()
})

test("calendar settings expose every supported display option and persist the selection", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  vi.spyOn(api, "calendarAccounts").mockResolvedValue([{
    id: "calendar-account-1",
    name: "Personal calendar",
    baseUrl: "https://calendar.example.com/dav",
    username: "admin",
    enabled: true,
    hasPassword: true,
    createdAt: 1,
    updatedAt: 1,
  }])
  vi.spyOn(api, "calendars").mockResolvedValue([])
  vi.spyOn(api, "calendarPreferences").mockResolvedValue({ enabledFeatures: ["lunarDate"] })
  const sync = vi.spyOn(api, "syncCalendarAccount").mockResolvedValue({ imported: 0 })
  const update = vi.spyOn(api, "updateCalendarPreferences").mockImplementation(async (preferences) => preferences)

  renderSettings()
  await useEnglish(user)
  await user.click(screen.getByRole("tab", { name: "Calendar" }))

  const options = await screen.findByRole("region", { name: "Calendar display options" })
  expect(within(options).getAllByRole("checkbox")).toHaveLength(61)
  const julianDay = within(options).getByRole("checkbox", { name: "Julian day" })
  await user.click(julianDay)
  await user.click(screen.getByRole("button", { name: "Sync" }))
  await waitFor(() => expect(sync).toHaveBeenCalledWith("calendar-account-1"))
  expect(julianDay).toBeChecked()
  await user.click(within(options).getByRole("button", { name: "Save" }))
  await waitFor(() => expect(update).toHaveBeenCalledWith({ enabledFeatures: ["lunarDate", "julianDay"] }))
})

test("profile settings submit the editable username and nickname together", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  const update = vi.spyOn(api, "updateProfile").mockResolvedValue({
    ...session.user,
    username: "new.admin",
    nickname: "New Admin",
  })
  renderSettings()
  await useEnglish(user)
  const profile = screen.getByRole("region", { name: "Profile" })
  await user.click(within(profile).getByRole("button", { name: "Edit" }))
  const username = within(profile).getByRole("textbox", { name: "Username" })
  const nickname = within(profile).getByRole("textbox", { name: "Nickname" })
  await user.clear(username)
  await user.type(username, "New.Admin")
  await user.clear(nickname)
  await user.type(nickname, "New Admin")
  await user.click(within(profile).getByRole("button", { name: "Save" }))

  await waitFor(() => expect(update).toHaveBeenCalledWith("New.Admin", "New Admin"))
})

test("security settings expose a verified sign-in password change", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  const update = vi.spyOn(api, "updatePassword").mockResolvedValue(session.user)
  const onLoggedOut = vi.fn()
  renderSettings([], vi.fn(), onLoggedOut)
  await useEnglish(user)
  await user.click(screen.getByRole("tab", { name: "Security" }))
  const password = await screen.findByRole("region", { name: "Sign-in password" })
  await user.type(within(password).getByLabelText("Current password"), "old password")
  await user.type(within(password).getByLabelText("New password"), "new password")
  await user.type(within(password).getByLabelText("Confirm new password"), "new password")
  await user.click(within(password).getByRole("button", { name: "Change sign-in password" }))

  await waitFor(() => expect(update).toHaveBeenCalledWith("old password", "new password"))
  expect(onLoggedOut).toHaveBeenCalledOnce()
})

async function useEnglish(user: ReturnType<typeof userEvent.setup>) {
  const english = screen.getByRole("radio", { name: "English" })
  if (english.getAttribute("aria-checked") !== "true") await user.click(english)
}

test("notification feedback follows a language change", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  vi.spyOn(api, "testNotificationSettings").mockResolvedValue(undefined)

  renderSettings()

  await useEnglish(user)
  await user.click(screen.getByRole("tab", { name: "Automation" }))
  await user.click(await screen.findByRole("switch", { name: /Enable new mail notifications/i }))
  await user.click(screen.getByRole("button", { name: "Send test notification" }))
  expect(await screen.findByText("Test notification sent")).toBeInTheDocument()

  await user.click(screen.getByRole("tab", { name: "General" }))
  await user.click(screen.getByRole("radio", { name: "中文" }))
  expect(screen.getByText("测试推送已发送")).toBeInTheDocument()
})

test("all compatible Astryx themes are visible and selectable", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  renderSettings()

  await useEnglish(user)
  const themeNames = ["Neutral", "Stone", "Butter", "Matcha", "Chocolate", "Gothic", "Y2K"]
  for (const name of themeNames) expect(screen.getByRole("radio", { name })).toBeInTheDocument()

  const y2k = screen.getByRole("radio", { name: "Y2K" })
  await user.click(y2k)
  expect(y2k).toBeChecked()
  expect(document.documentElement).toHaveAttribute("data-astryx-theme", "y2k")
})

test("administrator can choose only my configuration or all users", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  renderSettings()

  await useEnglish(user)
  await user.click(screen.getByRole("tab", { name: "Data" }))
  const mine = screen.getByRole("radio", { name: "Only my configuration" })
  const all = screen.getByRole("radio", { name: "All users" })
  expect(mine).toHaveAttribute("aria-checked", "true")
  await user.click(all)
  expect(all).toHaveAttribute("aria-checked", "true")
  expect(screen.getByText(/password and PIN hashes/i)).toBeInTheDocument()
})

test("user can generate an MCP token and explicitly enable deletion", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  const generateToken = vi.spyOn(api, "generateMcpToken").mockResolvedValue({
    hasToken: true,
    allowDelete: false,
    endpoint: "/mcp",
    createdAt: 1,
    token: "mmcp_test_secret",
  })
  const update = vi.spyOn(api, "updateMcpSettings").mockResolvedValue({
    hasToken: true,
    allowDelete: true,
    endpoint: "/mcp",
    createdAt: 1,
  })

  renderSettings()
  await useEnglish(user)
  await user.click(screen.getByRole("tab", { name: "Security" }))
  const generate = screen.getByRole("button", { name: "Generate token" })
  await waitFor(() => expect(generate).toBeEnabled())
  await user.click(generate)
  await waitFor(() => expect(generateToken).toHaveBeenCalledOnce())
  expect(await screen.findByDisplayValue("mmcp_test_secret")).toBeInTheDocument()
  expect(screen.getByText(/shown again/i)).toBeInTheDocument()

  await user.click(screen.getByRole("tab", { name: "General" }))
  await user.click(screen.getByRole("tab", { name: "Security" }))
  expect(screen.getByDisplayValue("mmcp_test_secret")).toBeVisible()

  await user.click(screen.getByRole("switch", { name: /Allow MCP to permanently delete email/i }))
  expect(update).toHaveBeenCalledWith(true)
})

test("user can fetch all mail or a specified recent count", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  const update = vi.spyOn(api, "updateMailSettings").mockImplementation(async (settings) => settings)

  renderSettings()
  await useEnglish(user)
  await user.click(screen.getByRole("tab", { name: "Automation" }))
  const form = await screen.findByRole("form", { name: "Sync fetch range" })

  await user.click(within(form).getByRole("radio", { name: "Fetch all" }))
  await user.click(within(form).getByRole("button", { name: "Save" }))
  await waitFor(() => expect(update).toHaveBeenLastCalledWith({
    keepLocalAfterServerDelete: true,
    syncFetchLimit: null,
  }))

  await user.click(within(form).getByRole("radio", { name: "Recent count" }))
  const count = within(form).getByRole("spinbutton", { name: "Recent message count" })
  await user.clear(count)
  await user.type(count, "125")
  await user.click(within(form).getByRole("button", { name: "Save" }))
  await waitFor(() => expect(update).toHaveBeenLastCalledWith({
    keepLocalAfterServerDelete: true,
    syncFetchLimit: 125,
  }))
})

test("mail settings save a vacation responder schedule and contacts-only scope", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  const update = vi.spyOn(api, "updateMailPreferences").mockImplementation(async (preferences) => preferences)

  renderSettings([account])
  await useEnglish(user)
  await user.click(screen.getByRole("tab", { name: "Mail" }))

  await user.click(await screen.findByRole("switch", { name: "Automatic reply" }))
  await user.click(screen.getByRole("combobox", { name: "Active mailboxes" }))
  await user.click(await screen.findByRole("option", { name: /Work/ }))
  fireEvent.change(screen.getByLabelText("Start time"), { target: { value: "01/01/2024" } })
  fireEvent.change(screen.getByLabelText("Start time time"), { target: { value: "09:00" } })
  fireEvent.change(screen.getByLabelText("End time"), { target: { value: "01/05/2024" } })
  fireEvent.change(screen.getByLabelText("End time time"), { target: { value: "18:30" } })
  fireEvent.change(screen.getByRole("textbox", { name: "Automatic reply subject" }), { target: { value: "Out of office" } })
  fireEvent.change(screen.getByRole("textbox", { name: /^Automatic reply content/ }), { target: { value: "I am away this week." } })
  await user.click(screen.getByRole("switch", { name: "Reply only to contacts" }))
  await user.click(screen.getByRole("button", { name: "Save mail preferences" }))

  await waitFor(() => expect(update).toHaveBeenCalledWith(expect.objectContaining({
    autoReplyEnabled: true,
    autoReplySubject: "Out of office",
    autoReplyText: "I am away this week.",
    autoReplyStartAt: localEpoch("2024-01-01T09:00"),
    autoReplyEndAt: localEpoch("2024-01-05T18:30"),
    autoReplyAccountIds: [account.id],
    autoReplyContactsOnly: true,
  })))
}, 10_000)

test("settings tabs use arrow-key navigation and expose one tabpanel", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  renderSettings()
  await useEnglish(user)

  const general = screen.getByRole("tab", { name: "General" })
  general.focus()
  await user.keyboard("{ArrowRight}{Enter}")

  expect(screen.getByRole("tab", { name: "Mail" })).toHaveAttribute("aria-selected", "true")
  expect(screen.getAllByRole("tabpanel")).toHaveLength(1)
  expect(screen.getByRole("tabpanel", { name: "Mail" })).toBeInTheDocument()
})

function localEpoch(value: string) {
  return Math.floor(new Date(value).getTime() / 1_000)
}
