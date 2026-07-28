import type { DatesSetArg, DayCellContentArg, EventClickArg, EventContentArg, EventInput } from "@fullcalendar/core"
import zhCnLocale from "@fullcalendar/core/locales/zh-cn"
import dayGridPlugin from "@fullcalendar/daygrid"
import interactionPlugin from "@fullcalendar/interaction"
import FullCalendar from "@fullcalendar/react"
import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { DateTimeInput, type ISODateTimeString } from "@astryxdesign/core/DateTimeInput"
import { IconButton } from "@astryxdesign/core/IconButton"
import { MetadataList, MetadataListItem } from "@astryxdesign/core/MetadataList"
import { Skeleton } from "@astryxdesign/core/Skeleton"
import { TextArea } from "@astryxdesign/core/TextArea"
import { TextInput } from "@astryxdesign/core/TextInput"
import { useToast } from "@astryxdesign/core/Toast"
import { CalendarDays, ChevronLeft, ChevronRight, Clock3, Edit3, Mail, MapPin, Plus, Save, Settings2, Trash2 } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"

import { api } from "../../app/api"
import type {
  Calendar as SyncedCalendar,
  CalendarDayInfo,
  CalendarEvent,
  CalendarPreferences,
  EmailDraft,
  LocalCalendarEvent,
  LocalCalendarEventInput,
} from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"
import { calendarCellFeaturePriority, calendarFeatureLabels, defaultCalendarFeatures } from "./calendarFeatures"

const defaultPreferences: CalendarPreferences = { enabledFeatures: defaultCalendarFeatures }

type WorkspaceEventKind = "caldav" | "local" | "scheduled"

interface WorkspaceEvent {
  key: string
  id: string
  kind: WorkspaceEventKind
  summary: string
  description: string
  location: string
  startsAt: number
  endsAt: number
  allDay: boolean
  color: string
  sourceLabel: string
  draft?: EmailDraft
}

interface LocalEventEditor {
  id: string | null
  summary: string
  description: string
  location: string
  startsAt: ISODateTimeString
  endsAt: ISODateTimeString
  allDay: boolean
}

interface VisibleRange {
  start: Date
  end: Date
}

