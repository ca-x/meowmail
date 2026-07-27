import type {
  CleanupRule, CleanupRuleInput, MailAccount, RuleAction, RuleActionKind,
  RuleCondition, RuleField, RuleOperator,
} from "../../app/types"
import type { MessageKey } from "../../i18n/messages"

export type RuleDraft = CleanupRuleInput & { id?: string }

export const defaultCondition: RuleCondition = { field: "sender", operator: "containsAny", values: [""] }
export const defaultAction: RuleAction = { kind: "deleteLocal", value: null }
export const fieldOptions: RuleField[] = ["sender", "senderDomain", "recipient", "cc", "recipientOrCc", "subject", "body", "attachmentName", "messageSize", "receivedAt", "ageDays", "hasAttachment"]
export const actionOptions: RuleActionKind[] = ["deleteLocal", "deleteServer", "markRead", "markUnread", "star", "unstar", "forward", "autoReply"]
export const numericFields = new Set<RuleField>(["messageSize", "receivedAt", "ageDays"])

export function emptyRule(position: number): RuleDraft {
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

export function operatorsFor(field: RuleField): RuleOperator[] {
  if (field === "hasAttachment") return ["isTrue", "isFalse"]
  if (numericFields.has(field)) return field === "receivedAt" ? ["before", "after"] : ["greaterThan", "lessThan", "equals"]
  return ["containsAny", "containsAll", "equals", "notContains"]
}

export function normalizeConditionForField(condition: RuleCondition): RuleCondition {
  const operators = operatorsFor(condition.field)
  return {
    ...condition,
    operator: operators.includes(condition.operator) ? condition.operator : operators[0],
    values: condition.field === "hasAttachment" ? [] : condition.values.length ? condition.values : [""],
  }
}

export function toRuleInput(rule: CleanupRule): CleanupRuleInput {
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

export function ruleSummary(rule: CleanupRule, accounts: MailAccount[], t: (key: MessageKey, values?: Record<string, string | number>) => string) {
  const account = rule.accountId ? accounts.find((item) => item.id === rule.accountId)?.displayName || t("oneAccount") : t("allAccounts")
  return `${account} · ${rule.matchMode === "all" ? t("allConditions") : t("anyCondition")} · ${t("ruleConditionCount", { count: rule.conditions.length })} · ${t("ruleActionCount", { count: rule.actions.length })}`
}

export function fieldKey(field: RuleField): MessageKey {
  return ({ sender: "ruleFieldSender", senderDomain: "ruleFieldSenderDomain", recipient: "ruleFieldRecipient", cc: "ruleFieldCc", recipientOrCc: "ruleFieldRecipientOrCc", subject: "ruleFieldSubject", body: "ruleFieldBody", attachmentName: "ruleFieldAttachmentName", messageSize: "ruleFieldMessageSize", receivedAt: "ruleFieldReceivedAt", ageDays: "ruleFieldAgeDays", hasAttachment: "ruleFieldHasAttachment" } as const)[field]
}

export function operatorKey(operator: RuleOperator): MessageKey {
  return ({ containsAny: "ruleOperatorContainsAny", containsAll: "ruleOperatorContainsAll", equals: "ruleOperatorEquals", notContains: "ruleOperatorNotContains", greaterThan: "ruleOperatorGreaterThan", lessThan: "ruleOperatorLessThan", before: "ruleOperatorBefore", after: "ruleOperatorAfter", isTrue: "ruleOperatorIsTrue", isFalse: "ruleOperatorIsFalse" } as const)[operator]
}

export function actionKey(kind: RuleActionKind): MessageKey {
  return ({ deleteLocal: "ruleActionDeleteLocal", deleteServer: "ruleActionDeleteServer", markRead: "ruleActionMarkRead", markUnread: "ruleActionMarkUnread", star: "ruleActionStar", unstar: "ruleActionUnstar", forward: "ruleActionForward", autoReply: "ruleActionAutoReply" } as const)[kind]
}

export function conditionPlaceholder(field: RuleField, t: (key: MessageKey) => string) {
  if (field === "messageSize") return t("ruleMessageSizePlaceholder")
  return t("ruleValuesPlaceholder")
}

export function epochToLocalDateTime(value?: string) {
  if (!value) return ""
  const seconds = Number(value)
  if (!Number.isFinite(seconds)) return ""
  const date = new Date(seconds * 1_000)
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000)
  return local.toISOString().slice(0, 16)
}

export function localDateTimeToEpoch(value: string) {
  if (!value) return ""
  const timestamp = new Date(value).getTime()
  return Number.isFinite(timestamp) ? String(Math.floor(timestamp / 1_000)) : ""
}
