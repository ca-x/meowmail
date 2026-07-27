import { render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import { defaultMailPreferences } from "../../app/mailPreferences"
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
    <Providers>
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
    </Providers>,
  )
}

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
  await user.click(await screen.findByRole("checkbox", { name: /Enable new mail notifications/i }))
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
