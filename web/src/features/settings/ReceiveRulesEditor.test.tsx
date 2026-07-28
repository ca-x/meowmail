import { fireEvent, render, screen, waitFor, within } from "@testing-library/react"
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

test("saves a received-time filter from the framework date-time input", async () => {
  const user = userEvent.setup()
  const create = vi.spyOn(api, "createCleanupRule").mockResolvedValue({
    ...rule,
    id: "rule-2",
    name: "Recent project mail",
    conditions: [{ field: "receivedAt", operator: "after", values: ["1704067200"] }],
  })
  vi.spyOn(api, "cleanupRules").mockResolvedValue([])

  render(
    <Providers>
      <ReceiveRulesEditor rules={[]} accounts={[account]} onRulesChanged={vi.fn()} onNotice={vi.fn()} />
    </Providers>,
  )

  await user.click(screen.getByRole("button", { name: "Add rule" }))
  const form = screen.getByRole("form", { name: "New incoming mail rule" })
  await user.type(within(form).getByRole("textbox", { name: /^Rule name/ }), "Recent project mail")
  await user.click(within(form).getByRole("combobox", { name: "Condition field" }))
  await user.click(await screen.findByRole("option", { name: "Received time" }))
  await user.click(within(form).getByRole("combobox", { name: "Match operator" }))
  await user.click(await screen.findByRole("option", { name: "After" }))
  fireEvent.change(within(form).getByLabelText("Condition values"), { target: { value: "01/01/2024" } })
  fireEvent.change(within(form).getByLabelText("Condition values time"), { target: { value: "00:00" } })
  await user.click(within(form).getByRole("button", { name: "Save rule" }))

  await waitFor(() => expect(create).toHaveBeenCalledWith(expect.objectContaining({
    name: "Recent project mail",
    conditions: [{ field: "receivedAt", operator: "after", values: [localEpoch("2024-01-01T00:00")] }],
  })))
})

function localEpoch(value: string) {
  return String(Math.floor(new Date(value).getTime() / 1_000))
}
