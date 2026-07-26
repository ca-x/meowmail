import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import type { MailAccount, MessageSummary } from "../../app/types"
import { I18nProvider } from "../../i18n/I18nProvider"
import { ComposeDialog } from "./ComposeDialog"
import { MessageList } from "./MessageList"

afterEach(() => vi.restoreAllMocks())

const message: MessageSummary = {
  id: "message-1",
  accountId: "account-1",
  folder: "INBOX",
  uid: 1,
  senderName: "Alice",
  senderEmail: "alice@example.com",
  subject: "Project update",
  preview: "The latest status",
  receivedAt: 1_700_000_000,
  isRead: false,
  isStarred: false,
  attachmentCount: 0,
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
  createdAt: 1_700_000_000,
  updatedAt: 1_700_000_000,
  hasPassword: true,
}

test("keyboard activation of the star button does not select the message row", async () => {
  const user = userEvent.setup()
  const onSelect = vi.fn()
  const onToggleStar = vi.fn()

  render(
    <I18nProvider>
      <MessageList messages={[message]} selectedId={null} loading={false} onSelect={onSelect} onToggleStar={onToggleStar} />
    </I18nProvider>,
  )

  const star = screen.getByRole("button", { name: /Star|星标/ })
  star.focus()
  await user.keyboard("{Enter}")

  expect(onToggleStar).toHaveBeenCalledOnce()
  expect(onSelect).not.toHaveBeenCalled()
})

test("a compose dialog cannot be dismissed while a send request is pending", async () => {
  const user = userEvent.setup()
  let finishSend: () => void = () => {}
  const pendingSend = new Promise<void>((resolve) => { finishSend = resolve })
  vi.spyOn(api, "sendMessage").mockReturnValue(pendingSend)
  const onClose = vi.fn()
  const onSent = vi.fn()

  render(
    <I18nProvider>
      <ComposeDialog accounts={[account]} activeAccountId={account.id} onClose={onClose} onSent={onSent} />
    </I18nProvider>,
  )

  await user.type(screen.getByLabelText(/To|收件人/), "alice@example.com")
  await user.type(screen.getByPlaceholderText(/Write your message|输入邮件正文/), "Hello")
  await user.click(screen.getByRole("button", { name: /Send|发送/ }))

  const cancel = screen.getByRole("button", { name: /Cancel|取消/ })
  expect(cancel).toBeDisabled()
  await user.keyboard("{Escape}")
  expect(onClose).not.toHaveBeenCalled()

  finishSend()
  await waitFor(() => expect(onSent).toHaveBeenCalledOnce())
})
