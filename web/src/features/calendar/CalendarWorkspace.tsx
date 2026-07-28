import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { Calendar } from "@astryxdesign/core/Calendar"
import type { ISODateString } from "@astryxdesign/core/Calendar"
import { IconButton } from "@astryxdesign/core/IconButton"
import { List } from "@astryxdesign/core/List"
import { Skeleton } from "@astryxdesign/core/Skeleton"
import { useMediaQuery } from "@astryxdesign/core/hooks"
import { CalendarDays, Settings2 } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"

import { api } from "../../app/api"
import type { Calendar as SyncedCalendar, CalendarDayInfo, CalendarEvent, CalendarPreferences } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { calendarCellFeaturePriority, calendarFeatureLabels, defaultCalendarFeatures } from "./calendarFeatures"

const defaultPreferences: CalendarPreferences = { enabledFeatures: defaultCalendarFeatures }

export function CalendarWorkspace({ revision, onOpenSettings }: {
  revision: number
  onOpenSettings: () => void
}) {
  const { locale, t } = useI18n()
  const isCompactCalendar = useMediaQuery("(max-width: 520px)")
  const calendarRef = useRef<HTMLDivElement>(null)
  const [focusDate, setFocusDate] = useState(toISODate(new Date()))
  const [selectedDate, setSelectedDate] = useState(toISODate(new Date()))
  const [preferences, setPreferences] = useState(defaultPreferences)
  const [calendars, setCalendars] = useState<SyncedCalendar[]>([])
  const [events, setEvents] = useState<CalendarEvent[]>([])
  const [monthInfo, setMonthInfo] = useState<CalendarDayInfo[]>([])
  const [selectedInfo, setSelectedInfo] = useState<CalendarDayInfo | null>(null)
  const [monthLoading, setMonthLoading] = useState(true)
  const [detailLoading, setDetailLoading] = useState(true)

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
    const dates = monthDateBounds(focusDate)
    const timestamps = monthTimestampBounds(focusDate)
    void Promise.all([
      api.calendarDayInfo(new URLSearchParams({ start: dates.start, end: dates.end })),
      api.calendarEvents(new URLSearchParams({ start: String(timestamps.start), end: String(timestamps.end) })),
    ]).then(([days, nextEvents]) => {
      if (!active) return
      setMonthInfo(days)
      setEvents(nextEvents)
    }).catch(() => {
      if (!active) return
      setMonthInfo([])
      setEvents([])
    }).finally(() => {
      if (active) setMonthLoading(false)
    })
    return () => { active = false }
  }, [focusDate, revision])

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

  const visibleEvents = useMemo(() => events
    .filter((event) => isSameDay(event.startsAt, selectedDate))
    .sort((left, right) => left.startsAt - right.startsAt), [events, selectedDate])
  const monthLabel = useMemo(() => new Intl.DateTimeFormat(locale, { month: "long", year: "numeric" }).format(new Date(`${focusDate}T00:00:00`)), [focusDate, locale])
  const selectedLabel = useMemo(() => new Intl.DateTimeFormat(locale, { weekday: "long", month: "long", day: "numeric" }).format(new Date(`${selectedDate}T00:00:00`)), [locale, selectedDate])

  useEffect(() => {
    const root = calendarRef.current
    if (!root) return
    const infoByDate = new Map(monthInfo.map((day) => [day.date, day]))
    const eventCounts = events.reduce((counts, event) => {
      const date = toISODate(new Date(event.startsAt * 1000))
      counts.set(date, (counts.get(date) || 0) + 1)
      return counts
    }, new Map<string, number>())
    const buttons = root.querySelectorAll<HTMLButtonElement>("button[data-date]")
    buttons.forEach((button) => {
      const date = button.dataset.date || ""
      const day = infoByDate.get(date)
      const shortDetail = calendarCellFeaturePriority
        .map((feature) => day?.details.find((detail) => detail.feature === feature))
        .find((detail) => detail?.shortValue)
      if (button.dataset.baseDate !== date) {
        button.dataset.baseDate = date
        button.dataset.baseAriaLabel = button.getAttribute("aria-label") || date
      }
      if (shortDetail?.shortValue) {
        button.dataset.calendarLabel = shortDetail.shortValue
        button.dataset.calendarTone = shortDetail.feature === "lunarDate" ? "neutral" : "accent"
        button.setAttribute("aria-label", `${button.dataset.baseAriaLabel}; ${shortDetail.shortValue}`)
      } else {
        delete button.dataset.calendarLabel
        delete button.dataset.calendarTone
        button.setAttribute("aria-label", button.dataset.baseAriaLabel || date)
      }
      const count = eventCounts.get(date) || 0
      if (count > 0) button.dataset.eventCount = String(count)
      else delete button.dataset.eventCount
    })
  }, [events, focusDate, monthInfo])

  const enabledDetails = (selectedInfo?.details || []).filter((detail) => preferences.enabledFeatures.includes(detail.feature))

  return (
    <main className="calendar-workspace" aria-label={t("calendarView")}>
      <header className="calendar-workspace-header">
        <div className="calendar-workspace-title">
          <span className="calendar-workspace-icon" aria-hidden="true"><CalendarDays /></span>
          <div>
            <h1>{monthLabel}</h1>
            <p>{t("calendarEvents", { count: events.length })}</p>
          </div>
        </div>
        <div className="calendar-workspace-actions">
          <Button label={t("calendarToday")} variant="secondary" size="sm" onClick={() => {
            const today = toISODate(new Date())
            setFocusDate(today)
            setSelectedDate(today)
          }} />
          <IconButton label={t("openCalendarSettings")} icon={<Settings2 aria-hidden="true" />} variant="ghost" size="sm" onClick={onOpenSettings} />
        </div>
      </header>

      <div className="calendar-workspace-grid">
        <section className="calendar-month-view" aria-label={monthLabel}>
          <div className="calendar-month-picker" ref={calendarRef} data-loading={monthLoading || undefined}>
            <Calendar
              focusDate={focusDate as ISODateString}
              onFocusDateChange={(value) => setFocusDate(value)}
              value={selectedDate as ISODateString}
              onChange={(value) => setSelectedDate(value)}
              hasWeekNumbers={!isCompactCalendar}
              weekStartsOn="mon"
            />
          </div>
        </section>

        <aside className="calendar-day-panel" aria-label={t("calendarDetails")}>
          <header className="calendar-day-header">
            <div>
              <strong>{selectedLabel}</strong>
              <small>{t("calendarDetails")}</small>
            </div>
            <Badge variant="blue" label={t("calendarEvents", { count: visibleEvents.length })} />
          </header>

          {detailLoading ? <CalendarDetailSkeleton /> : enabledDetails.length ? (
            <dl className="calendar-detail-list">
              {enabledDetails.map((detail) => (
                <div key={detail.feature} className="calendar-detail-row">
                  <dt>{t(calendarFeatureLabels.get(detail.feature) || "calendarDetails")}</dt>
                  <dd>{detail.values.join(" · ")}</dd>
                </div>
              ))}
            </dl>
          ) : <p className="calendar-empty-copy">{t("noCalendarDetails")}</p>}

          <section className="calendar-day-events" aria-label={t("calendarAgendaDescription")}>
            <div className="calendar-day-section-heading"><strong>{t("calendarAgendaDescription")}</strong></div>
            <List hasDividers density="compact">
              {visibleEvents.length ? visibleEvents.map((event) => (
                <CalendarEventRow key={event.id} event={event} calendars={calendars} />
              )) : <p className="calendar-empty-copy">{t("noCalendarEvents")}</p>}
            </List>
          </section>
        </aside>
      </div>
    </main>
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

function CalendarEventRow({ event, calendars }: {
  event: CalendarEvent
  calendars: SyncedCalendar[]
}) {
  const { locale, t } = useI18n()
  const calendar = calendars.find((item) => item.id === event.calendarId)
  return (
    <div className="calendar-event-row">
      <span className="calendar-event-color" style={{ background: calendar?.color || "var(--accent)" }} aria-hidden="true" />
      <div>
        <strong>{event.summary || t("calendarEventNoTitle")}</strong>
        <small>{formatEventRange(event, locale)}</small>
      </div>
      {event.allDay && <Badge variant="neutral" label={t("allDay")} />}
    </div>
  )
}

function monthDateBounds(date: string) {
  const base = new Date(`${date}T00:00:00`)
  return {
    start: toISODate(new Date(base.getFullYear(), base.getMonth(), 1)),
    end: toISODate(new Date(base.getFullYear(), base.getMonth() + 1, 0)),
  }
}

function monthTimestampBounds(date: string) {
  const base = new Date(`${date}T00:00:00`)
  const start = new Date(base.getFullYear(), base.getMonth(), 1, 0, 0, 0)
  const end = new Date(base.getFullYear(), base.getMonth() + 1, 0, 23, 59, 59)
  return { start: Math.floor(start.getTime() / 1000), end: Math.floor(end.getTime() / 1000) }
}

function isSameDay(timestamp: number, isoDate: string) {
  return toISODate(new Date(timestamp * 1000)) === isoDate
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