export function CalendarWorkspace({ revision, drafts, onOpenDraft, onOpenSettings }: {
  revision: number
  drafts: EmailDraft[]
  onOpenDraft: (draft: EmailDraft) => void
  onOpenSettings: () => void
}) {
  const { locale, t } = useI18n()
  const showToast = useToast()
  const deleteDialog = useImperativeConfirmDialog()
  const calendarRef = useRef<FullCalendar | null>(null)
  const today = useMemo(() => toISODate(new Date()), [])
  const [focusDate, setFocusDate] = useState(today)
  const [selectedDate, setSelectedDate] = useState(today)
  const [visibleRange, setVisibleRange] = useState<VisibleRange>(() => initialVisibleRange(new Date()))
  const [preferences, setPreferences] = useState(defaultPreferences)
  const [calendars, setCalendars] = useState<SyncedCalendar[]>([])
  const [calendarEvents, setCalendarEvents] = useState<CalendarEvent[]>([])
  const [localEvents, setLocalEvents] = useState<LocalCalendarEvent[]>([])
  const [monthInfo, setMonthInfo] = useState<CalendarDayInfo[]>([])
  const [selectedInfo, setSelectedInfo] = useState<CalendarDayInfo | null>(null)
  const [monthLoading, setMonthLoading] = useState(true)
  const [detailLoading, setDetailLoading] = useState(true)
  const [editor, setEditor] = useState<LocalEventEditor | null>(null)
  const [editorBusy, setEditorBusy] = useState(false)

  useEffect(() => {
    let active = true
    void Promise.all([api.calendarPreferences(), api.calendars()])
      .then(([nextPreferences, nextCalendars]) => {
        if (!active) return
        setPreferences(nextPreferences)
        setCalendars(nextCalendars)
      })
      .catch(() => undefined)
    return () => { active = false }
  }, [revision])

  useEffect(() => {
    let active = true
    setMonthLoading(true)
    const startDate = toISODate(visibleRange.start)
    const inclusiveEnd = new Date(visibleRange.end.getTime() - 86_400_000)
    const endDate = toISODate(inclusiveEnd)
    const start = Math.floor(visibleRange.start.getTime() / 1000)
    const end = Math.floor(visibleRange.end.getTime() / 1000)
    void Promise.all([
      api.calendarDayInfo(new URLSearchParams({ start: startDate, end: endDate })),
      api.calendarEvents(new URLSearchParams({ start: String(start), end: String(end) })),
      api.localCalendarEvents(new URLSearchParams({ start: String(start), end: String(end) })),
    ]).then(([days, nextCalendarEvents, nextLocalEvents]) => {
      if (!active) return
      setMonthInfo(days)
      setCalendarEvents(nextCalendarEvents)
      setLocalEvents(nextLocalEvents)
    }).catch(() => {
      if (!active) return
      setMonthInfo([])
      setCalendarEvents([])
      setLocalEvents([])
      notify(t("genericError"), "error")
    }).finally(() => {
      if (active) setMonthLoading(false)
    })
    return () => { active = false }
  }, [revision, visibleRange.end, visibleRange.start])

  useEffect(() => {
    let active = true
    setDetailLoading(true)
    void api.calendarDayInfo(new URLSearchParams({ start: selectedDate, end: selectedDate, detail: "true" }))
      .then((days) => {
        if (active) setSelectedInfo(days[0] || null)
      })
      .catch(() => {
        if (active) setSelectedInfo(null)
      })
      .finally(() => {
        if (active) setDetailLoading(false)
      })
    return () => { active = false }
  }, [revision, selectedDate])

  const infoByDate = useMemo(() => new Map(monthInfo.map((day) => [day.date, day])), [monthInfo])
  const workspaceEvents = useMemo<WorkspaceEvent[]>(() => {
    const synced = calendarEvents.map((event): WorkspaceEvent => {
      const calendar = calendars.find((item) => item.id === event.calendarId)
      return {
        key: `caldav:${event.id}`,
        id: event.id,
        kind: "caldav",
        summary: event.summary || t("calendarEventNoTitle"),
        description: event.description,
        location: event.location,
        startsAt: event.startsAt,
        endsAt: event.endsAt,
        allDay: event.allDay,
        color: calendar?.color || "var(--accent)",
        sourceLabel: calendar?.displayName || t("caldavEvent"),
      }
    })
    const local = localEvents.map((event): WorkspaceEvent => ({
      key: `local:${event.id}`,
      id: event.id,
      kind: "local",
      summary: event.summary,
      description: event.description,
      location: event.location,
      startsAt: event.startsAt,
      endsAt: event.endsAt,
      allDay: event.allDay,
      color: "var(--accent)",
      sourceLabel: t("localCalendarEvent"),
    }))
    const scheduled = drafts
      .filter((draft) => draft.scheduledAt && draft.status === "draft")
      .map((draft): WorkspaceEvent => ({
        key: `scheduled:${draft.id}`,
        id: draft.id,
        kind: "scheduled",
        summary: draft.subject || t("noSubject"),
        description: draft.textBody,
        location: "",
        startsAt: draft.scheduledAt as number,
        endsAt: (draft.scheduledAt as number) + 30 * 60,
        allDay: false,
        color: "var(--warning)",
        sourceLabel: t("scheduledMail"),
        draft,
      }))
    return [...synced, ...local, ...scheduled]
  }, [calendarEvents, calendars, drafts, localEvents, t])

  const fullCalendarEvents = useMemo<EventInput[]>(() => workspaceEvents.map((event) => ({
    id: event.key,
    title: event.summary,
    start: new Date(event.startsAt * 1000),
    end: new Date(event.endsAt * 1000),
    allDay: event.allDay,
    extendedProps: { kind: event.kind, sourceId: event.id, color: event.color },
    classNames: [`calendar-fc-event-${event.kind}`],
  })), [workspaceEvents])

  const selectedEvents = useMemo(() => {
    const { start, end } = dayTimestampBounds(selectedDate)
    return workspaceEvents
      .filter((event) => event.startsAt < end && event.endsAt > start)
      .sort((left, right) => left.startsAt - right.startsAt)
  }, [selectedDate, workspaceEvents])
  const enabledDetails = (selectedInfo?.details || []).filter((detail) => preferences.enabledFeatures.includes(detail.feature))
  const monthLabel = useMemo(() => new Intl.DateTimeFormat(locale, { month: "long", year: "numeric" }).format(new Date(`${focusDate}T00:00:00`)), [focusDate, locale])
  const selectedLabel = useMemo(() => new Intl.DateTimeFormat(locale, { weekday: "long", month: "long", day: "numeric" }).format(new Date(`${selectedDate}T00:00:00`)), [locale, selectedDate])

  function notify(body: string, type: "info" | "error" = "info") {
    showToast({ body, type, uniqueID: "calendar-workspace-notice", collisionBehavior: "overwrite" })
  }

  function handleDatesSet(value: DatesSetArg) {
    setFocusDate(toISODate(value.view.currentStart))
    setVisibleRange((current) => current.start.getTime() === value.start.getTime() && current.end.getTime() === value.end.getTime()
      ? current
      : { start: value.start, end: value.end })
  }

  function handleEventClick(value: EventClickArg) {
    value.jsEvent.preventDefault()
    const event = workspaceEvents.find((item) => item.key === value.event.id)
    if (!event) return
    setSelectedDate(toISODate(new Date(event.startsAt * 1000)))
    if (event.kind === "scheduled" && event.draft) {
      onOpenDraft(event.draft)
      return
    }
    if (event.kind === "local") {
      const local = localEvents.find((item) => item.id === event.id)
      if (local) setEditor(editorFromEvent(local))
    }
  }

  function openNewEvent() {
    setEditor(newEventEditor(selectedDate))
  }

  async function saveEvent() {
    if (!editor || editorBusy) return
    const input = eventInput(editor)
    if (!input) {
      notify(t("calendarEventInvalid"), "error")
      return
    }
    setEditorBusy(true)
    try {
      if (editor.id) {
        const updated = await api.updateLocalCalendarEvent(editor.id, input)
        setLocalEvents((items) => items.map((item) => item.id === updated.id ? updated : item))
      } else {
        const created = await api.createLocalCalendarEvent(input)
        setLocalEvents((items) => [...items, created])
      }
      setEditor(null)
      notify(t("calendarEventSaved"))
    } catch {
      notify(t("genericError"), "error")
    } finally {
      setEditorBusy(false)
    }
  }

  async function deleteEvent() {
    if (!editor?.id || editorBusy) return
    const confirmed = await deleteDialog.confirm({
      title: t("deleteCalendarEventTitle"),
      description: t("deleteCalendarEventConfirm"),
      cancelLabel: t("cancel"),
      actionLabel: t("delete"),
      actionVariant: "destructive",
    })
    if (!confirmed) return
    setEditorBusy(true)
    try {
      await api.deleteLocalCalendarEvent(editor.id)
      setLocalEvents((items) => items.filter((item) => item.id !== editor.id))
      setEditor(null)
      notify(t("calendarEventDeleted"))
    } catch {
      notify(t("genericError"), "error")
    } finally {
      setEditorBusy(false)
    }
  }

  function goToday() {
    const date = new Date()
    calendarRef.current?.getApi().today()
    setSelectedDate(toISODate(date))
  }

  return (
    <main className="calendar-workspace" aria-label={t("calendarView")}>
      <header className="calendar-workspace-header">
        <div className="calendar-workspace-title">
          <span className="calendar-workspace-icon" aria-hidden="true"><CalendarDays /></span>
          <div>
            <h1>{monthLabel}</h1>
            <p>{t("calendarEvents", { count: workspaceEvents.length })}</p>
          </div>
        </div>
        <div className="calendar-workspace-actions">
          <Button label={t("calendarToday")} variant="secondary" size="sm" onClick={goToday} />
          <IconButton label={t("previousMonth")} icon={<ChevronLeft aria-hidden="true" />} variant="ghost" size="sm" onClick={() => calendarRef.current?.getApi().prev()} />
          <IconButton label={t("nextMonth")} icon={<ChevronRight aria-hidden="true" />} variant="ghost" size="sm" onClick={() => calendarRef.current?.getApi().next()} />
          <IconButton label={t("openCalendarSettings")} icon={<Settings2 aria-hidden="true" />} variant="ghost" size="sm" onClick={onOpenSettings} />
        </div>
      </header>

      <div className="calendar-workspace-grid">
        <section className="calendar-month-view" aria-label={monthLabel}>
          <div className="calendar-full-view" data-loading={monthLoading || undefined}>
            <FullCalendar
              ref={calendarRef}
              plugins={[dayGridPlugin, interactionPlugin]}
              initialView="dayGridMonth"
              initialDate={today}
              headerToolbar={false}
              locales={[zhCnLocale]}
              locale={locale === "zh-CN" ? "zh-cn" : "en"}
              firstDay={1}
              fixedWeekCount={false}
              showNonCurrentDates
              dayMaxEvents={3}
              height="100%"
              events={fullCalendarEvents}
              datesSet={handleDatesSet}
              dateClick={(value) => {
                setSelectedDate(value.dateStr)
                setEditor(null)
              }}
              eventClick={handleEventClick}
              dayCellClassNames={(value) => toISODate(value.date) === selectedDate ? ["is-selected-date"] : []}
              dayCellContent={(value) => <CalendarDayCell value={value} info={infoByDate.get(toISODate(value.date))} preferences={preferences} />}
              eventContent={(value) => <CalendarEventContent value={value} />}
              moreLinkClick="popover"
              eventDisplay="block"
              displayEventEnd={false}
              nowIndicator
            />
            {monthLoading && <div className="calendar-month-loading" aria-label={t("loading")}><Skeleton width="42%" height={12} /><Skeleton width="74%" height={12} index={1} /></div>}
          </div>
        </section>

        <aside className="calendar-day-panel" aria-label={t("calendarDetails")}>
          <header className="calendar-day-header">
            <div>
              <strong>{selectedLabel}</strong>
              <small>{t("calendarAgendaDescription")}</small>
            </div>
            <Badge variant="blue" label={t("calendarEvents", { count: selectedEvents.length })} />
          </header>

          {detailLoading ? <CalendarDetailSkeleton /> : enabledDetails.length ? (
            <MetadataList className="calendar-detail-list" columns="single" label={{ position: "start", width: 104 }} maxNumOfItems={8}>
              {enabledDetails.map((detail) => (
                <MetadataListItem key={detail.feature} label={t(calendarFeatureLabels.get(detail.feature) || "calendarDetails")}>
                  {detail.values.join(" · ")}
                </MetadataListItem>
              ))}
            </MetadataList>
          ) : <p className="calendar-empty-copy">{t("noCalendarDetails")}</p>}

          <section className="calendar-day-events" aria-label={t("calendarAgendaDescription")}>
            <div className="calendar-day-section-heading">
              <strong>{t("calendarAgendaDescription")}</strong>
              <Button label={t("addCalendarEvent")} icon={<Plus aria-hidden="true" />} variant="secondary" size="sm" onClick={openNewEvent} />
            </div>
            <div className="calendar-event-list">
              {selectedEvents.length ? selectedEvents.map((event) => (
                <CalendarEventRow
                  key={event.key}
                  event={event}
                  onEdit={() => {
                    const local = localEvents.find((item) => item.id === event.id)
                    if (local) setEditor(editorFromEvent(local))
                  }}
                  onOpenDraft={() => event.draft && onOpenDraft(event.draft)}
                />
              )) : <p className="calendar-empty-copy">{t("noCalendarEvents")}</p>}
            </div>
          </section>

          {editor && (
            <section className="calendar-event-editor" aria-label={editor.id ? t("editCalendarEvent") : t("addCalendarEvent")}>
              <header>
                <strong>{editor.id ? t("editCalendarEvent") : t("addCalendarEvent")}</strong>
                <Badge variant="neutral" label={t("localCalendarEvent")} />
              </header>
              <TextInput label={t("calendarEventTitle")} value={editor.summary} onChange={(summary) => setEditor({ ...editor, summary })} placeholder={t("calendarEventTitlePlaceholder")} width="100%" />
              <div className="calendar-event-time-grid">
                <DateTimeInput label={t("calendarEventStarts")} value={editor.startsAt} onChange={(startsAt) => startsAt && setEditor({ ...editor, startsAt })} hourFormat="24h" timeIncrement={5} width="100%" />
                <DateTimeInput label={t("calendarEventEnds")} value={editor.endsAt} onChange={(endsAt) => endsAt && setEditor({ ...editor, endsAt })} min={editor.startsAt} hourFormat="24h" timeIncrement={5} width="100%" />
              </div>
              <CheckboxInput label={t("allDay")} value={editor.allDay} onChange={(allDay) => setEditor({ ...editor, allDay })} labelIcon={<Clock3 aria-hidden="true" />} />
              <TextInput label={t("calendarEventLocation")} value={editor.location} onChange={(location) => setEditor({ ...editor, location })} placeholder={t("calendarEventLocationPlaceholder")} startIcon={<MapPin aria-hidden="true" />} width="100%" />
              <TextArea label={t("calendarEventDescription")} value={editor.description} onChange={(description) => setEditor({ ...editor, description })} rows={3} width="100%" />
              <footer>
                {editor.id && <Button label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="destructive" isLoading={editorBusy} isDisabled={editorBusy} onClick={() => void deleteEvent()} />}
                <span />
                <Button label={t("cancel")} variant="ghost" isDisabled={editorBusy} onClick={() => setEditor(null)} />
                <Button label={t("save")} icon={<Save aria-hidden="true" />} variant="primary" isLoading={editorBusy} isDisabled={editorBusy || !editor.summary.trim()} onClick={() => void saveEvent()} />
              </footer>
            </section>
          )}
        </aside>
      </div>
      {deleteDialog.element}
    </main>
  )
}

