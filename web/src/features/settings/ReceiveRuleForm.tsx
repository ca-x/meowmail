import { Banner } from "@astryxdesign/core/Banner"
import { Button } from "@astryxdesign/core/Button"
import { Card } from "@astryxdesign/core/Card"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { IconButton } from "@astryxdesign/core/IconButton"
import { NumberInput } from "@astryxdesign/core/NumberInput"
import { Selector } from "@astryxdesign/core/Selector"
import { TextInput } from "@astryxdesign/core/TextInput"
import { Plus, Save, ServerOff, X } from "lucide-react"
import type { FormEvent } from "react"

import type { MailAccount, RuleActionKind, RuleCondition, RuleField, RuleOperator } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import {
  actionKey, actionOptions, conditionPlaceholder, defaultAction, defaultCondition,
  epochToLocalDateTime, fieldKey, fieldOptions, localDateTimeToEpoch, normalizeConditionForField,
  numericFields, operatorKey, operatorsFor, type RuleDraft,
} from "./receiveRuleUtils"

export function ReceiveRuleForm({ draft, accounts, busy, onChange, onConditionChange, onCancel, onSubmit }: {
  draft: RuleDraft
  accounts: MailAccount[]
  busy: boolean
  onChange: (draft: RuleDraft) => void
  onConditionChange: (index: number, condition: RuleCondition) => void
  onCancel: () => void
  onSubmit: (event: FormEvent) => void
}) {
  const { t } = useI18n()
  const hasServerDelete = draft.actions.some((action) => action.kind === "deleteServer")

  function updateCondition(index: number, update: Partial<RuleCondition>) {
    onConditionChange(index, normalizeConditionForField({ ...draft.conditions[index], ...update }))
  }

  return (
    <Card className="receive-rule-editor" padding={4}>
      <form aria-label={draft.id ? t("editReceiveRule") : t("newReceiveRule")} onSubmit={onSubmit}>
        <div className="receive-rule-editor-heading">
          <strong>{draft.id ? t("editReceiveRule") : t("newReceiveRule")}</strong>
          <IconButton label={t("close")} icon={<X aria-hidden="true" />} variant="ghost" size="sm" onClick={onCancel} />
        </div>
        <div className="receive-rule-basics">
          <TextInput label={`${t("ruleName")} · ${t("required")}`} value={draft.name} onChange={(name) => onChange({ ...draft, name })} placeholder={t("ruleNamePlaceholder")} hasAutoFocus width="100%" />
          <Selector label={t("mailAccountScope")} value={draft.accountId || "all"} onChange={(accountId) => onChange({ ...draft, accountId: accountId === "all" ? null : accountId })} options={[{ value: "all", label: t("allAccounts") }, ...accounts.map((account) => ({ value: account.id, label: account.displayName }))]} width="100%" />
          <Selector label={t("conditionMatchMode")} value={draft.matchMode} onChange={(matchMode) => onChange({ ...draft, matchMode: matchMode as RuleDraft["matchMode"] })} options={[{ value: "all", label: t("allConditions") }, { value: "any", label: t("anyCondition") }]} width="100%" />
        </div>

        <fieldset className="receive-rule-builder">
          <legend>{t("whenNewMailArrives")}</legend>
          <div className="receive-rule-builder-list">
            {draft.conditions.map((condition, index) => (
              <div className="receive-rule-builder-row" key={index}>
                <Selector label={t("ruleField")} isLabelHidden value={condition.field} onChange={(field) => updateCondition(index, { field: field as RuleField })} options={fieldOptions.map((field) => ({ value: field, label: t(fieldKey(field)) }))} width="100%" />
                <Selector label={t("ruleOperator")} isLabelHidden value={condition.operator} onChange={(operator) => updateCondition(index, { operator: operator as RuleOperator })} options={operatorsFor(condition.field).map((operator) => ({ value: operator, label: t(operatorKey(operator)) }))} width="100%" />
                <ConditionValue condition={condition} onChange={(values) => updateCondition(index, { values })} />
                <IconButton label={t("removeCondition")} icon={<X aria-hidden="true" />} variant="ghost" size="sm" isDisabled={draft.conditions.length === 1} onClick={() => onChange({ ...draft, conditions: draft.conditions.filter((_, current) => current !== index) })} />
              </div>
            ))}
          </div>
          <Button label={t("addCondition")} icon={<Plus aria-hidden="true" />} variant="ghost" size="sm" onClick={() => onChange({ ...draft, conditions: [...draft.conditions, { ...defaultCondition, values: [""] }] })} />
        </fieldset>

        <fieldset className="receive-rule-builder">
          <legend>{t("performActions")}</legend>
          <div className="receive-rule-builder-list">
            {draft.actions.map((action, index) => (
              <div className="receive-rule-action-row" key={index}>
                <Selector label={t("performActions")} isLabelHidden value={action.kind} onChange={(kind) => onChange({ ...draft, actions: draft.actions.map((item, current) => current === index ? { kind: kind as RuleActionKind, value: null } : item) })} options={actionOptions.map((kind) => ({ value: kind, label: t(actionKey(kind)) }))} width="100%" />
                {(action.kind === "forward" || action.kind === "autoReply") ? (
                  <TextInput label={action.kind === "forward" ? t("forwardToAddress") : t("automaticReplyContent")} isLabelHidden type={action.kind === "forward" ? "email" : "text"} value={action.value || ""} onChange={(value) => onChange({ ...draft, actions: draft.actions.map((item, current) => current === index ? { ...item, value } : item) })} placeholder={action.kind === "forward" ? t("forwardToAddressPlaceholder") : t("automaticReplyContentPlaceholder")} width="100%" />
                ) : <span className="receive-rule-action-spacer" />}
                <IconButton label={t("removeAction")} icon={<X aria-hidden="true" />} variant="ghost" size="sm" isDisabled={draft.actions.length === 1} onClick={() => onChange({ ...draft, actions: draft.actions.filter((_, current) => current !== index) })} />
              </div>
            ))}
          </div>
          <Button label={t("addAction")} icon={<Plus aria-hidden="true" />} variant="ghost" size="sm" onClick={() => onChange({ ...draft, actions: [...draft.actions, { ...defaultAction }] })} />
        </fieldset>

        {hasServerDelete && <Banner status="warning" title={t("serverDeleteWarning")} icon={<ServerOff aria-hidden="true" />} />}
        <div className="receive-rule-footer-options">
          <CheckboxInput label={t("enableRule")} value={draft.enabled} onChange={(enabled) => onChange({ ...draft, enabled })} />
          <CheckboxInput label={t("stopProcessingRules")} value={draft.stopProcessing} onChange={(stopProcessing) => onChange({ ...draft, stopProcessing })} />
        </div>
        <div className="receive-rule-editor-actions">
          <Button label={t("cancel")} variant="ghost" isDisabled={busy} onClick={onCancel} />
          <Button label={t("saveRule")} icon={<Save aria-hidden="true" />} variant="primary" type="submit" isLoading={busy} isDisabled={busy || !draft.name.trim()} />
        </div>
      </form>
    </Card>
  )
}

function ConditionValue({ condition, onChange }: {
  condition: RuleCondition
  onChange: (values: string[]) => void
}) {
  const { t } = useI18n()
  if (condition.field === "hasAttachment") return <span className="receive-rule-value-empty" aria-hidden="true" />
  if (condition.field === "receivedAt") {
    return <label className="receive-rule-native-field"><span>{t("ruleValues")}</span><input type="datetime-local" value={epochToLocalDateTime(condition.values[0])} onChange={(event) => onChange([localDateTimeToEpoch(event.target.value)])} /></label>
  }
  if (numericFields.has(condition.field)) {
    const parsed = Number(condition.values[0])
    return <NumberInput label={t("ruleValues")} isLabelHidden value={Number.isFinite(parsed) ? parsed : null} onChange={(value) => onChange([value === null ? "" : String(value)])} placeholder={conditionPlaceholder(condition.field, t)} min={0} isIntegerOnly hasClear width="100%" />
  }
  return <TextInput label={t("ruleValues")} isLabelHidden value={condition.values.join(", ")} onChange={(value) => onChange(value.split(/[,，]/))} placeholder={conditionPlaceholder(condition.field, t)} width="100%" />
}
