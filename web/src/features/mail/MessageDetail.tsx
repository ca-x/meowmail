import { Avatar } from "@astryxdesign/core/Avatar"
import { Button } from "@astryxdesign/core/Button"
import { EmptyState } from "@astryxdesign/core/EmptyState"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Item } from "@astryxdesign/core/Item"
import { List } from "@astryxdesign/core/List"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { Spinner } from "@astryxdesign/core/Spinner"
import { Toolbar } from "@astryxdesign/core/Toolbar"
import { ArrowLeft, CornerUpLeft, Download, Eye, FileText, Forward, MailOpen, ShieldCheck, Star, Trash2 } from "lucide-react"
import { useEffect, useMemo, useState } from "react"

import { api } from "../../app/api"
import type { MailAttachment, MailPreferences, MessageDetail as Detail } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { useTheme } from "../../theme/ThemeProvider"
import { AttachmentPreviewDialog } from "./AttachmentPreviewDialog"

export function MessageDetail({ message, thread, loading, isDeleting = false, preferences, onBack, onToggleStar, onToggleRead, onReply, onForward, onDelete }: {
  message: Detail | null
  thread: Detail[]
  loading: boolean
  isDeleting?: boolean
  preferences: MailPreferences
  onBack: () => void
  onToggleStar: () => void
  onToggleRead: () => void
  onReply: () => void
  onForward: () => void
  onDelete: () => void
}) {
  const { locale, t } = useI18n()
  const { resolved: themeMode } = useTheme()
  const [view, setView] = useState<"html" | "text">("html")
  const [previewAttachment, setPreviewAttachment] = useState<MailAttachment | null>(null)
  const srcDoc = useMemo(() => message?.bodyHtml ? emailDocument(message.bodyHtml, themeMode) : "", [message?.bodyHtml, themeMode])

  useEffect(() => {
    setView(preferences.plainTextReading ? "text" : "html")
    setPreviewAttachment(null)
  }, [message?.id, preferences.plainTextReading])

  if (loading) return <div className="detail-loading"><Spinner size="xl" label={t("loading")} /></div>
  if (!message) {
    return (
      <div className="detail-empty">
        <EmptyState
          icon={<MailOpen aria-hidden="true" />}
          title={t("selectMail")}
          description={t("selectMailDescription")}
        />
      </div>
    )
  }

  return (
    <article className="mail-detail">
      <Toolbar
        className="detail-toolbar"
        label={t("messageActions")}
        size="sm"
        dividers={["bottom"]}
        startContent={<IconButton className="mobile-back" label={t("back")} icon={<ArrowLeft aria-hidden="true" />} variant="ghost" onClick={onBack} />}
        endContent={
          <>
            <IconButton label={message.isRead ? t("markUnread") : t("markRead")} icon={<MailOpen aria-hidden="true" />} variant="ghost" onClick={onToggleRead} />
            <IconButton className={message.isStarred ? "star-active" : undefined} label={message.isStarred ? t("unstar") : t("star")} icon={<Star fill={message.isStarred ? "currentColor" : "none"} aria-hidden="true" />} variant="ghost" onClick={onToggleStar} />
            <IconButton className="danger-text" label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="ghost" isDisabled={isDeleting} onClick={onDelete} />
          </>
        }
      />

      <div className="detail-scroll">
        <div className="detail-content">
          <header className="message-heading">
            <h1>{message.subject || t("noSubject")}</h1>
            <div className="sender-block">
              <Avatar name={message.senderName || message.senderEmail} size="lg" />
              <div className="sender-meta">
                <div><strong>{message.senderName || message.senderEmail}</strong>{message.senderName && <span>&lt;{message.senderEmail}&gt;</span>}</div>
                <p>{t("to")}: {message.recipients.join(", ")}</p>
              </div>
              <time>{formatDate(message.receivedAt, locale)}</time>
            </div>
          </header>

          {preferences.conversationMode && thread.length > 1 && (
            <section className="conversation-thread" aria-label={t("conversationThread")}>
              <header><span>{t("conversationThread")}</span><small>{t("conversationMessageCount", { count: thread.length })}</small></header>
              {thread.filter((item) => item.id !== message.id).map((item) => (
                <details className="conversation-message" key={item.id}>
                  <summary><Avatar name={item.senderName || item.senderEmail} size="sm" /><span><strong>{item.senderName || item.senderEmail}</strong><small>{item.preview}</small></span><time>{formatDate(item.receivedAt, locale)}</time></summary>
                  <pre>{item.bodyText || item.preview}</pre>
                </details>
              ))}
            </section>
          )}

          {message.bodyHtml && !preferences.plainTextReading && (
            <div className="mail-view-switch">
              <SegmentedControl value={view} onChange={(value) => setView(value as "html" | "text")} label={t("readingFormat")} size="sm">
                <SegmentedControlItem value="html" label={t("showHtml")} />
                <SegmentedControlItem value="text" label={t("showText")} />
              </SegmentedControl>
              <span><ShieldCheck aria-hidden="true" />{t("remoteImagesBlocked")}</span>
            </div>
          )}

          <div className="message-body" lang={locale}>
            {view === "html" && message.bodyHtml
              ? <iframe title={message.subject || t("noSubject")} sandbox="" srcDoc={srcDoc} />
              : <pre className="mail-reading-text">{message.bodyText || message.preview}</pre>}
          </div>

          {message.attachmentCount > 0 && (
            <section className="attachment-section" aria-labelledby="attachment-section-title">
              <header>
                <div><FileText aria-hidden="true" /><h2 id="attachment-section-title">{t("attachmentFiles")}</h2></div>
                <span>{t("attachmentCount", { count: message.attachmentCount })}</span>
              </header>
              {message.attachments.length ? (
                <List className="attachment-list" hasDividers density="compact">
                  {message.attachments.map((attachment) => (
                    <Item
                      key={attachment.id}
                      as="li"
                      align="start"
                      density="compact"
                      startContent={<span className="attachment-file-icon"><FileText aria-hidden="true" /></span>}
                      label={<span className="attachment-name">{attachment.filename}</span>}
                      description={
                        <span className="attachment-meta">
                          <small>{attachment.contentType} · {formatFileSize(attachment.size, locale)}</small>
                          {!attachment.available && <small className="attachment-unavailable">{t("attachmentUnavailable")}</small>}
                        </span>
                      }
                      endContent={
                        <span className="attachment-row-actions">
                          {preferences.showAttachmentPreview && (
                            <Button label={t("previewAttachment")} icon={<Eye aria-hidden="true" />} variant="ghost" size="sm" isDisabled={!attachment.available} onClick={() => setPreviewAttachment(attachment)} />
                          )}
                          {attachment.available && (
                            <IconButton label={`${t("downloadAttachment")}: ${attachment.filename}`} icon={<Download aria-hidden="true" />} variant="ghost" size="sm" href={api.attachmentUrl(message.id, attachment.id, true)} />
                          )}
                        </span>
                      }
                    />
                  ))}
                </List>
              ) : <p className="attachment-metadata-pending">{t("attachmentMetadataPending")}</p>}
            </section>
          )}

          <div className="reply-actions">
            <Button label={t("reply")} icon={<CornerUpLeft aria-hidden="true" />} variant="secondary" onClick={onReply} />
            <Button label={t("forward")} icon={<Forward aria-hidden="true" />} variant="secondary" onClick={onForward} />
          </div>
        </div>
      </div>
      <AttachmentPreviewDialog isOpen={Boolean(previewAttachment)} messageId={message.id} attachment={previewAttachment} onClose={() => setPreviewAttachment(null)} />
    </article>
  )
}

