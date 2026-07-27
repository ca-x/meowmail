import { useEffect, useMemo, useState } from "react"
import { ArrowLeft, CornerUpLeft, Download, Eye, FileText, Forward, MailOpen, ShieldCheck, Star, Trash2 } from "lucide-react"

import { api } from "../../app/api"
import type { MailAttachment } from "../../app/types"
import type { MessageDetail as Detail } from "../../app/types"
import type { MailPreferences } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { AttachmentPreviewDialog } from "./AttachmentPreviewDialog"

export function MessageDetail({ message, thread, loading, preferences, onBack, onToggleStar, onToggleRead, onReply, onForward, onDelete }: {
  message: Detail | null
  thread: Detail[]
  loading: boolean
  preferences: MailPreferences
  onBack: () => void
  onToggleStar: () => void
  onToggleRead: () => void
  onReply: () => void
  onForward: () => void
  onDelete: () => void
}) {
  const { locale, t } = useI18n()
  const [view, setView] = useState<"html" | "text">("html")
  const [previewAttachment, setPreviewAttachment] = useState<MailAttachment | null>(null)
  const srcDoc = useMemo(() => message?.bodyHtml ? emailDocument(message.bodyHtml) : "", [message?.bodyHtml])

  useEffect(() => {
    setView(preferences.plainTextReading ? "text" : "html")
    setPreviewAttachment(null)
  }, [message?.id, preferences.plainTextReading])

  if (loading) return <div className="detail-loading"><span className="spinner" /></div>
  if (!message) {
    return (
      <div className="detail-empty">
        <div className="detail-empty-art"><MailOpen size={34} /><span /></div>
        <h2>{t("selectMail")}</h2>
        <p>{t("selectMailDescription")}</p>
      </div>
    )
  }
  return (
    <article className="mail-detail">
      <header className="detail-toolbar">
        <button className="icon-button mobile-back" type="button" onClick={onBack} aria-label={t("back")}><ArrowLeft size={18} /></button>
        <div className="detail-toolbar-spacer" />
        <button className="icon-button" type="button" onClick={onToggleRead} aria-label={message.isRead ? t("markUnread") : t("markRead")}><MailOpen size={17} /></button>
        <button className={`icon-button ${message.isStarred ? "star-active" : ""}`} type="button" onClick={onToggleStar} aria-label={message.isStarred ? t("unstar") : t("star")}><Star size={17} fill={message.isStarred ? "currentColor" : "none"} /></button>
        <button className="icon-button danger-text" type="button" onClick={onDelete} aria-label={t("delete")}><Trash2 size={17} /></button>
      </header>
      <div className="detail-scroll">
        <div className="detail-content">
          <header className="message-heading">
            <h1>{message.subject || t("noSubject")}</h1>
            <div className="sender-block">
              <span className="sender-avatar large">{[...(message.senderName || message.senderEmail)][0]?.toUpperCase()}</span>
              <div className="sender-meta">
                <div><strong>{message.senderName || message.senderEmail}</strong>{message.senderName && <span>&lt;{message.senderEmail}&gt;</span>}</div>
                <p>{t("to")}: {message.recipients.join(", ")}</p>
              </div>
              <time>{new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(message.receivedAt * 1000))}</time>
            </div>
          </header>
          {preferences.conversationMode && thread.length > 1 && (
            <section className="conversation-thread" aria-label={t("conversationThread")}>
              <header><span>{t("conversationThread")}</span><small>{t("conversationMessageCount", { count: thread.length })}</small></header>
              {thread.filter((item) => item.id !== message.id).map((item) => (
                <details className="conversation-message" key={item.id}>
                  <summary><span className="sender-avatar small">{[...(item.senderName || item.senderEmail)][0]?.toUpperCase()}</span><span><strong>{item.senderName || item.senderEmail}</strong><small>{item.preview}</small></span><time>{new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(item.receivedAt * 1000))}</time></summary>
                  <pre>{item.bodyText || item.preview}</pre>
                </details>
              ))}
            </section>
          )}
          {message.bodyHtml && !preferences.plainTextReading && (
            <div className="mail-view-switch">
              <div className="segmented-control compact">
                <button type="button" className={view === "html" ? "active" : ""} aria-pressed={view === "html"} onClick={() => setView("html")}>{t("showHtml")}</button>
                <button type="button" className={view === "text" ? "active" : ""} aria-pressed={view === "text"} onClick={() => setView("text")}>{t("showText")}</button>
              </div>
              <span><ShieldCheck size={14} />{t("remoteImagesBlocked")}</span>
            </div>
          )}
          <div className="message-body">
            {view === "html" && message.bodyHtml
              ? <iframe title={message.subject || t("noSubject")} sandbox="" srcDoc={srcDoc} />
              : <pre>{message.bodyText || message.preview}</pre>}
          </div>
          {message.attachmentCount > 0 && (
            <section className="attachment-section" aria-labelledby="attachment-section-title">
              <header>
                <div><FileText size={17} /><h2 id="attachment-section-title">{t("attachmentFiles")}</h2></div>
                <span>{t("attachmentCount", { count: message.attachmentCount })}</span>
              </header>
              {message.attachments.length ? (
                <div className="attachment-list">
                  {message.attachments.map((attachment) => (
                    <div className="attachment-row" key={attachment.id}>
                      <span className="attachment-file-icon"><FileText size={18} /></span>
                      <span className="attachment-meta">
                        <strong>{attachment.filename}</strong>
                        <small>{attachment.contentType} · {formatFileSize(attachment.size, locale)}</small>
                        {!attachment.available && <small className="attachment-unavailable">{t("attachmentUnavailable")}</small>}
                      </span>
                      <span className="attachment-row-actions">
                        {preferences.showAttachmentPreview && <button className="quiet-button" type="button" disabled={!attachment.available} onClick={() => setPreviewAttachment(attachment)}><Eye size={15} />{t("previewAttachment")}</button>}
                        {attachment.available && <a className="icon-button small" href={api.attachmentUrl(message.id, attachment.id, true)} download={attachment.filename} aria-label={`${t("downloadAttachment")}: ${attachment.filename}`}><Download size={16} /></a>}
                      </span>
                    </div>
                  ))}
                </div>
              ) : <p className="attachment-metadata-pending">{t("attachmentMetadataPending")}</p>}
            </section>
          )}
          <div className="reply-actions">
            <button className="secondary-button" type="button" onClick={onReply}><CornerUpLeft size={16} />{t("reply")}</button>
            <button className="secondary-button" type="button" onClick={onForward}><Forward size={16} />{t("forward")}</button>
          </div>
        </div>
      </div>
      {previewAttachment && <AttachmentPreviewDialog messageId={message.id} attachment={previewAttachment} onClose={() => setPreviewAttachment(null)} />}
    </article>
  )
}

function formatFileSize(size: number, locale: string) {
  const units = ["B", "KB", "MB", "GB"]
  let value = Math.max(0, size)
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  const formatted = new Intl.NumberFormat(locale, { maximumFractionDigits: unit === 0 ? 0 : 1 }).format(value)
  return `${formatted} ${units[unit]}`
}

function emailDocument(html: string) {
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data: cid:; style-src 'unsafe-inline'; font-src data:"><meta name="color-scheme" content="light"><style>html{background:#fff;color:#202124;font:15px/1.65 -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif}body{margin:0;padding:4px 2px 40px;overflow-wrap:anywhere}img{max-width:100%;height:auto}a{color:#1769e0}pre,code{white-space:pre-wrap}blockquote{border-left:3px solid #d9dce2;margin-left:0;padding-left:16px;color:#656b76}</style></head><body>${html}</body></html>`
}