function CalendarDayCell({ value, info, preferences }: {
  value: DayCellContentArg
  info?: CalendarDayInfo
  preferences: CalendarPreferences
}) {
  const detail = calendarCellFeaturePriority
    .filter((feature) => preferences.enabledFeatures.includes(feature))
    .map((feature) => info?.details.find((item) => item.feature === feature))
    .find((item) => item?.shortValue)
  const accent = detail && detail.feature !== "lunarDate"
  return (
    <span className="calendar-day-cell-label">
      <span>{value.dayNumberText}</span>
      {detail?.shortValue && <small data-tone={accent ? "accent" : "neutral"}>{detail.shortValue}</small>}
    </span>
  )
}

function CalendarEventContent({ value }: { value: EventContentArg }) {
  const kind = value.event.extendedProps.kind as WorkspaceEventKind
  const color = String(value.event.extendedProps.color || "var(--accent)")
  const Icon = kind === "scheduled" ? Mail : kind === "local" ? Edit3 : CalendarDays
  return (
    <span className="calendar-fc-event-content">
      <span className="calendar-fc-event-dot" style={{ background: color }} aria-hidden="true" />
      <Icon aria-hidden="true" />
      <span>{value.timeText && <time>{value.timeText}</time>}{value.event.title}</span>
    </span>
  )
}

