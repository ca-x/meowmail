import { render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import { defaultMailPreferences } from "../../app/mailPreferences"
import { I18nProvider } from "../../i18n/I18nProvider"
import { ThemeProvider } from "../../theme/ThemeProvider"
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

function renderSettings() {
  render(
    <ThemeProvider>
      <I18nProvider>
        <SettingsDialog
          session={session}
          accounts={[]}
          mailPreferences={defaultMailPreferences}
          onMailPreferencesChanged={vi.fn()}
          onAccountsChanged={vi.fn()}
          onSessionChanged={vi.fn()}
          onLocked={vi.fn()}
          onClose={vi.fn()}
          onOpenAccounts={vi.fn()}
        />
      </I18nProvider>
    </ThemeProvider>,
  )
}

test("notification feedback follows a language change", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  vi.spyOn(api, "testNotificationSettings").mockResolvedValue(undefined)

  renderSettings()

  await user.click(screen.getByRole("button", { name: "English" }))
  await user.click(screen.getByText("New mail notifications"))
  await user.click(screen.getByRole("button", { name: "Send test notification" }))
  expect(await screen.findByText("Test notification sent")).toBeInTheDocument()

  await user.click(screen.getByRole("button", { name: "中文" }))
  expect(screen.getByText("测试推送已发送")).toBeInTheDocument()
})

test("administrator can choose only my configuration or all users", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  renderSettings()

  const english = screen.getByRole("button", { name: "English" })
  await user.click(english)
  const mine = screen.getByRole("button", { name: "Only my configuration" })
  const all = screen.getByRole("button", { name: "All users" })
  expect(mine).toHaveAttribute("aria-pressed", "true")
  await user.click(all)
  expect(all).toHaveAttribute("aria-pressed", "true")
  expect(screen.getByText(/password and PIN hashes/i)).toBeInTheDocument()
})

test("user can generate an MCP token and explicitly enable deletion", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  vi.spyOn(api, "generateMcpToken").mockResolvedValue({
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
  await user.click(screen.getByRole("button", { name: "English" }))
  await user.click(screen.getByRole("button", { name: "Generate token" }))
  expect(await screen.findByDisplayValue("mmcp_test_secret")).toBeInTheDocument()
  expect(screen.getByText(/shown again/i)).toBeInTheDocument()

  await user.click(screen.getByRole("checkbox", { name: /Allow MCP to permanently delete email/i }))
  expect(update).toHaveBeenCalledWith(true)
})

test("user can fetch all mail or a specified recent count", async () => {
  const user = userEvent.setup()
  mockSettingsLoad()
  const update = vi.spyOn(api, "updateMailSettings").mockImplementation(async (settings) => settings)

  renderSettings()
  await user.click(screen.getByRole("button", { name: "English" }))
  const form = await screen.findByRole("form", { name: "Sync fetch range" })

  await user.click(within(form).getByRole("button", { name: "Fetch all" }))
  await user.click(within(form).getByRole("button", { name: "Save" }))
  await waitFor(() => expect(update).toHaveBeenLastCalledWith({
    keepLocalAfterServerDelete: true,
    syncFetchLimit: null,
  }))

  await user.click(within(form).getByRole("button", { name: "Recent count" }))
  const count = within(form).getByRole("spinbutton", { name: "Recent message count" })
  await user.clear(count)
  await user.type(count, "125")
  await user.click(within(form).getByRole("button", { name: "Save" }))
  await waitFor(() => expect(update).toHaveBeenLastCalledWith({
    keepLocalAfterServerDelete: true,
    syncFetchLimit: 125,
  }))
})
