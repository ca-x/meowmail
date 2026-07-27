import { useState, type FormEvent } from "react"
import { ArrowDown, ArrowUp, Pencil, Plus, Save, Trash2, X } from "lucide-react"

import { api } from "../../app/api"
import type {
  CleanupRule, CleanupRuleInput, MailAccount, RuleAction, RuleActionKind,
  RuleCondition, RuleField, RuleOperator,
} from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"

type RuleDraft = CleanupRuleInput & { id?: string }

const defaultCondition: RuleCondition = { field: "sender", operator: "containsAny", values: [""] }
const defaultAction: RuleAction = { kind: "deleteLocal", value: null }

function emptyRule(position: number): RuleDraft {
  return {
    accountId: null,
    name: "",
    matchMode: "all",
    conditions: [{ ...defaultCondition, values: [""] }],
    actions: [{ ...defaultAction }],
    position,
    stopProcessing: false,
    senderContains: null,
    subjectContains: null,
    bodyContains: null,
    olderThanDays: null,
    deleteFromServer: false,
    enabled: true,
  }
}

export function ReceiveRulesEditor({ rules, accounts, onRulesChanged, onNotice }: {
  rules: CleanupRule[]
  accounts: MailAccount[]
  onRulesChanged: (rules: CleanupRule[]) => void
  onNotice: (key: MessageKey, error?: boolean) => void
}) {
  const { t } = useI18n()
  const [draft, setDraft] = useState<RuleDraft | null>(null)
  const [busy, setBusy] = useState(false)

  function edit(rule?: CleanupRule) {
    setDraft(rule ? {
      id: rule.id,
      accountId: rule.accountId || null,
      name: rule.name,
      matchMode: rule.matchMode,
      conditions: rule.conditions.map((condition) => ({ ...condition, values: [...condition.values] })),
      actions: rule.actions.map((action) => ({ ...action })),
      position: rule.position,
      stopProcessing: rule.stopProcessing,
      senderContains: null,
      subjectContains: null,
      bodyContains: null,
      olderThanDays: null,
      deleteFromServer: rule.actions.some((action) => action.kind === "deleteServer"),
      enabled: rule.enabled,
    } : emptyRule(rules.length))
  }

  async function save(event: FormEvent) {
    event.preventDefault()
    if (!draft) return
    setBusy(true)
    const input: CleanupRuleInput = {
      ...draft,
      accountId: draft.accountId || null,
      conditions: draft.conditions.map((condition) => ({
        ...condition,
        values: condition.values.map((value) => value.trim()).filter(Boolean),
      })),
      actions: draft.actions.map((action) => ({ ...action, value: action.value?.trim() || null })),
      deleteFromServer: draft.actions.some((action) => action.kind === "deleteServer"),
    }
    try {
      if (draft.id) await api.updateCleanupRule(draft.id, input)
      else await api.createCleanupRule(input)
      onRulesChanged(await api.cleanupRules())
      setDraft(null)
      onNotice("cleanupRuleSaved")
    } catch {
      onNotice("cleanupRuleInvalid", true)
    } finally {
      setBusy(false)
    }
  }

  async function remove(id: string) {
    setBusy(true)
    try {
      await api.deleteCleanupRule(id)
      onRulesChanged(rules.filter((rule) => rule.id !== id))
      if (draft?.id === id) setDraft(null)
      onNotice("cleanupRuleDeleted")
    } catch {
      onNotice("genericError", true)
    } finally {
      setBusy(false)
    }
  }

  async function move(index: number, direction: -1 | 1) {
    const target = index + direction
    if (target < 0 || target >= rules.length) return
    const next = [...rules]
    ;[next[index], next[target]] = [next[target], next[index]]
    onRulesChanged(next)
    try {
      onRulesChanged(await api.reorderCleanupRules(next.map((rule) => rule.id)))
    } catch {
      onRulesChanged(rules)
      onNotice("genericError", true)
    }
  }

  async function toggleRule(rule: CleanupRule, enabled: boolean) {
    try {
      await api.updateCleanupRule(rule.id, { ...toInput(rule), enabled })
      onRulesChanged(await api.cleanupRules())
    } catch {
      onNotice("genericError", true)
    }
  }

  function updateCondition(index: number, update: Partial<RuleCondition>) {
    if (!draft) return
    const conditions = draft.conditions.map((condition, current) => current === index
      ? normalizeConditionForField({ ...condition, ...update })
      : condition)
    setDraft({ ...draft, conditions })
  }

  return (
    <div className="receive-rules">
      <div className="cleanup-heading"><div><strong>{t("receiveRules")}</strong><small>{t("receiveRulesDescription")}</small></div><button className="quiet-button" type="button" onClick={() => edit()}><Plus size={14} />{t("addRule")}</button></div>
      <div className="cleanup-list rule-list">
        {!rules.length && <p className="empty-inline">{t("noCleanupRules")}</p>}
        {rules.map((rule, index) => (
          <div className="cleanup-rule-row structured-rule-row" key={rule.id}>
            <label className="mini-toggle" aria-label={rule.enabled ? t("disableRule") : t("enableRule")}><input type="checkbox" checked={rule.enabled} onChange={(event) => void toggleRule(rule, event.target.checked)} /><span /></label>
            <div><strong>{rule.name}</strong><small>{ruleSummary(rule, accounts, t)}</small></div>
            <div className="rule-order-actions"><button className="icon-button small" type="button" disabled={index === 0} onClick={() => void move(index, -1)} aria-label={t("moveRuleUp")}><ArrowUp size={13} /></button><button className="icon-button small" type="button" disabled={index === rules.length - 1} onClick={() => void move(index, 1)} aria-label={t("moveRuleDown")}><ArrowDown size={13} /></button></div>
            {rule.actions.some((action) => action.kind === "deleteServer") && <span className="danger-chip">{t("serverDelete")}</span>}
            <button className="icon-button small" type="button" onClick={() => edit(rule)} aria-label={t("edit")}><Pencil size={14} /></button>
            <button className="icon-button small danger-text" type="button" onClick={() => void remove(rule.id)} aria-label={t("delete")}><Trash2 size={14} /></button>
          </div>
        ))}
      </div>

      {draft && (
        <form className="cleanup-editor structured-rule-editor" onSubmit={save}>
          <div className="cleanup-editor-heading"><strong>{draft.id ? t("editReceiveRule") : t("newReceiveRule")}</strong><button className="icon-button small" type="button" onClick={() => setDraft(null)} aria-label={t("close")}><X size={14} /></button></div>
          <div className="rule-basics">
            <label className="form-field"><span>{t("ruleName")}</span><input autoFocus value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} placeholder={t("ruleNamePlaceholder")} /></label>
            <label className="form-field"><span>{t("mailAccountScope")}</span><select value={draft.accountId || ""} onChange={(event) => setDraft({ ...draft, accountId: event.target.value || null })}><option value="">{t("allAccounts")}</option>{accounts.map((account) => <option key={account.id} value={account.id}>{account.displayName}</option>)}</select></label>
            <label className="form-field"><span>{t("conditionMatchMode")}</span><select value={draft.matchMode} onChange={(event) => setDraft({ ...draft, matchMode: event.target.value as "all" | "any" })}><option value="all">{t("allConditions")}</option><option value="any">{t("anyCondition")}</option></select></label>
          </div>

          <fieldset className="rule-builder"><legend>{t("whenNewMailArrives")}</legend>{draft.conditions.map((condition, index) => <div className="rule-builder-row" key={index}><select aria-label={t("ruleField")} value={condition.field} onChange={(event) => updateCondition(index, { field: event.target.value as RuleField })}>{fieldOptions.map((field) => <option value={field} key={field}>{t(fieldKey(field))}</option>)}</select><select aria-label={t("ruleOperator")} value={condition.operator} onChange={(event) => updateCondition(index, { operator: event.target.value as RuleOperator })}>{operatorsFor(condition.field).map((operator) => <option key={operator} value={operator}>{t(operatorKey(operator))}</option>)}</select>{condition.field !== "hasAttachment" && <input aria-label={t("ruleValues")} type={condition.field === "receivedAt" ? "datetime-local" : numericFields.has(condition.field) ? "number" : "text"} value={condition.field === "receivedAt" ? epochToLocalDateTime(condition.values[0]) : condition.values.join(", ")} onChange={(event) => updateCondition(index, { values: condition.field === "receivedAt" ? [localDateTimeToEpoch(event.target.value)] : numericFields.has(condition.field) ? [event.target.value] : event.target.value.split(/[,，]/) })} placeholder={conditionPlaceholder(condition.field, t)} />}<button className="icon-button small" type="button" disabled={draft.conditions.length === 1} onClick={() => setDraft({ ...draft, conditions: draft.conditions.filter((_, current) => current !== index) })} aria-label={t("removeCondition")}><X size={13} /></button></div>)}<button className="quiet-button add-rule-line" type="button" onClick={() => setDraft({ ...draft, conditions: [...draft.conditions, { ...defaultCondition, values: [""] }] })}><Plus size={13} />{t("addCondition")}</button></fieldset>

          <fieldset className="rule-builder"><legend>{t("performActions")}</legend>{draft.actions.map((action, index) => <div className="rule-builder-row action-row" key={index}><select value={action.kind} onChange={(event) => setDraft({ ...draft, actions: draft.actions.map((item, current) => current === index ? { kind: event.target.value as RuleActionKind, value: null } : item) })}>{actionOptions.map((kind) => <option key={kind} value={kind}>{t(actionKey(kind))}</option>)}</select>{(action.kind === "forward" || action.kind === "autoReply") && <input type={action.kind === "forward" ? "email" : "text"} value={action.value || ""} onChange={(event) => setDraft({ ...draft, actions: draft.actions.map((item, current) => current === index ? { ...item, value: event.target.value } : item) })} placeholder={action.kind === "forward" ? t("forwardToAddressPlaceholder") : t("automaticReplyContentPlaceholder")} />}<button className="icon-button small" type="button" disabled={draft.actions.length === 1} onClick={() => setDraft({ ...draft, actions: draft.actions.filter((_, current) => current !== index) })} aria-label={t("removeAction")}><X size={13} /></button></div>)}<button className="quiet-button add-rule-line" type="button" onClick={() => setDraft({ ...draft, actions: [...draft.actions, { ...defaultAction }] })}><Plus size={13} />{t("addAction")}</button></fieldset>

          <div className="rule-footer-options"><label className="check-row inline-check"><input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} /><span className="custom-check">✓</span>{t("enableRule")}</label><label className="check-row inline-check"><input type="checkbox" checked={draft.stopProcessing} onChange={(event) => setDraft({ ...draft, stopProcessing: event.target.checked })} /><span className="custom-check">✓</span>{t("stopProcessingRules")}</label></div>
          <div className="editor-actions"><button className="quiet-button" type="button" onClick={() => setDraft(null)}>{t("cancel")}</button><button className="primary-button" type="submit" disabled={busy}><Save size={14} />{t("saveRule")}</button></div>
        </form>
      )}
    </div>
  )
}

