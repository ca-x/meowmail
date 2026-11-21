import { Avatar } from "@astryxdesign/core/Avatar"
import { Button } from "@astryxdesign/core/Button"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { EmptyState } from "@astryxdesign/core/EmptyState"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Item } from "@astryxdesign/core/Item"
import { List } from "@astryxdesign/core/List"
import { Skeleton } from "@astryxdesign/core/Skeleton"
import { ChevronDown, ChevronRight, Inbox, Megaphone, MessagesSquare, Paperclip, Search, Star } from "lucide-react"
import { useMemo, useState } from "react"

import { defaultMailPreferences } from "../../app/mailPreferences"
import type { MailPreferences, MessageSummary } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function MessageList({ messages, query = "", onClearSearch, selectedId, selectedIds = new Set(), loading, preferences = defaultMailPreferences, selectionDisabled = false, onSelect, onToggleStar, onToggleSelection }: {
  messages: MessageSummary[]
  query?: string
  onClearSearch?: () => void
  selectedId: string | null
  selectedIds?: Set<string>
  loading: boolean
  preferences?: MailPreferences
  selectionDisabled?: boolean
  onSelect: (message: MessageSummary) => void
  onToggleStar: (message: MessageSummary) => void
  onToggleSelection?: (id: string, selected?: boolean) => void
}) {
  const { locale, t } = useI18n()
  const formatters = useMemo(() => ({
    relative: new Intl.RelativeTimeFormat(locale, { numeric: "auto" }),
    date: new Intl.DateTimeFormat(locale, { month: "short", day: "numeric" }),
    size: new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }),
  }), [locale])
  const [promotionsExpanded, setPromotionsExpanded] = useState(false)
  const promotional = useMemo(() => messages.filter((message) => message.isPromotional), [messages])
  const visibleMessages = useMemo(
    () => preferences.aggregatePromotions && promotional.length > 1 && !promotionsExpanded
      ? messages.filter((message) => !message.isPromotional)
      : messages,
    [messages, preferences.aggregatePromotions, promotional.length, promotionsExpanded],
  )
  const groups = useMemo(
    () => groupMessages(visibleMessages, preferences.conversationMode),
    [preferences.conversationMode, visibleMessages],
  )

  if (loading) return <MessageListSkeleton label={t("loading")} />
  if (!messages.length) {
    return (
      <div className="message-list-empty">
        <EmptyState
          isCompact
          icon={query ? <Search aria-hidden="true" /> : <Inbox aria-hidden="true" />}
          title={query ? t("noSearchResults") : t("noMail")}
          description={query ? t("noSearchResultsDescription", { query }) : t("noMailDescription")}
          actions={query && onClearSearch ? <Button label={t("clearMailSearch")} variant="secondary" onClick={onClearSearch} /> : undefined}
        />
      </div>
    )
  }

  return (
    <div className="message-list-scroll" data-testid="message-list-scroll">
      {preferences.aggregatePromotions && promotional.length > 1 && (
        <div className="promotion-group-row">
          <Button
            label={t("promotionalMail")}
            icon={<Megaphone aria-hidden="true" />}
            endContent={
              <span className="promotion-group-end">
                <span>{t("promotionalMailCount", { count: promotional.length })}</span>
                {promotionsExpanded ? <ChevronDown aria-hidden="true" /> : <ChevronRight aria-hidden="true" />}
              </span>
            }
            variant="ghost"
            width="100%"
            onClick={() => setPromotionsExpanded((value) => !value)}
            aria-expanded={promotionsExpanded}
          />
        </div>
      )}

      <List className="message-list" density={preferences.listDensity === "compact" ? "compact" : "balanced"} hasDividers>
        {groups.map(({ message, count }, index) => {
          const isSelected = message.id === selectedId
          const isChecked = selectedIds.has(message.id)
          return (
            <Item
              key={message.id}
              as="li"
              role="listitem"
              className={`message-item ${message.isRead ? "is-read" : "is-unread"}${isChecked ? " is-checked" : ""}`}
              aria-current={isSelected ? "true" : undefined}
              tabIndex={isSelected || (!selectedId && index === 0) ? 0 : -1}
              align="start"
              density={preferences.listDensity === "compact" ? "compact" : "balanced"}
              isHighlighted={isSelected}
              onClick={() => onSelect(message)}
              onKeyDown={(event) => {
                if (event.target !== event.currentTarget) return
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault()
                  onSelect(message)
                }
              }}
              startContent={
                <span className="message-item-start">
                  <CheckboxInput
                    label={t("selectMessage")}
                    isLabelHidden
                    value={isChecked}
                    isDisabled={selectionDisabled}
                    onChange={(checked) => onToggleSelection?.(message.id, checked)}
                    onClick={(event) => event.stopPropagation()}
                    onKeyDown={(event) => event.stopPropagation()}
                  />
                  <Avatar name={message.senderName || message.senderEmail} size="md" />
                </span>
              }
              label={
                <span className="message-item-heading">
                  <strong>{message.senderName || message.senderEmail}</strong>
                  <time dateTime={new Date(message.receivedAt * 1000).toISOString()}>{relativeTime(message.receivedAt, formatters.relative, formatters.date)}</time>
                </span>
              }
              description={
                <span className="message-item-copy">
                  <span className="message-item-subject">
                    <span>{message.subject || t("noSubject")}</span>
                    {count > 1 && <span className="conversation-count"><MessagesSquare aria-hidden="true" />{count}</span>}
                    {message.attachmentCount > 0 && <Paperclip aria-label={t("attachments")} />}
                  </span>
                  {preferences.showSummary && <span className="message-item-preview">{message.preview}</span>}
                  {(preferences.showMessageSize || (preferences.showAttachmentPreview && message.attachmentCount > 0)) && (
                    <span className="message-item-meta">
                      {preferences.showMessageSize && <span>{formatFileSize(message.rawSize, formatters.size)}</span>}
                      {preferences.showAttachmentPreview && message.attachmentCount > 0 && <span><Paperclip aria-hidden="true" />{t("attachmentCount", { count: message.attachmentCount })}</span>}
                    </span>
                  )}
                </span>
              }
              endContent={
                <IconButton
                  className={`message-item-star ${message.isStarred ? "is-starred" : ""}`}
                  aria-pressed={message.isStarred}
                  label={message.isStarred ? t("unstar") : t("star")}
                  icon={<Star fill={message.isStarred ? "currentColor" : "none"} aria-hidden="true" />}
                  variant="ghost"
                  size="sm"
                  onClick={(event) => {
                    event.stopPropagation()
                    onToggleStar(message)
                  }}
                  onKeyDown={(event) => event.stopPropagation()}
                />
              }
            />
          )
        })}
      </List>
    </div>
  )
}