function formatDate(timestamp: number, locale: string) {
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(timestamp * 1000))
}

function formatFileSize(size: number, locale: string) {
  const units = ["B", "KB", "MB", "GB"]
  let value = Math.max(0, size)
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit += 1 }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: unit === 0 ? 0 : 1 }).format(value)} ${units[unit]}`
}

function emailDocument(html: string, themeMode: "light" | "dark") {
  const readingFont = `Charter, "Iowan Old Style", "Noto Serif CJK SC", "Source Han Serif SC", "Songti SC", Georgia, serif`
  const darkStyles = themeMode === "dark"
    ? `html,body{background:#111624!important;color:#e6e9f2!important;color-scheme:dark}body :where(table,tbody,thead,tfoot,tr,td,th,div,section,article,main,header,footer,p,span,h1,h2,h3,h4,h5,h6){background-color:transparent!important;color:inherit!important}a{color:#8ab4ff!important}blockquote{border-color:#495169!important;color:#b8bfd0!important}hr{border-color:#343b4f!important}pre,code{background:#191f2f!important;color:#edf0f7!important}`
    : `html,body{background:#fff;color:#202124;color-scheme:light}`
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data: cid:; style-src 'unsafe-inline'; font-src data:"><meta name="color-scheme" content="${themeMode}"><style>html{font:15px/1.62 ${readingFont}}${darkStyles}body{max-width:72ch;margin:0 auto;padding:24px 20px 56px;overflow-wrap:anywhere;text-rendering:optimizeLegibility}p,li{line-height:1.68}h1,h2,h3,h4{line-height:1.25;text-wrap:balance}img{max-width:100%;height:auto}a{text-underline-offset:2px}pre,code{white-space:pre-wrap;font-family:ui-monospace,SFMono-Regular,Menlo,monospace}blockquote{border-left:3px solid #d9dce2;margin-left:0;padding-left:16px;color:#656b76}@media(max-width:640px){html{font-size:16px}body{padding:18px 16px 44px}}</style></head><body>${html}</body></html>`
}
