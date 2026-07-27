import { useEffect, useRef, useState } from "react"
import { createPortal } from "react-dom"
import { Download, FileSearch, X } from "lucide-react"

import { api } from "../../app/api"
import type { MailAttachment } from "../../app/types"
import { useDialogBehavior } from "../../app/useDialogBehavior"
import { useI18n } from "../../i18n/I18nProvider"
import { useTheme } from "../../theme/ThemeProvider"

export function AttachmentPreviewDialog({ messageId, attachment, onClose }: {
  messageId: string
  attachment: MailAttachment
  onClose: () => void
}) {
  const { locale, t } = useI18n()
  const { resolved } = useTheme()
  const dialogRef = useRef<HTMLDivElement>(null)
  const viewerRef = useRef<HTMLDivElement>(null)
  const [loading, setLoading] = useState(true)
  const [failed, setFailed] = useState(false)
  const url = api.attachmentUrl(messageId, attachment.id)

  useDialogBehavior(dialogRef, onClose)

  useEffect(() => {
    let active = true
    let destroy: (() => void) | undefined
    setLoading(true)
    setFailed(false)
    void import("@file-viewer/web-full")
      .then(({ mountViewer }) => {
        if (!active || !viewerRef.current) return
        const controller = mountViewer(viewerRef.current, {
          url,
          filename: attachment.filename,
          size: attachment.size,
          options: {
            theme: resolved,
            locale: locale === "zh-CN" ? "zh-CN" : "en-US",
            styleIsolation: "shadow",
            fit: "contain",
            toolbar: {
              download: false,
              print: true,
              exportHtml: false,
              theme: false,
            },
          },
          onStateChange(state) {
            if (!active) return
            setLoading(state.loading)
            if (state.error) setFailed(true)
          },
        }, {
          onError() {
            if (active) {
              setLoading(false)
              setFailed(true)
            }
          },
        })
        destroy = () => controller.destroy()
      })
      .catch(() => {
        if (active) {
          setLoading(false)
          setFailed(true)
        }
      })
    return () => {
      active = false
      destroy?.()
    }
  }, [attachment.filename, attachment.id, attachment.size, locale, resolved, url])

  return createPortal(
    <div className="modal-backdrop attachment-preview-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}>
      <div className="modal-card attachment-preview-dialog" role="dialog" aria-modal="true" aria-labelledby="attachment-preview-title" ref={dialogRef} tabIndex={-1}>
        <header className="attachment-preview-header">
          <div className="modal-title-group">
            <span className="modal-icon"><FileSearch size={18} /></span>
            <div><p>{t("attachmentPreview")}</p><h2 id="attachment-preview-title">{attachment.filename}</h2></div>
          </div>
          <div className="attachment-preview-actions">
            <a className="secondary-button" href={api.attachmentUrl(messageId, attachment.id, true)} download={attachment.filename}><Download size={16} />{t("download")}</a>
            <button className="icon-button" type="button" data-dialog-initial-focus onClick={onClose} aria-label={t("closePreview")}><X size={18} /></button>
          </div>
        </header>
        <div className="attachment-viewer-shell">
          <div className="attachment-viewer" ref={viewerRef} />
          {loading && !failed && <div className="attachment-viewer-status"><span className="spinner" /><p>{t("loadingAttachmentPreview")}</p></div>}
          {failed && <div className="attachment-viewer-status error"><FileSearch size={28} /><p>{t("attachmentPreviewError")}</p><a className="secondary-button" href={api.attachmentUrl(messageId, attachment.id, true)} download={attachment.filename}><Download size={16} />{t("download")}</a></div>}
        </div>
      </div>
    </div>,
    document.body,
  )
}