function MessageListSkeleton({ label }: { label: string }) {
  return (
    <div className="message-list-skeleton" aria-label={label} aria-busy="true">
      {Array.from({ length: 7 }, (_, index) => (
        <div className="message-skeleton-row" key={index}>
          <Skeleton width={36} height={36} radius="rounded" index={index} />
          <span>
            <Skeleton width="42%" height={12} index={index} />
            <Skeleton width="78%" height={11} index={index + 1} />
            <Skeleton width="62%" height={10} index={index + 2} />
          </span>
        </div>
      ))}
    </div>
  )
}

function groupMessages(messages: MessageSummary[], enabled: boolean) {
  if (!enabled) return messages.map((message) => ({ message, count: 1 }))
  const groups = new Map<string, { message: MessageSummary; count: number }>()
  for (const message of messages) {
    const key = message.threadKey ? `${message.accountId}:${message.threadKey}` : message.id
    const current = groups.get(key)
    if (current) current.count += 1
    else groups.set(key, { message, count: 1 })
  }
  return [...groups.values()]
}

function formatFileSize(size: number, formatter: Intl.NumberFormat) {
  const units = ["B", "KB", "MB", "GB"]
  let value = Math.max(0, size)
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${formatter.format(value)} ${units[unit]}`
}

function relativeTime(timestamp: number, formatter: Intl.RelativeTimeFormat, dateFormat: Intl.DateTimeFormat) {
  const difference = timestamp * 1000 - Date.now()
  const minute = 60_000
  const hour = 60 * minute
  const day = 24 * hour
  if (Math.abs(difference) < hour) return formatter.format(Math.round(difference / minute), "minute")
  if (Math.abs(difference) < day) return formatter.format(Math.round(difference / hour), "hour")
  if (Math.abs(difference) < 7 * day) return formatter.format(Math.round(difference / day), "day")
  return dateFormat.format(timestamp * 1000)
}
