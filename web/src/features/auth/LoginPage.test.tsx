import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { expect, test, vi } from "vitest"

import { I18nProvider } from "../../i18n/I18nProvider"
import { ThemeProvider } from "../../theme/ThemeProvider"
import { LoginPage } from "./LoginPage"

test("submits a PIN and completes authentication", async () => {
  const user = userEvent.setup()
  const authenticated = vi.fn()
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({
    authenticated: true,
    csrfToken: "csrf-token",
    version: "0.1.0",
  }), { status: 200, headers: { "content-type": "application/json" } })))

  render(
    <ThemeProvider>
      <I18nProvider>
        <LoginPage onAuthenticated={authenticated} />
      </I18nProvider>
    </ThemeProvider>,
  )
  await user.type(screen.getByLabelText(/^PIN$/), "2468")
  await user.click(screen.getByRole("button", { name: /Open Meowmail|进入妙邮/ }))
  expect(authenticated).toHaveBeenCalledOnce()
  vi.unstubAllGlobals()
})
