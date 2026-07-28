import { Badge } from "@astryxdesign/core/Badge"
import { Banner } from "@astryxdesign/core/Banner"
import { Button } from "@astryxdesign/core/Button"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { DialogHeader, useImperativeDialog } from "@astryxdesign/core/Dialog"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Layout, LayoutContent, LayoutFooter } from "@astryxdesign/core/Layout"
import { List } from "@astryxdesign/core/List"
import { MultiSelector } from "@astryxdesign/core/MultiSelector"
import { NumberInput } from "@astryxdesign/core/NumberInput"
import { Selector } from "@astryxdesign/core/Selector"
import { Switch } from "@astryxdesign/core/Switch"
import { TextArea } from "@astryxdesign/core/TextArea"
import { TextInput } from "@astryxdesign/core/TextInput"
import { Bot, CirclePlus, Download, Link2, Pencil, Play, RefreshCw, Save, Sparkles, Tag, Trash2 } from "lucide-react"
import { useEffect, useState } from "react"

import { api } from "../../app/api"
import type {
  AiApiType,
  AiProvider,
  AiProviderInput,
  AiProviderKind,
  AutoLabelRule,
  AutoLabelRuleInput,
  AutoLabelSubscription,
  AutoLabelSubscriptionInput,
  Label,
  LabelInput,
  MailAccount,
} from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"
import { SettingsPanelHeading } from "./SettingsPanelHeading"
import type { SettingsNotice } from "./settingsTypes"

type ProviderDraft = AiProviderInput & { id?: string }
type LabelDraft = LabelInput & { id?: string }
type RuleDraft = AutoLabelRuleInput & { id?: string }
type SubscriptionDraft = AutoLabelSubscriptionInput & { id?: string }

const providerKinds: AiProviderKind[] = ["openai", "claude", "gemini"]
const apiTypesByKind: Record<AiProviderKind, AiApiType[]> = {
  openai: ["chat", "responses"],
  claude: ["messages"],
  gemini: ["generateContent"],
}

const defaultProviderInput: AiProviderInput = {
  name: "",
  providerKind: "openai",
  apiType: "chat",
  model: "",
  baseUrl: "",
  apiKey: "",
  proxy: { kind: "direct" },
  isDefault: false,
  enabled: true,
}

const defaultLabelInput: LabelInput = {
  name: "",
  color: "#6B7280",
  isAuto: false,
}

const defaultRuleInput: AutoLabelRuleInput = {
  accountId: null,
  providerId: null,
  name: "",
  labelIds: [],
  instructions: "",
  enabled: true,
  applyAutomatically: true,
}

const defaultSubscriptionInput: AutoLabelSubscriptionInput = {
  name: "",
  url: "",
  enabled: true,
}

