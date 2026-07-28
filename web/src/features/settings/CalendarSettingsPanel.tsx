import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { DialogHeader, useImperativeDialog } from "@astryxdesign/core/Dialog"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Layout, LayoutContent, LayoutFooter } from "@astryxdesign/core/Layout"
import { List } from "@astryxdesign/core/List"
import { TextInput } from "@astryxdesign/core/TextInput"
import { CalendarDays, CirclePlus, Download, Pencil, RefreshCw, Save, Trash2 } from "lucide-react"
import { useEffect, useMemo, useState } from "react"

import { api } from "../../app/api"
import type { Calendar, CalendarAccount, CalendarAccountInput, CalendarFeature, CalendarPreferences, CalendarUpdate } from "../../app/types"
import { allCalendarFeatures, calendarFeatureGroups, defaultCalendarFeatures } from "../calendar/calendarFeatures"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"
import { SettingsPanelHeading } from "./SettingsPanelHeading"
import type { SettingsNotice } from "./settingsTypes"

type AccountDraft = CalendarAccountInput & { id?: string }

const defaultAccount: CalendarAccountInput = {
  name: "",
  baseUrl: "",
  username: "",
  password: "",
  enabled: true,
}

export function CalendarSettingsPanel({ onNotice, onCalendarChanged }: {
  onNotice: (notice: SettingsNotice) => void
  onCalendarChanged: () => void
}) {
  const { t } = useI18n()
  const accountDialog = useImperativeDialog({ purpose: "form", width: 660, padding: 0 })
  const calendarDialog = useImperativeDialog({ purpose: "form", width: 560, padding: 0 })
  const confirmDialog = useImperativeConfirmDialog()
  const [accounts, setAccounts] = useState<CalendarAccount[]>([])
  const [calendars, setCalendars] = useState<Calendar[]>([])
  const [preferences, setPreferences] = useState<CalendarPreferences>({ enabledFeatures: defaultCalendarFeatures })
  const [preferenceDraft, setPreferenceDraft] = useState<CalendarPreferences>({ enabledFeatures: defaultCalendarFeatures })
  const [busy, setBusy] = useState<string | null>(null)

  useEffect(() => {
    void refreshAll()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const preferencesChanged = useMemo(() => {
    const saved = [...preferences.enabledFeatures].sort().join(",")
    const draft = [...preferenceDraft.enabledFeatures].sort().join(",")
    return saved !== draft
  }, [preferenceDraft.enabledFeatures, preferences.enabledFeatures])

  async function refreshAll() {
    try {
      const [nextAccounts, nextCalendars, nextPreferences] = await Promise.all([
        api.calendarAccounts(),
        api.calendars(),
        api.calendarPreferences(),
      ])
      setAccounts(nextAccounts)
      setCalendars(nextCalendars)
      setPreferences(nextPreferences)
      setPreferenceDraft(nextPreferences)
    } catch {
      onNotice({ key: "genericError", error: true })
    }
  }

  async function refreshConfiguration() {
    const [nextAccounts, nextCalendars] = await Promise.all([
      api.calendarAccounts(),
      api.calendars(),
    ])
    setAccounts(nextAccounts)
    setCalendars(nextCalendars)
  }

  function toggleFeature(feature: CalendarFeature, enabled: boolean) {
    setPreferenceDraft((current) => ({
      enabledFeatures: enabled
        ? [...new Set([...current.enabledFeatures, feature])]
        : current.enabledFeatures.filter((value) => value !== feature),
    }))
  }

  async function savePreferences() {
    setBusy("preferences")
    try {
      const saved = await api.updateCalendarPreferences(preferenceDraft)
      setPreferences(saved)
      setPreferenceDraft(saved)
      onCalendarChanged()
      onNotice({ key: "calendarPreferencesSaved" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  function openAccountDialog(account: CalendarAccount | null) {
    const draft = account ? {
      id: account.id,
      name: account.name,
      baseUrl: account.baseUrl,
      username: account.username,
      password: "",
      enabled: account.enabled,
    } : { ...defaultAccount }
    accountDialog.show(
      <AccountEditor
        draft={draft}
        busy={busy === "account"}
        onCancel={accountDialog.hide}
        onSubmit={saveAccount}
        t={t}
      />,
      { "aria-label": account ? t("editCalendarAccount") : t("newCalendarAccount") },
    )
  }

  function openCalendarDialog(calendar: Calendar | null) {
    const draft = calendar ? {
      id: calendar.id,
      displayName: calendar.displayName,
      color: calendar.color,
      enabled: calendar.enabled,
    } : null
    calendarDialog.show(
      <CalendarEditor
        calendar={calendar}
        draft={draft || { displayName: "", color: "var(--accent)", enabled: true }}
        busy={busy === "calendar"}
        onCancel={calendarDialog.hide}
        onSubmit={saveCalendar}
        t={t}
      />,
      { "aria-label": calendar ? t("editCalendar") : t("newCalendar") },
    )
  }

  async function saveAccount(input: AccountDraft) {
    setBusy("account")
    try {
      const payload: CalendarAccountInput = {
        name: input.name,
        baseUrl: input.baseUrl,
        username: input.username,
        password: input.password || null,
        enabled: input.enabled,
      }
      if (input.id) await api.updateCalendarAccount(input.id, payload)
      else await api.createCalendarAccount(payload)
      await refreshConfiguration()
      accountDialog.hide()
      onCalendarChanged()
      onNotice({ key: input.id ? "calendarAccountUpdated" : "calendarAccountCreated" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function requestDelete(account: CalendarAccount) {
    const confirmed = await confirmDialog.confirm({
      title: t("deleteCalendarAccountTitle"),
      description: account.name,
      cancelLabel: t("cancel"),
      actionLabel: t("delete"),
      actionVariant: "destructive",
    })
    if (!confirmed) return
    setBusy("account")
    try {
      await api.deleteCalendarAccount(account.id)
      await refreshConfiguration()
      onCalendarChanged()
      onNotice({ key: "calendarAccountDeleted" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function syncAccount(account: CalendarAccount) {
    setBusy(account.id)
    try {
      await api.syncCalendarAccount(account.id)
      await refreshConfiguration()
      onCalendarChanged()
      onNotice({ key: "calendarSynced" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function discoverAccount(account: CalendarAccount) {
    setBusy(`${account.id}-discover`)
    try {
      await api.discoverCalendarAccount(account.id)
      await refreshConfiguration()
      onCalendarChanged()
      onNotice({ key: "calendarDiscovered" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function saveCalendar(calendarId: string, input: CalendarUpdate) {
    setBusy("calendar")
    try {
      await api.updateCalendar(calendarId, input)
      setCalendars(await api.calendars())
      onCalendarChanged()
      onNotice({ key: "calendarUpdated" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  return (
    <>
      <div className="settings-panel-stack">
        <SettingsPanelHeading icon={<CalendarDays />} title={t("calendarDisplayOptions")} description={t("calendarDisplayOptionsDescription")} />
        <section className="settings-calendar-options" aria-label={t("calendarDisplayOptions")}>
          <div className="settings-calendar-option-actions">
            <Button label={t("selectAll")} variant="ghost" size="sm" onClick={() => setPreferenceDraft({ enabledFeatures: allCalendarFeatures })} />
            <Button label={t("restoreDefaults")} variant="ghost" size="sm" onClick={() => setPreferenceDraft({ enabledFeatures: defaultCalendarFeatures })} />
          </div>
          <div className="settings-calendar-option-groups">
            {calendarFeatureGroups.map((group) => (
              <fieldset key={group.key} className="settings-calendar-option-group">
                <legend>{t(group.label)}</legend>
                <div className="settings-calendar-option-grid">
                  {group.features.map((feature) => (
                    <CheckboxInput
                      key={feature.value}
                      label={t(feature.label)}
                      value={preferenceDraft.enabledFeatures.includes(feature.value)}
                      onChange={(enabled) => toggleFeature(feature.value, enabled)}
                    />
                  ))}
                </div>
              </fieldset>
            ))}
          </div>
          <div className="settings-calendar-option-footer">
            <Button
              label={busy === "preferences" ? t("saving") : t("save")}
              icon={<Save aria-hidden="true" />}
              variant="primary"
              size="sm"
              isLoading={busy === "preferences"}
              isDisabled={!preferencesChanged || busy !== null}
              onClick={() => void savePreferences()}
            />
          </div>
        </section>

        <SettingsPanelHeading icon={<CalendarDays />} title={t("calendarConfiguration")} description={t("calendarConfigurationDescription")} />
        <section className="settings-calendar-block" aria-label={t("calendarConfiguration")}>
          <div className="settings-row-header">
            <div><strong>{t("calendarAccounts")}</strong><small>{t("calendarAccountsDescription")}</small></div>
            <Button label={t("newCalendarAccount")} icon={<CirclePlus aria-hidden="true" />} variant="secondary" size="sm" onClick={() => openAccountDialog(null)} />
          </div>
          <List className="settings-inline-list" hasDividers density="compact">
            {accounts.length ? accounts.map((account) => (
              <CalendarAccountRow
                key={account.id}
                account={account}
                busy={busy === account.id}
                onEdit={() => openAccountDialog(account)}
                onDelete={() => void requestDelete(account)}
                onDiscover={() => void discoverAccount(account)}
                onSync={() => void syncAccount(account)}
              />
            )) : <p className="settings-empty-copy">{t("noCalendarAccounts")}</p>}
          </List>

          <div className="settings-subsection-divider" />

          <div className="settings-row-header">
            <div><strong>{t("calendars")}</strong><small>{t("calendarsDescription")}</small></div>
          </div>
          <List className="settings-inline-list" hasDividers density="compact">
            {calendars.length ? calendars.map((calendar) => (
              <CalendarRow
                key={calendar.id}
                calendar={calendar}
                busy={busy === calendar.id}
                onEdit={() => openCalendarDialog(calendar)}
              />
            )) : <p className="settings-empty-copy">{t("noCalendars")}</p>}
          </List>

        </section>
      </div>
      {accountDialog.element}
      {calendarDialog.element}
      {confirmDialog.element}
    </>
  )
}

function CalendarAccountRow({ account, busy, onEdit, onDelete, onDiscover, onSync }: {
  account: CalendarAccount
  busy: boolean
  onEdit: () => void
  onDelete: () => void
  onDiscover: () => void
  onSync: () => void
}) {
  const { t } = useI18n()
  return (
    <div className="settings-inline-row">
      <div className="settings-inline-summary">
        <strong>{account.name}</strong>
        <small>{account.username} · {account.baseUrl}</small>
      </div>
      <div className="settings-inline-badges">
        {account.enabled ? <Badge variant="success" label={t("enabled")} /> : <Badge variant="warning" label={t("disabled")} />}
        {account.lastError && <Badge variant="error" label={t("calendarSyncError")} />}
      </div>
      <div className="settings-inline-actions">
        <Button label={t("discover")} icon={<Download aria-hidden="true" />} variant="ghost" size="sm" isDisabled={busy} onClick={onDiscover} />
        <Button label={t("sync")} icon={<RefreshCw aria-hidden="true" />} variant="ghost" size="sm" isDisabled={busy} onClick={onSync} />
        <IconButton label={t("edit")} icon={<Pencil aria-hidden="true" />} variant="ghost" size="sm" onClick={onEdit} />
        <IconButton label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="ghost" size="sm" className="danger-text" onClick={onDelete} />
      </div>
    </div>
  )
}

function CalendarRow({ calendar, busy, onEdit }: {
  calendar: Calendar
  busy: boolean
  onEdit: () => void
}) {
  const { t } = useI18n()
  return (
    <div className="settings-inline-row">
      <div className="settings-inline-summary">
        <strong>{calendar.displayName}</strong>
        <small>{calendar.remoteHref}</small>
      </div>
      <div className="settings-inline-badges">
        <Badge variant="blue" label={calendar.color} />
        {calendar.enabled ? <Badge variant="success" label={t("enabled")} /> : <Badge variant="warning" label={t("disabled")} />}
      </div>
      <div className="settings-inline-actions">
        <IconButton label={t("edit")} icon={<Pencil aria-hidden="true" />} variant="ghost" size="sm" isDisabled={busy} onClick={onEdit} />
      </div>
    </div>
  )
}

function AccountEditor({ draft, busy, onCancel, onSubmit, t }: {
  draft: AccountDraft
  busy: boolean
  onCancel: () => void
  onSubmit: (input: AccountDraft) => Promise<void>
  t: (key: MessageKey, values?: Record<string, string | number>) => string
}) {
  const [value, setValue] = useState(draft)
  useEffect(() => { setValue(draft) }, [draft])
  return (
    <form className="settings-dialog-form" onSubmit={(event) => { event.preventDefault(); void onSubmit(value) }}>
      <Layout className="settings-dialog-form-layout" padding={4} header={<DialogHeader title={draft.id ? t("editCalendarAccount") : t("newCalendarAccount")} startContent={<span className="settings-dialog-icon"><CalendarDays aria-hidden="true" /></span>} hasDivider onOpenChange={(open) => { if (!open) onCancel() }} />} content={<LayoutContent className="settings-dialog-form-content" padding={4}>
        <TextInput label={`${t("calendarAccountName")} · ${t("required")}`} value={value.name} onChange={(name) => setValue({ ...value, name })} placeholder={t("calendarAccountNamePlaceholder")} width="100%" />
        <TextInput label={`${t("baseUrl")} · ${t("required")}`} value={value.baseUrl} onChange={(baseUrl) => setValue({ ...value, baseUrl })} placeholder={t("calendarBaseUrlPlaceholder")} width="100%" />
        <TextInput label={`${t("username")} · ${t("required")}`} value={value.username} onChange={(username) => setValue({ ...value, username })} placeholder={t("usernamePlaceholder")} width="100%" />
        <TextInput label={t("password")} type="password" value={value.password || ""} onChange={(password) => setValue({ ...value, password })} placeholder={t("passwordPlaceholder")} width="100%" />
        <CheckboxInput label={t("enabled")} value={value.enabled} onChange={(enabled) => setValue({ ...value, enabled })} />
      </LayoutContent>} footer={<LayoutFooter className="settings-dialog-form-footer" padding={3} hasDivider>
        <Button label={t("cancel")} variant="secondary" onClick={onCancel} />
        <Button label={busy ? t("saving") : t("save")} icon={<Save aria-hidden="true" />} variant="primary" type="submit" isLoading={busy} isDisabled={!value.name.trim() || !value.baseUrl.trim() || !value.username.trim() || busy} />
      </LayoutFooter>} />
    </form>
  )
}

function CalendarEditor({ calendar, draft, busy, onCancel, onSubmit, t }: {
  calendar: Calendar | null
  draft: { displayName: string; color: string; enabled: boolean }
  busy: boolean
  onCancel: () => void
  onSubmit: (id: string, input: CalendarUpdate) => Promise<void>
  t: (key: MessageKey, values?: Record<string, string | number>) => string
}) {
  const [value, setValue] = useState(draft)
  useEffect(() => { setValue(draft) }, [draft])
  const canSave = value.displayName.trim() && value.color.trim()
  return (
    <form className="settings-dialog-form" onSubmit={(event) => { event.preventDefault(); if (calendar) void onSubmit(calendar.id, { displayName: value.displayName, color: value.color, enabled: value.enabled }) }}>
      <Layout className="settings-dialog-form-layout" padding={4} header={<DialogHeader title={calendar ? t("editCalendar") : t("newCalendar")} startContent={<span className="settings-dialog-icon"><CalendarDays aria-hidden="true" /></span>} hasDivider onOpenChange={(open) => { if (!open) onCancel() }} />} content={<LayoutContent className="settings-dialog-form-content" padding={4}>
        <TextInput label={`${t("calendarName")} · ${t("required")}`} value={value.displayName} onChange={(displayName) => setValue({ ...value, displayName })} placeholder={t("calendarNamePlaceholder")} width="100%" />
        <TextInput label={t("calendarColor")} value={value.color} onChange={(color) => setValue({ ...value, color })} placeholder="var(--accent)" width="100%" />
        <CheckboxInput label={t("enabled")} value={value.enabled} onChange={(enabled) => setValue({ ...value, enabled })} />
      </LayoutContent>} footer={<LayoutFooter className="settings-dialog-form-footer" padding={3} hasDivider>
        <Button label={t("cancel")} variant="secondary" onClick={onCancel} />
        <Button label={busy ? t("saving") : t("save")} icon={<Save aria-hidden="true" />} variant="primary" type="submit" isLoading={busy} isDisabled={!canSave || busy || !calendar} />
      </LayoutFooter>} />
    </form>
  )
}