function CalendarDetailSkeleton() {
  return (
    <div className="calendar-detail-skeleton" aria-hidden="true">
      {[0, 1, 2, 3].map((index) => (
        <div key={index}>
          <Skeleton width="28%" height={11} index={index} />
          <Skeleton width={`${72 - index * 7}%`} height={12} index={index + 1} />
        </div>
      ))}
    </div>
  )
}

function CalendarEventRow({ event, onEdit, onOpenDraft }: {
  event: WorkspaceEvent
  onEdit: () => void
  onOpenDraft: () => void
}) {
  const { locale, t } = useI18n()
  return (
    <div className="calendar-event-row">
      <span className="calendar-event-color" style={{ background: event.color }} aria-hidden="true" />
      <div>
        <strong>{event.summary}</strong>
        <small>{formatEventRange(event, locale)} · {event.sourceLabel}</small>
        {event.location && <small><MapPin aria-hidden="true" />{event.location}</small>}
      </div>
      {event.kind === "local"
        ? <IconButton label={t("editCalendarEvent")} icon={<Edit3 aria-hidden="true" />} variant="ghost" size="sm" onClick={onEdit} />
        : event.kind === "scheduled"
          ? <IconButton label={t("openScheduledDraft")} icon={<Mail aria-hidden="true" />} variant="ghost" size="sm" onClick={onOpenDraft} />
          : event.allDay ? <Badge variant="neutral" label={t("allDay")} /> : null}
    </div>
  )
}

