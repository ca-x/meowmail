import { useMemo, useState } from "react"
import { ChevronDown, ChevronRight, Inbox, Megaphone, MessagesSquare, Paperclip, Star } from "lucide-react"

import { defaultMailPreferences } from "../../app/mailPreferences"
import type { MailPreferences, MessageSummary } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function MessageList({ messages, selectedId, loading, preferences = defaultMailPreferences, onSelect, onToggleStar }: {
  messages: MessageSummary[]
  selectedId: string | null
  loading: boolean
  preferences?: MailPreferences
  onSelect: (message: MessageSummary) => void
  onToggleStar: (message: MessageSummary) => void
}) {
  const { locale, t } = useI18n()
  const [promotionsExpanded, setPromotionsExpanded] = useState(false)
  const promotional = useMemo(() => messages.filter((message) => message.isPromotional), [messages])
  const visibleMessages = useMemo(
    () => preferences.aggregatePromotions && promotional.length > 1 && !promotionsExpanded
      ? messages.filter((message) => !message.isPromotional)
      : messages,
    [messages, preferences.aggregatePromotions, promotional.length, promotionsExpanded],
  )
  const groups = useMemo(() => groupMessages(visibleMessages, preferences.conversationMode), [visibleMessages, preferences.conversationMode])
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
      {preferences.aggregatePromotions && promotional.length > 1 && (
        <button className={`promotion-group ${promotionsExpanded ? "expanded" : ""}`} type="button" onClick={() => setPromotionsExpanded((value) => !value)} aria-expanded={promotionsExpanded}>
          <span className="promotion-icon"><Megaphone size={17} /></span>
          <span><strong>{t("promotionalMail")}</strong><small>{t("promotionalMailCount", { count: promotional.length })}</small></span>
          {promotionsExpanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
        </button>
      )}
      {groups.map(({ message, count }, index) => (
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
              {count > 1 && <span className="conversation-count"><MessagesSquare size={12} />{count}</span>}
              {message.attachmentCount > 0 && <Paperclip size={13} aria-label={t("attachments")} />}
            </div>
            {preferences.showSummary && <p>{message.preview}</p>}
            {(preferences.showMessageSize || (preferences.showAttachmentPreview && message.attachmentCount > 0)) && <div className="message-row-meta">{preferences.showMessageSize && <span>{formatFileSize(message.rawSize, locale)}</span>}{preferences.showAttachmentPreview && message.attachmentCount > 0 && <span><Paperclip size={11} />{t("attachmentCount", { count: message.attachmentCount })}</span>}</div>}
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

function groupMessages(messages: MessageSummary[], enabled: boolean) {
  if (!enabled) return messages.map((message) => ({ message, count: 1 }))
  const groups = new Map<string, { message: MessageSummary; count: number }>()
  for (const message of messages) {
    const key = `${message.accountId}:${message.threadKey}`
    const current = groups.get(key)
    if (current) current.count += 1
    else groups.set(key, { message, count: 1 })
  }
  return [...groups.values()]
}

function formatFileSize(size: number, locale: string) {
  const units = ["B", "KB", "MB", "GB"]
  let value = Math.max(0, size)
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: unit ? 1 : 0 }).format(value)} ${units[unit]}`
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
