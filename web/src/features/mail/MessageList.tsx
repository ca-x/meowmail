import { Inbox, Paperclip, Star } from "lucide-react"

import type { MessageSummary } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function MessageList({ messages, selectedId, loading, onSelect, onToggleStar }: {
  messages: MessageSummary[]
  selectedId: string | null
  loading: boolean
  onSelect: (message: MessageSummary) => void
  onToggleStar: (message: MessageSummary) => void
}) {
  const { locale, t } = useI18n()
  if (loading) {
    return <div className="message-skeletons" aria-label={t("loading")}>{Array.from({ length: 7 }, (_, index) => <div className="message-skeleton" key={index}><i /><span><b /><b /><b /></span></div>)}</div>
  }
  if (!messages.length) {
    return (
      <div className="column-empty">
        <span className="empty-icon"><Inbox size={24} /></span>
        <h3>{t("noMail")}</h3>
        <p>{t("noMailDescription")}</p>
      </div>
    )
  }
  return (
    <div className="message-list" role="list" aria-label={t("inbox")}>
      {messages.map((message, index) => (
        <article
          key={message.id}
          className={`message-row ${message.id === selectedId ? "selected" : ""} ${message.isRead ? "read" : "unread"}`}
          role="listitem"
          aria-current={message.id === selectedId || undefined}
          tabIndex={message.id === selectedId || (!selectedId && index === 0) ? 0 : -1}
          onClick={() => onSelect(message)}
          onKeyDown={(event) => {
            if (event.target !== event.currentTarget) return
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault()
              onSelect(message)
            }
          }}
        >
          <span className="unread-marker" aria-hidden="true" />
          <Avatar name={message.senderName || message.senderEmail} />
          <div className="message-row-content">
            <div className="message-row-top">
              <strong>{message.senderName || message.senderEmail}</strong>
              <time dateTime={new Date(message.receivedAt * 1000).toISOString()}>{relativeTime(message.receivedAt, locale)}</time>
            </div>
            <div className="message-subject-line">
              <span>{message.subject || t("noSubject")}</span>
              {message.attachmentCount > 0 && <Paperclip size={13} aria-label={t("attachments")} />}
            </div>
            <p>{message.preview}</p>
          </div>
          <button
            className={`row-star ${message.isStarred ? "active" : ""}`}
            type="button"
            onClick={(event) => { event.stopPropagation(); onToggleStar(message) }}
            aria-label={message.isStarred ? t("unstar") : t("star")}
          >
            <Star size={16} fill={message.isStarred ? "currentColor" : "none"} />
          </button>
        </article>
      ))}
    </div>
  )
}

function Avatar({ name }: { name: string }) {
  const value = [...name.trim()][0]?.toUpperCase() || "M"
  const hue = [...name].reduce((total, character) => total + character.codePointAt(0)!, 0) % 360
  return <span className="sender-avatar" style={{ "--avatar-hue": hue } as React.CSSProperties}>{value}</span>
}

function relativeTime(timestamp: number, locale: string) {
  const difference = timestamp * 1000 - Date.now()
  const minute = 60_000
  const hour = 60 * minute
  const day = 24 * hour
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" })
  if (Math.abs(difference) < hour) return formatter.format(Math.round(difference / minute), "minute")
  if (Math.abs(difference) < day) return formatter.format(Math.round(difference / hour), "hour")
  if (Math.abs(difference) < 7 * day) return formatter.format(Math.round(difference / day), "day")
  return new Intl.DateTimeFormat(locale, { month: "short", day: "numeric" }).format(new Date(timestamp * 1000))
}
