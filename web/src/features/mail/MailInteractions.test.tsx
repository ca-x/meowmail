import { render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import { defaultMailPreferences } from "../../app/mailPreferences"
import type { MailAccount, MessageDetail as MessageDetailType, MessageSummary, SessionResponse } from "../../app/types"
import { Providers } from "../../app/Providers"
import { ComposeDialog } from "./ComposeDialog"
import { MessageDetail } from "./MessageDetail"
import { MessageList } from "./MessageList"
import { MailWorkspace } from "./MailWorkspace"

afterEach(() => {
  vi.restoreAllMocks()
  Object.defineProperty(window.navigator, "language", { configurable: true, value: "en-US" })
  window.history.replaceState(null, "", "/mail/inbox")
})

vi.mock("@file-viewer/web-full", () => ({
  mountViewer: vi.fn(() => ({ destroy: vi.fn() })),
}))

vi.mock("@react-email/editor", async () => {
  const React = await import("react")

  function textFromContent(value: unknown): string {
    if (typeof value === "string") return value.replace(/<br\s*\/?>/gi, "\n").replace(/<[^>]*>/g, "").trim()
    if (!value || typeof value !== "object") return ""
    const node = value as { text?: string; content?: unknown[] }
    return [node.text || "", ...(node.content || []).map(textFromContent)].join("").trim()
  }

  function escapeHtml(value: string) {
    return value
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
  }

  function documentFromText(value: string) {
    return {
      type: "doc",
      content: [{
        type: "paragraph",
        attrs: { textAlign: "left" },
        content: value ? [{ type: "text", text: value }] : [],
      }],
    }
  }

  const EmailEditor = React.forwardRef<unknown, {
    className?: string
    content?: unknown
    onReady?: (ref: unknown) => void
    onUpdate?: (ref: unknown) => void
  }>(function MockEmailEditor({ className, content = "", onReady, onUpdate }, forwardedRef) {
    const hostRef = React.useRef<HTMLDivElement | null>(null)
    const textRef = React.useRef(textFromContent(content))
    const documentRef = React.useRef(typeof content === "object" ? content : documentFromText(textRef.current))
    const helperRef = React.useRef<unknown>(null)

    if (!helperRef.current) {
      helperRef.current = {
        getEmail: async () => ({ html: `<html><body data-delivery-layout="centered"><p>${escapeHtml(textRef.current)}</p></body></html>`, text: textRef.current }),
        getEmailHTML: async () => `<p>${escapeHtml(textRef.current)}</p>`,
        getEmailText: async () => textRef.current,
        getJSON: () => documentRef.current,
        editor: {
          getText: () => textRef.current,
          view: { dom: null as HTMLDivElement | null },
        },
      }
    }

    React.useImperativeHandle(forwardedRef, () => helperRef.current)
    React.useEffect(() => {
      textRef.current = textFromContent(content)
      documentRef.current = typeof content === "object" ? content : documentFromText(textRef.current)
      if (hostRef.current) hostRef.current.textContent = textRef.current
    }, [content])
    React.useEffect(() => {
      const helper = helperRef.current as { editor: { view: { dom: HTMLDivElement | null } } }
      helper.editor.view.dom = hostRef.current
      onReady?.(helperRef.current)
    }, [onReady])

    return React.createElement("div", {
      className,
      contentEditable: true,
      "data-testid": "compose-rich-editor",
      "data-editor-content": JSON.stringify(content),
      ref: hostRef,
      role: "textbox",
      "aria-label": "Message",
      "aria-multiline": "true",
      suppressContentEditableWarning: true,
      tabIndex: 0,
      onInput: (event: React.FormEvent<HTMLDivElement>) => {
        textRef.current = event.currentTarget.textContent || ""
        documentRef.current = documentFromText(textRef.current)
        onUpdate?.(helperRef.current)
      },
    }, textRef.current)
  })

  return { EmailEditor }
})

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

const session: SessionResponse = {
  authenticated: true,
  locked: false,
  csrfToken: "csrf-token",
  version: "0.3.0",
  user: {
    id: "user-1",
    username: "admin",
    nickname: "Admin",
    email: "admin@example.com",
    role: "admin",
    hasPassword: true,
    hasPin: true,
    hasAvatar: false,
    aiEnabled: false,
    autoLockMinutes: null,
    updatedAt: 1_700_000_000,
  },
}

const messageDetail: MessageDetailType = {
  ...message,
  isRead: true,
  references: [],
  recipients: [account.email],
  ccRecipients: [],
  bodyText: "The full project update.",
  bodyHtml: null,
  attachments: [],
}

function stubWorkspaceApi() {
  vi.spyOn(api, "accounts").mockResolvedValue([account])
  vi.spyOn(api, "mailPreferences").mockResolvedValue(defaultMailPreferences)
  vi.spyOn(api, "messages").mockResolvedValue([message])
  vi.spyOn(api, "message").mockResolvedValue(messageDetail)
  vi.spyOn(api, "messageThread").mockResolvedValue([messageDetail])
  vi.spyOn(api, "updateMessage").mockImplementation(async (_id, update) => ({ ...message, ...update }))
  vi.spyOn(api, "syncAccount").mockResolvedValue({ inserted: 0, syncedAt: 1_700_000_000 })
  vi.spyOn(api, "deleteMessage").mockResolvedValue(undefined)
  vi.spyOn(api, "logout").mockResolvedValue(undefined)
  vi.spyOn(api, "calendarPreferences").mockResolvedValue({ enabledFeatures: ["lunarDate", "solarTerm"] })
  vi.spyOn(api, "calendars").mockResolvedValue([])
  vi.spyOn(api, "calendarDayInfo").mockResolvedValue([])
  vi.spyOn(api, "calendarEvents").mockResolvedValue([])
  vi.spyOn(api, "localCalendarEvents").mockResolvedValue([])
}

function renderWorkspace(currentSession = session, onLocked = vi.fn()) {
  render(
    <Providers>
      <MailWorkspace
        session={currentSession}
        onSessionChanged={vi.fn()}
        onLocked={onLocked}
        onLoggedOut={vi.fn()}
      />
    </Providers>,
  )
}

test("keyboard activation of the star button does not select the message row", async () => {
  const user = userEvent.setup()
  const onSelect = vi.fn()
  const onToggleStar = vi.fn()

  render(
    <Providers>
      <MessageList messages={[message]} selectedId={null} loading={false} onSelect={onSelect} onToggleStar={onToggleStar} />
    </Providers>,
  )

  const star = screen.getByRole("button", { name: /Star|星标/ })
  star.focus()
  await user.keyboard("{Enter}")

  expect(onToggleStar).toHaveBeenCalledOnce()
  expect(onSelect).not.toHaveBeenCalled()
})

test("the compose workspace cannot be dismissed while a send request is pending", async () => {
  const user = userEvent.setup()
  let finishSend: () => void = () => {}
  const pendingSend = new Promise<void>((resolve) => { finishSend = resolve })
  vi.spyOn(api, "sendMessage").mockReturnValue(pendingSend)
  const onClose = vi.fn()
  const onSent = vi.fn()

  render(
    <Providers>
      <ComposeDialog accounts={[account]} activeAccountId={account.id} preferences={defaultMailPreferences} onClose={onClose} onSent={onSent} />
    </Providers>,
  )

  await user.type(screen.getByRole("combobox", { name: /To|收件人/ }), "alice@example.com{Enter}")
  await user.type(screen.getByRole("textbox", { name: /Message|正文/ }), "Hello")
  const send = screen.getByRole("button", { name: /Send|发送/ })
  await waitFor(() => expect(send).toBeEnabled())
  await user.click(send)

  const back = screen.getByRole("button", { name: /Back|返回/ })
  expect(back).toBeDisabled()
  await user.keyboard("{Escape}")
  expect(onClose).not.toHaveBeenCalled()

  finishSend()
  await waitFor(() => expect(onSent).toHaveBeenCalledOnce())
})

test("drafts preserve the editor document separately from delivery HTML", async () => {
  vi.spyOn(api, "signatures").mockResolvedValue([])
  vi.spyOn(api, "contacts").mockResolvedValue([])
  const createDraft = vi.spyOn(api, "createDraft").mockImplementation(async (input) => ({
    id: "draft-rich-text",
    accountId: input.accountId,
    to: input.to,
    cc: input.cc,
    bcc: input.bcc,
    subject: input.subject,
    textBody: input.textBody,
    htmlBody: input.htmlBody,
    editorDocument: input.editorDocument,
    attachments: input.attachments || [],
    signatureId: input.signatureId,
    applySignature: input.applySignature ?? true,
    scheduledAt: input.scheduledAt,
    status: "draft",
    createdAt: 1,
    updatedAt: 1,
  }))
  const user = userEvent.setup()
  const firstRender = render(
    <Providers>
      <ComposeDialog accounts={[account]} activeAccountId={account.id} preferences={defaultMailPreferences} onClose={vi.fn()} onSent={vi.fn()} />
    </Providers>,
  )

  await user.type(screen.getByRole("textbox", { name: /Message|正文/ }), "Left aligned body")
  await user.click(screen.getByRole("button", { name: /Save draft|存为草稿/ }))
  await waitFor(() => expect(createDraft).toHaveBeenCalledOnce())

  const saved = createDraft.mock.calls[0][0]
  expect(saved.htmlBody).toContain("data-delivery-layout=\"centered\"")
  expect(saved.editorDocument).toMatchObject({
    type: "doc",
    content: [{ attrs: { textAlign: "left" } }],
  })

  firstRender.unmount()
  render(
    <Providers>
      <ComposeDialog
        accounts={[account]}
        activeAccountId={account.id}
        preferences={defaultMailPreferences}
        draft={{
          id: "draft-rich-text",
          accountId: account.id,
          body: saved.textBody,
          htmlBody: saved.htmlBody,
          editorDocument: saved.editorDocument,
        }}
        onClose={vi.fn()}
        onSent={vi.fn()}
      />
    </Providers>,
  )

  expect(screen.getByTestId("compose-rich-editor")).toHaveAttribute("data-editor-content", JSON.stringify(saved.editorDocument))
})

test("an unchanged saved draft closes without a discard confirmation", async () => {
  vi.spyOn(api, "signatures").mockResolvedValue([])
  vi.spyOn(api, "contacts").mockResolvedValue([])
  const user = userEvent.setup()
  const onClose = vi.fn()

  render(
    <Providers>
      <ComposeDialog
        accounts={[account]}
        activeAccountId={account.id}
        preferences={defaultMailPreferences}
        draft={{
          id: "draft-1",
          accountId: account.id,
          to: "alice@example.com",
          subject: "Saved draft",
          body: "Existing body",
          htmlBody: "<p>Existing body</p>",
          signatureId: null,
          applySignature: true,
        }}
        onClose={onClose}
        onSent={vi.fn()}
      />
    </Providers>,
  )

  await screen.findByRole("region", { name: /Compose|写邮件/ })
  await user.keyboard("{Escape}")

  expect(onClose).toHaveBeenCalledOnce()
  expect(screen.queryByText(/Discard this message|放弃这封邮件/)).not.toBeInTheDocument()
})

test("a changed saved draft asks for discard confirmation before closing", async () => {
  vi.spyOn(api, "signatures").mockResolvedValue([])
  vi.spyOn(api, "contacts").mockResolvedValue([])
  const user = userEvent.setup()
  const onClose = vi.fn()

  render(
    <Providers>
      <ComposeDialog
        accounts={[account]}
        activeAccountId={account.id}
        preferences={defaultMailPreferences}
        draft={{
          id: "draft-1",
          accountId: account.id,
          to: "alice@example.com",
          subject: "Saved draft",
          body: "Existing body",
          htmlBody: "<p>Existing body</p>",
          signatureId: null,
          applySignature: true,
        }}
        onClose={onClose}
        onSent={vi.fn()}
      />
    </Providers>,
  )

  await screen.findByRole("region", { name: /Compose|写邮件/ })
  const editor = screen.getByRole("textbox", { name: /Message|正文/ })
  await user.click(editor)
  await user.keyboard(" updated")
  await user.keyboard("{Escape}")

  expect(await screen.findByText(/Discard this message|放弃这封邮件/)).toBeInTheDocument()
  expect(onClose).not.toHaveBeenCalled()
  await user.click(screen.getByRole("button", { name: /^Discard$|^放弃$/ }))
  expect(onClose).toHaveBeenCalledOnce()
})

test("cancelling a draft discard confirmation keeps the composer open", async () => {
  vi.spyOn(api, "signatures").mockResolvedValue([])
  vi.spyOn(api, "contacts").mockResolvedValue([])
  const user = userEvent.setup()
  const onClose = vi.fn()

  render(
    <Providers>
      <ComposeDialog
        accounts={[account]}
        activeAccountId={account.id}
        preferences={defaultMailPreferences}
        draft={{
          id: "draft-1",
          accountId: account.id,
          to: "alice@example.com",
          subject: "Saved draft",
          body: "Existing body",
          htmlBody: "<p>Existing body</p>",
          signatureId: null,
          applySignature: true,
        }}
        onClose={onClose}
        onSent={vi.fn()}
      />
    </Providers>,
  )

  await screen.findByRole("region", { name: /Compose|写邮件/ })
  const editor = screen.getByRole("textbox", { name: /Message|正文/ })
  await user.click(editor)
  await user.keyboard(" updated")
  await user.keyboard("{Escape}")

  expect(await screen.findByText(/Discard this message|放弃这封邮件/)).toBeInTheDocument()
  await user.click(screen.getByRole("button", { name: /Keep editing|继续编辑/ }))

  expect(onClose).not.toHaveBeenCalled()
  expect(screen.getByRole("region", { name: /Compose|写邮件/ })).toBeInTheDocument()
  await waitFor(() => expect(screen.queryByRole("alertdialog", { name: /Discard this message|放弃这封邮件/ })).not.toBeInTheDocument())
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
    <Providers>
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
    </Providers>,
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

test("HTML mail follows the resolved dark application theme", () => {
  const previousMatchMedia = window.matchMedia
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn((query: string) => ({
      matches: query === "(prefers-color-scheme: dark)",
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  })
  const detail: MessageDetailType = {
    ...messageDetail,
    bodyHtml: '<table style="background:#fff"><tr><td style="color:#111">Dark-aware mail</td></tr></table>',
  }

  render(
    <Providers>
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
    </Providers>,
  )

  const source = screen.getByTitle("Project update").getAttribute("srcdoc") || ""
  expect(source).toContain('content="dark"')
  expect(source).toContain("background:#111624!important")
  Object.defineProperty(window, "matchMedia", { configurable: true, value: previousMatchMedia })
})

test("the Astryx mail shell keeps search and message navigation immediate", async () => {
  stubWorkspaceApi()
  const user = userEvent.setup()
  renderWorkspace()

  const search = await screen.findByRole("textbox", { name: /Search mail|搜索邮件/ })
  await user.keyboard("{Control>}k{/Control}")
  expect(search).toHaveFocus()

  expect(await screen.findByTestId("message-list-scroll")).toBeInTheDocument()
  await user.click(screen.getByText("Project update"))
  await waitFor(() => expect(api.message).toHaveBeenCalledWith(message.id))
  expect(await screen.findByText("The full project update.")).toBeInTheDocument()
})

test("calendar is a top-level workspace and the unused notification bell is absent", async () => {
  stubWorkspaceApi()
  const user = userEvent.setup()
  renderWorkspace()

  await user.click(await screen.findByRole("radio", { name: /Calendar|日历/ }))
  expect(await screen.findByRole("main", { name: /Calendar|日历/ })).toBeInTheDocument()
  expect(window.location.pathname).toBe("/calendar")
  expect(screen.queryByRole("button", { name: /^Notifications$|^通知$/ })).not.toBeInTheDocument()
})

test("local calendar event deletion uses the component confirmation dialog", async () => {
  stubWorkspaceApi()
  const user = userEvent.setup()
  const localEvent = {
    id: "local-event-1",
    summary: "Calendar QA event",
    description: "",
    location: "",
    startsAt: Math.floor(new Date("2026-07-28T18:30:00").getTime() / 1000),
    endsAt: Math.floor(new Date("2026-07-28T19:30:00").getTime() / 1000),
    allDay: false,
    createdAt: 1_700_000_000,
    updatedAt: 1_700_000_000,
  }
  vi.mocked(api.localCalendarEvents).mockResolvedValue([localEvent])
  const deleteEvent = vi.spyOn(api, "deleteLocalCalendarEvent").mockResolvedValue(undefined)
  renderWorkspace()

  await user.click(await screen.findByRole("radio", { name: /Calendar|日历/ }))
  await screen.findAllByText(localEvent.summary)
  await user.click(screen.getByRole("button", { name: /Edit local event|编辑本地事件/ }))
  await user.click(screen.getByRole("button", { name: /^Delete$|^删除$/ }))

  const dialog = await screen.findByRole("alertdialog", { name: /Delete local event|删除本地事件/ })
  await user.click(within(dialog).getByRole("button", { name: /^Delete$|^删除$/ }))

  expect(deleteEvent).toHaveBeenCalledWith(localEvent.id)
  await waitFor(() => expect(screen.queryByText(localEvent.summary)).not.toBeInTheDocument())
})

test("the account lock action guides users without a PIN to security settings", async () => {
  stubWorkspaceApi()
  const user = userEvent.setup()
  const lock = vi.spyOn(api, "lock")
  renderWorkspace({ ...session, user: { ...session.user, hasPin: false } })

  await user.click(await screen.findByRole("button", { name: /Profile and settings|个人资料与设置/ }))
  await user.click(await screen.findByRole("menuitem", { name: /Lock current session|锁定当前会话/ }))

  expect(lock).not.toHaveBeenCalled()
  expect(await screen.findByRole("tab", { name: /Security|安全/ })).toHaveAttribute("aria-selected", "true")
  await waitFor(() => expect(screen.getByLabelText(/Set personal PIN|设置个人 PIN/)).toHaveFocus())
})

test("tree navigation does not trigger global compose or message shortcuts", async () => {
  stubWorkspaceApi()
  const user = userEvent.setup()
  renderWorkspace()
  await screen.findByText("Project update")

  const inbox = screen.getByRole("treeitem", { name: /Inbox|收件箱/ })
  inbox.focus()
  await user.keyboard("{ArrowDown}c")

  expect(api.message).not.toHaveBeenCalled()
  expect(screen.queryByRole("region", { name: /Compose|写邮件/ })).not.toBeInTheDocument()
})

test("closing the compose workspace restores focus to its trigger", async () => {
  stubWorkspaceApi()
  const user = userEvent.setup()
  renderWorkspace()
  const compose = await screen.findByRole("button", { name: /Compose|写邮件/ })

  await user.click(compose)
  expect(await screen.findByRole("region", { name: /Compose|写邮件/ })).toBeInTheDocument()
  await user.keyboard("{Escape}")

  await waitFor(() => expect(screen.queryByRole("region", { name: /Compose|写邮件/ })).not.toBeInTheDocument())
  expect(compose).toHaveFocus()
})

test("mail accounts can be collapsed and compose has no stray shortcut glyph", async () => {
  stubWorkspaceApi()
  const user = userEvent.setup()
  renderWorkspace()
  await screen.findByText("Project update")

  expect(document.querySelector(".mail-navigation-compose kbd")).not.toBeInTheDocument()
  expect(screen.getByRole("treeitem", { name: /Work/ })).toBeInTheDocument()

  await user.click(screen.getByRole("button", { name: /Collapse mail account list|折叠邮件账户列表/ }))
  expect(screen.queryByRole("treeitem", { name: /Work/ })).not.toBeInTheDocument()
  await user.click(screen.getByRole("button", { name: /Expand mail account list|展开邮件账户列表/ }))
  expect(screen.getByRole("treeitem", { name: /Work/ })).toBeInTheDocument()
})

test("folder filtering reaches the existing messages API contract", async () => {
  stubWorkspaceApi()
  const user = userEvent.setup()
  renderWorkspace()
  await screen.findByText("Project update")
  vi.mocked(api.messages).mockClear()

  await user.click(screen.getByRole("radio", { name: /Unread|未读邮件/ }))
  await waitFor(() => expect(api.messages).toHaveBeenCalled())
  const params = vi.mocked(api.messages).mock.calls.at(-1)?.[0]
  expect(params?.get("unread")).toBe("true")
})

test("sync exposes a loading state and refreshes the mailbox", async () => {
  stubWorkspaceApi()
  let finishSync: (value: { inserted: number; syncedAt: number }) => void = () => {}
  vi.mocked(api.syncAccount).mockReturnValue(new Promise((resolve) => { finishSync = resolve }))
  const user = userEvent.setup()
  renderWorkspace()
  const sync = await screen.findByRole("button", { name: /^Sync$|^同步$/ })

  await user.click(sync)
  expect(await screen.findByRole("button", { name: /Syncing|同步中/ })).toBeDisabled()
  finishSync({ inserted: 2, syncedAt: 1_700_000_001 })
  await waitFor(() => expect(api.accounts).toHaveBeenCalledTimes(2))
  expect(await screen.findByRole("button", { name: /^Sync$|^同步$/ })).toBeEnabled()
})

test("sync with zero inserted messages uses a no-new-mail notice", async () => {
  stubWorkspaceApi()
  const user = userEvent.setup()
  renderWorkspace()
  await screen.findByText("Project update")

  await user.click(screen.getByRole("button", { name: /^Sync$|^同步$/ }))

  expect(await screen.findByText("No new mail right now")).toBeInTheDocument()
  expect(screen.queryByText(/0 new messages/)).not.toBeInTheDocument()
})

test("sync is unavailable while a delete request is pending", async () => {
  stubWorkspaceApi()
  let finishDelete: () => void = () => {}
  vi.mocked(api.deleteMessage).mockReturnValue(new Promise((resolve) => { finishDelete = resolve }))
  const user = userEvent.setup()
  renderWorkspace()
  await screen.findByText("Project update")

  await user.click(screen.getByText("Project update"))
  await screen.findByText("The full project update.")
  await user.click(screen.getByRole("button", { name: /^Delete$|^删除$/ }))

  expect(screen.getByRole("button", { name: /^Delete$|^删除$/ })).toBeDisabled()
  expect(screen.getByRole("button", { name: /^Sync$|^同步$/ })).toBeDisabled()

  finishDelete()
  await waitFor(() => expect(api.deleteMessage).toHaveBeenCalledWith(message.id))
})
