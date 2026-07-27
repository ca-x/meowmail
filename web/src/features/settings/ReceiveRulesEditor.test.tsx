import { render, screen, waitFor, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, test, vi } from "vitest"

import { api } from "../../app/api"
import type { CleanupRule, MailAccount } from "../../app/types"
import { Providers } from "../../app/Providers"
import { ReceiveRulesEditor } from "./ReceiveRulesEditor"

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

const rule: CleanupRule = {
  id: "rule-1",
  accountId: account.id,
  name: "Remove old alerts",
  matchMode: "all",
  conditions: [{ field: "sender", operator: "containsAny", values: ["alerts@example.com"] }],
  actions: [{ kind: "deleteServer", value: null }],
  position: 0,
  stopProcessing: true,
  senderContains: null,
  subjectContains: null,
  bodyContains: null,
  olderThanDays: null,
  deleteFromServer: true,
  enabled: true,
  createdAt: 1,
  updatedAt: 1,
}

test("edits a server-delete rule with an explicit warning and preserves its structured actions", async () => {
  const user = userEvent.setup()
  const update = vi.spyOn(api, "updateCleanupRule").mockResolvedValue(rule)
  vi.spyOn(api, "cleanupRules").mockResolvedValue([{ ...rule, name: "Remove archived alerts" }])
  const onRulesChanged = vi.fn()

  render(
    <Providers>
      <ReceiveRulesEditor rules={[rule]} accounts={[account]} onRulesChanged={onRulesChanged} onNotice={vi.fn()} />
    </Providers>,
  )

  expect(screen.getByText("Deletes from server")).toBeInTheDocument()
  await user.click(screen.getByRole("button", { name: "Edit" }))
  const form = screen.getByRole("form", { name: "Edit incoming mail rule" })
  expect(within(form).getByText(/cannot be restored/i)).toBeInTheDocument()
  expect(within(form).getByRole("checkbox", { name: /Enable this rule/i })).toBeChecked()
  expect(within(form).getByRole("checkbox", { name: /Stop processing later rules/i })).toBeChecked()

  const name = within(form).getByRole("textbox", { name: /^Rule name/ })
  await user.clear(name)
  await user.type(name, "Remove archived alerts")
  await user.click(within(form).getByRole("button", { name: "Save rule" }))

  await waitFor(() => expect(update).toHaveBeenCalledOnce())
  expect(update).toHaveBeenCalledWith(rule.id, expect.objectContaining({
    name: "Remove archived alerts",
    conditions: rule.conditions,
    actions: rule.actions,
    deleteFromServer: true,
  }))
  await waitFor(() => expect(onRulesChanged).toHaveBeenCalledWith([{ ...rule, name: "Remove archived alerts" }]))
})
