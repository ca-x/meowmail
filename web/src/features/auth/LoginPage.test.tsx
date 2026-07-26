import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { expect, test, vi } from "vitest"

import { I18nProvider } from "../../i18n/I18nProvider"
import { ThemeProvider } from "../../theme/ThemeProvider"
import { LoginPage } from "./LoginPage"

function renderLogin(onAuthenticated = vi.fn(), config = { localEnabled: true, oidcEnabled: false }) {
  render(
    <ThemeProvider>
      <I18nProvider>
        <LoginPage config={config} onAuthenticated={onAuthenticated} />
      </I18nProvider>
    </ThemeProvider>,
  )
  return onAuthenticated
}

test("submits a local username and password", async () => {
  const user = userEvent.setup()
  const authenticated = vi.fn()
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({
    authenticated: true,
    locked: false,
    csrfToken: "csrf-token",
    version: "0.2.0",
    user: {
      id: "user-id",
      username: "admin",
      nickname: "Admin",
      role: "admin",
      hasPassword: true,
      hasPin: false,
      hasAvatar: false,
      updatedAt: 1,
    },
  }), { status: 200, headers: { "content-type": "application/json" } })))

  renderLogin(authenticated)
  await user.type(screen.getByLabelText(/Username|用户名/), "admin")
  await user.type(screen.getByLabelText(/^Password$|^密码$/), "secret password")
  await user.click(screen.getByRole("button", { name: /^Sign in$|^登录$/ }))
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
  expect(screen.getByPlaceholderText("输入用户名")).toBeInTheDocument()
  expect(screen.getByPlaceholderText("输入登录密码")).toBeInTheDocument()
  await waitFor(() => expect(document.title).toBe("妙邮"))
  expect(description).toHaveAttribute("content", "多邮件账户 Web 邮件客户端")

  await user.click(screen.getByRole("button", { name: "切换到英文" }))
  expect(screen.getByRole("heading", { level: 1, name: "Meowmail" })).toBeInTheDocument()
  expect(screen.queryByRole("heading", { level: 1, name: "妙邮" })).not.toBeInTheDocument()
  expect(screen.getByPlaceholderText("Enter your username")).toBeInTheDocument()
  expect(screen.getByPlaceholderText("Enter your sign-in password")).toBeInTheDocument()
  await waitFor(() => expect(document.title).toBe("Meowmail"))
  description.remove()
})

test("OIDC-only mode shows one clear sign-in action", () => {
  renderLogin(vi.fn(), { localEnabled: false, oidcEnabled: true })
  expect(screen.getByRole("link", { name: /Sign in with organization|使用组织账号登录/ })).toHaveAttribute(
    "href",
    "/api/v1/auth/oidc/start",
  )
  expect(screen.queryByLabelText(/Username|用户名/)).not.toBeInTheDocument()
})
