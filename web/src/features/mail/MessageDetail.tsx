import { useEffect, useMemo, useState } from "react"
import { ArrowLeft, CornerUpLeft, Forward, MailOpen, ShieldCheck, Star } from "lucide-react"

import type { MessageDetail as Detail } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function MessageDetail({ message, loading, onBack, onToggleStar, onToggleRead, onReply, onForward }: {
  message: Detail | null
  loading: boolean
  onBack: () => void
  onToggleStar: () => void
  onToggleRead: () => void
  onReply: () => void
  onForward: () => void
}) {
  const { locale, t } = useI18n()
  const [view, setView] = useState<"html" | "text">("html")
  const srcDoc = useMemo(() => message?.bodyHtml ? emailDocument(message.bodyHtml) : "", [message?.bodyHtml])

  useEffect(() => setView("html"), [message?.id])

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
          {message.bodyHtml && (
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
          <div className="reply-actions">
            <button className="secondary-button" type="button" onClick={onReply}><CornerUpLeft size={16} />{t("reply")}</button>
            <button className="secondary-button" type="button" onClick={onForward}><Forward size={16} />{t("forward")}</button>
          </div>
        </div>
      </div>
    </article>
  )
}

function emailDocument(html: string) {
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data: cid:; style-src 'unsafe-inline'; font-src data:"><meta name="color-scheme" content="light"><style>html{background:#fff;color:#202124;font:15px/1.65 -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif}body{margin:0;padding:4px 2px 40px;overflow-wrap:anywhere}img{max-width:100%;height:auto}a{color:#1769e0}pre,code{white-space:pre-wrap}blockquote{border-left:3px solid #d9dce2;margin-left:0;padding-left:16px;color:#656b76}</style></head><body>${html}</body></html>`
}
