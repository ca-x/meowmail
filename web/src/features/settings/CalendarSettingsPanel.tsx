import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { Calendar as CalendarPicker } from "@astryxdesign/core/Calendar"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { DialogHeader, useImperativeDialog } from "@astryxdesign/core/Dialog"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Layout, LayoutContent, LayoutFooter } from "@astryxdesign/core/Layout"
import { List } from "@astryxdesign/core/List"
import { TextInput } from "@astryxdesign/core/TextInput"
import { CalendarDays, CirclePlus, Download, Pencil, RefreshCw, Save, Trash2 } from "lucide-react"
import { useEffect, useMemo, useState } from "react"

import { api } from "../../app/api"
import type { Calendar, CalendarAccount, CalendarAccountInput, CalendarEvent, CalendarUpdate } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"
import { SettingsPanelHeading } from "./SettingsPanelHeading"
import type { SettingsNotice } from "./settingsTypes"
import type { ISODateString } from "@astryxdesign/core/Calendar"

type AccountDraft = CalendarAccountInput & { id?: string }

const defaultAccount: CalendarAccountInput = {
  name: "",
  baseUrl: "",
  username: "",
  password: "",
  enabled: true,
}

export function CalendarSettingsPanel({ onNotice }: {
  onNotice: (notice: SettingsNotice) => void
}) {
  const { t } = useI18n()
  const accountDialog = useImperativeDialog({ purpose: "form", width: 660, padding: 0 })
  const calendarDialog = useImperativeDialog({ purpose: "form", width: 560, padding: 0 })
  const confirmDialog = useImperativeConfirmDialog()
  const [accounts, setAccounts] = useState<CalendarAccount[]>([])
  const [calendars, setCalendars] = useState<Calendar[]>([])
  const [events, setEvents] = useState<CalendarEvent[]>([])
  const [busy, setBusy] = useState<string | null>(null)
  const [focusDate, setFocusDate] = useState(toISODate(new Date()))
  const [selectedDate, setSelectedDate] = useState(toISODate(new Date()))

  useEffect(() => {
    void refreshAll()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    void refreshEvents(focusDate)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusDate])

  const visibleEvents = useMemo(() => events
    .filter((event) => isSameDay(event.startsAt, selectedDate))
    .sort((left, right) => left.startsAt - right.startsAt), [events, selectedDate])
  const monthLabel = useMemo(() => new Intl.DateTimeFormat(undefined, { month: "long", year: "numeric" }).format(new Date(`${focusDate}T00:00:00`)), [focusDate])
  const selectedLabel = useMemo(() => new Intl.DateTimeFormat(undefined, { weekday: "long", month: "long", day: "numeric" }).format(new Date(`${selectedDate}T00:00:00`)), [selectedDate])

  async function refreshAll() {
    try {
      const [nextAccounts, nextCalendars] = await Promise.all([api.calendarAccounts(), api.calendars()])
      setAccounts(nextAccounts)
      setCalendars(nextCalendars)
    } catch {
      onNotice({ key: "genericError", error: true })
    }
  }

  async function refreshEvents(date = focusDate) {
    const { start, end } = monthBounds(date)
    try {
      setEvents(await api.calendarEvents(new URLSearchParams({ start: String(start), end: String(end) })))
    } catch {
      onNotice({ key: "genericError", error: true })
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
      await refreshAll()
      accountDialog.hide()
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
      await refreshAll()
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
      await refreshAll()
      await refreshEvents()
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
      await refreshAll()
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

          <div className="settings-subsection-divider" />

          <div className="settings-calendar-layout">
            <section className="settings-calendar-card">
              <div className="settings-calendar-heading">
                <div>
                  <strong>{monthLabel}</strong>
                  <small>{t("calendarMonthDescription")}</small>
                </div>
                <Badge variant="blue" label={t("calendarEvents", { count: events.length })} />
              </div>
              <CalendarPicker
                focusDate={focusDate as ISODateString}
                onFocusDateChange={(value) => setFocusDate(value)}
                value={selectedDate as ISODateString}
                onChange={(value) => setSelectedDate(value)}
                hasWeekNumbers
                weekStartsOn="mon"
              />
            </section>
            <section className="settings-calendar-agenda">
              <div className="settings-calendar-heading">
                <div>
                  <strong>{selectedLabel}</strong>
                  <small>{t("calendarAgendaDescription")}</small>
                </div>
                <Button label={t("sync")} icon={<RefreshCw aria-hidden="true" />} variant="ghost" size="sm" onClick={() => void refreshEvents()} />
              </div>
              <List hasDividers density="compact">
                {visibleEvents.length ? visibleEvents.map((event) => (
                  <CalendarEventRow key={event.id} event={event} calendars={calendars} />
                )) : <p className="settings-empty-copy">{t("noCalendarEvents")}</p>}
              </List>
            </section>
          </div>
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

function CalendarEventRow({ event, calendars }: {
  event: CalendarEvent
  calendars: Calendar[]
}) {
  const { locale, t } = useI18n()
  const calendar = calendars.find((item) => item.id === event.calendarId)
  return (
    <div className="settings-inline-row">
      <div className="settings-inline-summary">
        <strong>{event.summary || t("calendarEventNoTitle")}</strong>
        <small>{formatEventRange(event, locale)} · {calendar?.displayName || t("unknownAccount")}</small>
      </div>
      <div className="settings-inline-badges">
        <Badge variant="blue" label={calendar?.displayName || t("unknownAccount")} />
        {event.allDay && <Badge variant="neutral" label={t("allDay")} />}
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

function monthBounds(date: string) {
  const base = new Date(`${date}T00:00:00`)
  const start = new Date(base.getFullYear(), base.getMonth(), 1, 0, 0, 0)
  const end = new Date(base.getFullYear(), base.getMonth() + 1, 0, 23, 59, 59)
  return { start: Math.floor(start.getTime() / 1000), end: Math.floor(end.getTime() / 1000) }
}

function isSameDay(timestamp: number, isoDate: string) {
  const date = new Date(timestamp * 1000)
  const local = toISODate(date)
  return local === isoDate
}

function formatEventRange(event: CalendarEvent, locale: string) {
  const date = new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(new Date(event.startsAt * 1000))
  if (event.allDay) return date
  const time = new Intl.DateTimeFormat(locale, { timeStyle: "short" }).format(new Date(event.startsAt * 1000))
  return `${date} ${time}`
}

function toISODate(date: Date) {
  const pad = (value: number) => String(value).padStart(2, "0")
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}
