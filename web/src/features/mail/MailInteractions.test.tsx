import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import { defaultMailPreferences } from "../../app/mailPreferences"
import type { MailAccount, MessageDetail as MessageDetailType, MessageSummary } from "../../app/types"
import { I18nProvider } from "../../i18n/I18nProvider"
import { ThemeProvider } from "../../theme/ThemeProvider"
import { ComposeDialog } from "./ComposeDialog"
import { MessageDetail } from "./MessageDetail"
import { MessageList } from "./MessageList"

afterEach(() => {
  vi.restoreAllMocks()
  Object.defineProperty(window.navigator, "language", { configurable: true, value: "en-US" })
})

vi.mock("@file-viewer/web-full", () => ({
  mountViewer: vi.fn(() => ({ destroy: vi.fn() })),
}))

const message: MessageSummary = {
  id: "message-1",
  accountId: "account-1",
  folder: "INBOX",
  uid: 1,
  senderName: "Alice",
  senderEmail: "alice@example.com",
  subject: "Project update",
  threadKey: "project update",
  preview: "The latest status",
  receivedAt: 1_700_000_000,
  isRead: false,
  isStarred: false,
  attachmentCount: 0,
  rawSize: 256,
  isPromotional: false,
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
      <ComposeDialog accounts={[account]} activeAccountId={account.id} preferences={defaultMailPreferences} onClose={onClose} onSent={onSent} />
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

test("attachment information is localized and opens the preview dialog", async () => {
  Object.defineProperty(window.navigator, "language", { configurable: true, value: "zh-CN" })
  const user = userEvent.setup()
  const detail: MessageDetailType = {
    ...message,
    attachmentCount: 1,
    references: [],
    recipients: ["me@example.com"],
    ccRecipients: [],
    bodyText: "Please review the attached handbook.",
    bodyHtml: null,
    attachments: [{
      id: "attachment-1",
      filename: "handbook.pdf",
      contentType: "application/pdf",
      size: 9,
      available: true,
    }],
  }

  render(
    <ThemeProvider>
      <I18nProvider>
        <MessageDetail
          message={detail}
          thread={[detail]}
          loading={false}
          preferences={defaultMailPreferences}
          onBack={vi.fn()}
          onToggleStar={vi.fn()}
          onToggleRead={vi.fn()}
          onReply={vi.fn()}
          onForward={vi.fn()}
          onDelete={vi.fn()}
        />
      </I18nProvider>
    </ThemeProvider>,
  )

  expect(screen.getByRole("heading", { name: "附件" })).toBeInTheDocument()
  expect(screen.getByText("handbook.pdf")).toBeInTheDocument()
  expect(screen.getByText("application/pdf · 9 B")).toBeInTheDocument()
  expect(screen.getByRole("link", { name: "下载附件: handbook.pdf" })).toHaveAttribute(
    "href",
    "/api/v1/messages/message-1/attachments/attachment-1?download=true",
  )

  await user.click(screen.getByRole("button", { name: "预览" }))
  expect(await screen.findByRole("dialog", { name: "handbook.pdf" })).toBeInTheDocument()
  expect(screen.getByText("附件预览")).toBeInTheDocument()
})