const fieldOptions: RuleField[] = ["sender", "senderDomain", "recipient", "cc", "recipientOrCc", "subject", "body", "attachmentName", "messageSize", "receivedAt", "ageDays", "hasAttachment"]
const actionOptions: RuleActionKind[] = ["deleteLocal", "deleteServer", "markRead", "markUnread", "star", "unstar", "forward", "autoReply"]
const numericFields = new Set<RuleField>(["messageSize", "receivedAt", "ageDays"])

function operatorsFor(field: RuleField): RuleOperator[] {
  if (field === "hasAttachment") return ["isTrue", "isFalse"]
  if (numericFields.has(field)) return field === "receivedAt" ? ["before", "after"] : ["greaterThan", "lessThan", "equals"]
  return ["containsAny", "containsAll", "equals", "notContains"]
}

function normalizeConditionForField(condition: RuleCondition): RuleCondition {
  const operators = operatorsFor(condition.field)
  return {
    ...condition,
    operator: operators.includes(condition.operator) ? condition.operator : operators[0],
    values: condition.field === "hasAttachment" ? [] : condition.values.length ? condition.values : [""],
  }
}

function toInput(rule: CleanupRule): CleanupRuleInput {
  return {
    accountId: rule.accountId || null,
    name: rule.name,
    matchMode: rule.matchMode,
    conditions: rule.conditions,
    actions: rule.actions,
    position: rule.position,
    stopProcessing: rule.stopProcessing,
    senderContains: null,
    subjectContains: null,
    bodyContains: null,
    olderThanDays: null,
    deleteFromServer: rule.actions.some((action) => action.kind === "deleteServer"),
    enabled: rule.enabled,
  }
}

