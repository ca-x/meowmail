import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import { I18nProvider } from "../../i18n/I18nProvider"
import { ThemeProvider } from "../../theme/ThemeProvider"
import { SettingsDialog } from "./SettingsDialog"

afterEach(() => vi.restoreAllMocks())

const session = {
  authenticated: true,
  locked: false,
  csrfToken: "csrf",
  version: "0.2.0",
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
  vi.spyOn(api, "mailSettings").mockResolvedValue({ keepLocalAfterServerDelete: true })
  vi.spyOn(api, "cleanupRules").mockResolvedValue([])
}

function renderSettings() {
  render(
    <ThemeProvider>
      <I18nProvider>
        <SettingsDialog
          session={session}
          accounts={[]}
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
