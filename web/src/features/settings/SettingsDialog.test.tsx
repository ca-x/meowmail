import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import { I18nProvider } from "../../i18n/I18nProvider"
import { ThemeProvider } from "../../theme/ThemeProvider"
import { SettingsDialog } from "./SettingsDialog"

afterEach(() => vi.restoreAllMocks())

test("notification feedback follows a language change", async () => {
  const user = userEvent.setup()
  vi.spyOn(api, "notificationSettings").mockResolvedValue({
    enabled: false,
    messageTemplate: "[{account}] {sender}: {subject}",
    commandTemplate: "",
    httpUrl: "",
  })
  vi.spyOn(api, "testNotificationSettings").mockResolvedValue(undefined)

  render(
    <ThemeProvider>
      <I18nProvider>
        <SettingsDialog onClose={vi.fn()} onOpenAccounts={vi.fn()} />
      </I18nProvider>
    </ThemeProvider>,
  )

  await user.click(screen.getByRole("button", { name: "English" }))
  await user.click(screen.getByRole("button", { name: "Send test notification" }))
  expect(await screen.findByText("Test notification sent")).toBeInTheDocument()

  await user.click(screen.getByRole("button", { name: "中文" }))
  expect(screen.getByText("测试推送已发送")).toBeInTheDocument()
})
