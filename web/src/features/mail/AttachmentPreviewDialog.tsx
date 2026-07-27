import { Banner } from "@astryxdesign/core/Banner"
import { Button } from "@astryxdesign/core/Button"
import { Dialog, DialogHeader } from "@astryxdesign/core/Dialog"
import { Layout, LayoutContent } from "@astryxdesign/core/Layout"
import { Spinner } from "@astryxdesign/core/Spinner"
import { Download, FileSearch } from "lucide-react"
import { useEffect, useRef, useState } from "react"

import { api } from "../../app/api"
import type { MailAttachment } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { useTheme } from "../../theme/ThemeProvider"

export function AttachmentPreviewDialog({ isOpen = true, messageId, attachment, onClose }: {
  isOpen?: boolean
  messageId: string
  attachment: MailAttachment | null
  onClose: () => void
}) {
  const { locale, t } = useI18n()
  const { resolved } = useTheme()
  const viewerRef = useRef<HTMLDivElement>(null)
  const [loading, setLoading] = useState(true)
  const [failed, setFailed] = useState(false)
  const filename = attachment?.filename || t("attachmentPreview")
  const url = attachment ? api.attachmentUrl(messageId, attachment.id) : ""
  const downloadUrl = attachment ? api.attachmentUrl(messageId, attachment.id, true) : ""

  useEffect(() => {
    if (!isOpen || !attachment) return
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
            toolbar: { download: false, print: true, exportHtml: false, theme: false },
          },
          onStateChange(state) {
            if (!active) return
            setLoading(state.loading)
            if (state.error) setFailed(true)
          },
        }, {
          onError() {
            if (active) { setLoading(false); setFailed(true) }
          },
        })
        destroy = () => controller.destroy()
      })
      .catch(() => {
        if (active) { setLoading(false); setFailed(true) }
      })
    return () => { active = false; destroy?.() }
  }, [attachment, isOpen, locale, resolved, url])

  return (
    <Dialog
      className="attachment-preview-dialog"
      isOpen={isOpen}
      onOpenChange={(open) => { if (!open) onClose() }}
      purpose="info"
      variant="fullscreen"
      padding={0}
      aria-label={filename}
    >
      <Layout
        height="fill"
        padding={4}
        header={
          <DialogHeader
            title={filename}
            subtitle={t("attachmentPreview")}
            onOpenChange={(open) => { if (!open) onClose() }}
            hasDivider
            endContent={<Button label={t("download")} icon={<Download aria-hidden="true" />} variant="secondary" size="sm" href={downloadUrl} />}
          />
        }
        content={
          <LayoutContent className="attachment-viewer-shell" padding={0} isScrollable={false}>
            <div className="attachment-viewer" ref={viewerRef} />
            {loading && !failed && <div className="attachment-viewer-status"><Spinner size="xl" label={t("loadingAttachmentPreview")} /></div>}
            {failed && (
              <div className="attachment-viewer-status error">
                <Banner
                  status="error"
                  title={t("attachmentPreviewError")}
                  icon={<FileSearch aria-hidden="true" />}
                  endContent={<Button label={t("download")} icon={<Download aria-hidden="true" />} variant="secondary" href={downloadUrl} />}
                />
              </div>
            )}
          </LayoutContent>
        }
      />
    </Dialog>
  )
}
