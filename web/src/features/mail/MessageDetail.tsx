import { Avatar } from "@astryxdesign/core/Avatar"
import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { EmptyState } from "@astryxdesign/core/EmptyState"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Item } from "@astryxdesign/core/Item"
import { List } from "@astryxdesign/core/List"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { Skeleton } from "@astryxdesign/core/Skeleton"
import { Toolbar } from "@astryxdesign/core/Toolbar"
import { useToast } from "@astryxdesign/core/Toast"
import { ArrowLeft, CornerUpLeft, Download, Eye, FileText, Forward, Languages, MailOpen, RefreshCw, ShieldCheck, Star, Tags, Trash2, X } from "lucide-react"
import { useEffect, useMemo, useState } from "react"

import { api } from "../../app/api"
import type { Label, MailAttachment, MailPreferences, MessageDetail as Detail } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { useTheme } from "../../theme/ThemeProvider"
import { AttachmentPreviewDialog } from "./AttachmentPreviewDialog"

export function MessageDetail({ message, thread, loading, isDeleting = false, isRefreshingAttachments = false, preferences, aiEnabled = false, onBack, onToggleStar, onToggleRead, onReply, onForward, onDelete, onRefreshAttachments }: {
  message: Detail | null
  thread: Detail[]
  loading: boolean
  isDeleting?: boolean
  isRefreshingAttachments?: boolean
  preferences: MailPreferences
  aiEnabled?: boolean
  onBack: () => void
  onToggleStar: () => void
  onToggleRead: () => void
  onReply: () => void
  onForward: () => void
  onDelete: () => void
  onRefreshAttachments?: () => void
}) {
  const { locale, t } = useI18n()
  const { resolved: themeMode } = useTheme()
  const showToast = useToast()
  const [view, setView] = useState<"html" | "text">("html")
  const [previewAttachment, setPreviewAttachment] = useState<MailAttachment | null>(null)
  const [translation, setTranslation] = useState("")
  const [appliedLabels, setAppliedLabels] = useState<Label[]>([])
  const [aiBusy, setAiBusy] = useState<"translate" | "label" | null>(null)
  const srcDoc = useMemo(() => message?.bodyHtml ? emailDocument(message.bodyHtml, themeMode) : "", [message?.bodyHtml, themeMode])

  useEffect(() => {
    setView(preferences.plainTextReading ? "text" : "html")
    setPreviewAttachment(null)
    setTranslation("")
    setAppliedLabels([])
  }, [message?.id, preferences.plainTextReading])

  async function translateMessage() {
    if (!message || !aiEnabled || aiBusy) return
    setAiBusy("translate")
    try {
      const result = await api.translateText({
        text: message.bodyText || message.preview,
        targetLanguage: locale === "zh-CN" ? "Simplified Chinese" : "English",
      })
      setTranslation(result.text)
    } catch {
      showToast({ body: t("genericError"), type: "error", uniqueID: "message-translate-error", collisionBehavior: "overwrite" })
    } finally {
      setAiBusy(null)
    }
  }

  async function autoLabelMessage() {
    if (!message || !aiEnabled || aiBusy) return
    setAiBusy("label")
    try {
      const result = await api.autoLabelMessage(message.id)
      setAppliedLabels(result.labels)
      showToast({ body: t("autoLabelApplied", { count: result.labels.length }), type: "info", uniqueID: "message-auto-label-success", collisionBehavior: "overwrite" })
    } catch {
      showToast({ body: t("genericError"), type: "error", uniqueID: "message-auto-label-error", collisionBehavior: "overwrite" })
    } finally {
      setAiBusy(null)
    }
  }

  if (loading) return <MessageDetailSkeleton label={t("loading")} />
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

          {aiEnabled && (
            <div className="message-ai-toolbar" aria-label={t("aiMailActions")}>
              <span>
                <Button
                  label={aiBusy === "translate" ? t("translatingEmail") : t("translateEmail")}
                  icon={<Languages aria-hidden="true" />}
                  variant="ghost"
                  size="sm"
                  isLoading={aiBusy === "translate"}
                  isDisabled={Boolean(aiBusy)}
                  onClick={() => void translateMessage()}
                />
                <Button
                  label={aiBusy === "label" ? t("autoLabelingEmail") : t("autoLabelEmail")}
                  icon={<Tags aria-hidden="true" />}
                  variant="ghost"
                  size="sm"
                  isLoading={aiBusy === "label"}
                  isDisabled={Boolean(aiBusy)}
                  onClick={() => void autoLabelMessage()}
                />
              </span>
              {appliedLabels.length > 0 && (
                <span className="message-ai-labels">
                  {appliedLabels.map((label) => <Badge key={label.id} variant="neutral" label={label.name} />)}
                </span>
              )}
            </div>
          )}

          {aiEnabled && translation && (
            <section className="message-translation" aria-labelledby="message-translation-title">
              <header>
                <div><Languages aria-hidden="true" /><h2 id="message-translation-title">{t("translatedEmail")}</h2></div>
                <IconButton label={t("close")} icon={<X aria-hidden="true" />} variant="ghost" size="sm" onClick={() => setTranslation("")} />
              </header>
              <pre>{translation}</pre>
            </section>
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
              ) : (
                <div className="attachment-metadata-pending">
                  <p>{t("attachmentMetadataPending")}</p>
                  {onRefreshAttachments && (
                    <Button
                      label={t("syncAttachmentMetadata")}
                      icon={<RefreshCw aria-hidden="true" />}
                      variant="secondary"
                      size="sm"
                      isLoading={isRefreshingAttachments}
                      isDisabled={isRefreshingAttachments}
                      onClick={onRefreshAttachments}
                    />
                  )}
                </div>
              )}
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

function MessageDetailSkeleton({ label }: { label: string }) {
  return (
    <div className="detail-loading detail-loading-skeleton" role="status" aria-label={label}>
      <div className="detail-skeleton-toolbar">
        <Skeleton width={36} height={36} radius={3} index={0} />
        <Skeleton width={36} height={36} radius={3} index={1} />
        <Skeleton width={36} height={36} radius={3} index={2} />
      </div>
      <div className="detail-skeleton-content">
        <Skeleton width="68%" height={30} radius={2} index={0} />
        <div className="detail-skeleton-sender">
          <Skeleton width={44} height={44} radius="rounded" index={1} />
          <div>
            <Skeleton width="42%" height={13} index={2} />
            <Skeleton width="58%" height={11} index={3} />
          </div>
        </div>
        <div className="detail-skeleton-body">
          <Skeleton width="100%" height={16} index={4} />
          <Skeleton width="94%" height={16} index={5} />
          <Skeleton width="97%" height={16} index={6} />
          <Skeleton width="72%" height={16} index={7} />
        </div>
      </div>
    </div>
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