export function AiSettingsPanel({ accounts, onNotice }: {
  accounts: MailAccount[]
  onNotice: (notice: SettingsNotice) => void
}) {
  const { t } = useI18n()
  const providerDialog = useImperativeDialog({ purpose: "form", width: 720, padding: 0 })
  const labelDialog = useImperativeDialog({ purpose: "form", width: 560, padding: 0 })
  const ruleDialog = useImperativeDialog({ purpose: "form", width: 740, padding: 0 })
  const subscriptionDialog = useImperativeDialog({ purpose: "form", width: 620, padding: 0 })
  const confirmDialog = useImperativeConfirmDialog()
  const [providers, setProviders] = useState<AiProvider[]>([])
  const [labels, setLabels] = useState<Label[]>([])
  const [rules, setRules] = useState<AutoLabelRule[]>([])
  const [subscriptions, setSubscriptions] = useState<AutoLabelSubscription[]>([])
  const [busy, setBusy] = useState<string | null>(null)

  useEffect(() => {
    Promise.all([api.aiProviders(), api.labels(), api.autoLabelRules(), api.autoLabelSubscriptions()])
      .then(([nextProviders, nextLabels, nextRules, nextSubscriptions]) => {
        setProviders(nextProviders)
        setLabels(nextLabels)
        setRules(nextRules)
        setSubscriptions(nextSubscriptions)
      })
      .catch(() => onNotice({ key: "genericError", error: true }))
  }, [onNotice])

  function openProviderDialog(provider: AiProvider | null) {
    const draft = provider ? {
      id: provider.id,
      name: provider.name,
      providerKind: provider.providerKind,
      apiType: provider.apiType,
      model: provider.model,
      baseUrl: provider.baseUrl || "",
      apiKey: "",
      proxy: {
        kind: provider.proxy.kind,
        host: provider.proxy.host || "",
        port: provider.proxy.port || null,
        username: provider.proxy.username || "",
        password: "",
      },
      isDefault: provider.isDefault,
      enabled: provider.enabled,
    } : { ...defaultProviderInput }
    providerDialog.show(
      <ProviderEditor
        draft={draft}
        busy={busy === "provider"}
        onCancel={providerDialog.hide}
        onSubmit={saveProvider}
        onTest={testProvider}
        t={t}
      />,
      { "aria-label": provider ? t("editAiProvider") : t("newAiProvider") },
    )
  }

  function openLabelDialog(label: Label | null) {
    const draft = label ? {
      id: label.id,
      name: label.name,
      color: label.color,
      isAuto: label.isAuto,
    } : { ...defaultLabelInput }
    labelDialog.show(
      <LabelEditor
        draft={draft}
        busy={busy === "label"}
        onCancel={labelDialog.hide}
        onSubmit={saveLabel}
        t={t}
      />,
      { "aria-label": label ? t("editLabel") : t("newLabel") },
    )
  }

  function openRuleDialog(rule: AutoLabelRule | null) {
    const draft = rule ? {
      id: rule.id,
      accountId: rule.accountId || null,
      providerId: rule.providerId || null,
      name: rule.name,
      labelIds: [...rule.labelIds],
      instructions: rule.instructions,
      enabled: rule.enabled,
      applyAutomatically: rule.applyAutomatically,
    } : { ...defaultRuleInput }
    ruleDialog.show(
      <RuleEditor
        draft={draft}
        accounts={accounts}
        providers={providers}
        labels={labels}
        busy={busy === "rule"}
        onCancel={ruleDialog.hide}
        onSubmit={saveRule}
        t={t}
      />,
      { "aria-label": rule ? t("editAutoLabelRule") : t("newAutoLabelRule") },
    )
  }

  function openSubscriptionDialog(subscription: AutoLabelSubscription | null) {
    const draft = subscription ? {
      id: subscription.id,
      name: subscription.name,
      url: subscription.url,
      enabled: subscription.enabled,
    } : { ...defaultSubscriptionInput }
    subscriptionDialog.show(
      <SubscriptionEditor
        draft={draft}
        busy={busy === "subscription"}
        onCancel={subscriptionDialog.hide}
        onSubmit={saveSubscription}
        t={t}
      />,
      { "aria-label": subscription ? t("editAutoLabelSubscription") : t("newAutoLabelSubscription") },
    )
  }

  async function saveProvider(input: ProviderDraft) {
    setBusy("provider")
    try {
      const payload: AiProviderInput = {
        ...input,
        apiKey: input.apiKey || null,
        baseUrl: input.baseUrl || null,
        proxy: {
          kind: input.proxy.kind,
          host: input.proxy.host || null,
          port: input.proxy.port || null,
          username: input.proxy.username || null,
          password: input.proxy.password || null,
        },
      }
      const saved = input.id
        ? await api.updateAiProvider(input.id, payload)
        : await api.createAiProvider(payload)
      const next = await api.aiProviders()
      setProviders(next)
      providerDialog.hide()
      onNotice({ key: input.id ? "aiProviderUpdated" : "aiProviderCreated" })
      if (saved.isDefault) {
        setProviders((items) => items.map((item) => ({ ...item, isDefault: item.id === saved.id })))
      }
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function saveLabel(input: LabelDraft) {
    setBusy("label")
    try {
      const payload: LabelInput = {
        name: input.name,
        color: input.color,
        isAuto: input.isAuto,
      }
      if (input.id) await api.updateLabel(input.id, payload)
      else await api.createLabel(payload)
      setLabels(await api.labels())
      labelDialog.hide()
      onNotice({ key: input.id ? "labelUpdated" : "labelCreated" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function saveRule(input: RuleDraft) {
    setBusy("rule")
    try {
      const payload: AutoLabelRuleInput = {
        accountId: input.accountId || null,
        providerId: input.providerId || null,
        name: input.name,
        labelIds: input.labelIds,
        instructions: input.instructions,
        enabled: input.enabled,
        applyAutomatically: input.applyAutomatically,
      }
      if (input.id) await api.updateAutoLabelRule(input.id, payload)
      else await api.createAutoLabelRule(payload)
      setRules(await api.autoLabelRules())
      ruleDialog.hide()
      onNotice({ key: input.id ? "autoLabelRuleUpdated" : "autoLabelRuleCreated" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function exportRules() {
    setBusy("export")
    try {
      const feed = await api.exportAutoLabelRules()
      const url = URL.createObjectURL(new Blob([JSON.stringify(feed, null, 2)], { type: "application/json" }))
      const anchor = document.createElement("a")
      anchor.href = url
      anchor.download = `meowmail-auto-label-rules-${new Date().toISOString().slice(0, 10)}.json`
      anchor.click()
      URL.revokeObjectURL(url)
      onNotice({ key: "autoLabelRulesExported" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function saveSubscription(input: SubscriptionDraft) {
    setBusy("subscription")
    try {
      const payload: AutoLabelSubscriptionInput = {
        name: input.name,
        url: input.url,
        enabled: input.enabled,
      }
      const saved = input.id
        ? await api.updateAutoLabelSubscription(input.id, payload)
        : await api.createAutoLabelSubscription(payload)
      subscriptionDialog.hide()
      if (saved.enabled) {
        try {
          const result = await api.syncAutoLabelSubscription(saved.id)
          const [nextLabels, nextRules, nextSubscriptions] = await Promise.all([
            api.labels(), api.autoLabelRules(), api.autoLabelSubscriptions(),
          ])
          setLabels(nextLabels)
          setRules(nextRules)
          setSubscriptions(nextSubscriptions)
          onNotice({
            key: "autoLabelSubscriptionSynced",
            values: { rules: result.rulesImported, skipped: result.rulesSkipped },
          })
        } catch {
          setSubscriptions(await api.autoLabelSubscriptions())
          onNotice({ key: "autoLabelSubscriptionSavedSyncFailed", error: true })
        }
      } else {
        setSubscriptions(await api.autoLabelSubscriptions())
        onNotice({ key: input.id ? "autoLabelSubscriptionUpdated" : "autoLabelSubscriptionCreated" })
      }
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function syncSubscription(id: string) {
    setBusy(`subscription:${id}`)
    try {
      const result = await api.syncAutoLabelSubscription(id)
      const [nextLabels, nextRules, nextSubscriptions] = await Promise.all([
        api.labels(), api.autoLabelRules(), api.autoLabelSubscriptions(),
      ])
      setLabels(nextLabels)
      setRules(nextRules)
      setSubscriptions(nextSubscriptions)
      onNotice({
        key: "autoLabelSubscriptionSynced",
        values: { rules: result.rulesImported, skipped: result.rulesSkipped },
      })
    } catch {
      setSubscriptions(await api.autoLabelSubscriptions().catch(() => subscriptions))
      onNotice({ key: "autoLabelSubscriptionSyncFailed", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function deleteSubscription(subscription: AutoLabelSubscription) {
    const confirmed = await confirmDialog.confirm({
      title: t("deleteAutoLabelSubscriptionTitle"),
      description: t("deleteAutoLabelSubscriptionDescription", { name: subscription.name }),
      cancelLabel: t("cancel"),
      actionLabel: t("delete"),
      actionVariant: "destructive",
    })
    if (!confirmed) return
    setBusy(`subscription:${subscription.id}`)
    try {
      await api.deleteAutoLabelSubscription(subscription.id)
      const [nextRules, nextSubscriptions] = await Promise.all([
        api.autoLabelRules(), api.autoLabelSubscriptions(),
      ])
      setRules(nextRules)
      setSubscriptions(nextSubscriptions)
      onNotice({ key: "autoLabelSubscriptionDeleted" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function requestDelete(kind: "provider" | "label" | "rule", id: string, name: string) {
    const confirmed = await confirmDialog.confirm({
      title: t(kind === "provider" ? "deleteAiProviderTitle" : kind === "label" ? "deleteLabelTitle" : "deleteAutoLabelRuleTitle"),
      description: name,
      cancelLabel: t("cancel"),
      actionLabel: t("delete"),
      actionVariant: "destructive",
    })
    if (!confirmed) return
    setBusy(kind)
    try {
      if (kind === "provider") await api.deleteAiProvider(id)
      if (kind === "label") await api.deleteLabel(id)
      if (kind === "rule") await api.deleteAutoLabelRule(id)
      const [nextProviders, nextLabels, nextRules] = await Promise.all([api.aiProviders(), api.labels(), api.autoLabelRules()])
      setProviders(nextProviders)
      setLabels(nextLabels)
      setRules(nextRules)
      onNotice({ key: kind === "provider" ? "aiProviderDeleted" : kind === "label" ? "labelDeleted" : "autoLabelRuleDeleted" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function testProvider(provider: ProviderDraft) {
    if (!provider.id) return
    setBusy("provider")
    try {
      await api.testAiProvider(provider.id)
      onNotice({ key: "aiProviderTestOk" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  return (
    <>
      <div className="settings-panel-stack">
        <SettingsPanelHeading icon={<Bot />} title={t("aiConfiguration")} description={t("aiConfigurationDescription")} />
        <section className="settings-ai-block" aria-label={t("aiConfiguration")}>
          <div className="settings-row-header">
            <div><strong>{t("aiProviders")}</strong><small>{t("aiProvidersDescription")}</small></div>
            <Button label={t("newAiProvider")} icon={<CirclePlus aria-hidden="true" />} variant="secondary" size="sm" onClick={() => openProviderDialog(null)} />
          </div>
          <List className="settings-inline-list" hasDividers density="compact">
            {providers.length ? providers.map((provider) => (
              <ProviderRow
                key={provider.id}
                provider={provider}
                onEdit={() => openProviderDialog(provider)}
                onDelete={() => void requestDelete("provider", provider.id, provider.name)}
                onTest={() => void testProvider({ id: provider.id, ...defaultProviderInput, name: provider.name, providerKind: provider.providerKind, apiType: provider.apiType, model: provider.model, baseUrl: provider.baseUrl || "", proxy: { kind: provider.proxy.kind, host: provider.proxy.host || "", port: provider.proxy.port || null, username: provider.proxy.username || "", password: "" }, isDefault: provider.isDefault, enabled: provider.enabled })}
                busy={busy === "provider"}
              />
            )) : <p className="settings-empty-copy">{t("noAiProviders")}</p>}
          </List>

          <div className="settings-subsection-divider" />

          <div className="settings-row-header">
            <div><strong>{t("aiLabels")}</strong><small>{t("aiLabelsDescription")}</small></div>
            <Button label={t("newLabel")} icon={<Tag aria-hidden="true" />} variant="secondary" size="sm" onClick={() => openLabelDialog(null)} />
          </div>
          <List className="settings-inline-list" hasDividers density="compact">
            {labels.length ? labels.map((label) => (
              <LabelRow
                key={label.id}
                label={label}
                onEdit={() => openLabelDialog(label)}
                onDelete={() => void requestDelete("label", label.id, label.name)}
              />
            )) : <p className="settings-empty-copy">{t("noAiLabels")}</p>}
          </List>

          <div className="settings-subsection-divider" />

          <div className="settings-row-header">
            <div><strong>{t("autoLabelRules")}</strong><small>{t("autoLabelRulesDescription")}</small></div>
            <div className="settings-button-row end">
              <Button label={t("exportAutoLabelRules")} icon={<Download aria-hidden="true" />} variant="ghost" size="sm" isLoading={busy === "export"} isDisabled={Boolean(busy)} onClick={() => void exportRules()} />
              <Button label={t("newAutoLabelRule")} icon={<Sparkles aria-hidden="true" />} variant="secondary" size="sm" onClick={() => openRuleDialog(null)} />
            </div>
          </div>
          <List className="settings-inline-list" hasDividers density="compact">
            {rules.length ? rules.map((rule) => (
              <RuleRow
                key={rule.id}
                rule={rule}
                accounts={accounts}
                providers={providers}
                labels={labels}
                sourceEnabled={!rule.sourceSubscriptionId || subscriptions.some((subscription) => subscription.id === rule.sourceSubscriptionId && subscription.enabled)}
                onEdit={() => openRuleDialog(rule)}
                onDelete={() => void requestDelete("rule", rule.id, rule.name)}
              />
            )) : <p className="settings-empty-copy">{t("noAutoLabelRules")}</p>}
          </List>

          <div className="settings-subsection-divider" />

          <div className="settings-row-header">
            <div><strong>{t("autoLabelSubscriptions")}</strong><small>{t("autoLabelSubscriptionsDescription")}</small></div>
            <Button label={t("newAutoLabelSubscription")} icon={<Link2 aria-hidden="true" />} variant="secondary" size="sm" onClick={() => openSubscriptionDialog(null)} />
          </div>
          <Banner status="info" title={t("autoLabelSubscriptionSafetyTitle")} description={t("autoLabelSubscriptionSafetyDescription")} />
          <List className="settings-inline-list" hasDividers density="compact">
            {subscriptions.length ? subscriptions.map((subscription) => (
              <SubscriptionRow
                key={subscription.id}
                subscription={subscription}
                busy={busy === `subscription:${subscription.id}`}
                onSync={() => void syncSubscription(subscription.id)}
                onEdit={() => openSubscriptionDialog(subscription)}
                onDelete={() => void deleteSubscription(subscription)}
              />
            )) : <p className="settings-empty-copy">{t("noAutoLabelSubscriptions")}</p>}
          </List>
        </section>
      </div>
      {providerDialog.element}
      {labelDialog.element}
      {ruleDialog.element}
      {subscriptionDialog.element}
      {confirmDialog.element}
    </>
  )
}

function ProviderRow({ provider, busy, onEdit, onDelete, onTest }: {
  provider: AiProvider
  busy: boolean
  onEdit: () => void
  onDelete: () => void
  onTest: () => void
}) {
  const { t } = useI18n()
  return (
    <div className="settings-inline-row">
      <div className="settings-inline-summary">
        <strong>{provider.name}</strong>
        <small>{provider.model} · {provider.baseUrl || t("direct")} · {t(`aiProviderKind_${provider.providerKind}` as MessageKey)} / {t(`aiApiType_${provider.apiType}` as MessageKey)}</small>
      </div>
      <div className="settings-inline-badges">
        {provider.isDefault && <Badge variant="blue" label={t("default")} />}
        {!provider.enabled && <Badge variant="warning" label={t("disabled")} />}
        {provider.hasApiKey && <Badge variant="success" label={t("enabled")} />}
      </div>
      <div className="settings-inline-actions">
        <Button label={t("testConnection")} icon={<Play aria-hidden="true" />} variant="ghost" size="sm" isDisabled={busy} onClick={onTest} />
        <IconButton label={t("edit")} icon={<Pencil aria-hidden="true" />} variant="ghost" size="sm" onClick={onEdit} />
        <IconButton label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="ghost" size="sm" className="danger-text" onClick={onDelete} />
      </div>
    </div>
  )
}

function LabelRow({ label, onEdit, onDelete }: {
  label: Label
  onEdit: () => void
  onDelete: () => void
}) {
  const { t } = useI18n()
  return (
    <div className="settings-inline-row">
      <div className="settings-inline-summary">
        <strong>{label.name}</strong>
        <small>{label.color}</small>
      </div>
      <div className="settings-inline-badges">
        <Badge variant={label.isAuto ? "purple" : "neutral"} label={label.isAuto ? t("autoLabelTag") : t("manualLabel")} />
      </div>
      <div className="settings-inline-actions">
        <IconButton label={t("edit")} icon={<Pencil aria-hidden="true" />} variant="ghost" size="sm" onClick={onEdit} />
        <IconButton label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="ghost" size="sm" className="danger-text" onClick={onDelete} />
      </div>
    </div>
  )
}

function RuleRow({ rule, accounts, providers, labels, sourceEnabled, onEdit, onDelete }: {
  rule: AutoLabelRule
  accounts: MailAccount[]
  providers: AiProvider[]
  labels: Label[]
  sourceEnabled: boolean
  onEdit: () => void
  onDelete: () => void
}) {
  const { t } = useI18n()
  return (
    <div className="settings-inline-row">
      <div className="settings-inline-summary">
        <strong>{rule.name}</strong>
        <small>
          {rule.accountId ? `${accounts.find((account) => account.id === rule.accountId)?.displayName || t("unknownAccount")} · ` : ""}
          {rule.providerId ? `${providers.find((provider) => provider.id === rule.providerId)?.name || t("unknownAccount")} · ` : ""}
          {labels.filter((label) => rule.labelIds.includes(label.id)).map((label) => label.name).join(", ") || t("disabled")}
        </small>
      </div>
      <div className="settings-inline-badges">
        {rule.enabled && sourceEnabled && <Badge variant="success" label={t("enabled")} />}
        {!sourceEnabled && <Badge variant="neutral" label={t("disabled")} />}
        {rule.applyAutomatically && <Badge variant="blue" label={t("autoLabelApplyAutomatically")} />}
        {rule.sourceSubscriptionId && <Badge variant="neutral" label={t("subscribedRule")} />}
      </div>
      <div className="settings-inline-actions">
        {!rule.sourceSubscriptionId && <IconButton label={t("edit")} icon={<Pencil aria-hidden="true" />} variant="ghost" size="sm" onClick={onEdit} />}
        {!rule.sourceSubscriptionId && <IconButton label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="ghost" size="sm" className="danger-text" onClick={onDelete} />}
      </div>
    </div>
  )
}

function SubscriptionRow({ subscription, busy, onSync, onEdit, onDelete }: {
  subscription: AutoLabelSubscription
  busy: boolean
  onSync: () => void
  onEdit: () => void
  onDelete: () => void
}) {
  const { locale, t } = useI18n()
  const syncLabel = subscription.lastSyncedAt
    ? t("autoLabelSubscriptionLastSynced", { time: new Date(subscription.lastSyncedAt * 1000).toLocaleString(locale === "zh-CN" ? "zh-CN" : "en") })
    : t("autoLabelSubscriptionNeverSynced")
  return (
    <div className="settings-inline-row">
      <div className="settings-inline-summary">
        <strong>{subscription.name}</strong>
        <small>{subscription.lastError || `${subscription.url} · ${syncLabel}`}</small>
      </div>
      <div className="settings-inline-badges">
        <Badge variant={subscription.enabled ? "success" : "neutral"} label={subscription.enabled ? t("enabled") : t("disabled")} />
        {subscription.lastError && <Badge variant="warning" label={t("syncFailed")} />}
      </div>
      <div className="settings-inline-actions">
        <IconButton label={t("syncNow")} icon={<RefreshCw className={busy ? "rotating" : undefined} aria-hidden="true" />} variant="ghost" size="sm" isDisabled={!subscription.enabled || busy} onClick={onSync} />
        <IconButton label={t("edit")} icon={<Pencil aria-hidden="true" />} variant="ghost" size="sm" isDisabled={busy} onClick={onEdit} />
        <IconButton label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="ghost" size="sm" className="danger-text" isDisabled={busy} onClick={onDelete} />
      </div>
    </div>
  )
}

function SubscriptionEditor({ draft, busy, onCancel, onSubmit, t }: {
  draft: SubscriptionDraft
  busy: boolean
  onCancel: () => void
  onSubmit: (input: SubscriptionDraft) => Promise<void>
  t: (key: MessageKey, values?: Record<string, string | number>) => string
}) {
  const [value, setValue] = useState(draft)
  useEffect(() => { setValue(draft) }, [draft])
  const canSave = Boolean(value.name.trim() && value.url.trim())
  return (
    <form className="settings-dialog-form" onSubmit={(event) => { event.preventDefault(); void onSubmit(value) }}>
      <Layout
        className="settings-dialog-form-layout"
        padding={4}
        header={<DialogHeader title={draft.id ? t("editAutoLabelSubscription") : t("newAutoLabelSubscription")} startContent={<span className="settings-dialog-icon"><Link2 aria-hidden="true" /></span>} hasDivider onOpenChange={(open) => { if (!open) onCancel() }} />}
        content={<LayoutContent className="settings-dialog-form-content" padding={4}>
          <Banner status="info" title={t("autoLabelSubscriptionSafetyTitle")} description={t("autoLabelSubscriptionSafetyDescription")} />
          <div className="settings-form-grid">
            <TextInput label={`${t("subscriptionName")} · ${t("required")}`} value={value.name} onChange={(name) => setValue({ ...value, name })} placeholder={t("subscriptionNamePlaceholder")} width="100%" />
            <TextInput label={`${t("subscriptionUrl")} · ${t("required")}`} labelTooltip={t("subscriptionUrlHint")} value={value.url} onChange={(url) => setValue({ ...value, url })} placeholder="https://example.com/meowmail-rules.json" width="100%" />
          </div>
          <div className="settings-switch-row">
            <Switch label={t("enabled")} labelTooltip={t("autoLabelSubscriptionEnabledHint")} value={value.enabled} onChange={(enabled) => setValue({ ...value, enabled })} labelPosition="start" labelSpacing="spread" />
          </div>
        </LayoutContent>}
        footer={<LayoutFooter className="settings-dialog-form-footer" padding={3} hasDivider>
          <Button label={t("cancel")} variant="secondary" isDisabled={busy} onClick={onCancel} />
          <Button label={busy ? t("saving") : t(value.enabled ? "saveAndSync" : "save")} icon={<Save aria-hidden="true" />} variant="primary" type="submit" isLoading={busy} isDisabled={!canSave || busy} />
        </LayoutFooter>}
      />
    </form>
  )
}

function ProviderEditor({ draft, busy, onCancel, onSubmit, onTest, t }: {
  draft: ProviderDraft
  busy: boolean
  onCancel: () => void
  onSubmit: (input: ProviderDraft) => Promise<void>
  onTest: (input: ProviderDraft) => Promise<void>
  t: (key: MessageKey, values?: Record<string, string | number>) => string
}) {
  const [value, setValue] = useState(draft)
  useEffect(() => { setValue(draft) }, [draft])
  const canTest = Boolean(value.id)
  const canSave = value.name.trim() && value.model.trim()
  return (
    <form className="settings-dialog-form" onSubmit={(event) => { event.preventDefault(); void onSubmit(value) }}>
      <Layout className="settings-dialog-form-layout" padding={4} header={<DialogHeader title={draft.id ? t("editAiProvider") : t("newAiProvider")} startContent={<span className="settings-dialog-icon"><Bot aria-hidden="true" /></span>} hasDivider onOpenChange={(open) => { if (!open) onCancel() }} />} content={<LayoutContent className="settings-dialog-form-content" padding={4}>
        <div className="settings-form-grid">
          <TextInput label={`${t("providerName")} · ${t("required")}`} value={value.name} onChange={(name) => setValue({ ...value, name })} placeholder={t("providerNamePlaceholder")} width="100%" />
          <Selector label={t("providerKind")} value={value.providerKind} onChange={(providerKind) => setValue({ ...value, providerKind: providerKind as AiProviderKind, apiType: apiTypesByKind[providerKind as AiProviderKind][0] })} options={providerKinds.map((kind) => ({ value: kind, label: t(`aiProviderKind_${kind}` as MessageKey) }))} width="100%" />
          <Selector label={t("apiType")} value={value.apiType} onChange={(apiType) => setValue({ ...value, apiType: apiType as AiApiType })} options={apiTypesByKind[value.providerKind].map((kind) => ({ value: kind, label: t(`aiApiType_${kind}` as MessageKey) }))} width="100%" />
          <TextInput label={`${t("model")} · ${t("required")}`} value={value.model} onChange={(model) => setValue({ ...value, model })} placeholder={t("modelPlaceholder")} width="100%" />
          <TextInput label={t("baseUrl")} value={value.baseUrl || ""} onChange={(baseUrl) => setValue({ ...value, baseUrl })} placeholder={t("baseUrlPlaceholder")} width="100%" />
          <TextInput label={t("apiKey")} value={value.apiKey || ""} onChange={(apiKey) => setValue({ ...value, apiKey })} placeholder={t("apiKeyPlaceholder")} width="100%" type="password" />
        </div>
        <div className="settings-switch-row">
          <Switch label={t("enabled")} value={value.enabled} onChange={(enabled) => setValue({ ...value, enabled })} labelPosition="start" labelSpacing="spread" />
          <Switch label={t("default")} value={value.isDefault} onChange={(isDefault) => setValue({ ...value, isDefault })} labelPosition="start" labelSpacing="spread" />
        </div>
        <div className="settings-form-grid">
          <Selector label={t("aiProxy")} value={value.proxy.kind} onChange={(kind) => setValue({ ...value, proxy: { ...value.proxy, kind: kind as "direct" | "http" | "socks5" } })} options={[
            { value: "direct", label: t("direct") },
            { value: "http", label: t("http") },
            { value: "socks5", label: t("socks5") },
          ]} width="100%" />
          {value.proxy.kind !== "direct" && (
            <>
              <TextInput label={t("proxyHostPlaceholder")} value={value.proxy.host || ""} onChange={(host) => setValue({ ...value, proxy: { ...value.proxy, host } })} placeholder={t("proxyHostPlaceholder")} width="100%" />
              <NumberInput label={t("proxyPortPlaceholder")} value={value.proxy.port || null} onChange={(port) => setValue({ ...value, proxy: { ...value.proxy, port } })} min={1} max={65535} step={1} isIntegerOnly hasClear width="100%" />
              <TextInput label={t("proxyUsername")} value={value.proxy.username || ""} onChange={(username) => setValue({ ...value, proxy: { ...value.proxy, username } })} placeholder={t("proxyUsernamePlaceholder")} width="100%" />
              <TextInput label={t("proxyPassword")} type="password" value={value.proxy.password || ""} onChange={(password) => setValue({ ...value, proxy: { ...value.proxy, password } })} placeholder={t("proxyPasswordPlaceholder")} width="100%" />
            </>
          )}
        </div>
      </LayoutContent>} footer={<LayoutFooter className="settings-dialog-form-footer" padding={3} hasDivider>
        <Button label={t("cancel")} variant="secondary" onClick={onCancel} />
        {canTest && <Button label={t("testConnection")} icon={<Play aria-hidden="true" />} variant="secondary" isDisabled={busy} onClick={() => void onTest(value)} />}
        <Button label={busy ? t("saving") : t("save")} icon={<Save aria-hidden="true" />} variant="primary" type="submit" isLoading={busy} isDisabled={!canSave || busy} />
      </LayoutFooter>} />
    </form>
  )
}

function LabelEditor({ draft, busy, onCancel, onSubmit, t }: {
  draft: LabelDraft
  busy: boolean
  onCancel: () => void
  onSubmit: (input: LabelDraft) => Promise<void>
  t: (key: MessageKey, values?: Record<string, string | number>) => string
}) {
  const [value, setValue] = useState(draft)
  useEffect(() => { setValue(draft) }, [draft])
  return (
    <form className="settings-dialog-form" onSubmit={(event) => { event.preventDefault(); void onSubmit(value) }}>
      <Layout className="settings-dialog-form-layout" padding={4} header={<DialogHeader title={draft.id ? t("editLabel") : t("newLabel")} startContent={<span className="settings-dialog-icon"><Tag aria-hidden="true" /></span>} hasDivider onOpenChange={(open) => { if (!open) onCancel() }} />} content={<LayoutContent className="settings-dialog-form-content" padding={4}>
        <TextInput label={`${t("labelName")} · ${t("required")}`} value={value.name} onChange={(name) => setValue({ ...value, name })} placeholder={t("labelNamePlaceholder")} width="100%" />
        <div className="settings-form-grid">
          <TextInput label={t("labelColor")} value={value.color} onChange={(color) => setValue({ ...value, color })} placeholder="#6B7280" width="100%" />
          <CheckboxInput label={t("autoLabelTag")} value={value.isAuto} onChange={(isAuto) => setValue({ ...value, isAuto })} />
        </div>
      </LayoutContent>} footer={<LayoutFooter className="settings-dialog-form-footer" padding={3} hasDivider>
        <Button label={t("cancel")} variant="secondary" onClick={onCancel} />
        <Button label={busy ? t("saving") : t("save")} icon={<Save aria-hidden="true" />} variant="primary" type="submit" isLoading={busy} isDisabled={!value.name.trim() || busy} />
      </LayoutFooter>} />
    </form>
  )
}

function RuleEditor({ draft, accounts, providers, labels, busy, onCancel, onSubmit, t }: {
  draft: RuleDraft
  accounts: MailAccount[]
  providers: AiProvider[]
  labels: Label[]
  busy: boolean
  onCancel: () => void
  onSubmit: (input: RuleDraft) => Promise<void>
  t: (key: MessageKey, values?: Record<string, string | number>) => string
}) {
  const [value, setValue] = useState(draft)
  useEffect(() => { setValue(draft) }, [draft])
  return (
    <form className="settings-dialog-form" onSubmit={(event) => { event.preventDefault(); void onSubmit(value) }}>
      <Layout className="settings-dialog-form-layout" padding={4} header={<DialogHeader title={draft.id ? t("editAutoLabelRule") : t("newAutoLabelRule")} startContent={<span className="settings-dialog-icon"><Sparkles aria-hidden="true" /></span>} hasDivider onOpenChange={(open) => { if (!open) onCancel() }} />} content={<LayoutContent className="settings-dialog-form-content" padding={4}>
        <TextInput label={`${t("ruleName")} · ${t("required")}`} value={value.name} onChange={(name) => setValue({ ...value, name })} placeholder={t("ruleNamePlaceholder")} width="100%" />
        <div className="settings-form-grid">
          <Selector label={t("mailAccountScope")} hasClear value={value.accountId || null} onChange={(accountId) => setValue({ ...value, accountId: accountId || null })} options={accounts.map((account) => ({ value: account.id, label: account.displayName, description: account.email }))} placeholder={t("allAccounts")} width="100%" />
          <Selector label={t("provider")} hasClear value={value.providerId || null} onChange={(providerId) => setValue({ ...value, providerId: providerId || null })} options={providers.map((provider) => ({ value: provider.id, label: provider.name, description: provider.model }))} placeholder={t("allAccounts")} width="100%" />
        </div>
        <MultiSelector
          label={t("labels")}
          value={value.labelIds}
          onChange={(labelIds) => setValue({ ...value, labelIds })}
          options={labels.map((label) => ({ value: label.id, label: label.name }))}
          hasSelectAll
          triggerDisplay="labels"
          width="100%"
        />
        <TextArea label={`${t("autoLabelInstructions")} · ${t("required")}`} value={value.instructions} onChange={(instructions) => setValue({ ...value, instructions })} rows={5} width="100%" />
        <div className="settings-switch-row">
          <Switch label={t("enabled")} value={value.enabled} onChange={(enabled) => setValue({ ...value, enabled })} labelPosition="start" labelSpacing="spread" />
          <Switch label={t("autoLabelApplyAutomatically")} value={value.applyAutomatically} onChange={(applyAutomatically) => setValue({ ...value, applyAutomatically })} labelPosition="start" labelSpacing="spread" />
        </div>
      </LayoutContent>} footer={<LayoutFooter className="settings-dialog-form-footer" padding={3} hasDivider>
        <Button label={t("cancel")} variant="secondary" onClick={onCancel} />
        <Button label={busy ? t("saving") : t("save")} icon={<Save aria-hidden="true" />} variant="primary" type="submit" isLoading={busy} isDisabled={!value.name.trim() || !value.labelIds.length || busy} />
      </LayoutFooter>} />
    </form>
  )
}