function initialVisibleRange(date: Date): VisibleRange {
  return {
    start: new Date(date.getFullYear(), date.getMonth(), 1),
    end: new Date(date.getFullYear(), date.getMonth() + 1, 1),
  }
}

function newEventEditor(date: string): LocalEventEditor {
  const now = new Date()
  let start = new Date(`${date}T09:00:00`)
  if (date === toISODate(now) && now > start) {
    start = new Date(now)
    start.setSeconds(0, 0)
    start.setMinutes(Math.ceil((start.getMinutes() + 1) / 30) * 30)
  }
  const end = new Date(start.getTime() + 60 * 60 * 1000)
  return {
    id: null,
    summary: "",
    description: "",
    location: "",
    startsAt: toLocalDateTime(start),
    endsAt: toLocalDateTime(end),
    allDay: false,
  }
}

function editorFromEvent(event: LocalCalendarEvent): LocalEventEditor {
  return {
    id: event.id,
    summary: event.summary,
    description: event.description,
    location: event.location,
    startsAt: toLocalDateTime(new Date(event.startsAt * 1000)),
    endsAt: toLocalDateTime(new Date(event.endsAt * 1000)),
    allDay: event.allDay,
  }
}

function eventInput(editor: LocalEventEditor): LocalCalendarEventInput | null {
  const startsAt = Math.floor(new Date(editor.startsAt).getTime() / 1000)
  const endsAt = Math.floor(new Date(editor.endsAt).getTime() / 1000)
  if (!editor.summary.trim() || !Number.isFinite(startsAt) || !Number.isFinite(endsAt) || endsAt <= startsAt) return null
  return {
    summary: editor.summary.trim(),
    description: editor.description.trim(),
    location: editor.location.trim(),
    startsAt,
    endsAt,
    allDay: editor.allDay,
  }
}

function dayTimestampBounds(date: string) {
  const start = new Date(`${date}T00:00:00`).getTime() / 1000
  return { start, end: start + 86_400 }
}

function formatEventRange(event: WorkspaceEvent, locale: string) {
  const start = new Date(event.startsAt * 1000)
  if (event.allDay) return new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(start)
  const end = new Date(event.endsAt * 1000)
  const date = new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(start)
  const time = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" })
  return `${date} ${time.format(start)}–${time.format(end)}`
}

function toLocalDateTime(date: Date) {
  const pad = (value: number) => String(value).padStart(2, "0")
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}` as ISODateTimeString
}

function toISODate(date: Date) {
  const pad = (value: number) => String(value).padStart(2, "0")
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}
