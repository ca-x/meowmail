import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { Card } from "@astryxdesign/core/Card"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Switch } from "@astryxdesign/core/Switch"
import { ArrowDown, ArrowUp, Pencil, Plus, Trash2 } from "lucide-react"
import { useState, type FormEvent } from "react"

import { api } from "../../app/api"
import type { CleanupRule, CleanupRuleInput, MailAccount, RuleCondition } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"
import { ReceiveRuleForm } from "./ReceiveRuleForm"
import { emptyRule, ruleSummary, toRuleInput, type RuleDraft } from "./receiveRuleUtils"

export function ReceiveRulesEditor({ rules, accounts, onRulesChanged, onNotice }: {
  rules: CleanupRule[]
  accounts: MailAccount[]
  onRulesChanged: (rules: CleanupRule[]) => void
  onNotice: (key: MessageKey, error?: boolean) => void
}) {
  const { t } = useI18n()
  const deleteDialog = useImperativeConfirmDialog()
  const [draft, setDraft] = useState<RuleDraft | null>(null)
  const [busy, setBusy] = useState<string | null>(null)

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
    setBusy("save")
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
      setBusy(null)
    }
  }

  async function requestRemove(rule: CleanupRule) {
    const confirmed = await deleteDialog.confirm({
      title: t("deleteRuleTitle"),
      description: t("deleteRuleDescription", { rule: rule.name }),
      cancelLabel: t("cancel"),
      actionLabel: t("delete"),
      actionVariant: "destructive",
    })
    if (!confirmed) return
    setBusy(rule.id)
    try {
      await api.deleteCleanupRule(rule.id)
      onRulesChanged(rules.filter((item) => item.id !== rule.id))
      if (draft?.id === rule.id) setDraft(null)
      onNotice("cleanupRuleDeleted")
    } catch {
      onNotice("genericError", true)
    } finally {
      setBusy(null)
    }
  }

  async function move(index: number, direction: -1 | 1) {
    const target = index + direction
    if (target < 0 || target >= rules.length) return
    const next = [...rules]
    ;[next[index], next[target]] = [next[target], next[index]]
    setBusy("reorder")
    onRulesChanged(next)
    try {
      onRulesChanged(await api.reorderCleanupRules(next.map((rule) => rule.id)))
    } catch {
      onRulesChanged(rules)
      onNotice("genericError", true)
    } finally {
      setBusy(null)
    }
  }

  async function toggleRule(rule: CleanupRule, enabled: boolean) {
    setBusy(rule.id)
    try {
      await api.updateCleanupRule(rule.id, { ...toRuleInput(rule), enabled })
      onRulesChanged(await api.cleanupRules())
    } catch {
      onNotice("genericError", true)
    } finally {
      setBusy(null)
    }
  }

  function updateCondition(index: number, condition: RuleCondition) {
    if (!draft) return
    setDraft({ ...draft, conditions: draft.conditions.map((item, current) => current === index ? condition : item) })
  }

  return (
    <div className="receive-rules">
      <div className="receive-rules-heading">
        <div><strong>{t("receiveRules")}</strong><small>{t("receiveRulesDescription")}</small></div>
        <Button label={t("addRule")} icon={<Plus aria-hidden="true" />} variant="secondary" size="sm" onClick={() => edit()} />
      </div>
      <Card className="receive-rule-list" padding={0}>
        {!rules.length && <p className="settings-empty-copy">{t("noCleanupRules")}</p>}
        {rules.map((rule, index) => (
          <div className="receive-rule-row" key={rule.id}>
            <Switch label={rule.enabled ? t("disableRule") : t("enableRule")} isLabelHidden value={rule.enabled} onChange={(enabled) => void toggleRule(rule, enabled)} isLoading={busy === rule.id} isDisabled={Boolean(busy)} />
            <div className="receive-rule-summary"><strong>{rule.name}</strong><small>{ruleSummary(rule, accounts, t)}</small></div>
            <div className="receive-rule-badges">
              {rule.actions.some((action) => action.kind === "deleteServer") && <Badge variant="error" label={t("serverDelete")} />}
            </div>
            <div className="receive-rule-order-actions">
              <IconButton label={t("moveRuleUp")} icon={<ArrowUp aria-hidden="true" />} variant="ghost" size="sm" isDisabled={Boolean(busy) || index === 0} onClick={() => void move(index, -1)} />
              <IconButton label={t("moveRuleDown")} icon={<ArrowDown aria-hidden="true" />} variant="ghost" size="sm" isDisabled={Boolean(busy) || index === rules.length - 1} onClick={() => void move(index, 1)} />
            </div>
            <IconButton label={t("edit")} icon={<Pencil aria-hidden="true" />} variant="ghost" size="sm" isDisabled={Boolean(busy)} onClick={() => edit(rule)} />
            <IconButton label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="ghost" size="sm" isDisabled={Boolean(busy)} onClick={() => void requestRemove(rule)} />
          </div>
        ))}
      </Card>

      {draft && (
        <ReceiveRuleForm
          draft={draft}
          accounts={accounts}
          busy={busy === "save"}
          onChange={setDraft}
          onConditionChange={updateCondition}
          onCancel={() => setDraft(null)}
          onSubmit={save}
        />
      )}
      {deleteDialog.element}
    </div>
  )
}
