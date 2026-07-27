import { render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import type { MailAccount } from "../../app/types"
import { Providers } from "../../app/Providers"
import { AccountDialog } from "./AccountDialog"

afterEach(() => {
  vi.restoreAllMocks()
  Object.defineProperty(window.navigator, "language", { configurable: true, value: "en-US" })
})

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

function renderDialog(existing: MailAccount | null = null) {
  render(
    <Providers>
      <AccountDialog account={existing} onClose={vi.fn()} onSaved={vi.fn()} onDeleted={vi.fn()} />
    </Providers>,
  )
}

test.each([
  ["HTTP CONNECT", "http"],
  ["SOCKS5", "socks5"],
] as const)("tests a new account through %s while preserving password semantics", async (proxyLabel, proxyKind) => {
  const user = userEvent.setup()
  const testAccount = vi.spyOn(api, "testAccount").mockResolvedValue({ imap: true, smtp: true })
  renderDialog()

  const dialog = screen.getByRole("dialog", { name: "Add mail account" })
  await user.click(within(dialog).getByRole("button", { name: "Gmail" }))
  await user.type(within(dialog).getByLabelText(/Email address/), "me@example.com")
  const mailPassword = within(dialog).getByLabelText("Mail password / app password")
  expect(mailPassword).toHaveAttribute("type", "password")
  expect(mailPassword).toHaveAttribute("autocomplete", "new-password")
  await user.type(mailPassword, "mail-secret")

  await user.click(within(dialog).getByRole("radio", { name: proxyLabel }))
  await user.type(within(dialog).getByPlaceholderText("e.g. 127.0.0.1"), "proxy.example.com")
  const port = within(dialog).getByPlaceholderText("e.g. 1080")
  await user.clear(port)
  await user.type(port, "1080")
  await user.type(within(dialog).getByLabelText("Proxy username (optional)"), "proxy-user")
  const proxyPassword = within(dialog).getByLabelText("Proxy password (optional)")
  expect(proxyPassword).toHaveAttribute("type", "password")
  await user.type(proxyPassword, "proxy-secret")

  await user.click(within(dialog).getByRole("button", { name: "Test connection" }))
  await waitFor(() => expect(testAccount).toHaveBeenCalledOnce())
  expect(testAccount).toHaveBeenCalledWith(expect.objectContaining({
    password: "mail-secret",
    proxy: {
      kind: proxyKind,
      host: "proxy.example.com",
      port: 1080,
      username: "proxy-user",
      password: "proxy-secret",
    },
  }))
  expect(await within(dialog).findByText("IMAP and SMTP connections are healthy")).toBeInTheDocument()
})

test("editing an account uses the saved credentials when both password fields stay empty", async () => {
  const user = userEvent.setup()
  const testSavedAccount = vi.spyOn(api, "testSavedAccount").mockResolvedValue({ imap: true, smtp: true })
  const testAccount = vi.spyOn(api, "testAccount")
  renderDialog(account)

  const dialog = screen.getByRole("dialog", { name: "Edit mail account" })
  const password = within(dialog).getByPlaceholderText("Leave blank to keep the saved password")
  expect(password).toHaveAttribute("type", "password")
  await user.click(within(dialog).getByRole("button", { name: "Test connection" }))

  await waitFor(() => expect(testSavedAccount).toHaveBeenCalledWith(account.id))
  expect(testAccount).not.toHaveBeenCalled()
})

test("clearing a required server port invalidates the account draft", async () => {
  const user = userEvent.setup()
  renderDialog(account)

  const dialog = screen.getByRole("dialog", { name: "Edit mail account" })
  const save = within(dialog).getByRole("button", { name: "Save" })
  expect(save).toBeEnabled()

  const imapPort = within(dialog).getAllByRole("spinbutton", { name: /^Port/ })[0]
  await user.clear(imapPort)
  await user.tab()

  expect(save).toBeDisabled()
  expect(imapPort).toHaveValue(null)
})

test.each([
  ["QQ Mail", "imap.qq.com", "smtp.qq.com", 465],
  ["163 Mail", "imap.163.com", "smtp.163.com", 465],
  ["Tencent Exmail", "imap.exmail.qq.com", "smtp.exmail.qq.com", 465],
  ["Alibaba Mail", "imap.qiye.aliyun.com", "smtp.qiye.aliyun.com", 465],
] as const)("applies the %s server preset", async (label, imapHost, smtpHost, smtpPort) => {
  const user = userEvent.setup()
  renderDialog()

  const dialog = screen.getByRole("dialog", { name: "Add mail account" })
  await user.click(within(dialog).getByRole("button", { name: label }))

  const imap = within(dialog).getByRole("region", { name: "IMAP incoming server" })
  const smtp = within(dialog).getByRole("region", { name: "SMTP outgoing server" })
  expect(within(imap).getByRole("textbox", { name: /Host/ })).toHaveValue(imapHost)
  expect(within(imap).getByRole("spinbutton", { name: /Port/ })).toHaveValue(993)
  expect(within(smtp).getByRole("textbox", { name: /Host/ })).toHaveValue(smtpHost)
  expect(within(smtp).getByRole("spinbutton", { name: /Port/ })).toHaveValue(smtpPort)
})