function ruleSummary(rule: CleanupRule, accounts: MailAccount[], t: (key: MessageKey, values?: Record<string, string | number>) => string) {
  const account = rule.accountId ? accounts.find((item) => item.id === rule.accountId)?.displayName || t("oneAccount") : t("allAccounts")
  return `${account} · ${rule.matchMode === "all" ? t("allConditions") : t("anyCondition")} · ${t("ruleConditionCount", { count: rule.conditions.length })} · ${t("ruleActionCount", { count: rule.actions.length })}`
}

function fieldKey(field: RuleField): MessageKey {
  return ({ sender: "ruleFieldSender", senderDomain: "ruleFieldSenderDomain", recipient: "ruleFieldRecipient", cc: "ruleFieldCc", recipientOrCc: "ruleFieldRecipientOrCc", subject: "ruleFieldSubject", body: "ruleFieldBody", attachmentName: "ruleFieldAttachmentName", messageSize: "ruleFieldMessageSize", receivedAt: "ruleFieldReceivedAt", ageDays: "ruleFieldAgeDays", hasAttachment: "ruleFieldHasAttachment" } as const)[field]
}

function operatorKey(operator: RuleOperator): MessageKey {
  return ({ containsAny: "ruleOperatorContainsAny", containsAll: "ruleOperatorContainsAll", equals: "ruleOperatorEquals", notContains: "ruleOperatorNotContains", greaterThan: "ruleOperatorGreaterThan", lessThan: "ruleOperatorLessThan", before: "ruleOperatorBefore", after: "ruleOperatorAfter", isTrue: "ruleOperatorIsTrue", isFalse: "ruleOperatorIsFalse" } as const)[operator]
}

function actionKey(kind: RuleActionKind): MessageKey {
  return ({ deleteLocal: "ruleActionDeleteLocal", deleteServer: "ruleActionDeleteServer", markRead: "ruleActionMarkRead", markUnread: "ruleActionMarkUnread", star: "ruleActionStar", unstar: "ruleActionUnstar", forward: "ruleActionForward", autoReply: "ruleActionAutoReply" } as const)[kind]
}

function conditionPlaceholder(field: RuleField, t: (key: MessageKey) => string) {
  if (field === "messageSize") return t("ruleMessageSizePlaceholder")
  return t("ruleValuesPlaceholder")
}

function epochToLocalDateTime(value?: string) {
  if (!value) return ""
  const seconds = Number(value)
  if (!Number.isFinite(seconds)) return ""
  const date = new Date(seconds * 1_000)
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000)
  return local.toISOString().slice(0, 16)
}

function localDateTimeToEpoch(value: string) {
  if (!value) return ""
  const timestamp = new Date(value).getTime()
  return Number.isFinite(timestamp) ? String(Math.floor(timestamp / 1_000)) : ""
}
