import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { expect, test, vi } from "vitest"

import { I18nProvider } from "../../i18n/I18nProvider"
import { ThemeProvider } from "../../theme/ThemeProvider"
import { LoginPage } from "./LoginPage"

function renderLogin(onAuthenticated = vi.fn()) {
  render(
    <ThemeProvider>
      <I18nProvider>
        <LoginPage onAuthenticated={onAuthenticated} />
      </I18nProvider>
    </ThemeProvider>,
  )
  return onAuthenticated
}

test("submits a PIN and completes authentication", async () => {
  const user = userEvent.setup()
  const authenticated = vi.fn()
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({
    authenticated: true,
    csrfToken: "csrf-token",
    version: "0.1.0",
  }), { status: 200, headers: { "content-type": "application/json" } })))

  renderLogin(authenticated)
  await user.type(screen.getByLabelText(/^PIN$/), "2468")
  await user.click(screen.getByRole("button", { name: /Open Meowmail|进入妙邮/ }))
  expect(authenticated).toHaveBeenCalledOnce()
  vi.unstubAllGlobals()
})

test("keeps the brand, placeholders, and document metadata in the selected language", async () => {
  const user = userEvent.setup()
  const description = document.createElement("meta")
  description.name = "description"
  document.head.append(description)
  renderLogin()

  const switchToChinese = screen.queryByRole("button", { name: "Switch to Chinese" })
  if (switchToChinese) await user.click(switchToChinese)

  expect(screen.getByRole("heading", { level: 1, name: "妙邮" })).toBeInTheDocument()
  expect(screen.queryByRole("heading", { level: 1, name: "Meowmail" })).not.toBeInTheDocument()
  expect(screen.getByPlaceholderText("输入部署时设置的 PIN 码")).toBeInTheDocument()
  await waitFor(() => expect(document.title).toBe("妙邮"))
  expect(description).toHaveAttribute("content", "多邮件账户 Web 邮件客户端")

  await user.click(screen.getByRole("button", { name: "切换到英文" }))
  expect(screen.getByRole("heading", { level: 1, name: "Meowmail" })).toBeInTheDocument()
  expect(screen.queryByRole("heading", { level: 1, name: "妙邮" })).not.toBeInTheDocument()
  expect(screen.getByPlaceholderText("Enter the deployment PIN")).toBeInTheDocument()
  await waitFor(() => expect(document.title).toBe("Meowmail"))
  description.remove()
})
