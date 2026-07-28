import { EmailEditor, type EmailEditorRef } from "@react-email/editor"
import { Button } from "@astryxdesign/core/Button"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { DateTimeInput, type ISODateTimeString } from "@astryxdesign/core/DateTimeInput"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Layout, LayoutContent, LayoutFooter, LayoutHeader } from "@astryxdesign/core/Layout"
import { Selector } from "@astryxdesign/core/Selector"
import { Token } from "@astryxdesign/core/Token"
import { Tokenizer } from "@astryxdesign/core/Tokenizer"
import { useToast } from "@astryxdesign/core/Toast"
import { createStaticSource, type SearchableItem } from "@astryxdesign/core/Typeahead"
import { ArrowLeft, Clock3, FilePenLine, FileText, MailPlus, Paperclip, Send, Trash2, UserRound, WandSparkles } from "lucide-react"
import { forwardRef, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState, type CSSProperties, type ChangeEvent, type FormEvent } from "react"

import { api } from "../../app/api"
import type { ComposeAttachmentInput, Contact, MailAccount, MailPreferences, Signature } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"

export interface ComposeDraft {
  id?: string
  accountId?: string
  to?: string
  cc?: string
  bcc?: string
  subject?: string
  body?: string
  htmlBody?: string | null
  attachments?: ComposeAttachmentInput[]
  signatureId?: string | null
  applySignature?: boolean
  scheduledAt?: number | null
}

interface RecipientItem extends SearchableItem<{ email: string; name?: string; aliases?: string[]; source: "contact" | "manual" }> {}

export interface ComposeWorkspaceRef {
  requestClose: (options?: { restoreFocus?: boolean }) => Promise<boolean>
}

export const ComposeDialog = forwardRef<ComposeWorkspaceRef, {
  accounts: MailAccount[]
  activeAccountId: string | null
  preferences: MailPreferences
  aiEnabled?: boolean
  draft?: ComposeDraft | null
  onClose: (restoreFocus?: boolean) => void
  onSent: () => void
  onDraftSaved?: (scheduled: boolean) => void
}>(function ComposeDialog({ accounts, activeAccountId, preferences, aiEnabled = false, draft, onClose, onSent, onDraftSaved }, ref) {
  const { locale, t } = useI18n()
  const showToast = useToast()
  const discardDialog = useImperativeConfirmDialog()
  const editorRef = useRef<EmailEditorRef | null>(null)
  const busyRef = useRef(false)
  const defaultAccount = useMemo(
    () => accounts.find((account) => account.id === draft?.accountId)
      || accounts.find((account) => account.id === activeAccountId)
      || accounts.find((account) => account.isDefault)
      || accounts[0],
    [accounts, activeAccountId, draft?.accountId],
  )
  const initialSignature = initialSignatureId(defaultAccount, draft)
  const initialScheduledAt = draft?.scheduledAt ? toLocalDateTimeInput(new Date(draft.scheduledAt * 1000)) : undefined
  const [accountId, setAccountId] = useState(defaultAccount?.id || "")
  const [to, setTo] = useState<RecipientItem[]>(() => recipientItems(draft?.to || ""))
  const [cc, setCc] = useState<RecipientItem[]>(() => recipientItems(draft?.cc || ""))
  const [bcc, setBcc] = useState<RecipientItem[]>(() => recipientItems(draft?.bcc || ""))
  const [subject, setSubject] = useState(draft?.subject || "")
  const [bodyRevision, setBodyRevision] = useState(0)
  const [bodyText, setBodyText] = useState(draft?.body || "")
  const [composeAttachments, setComposeAttachments] = useState<ComposeAttachmentInput[]>(() => draft?.attachments || [])
  const [signatures, setSignatures] = useState<Signature[]>([])
  const [contacts, setContacts] = useState<Contact[]>([])
  const [signatureId, setSignatureId] = useState<string>(initialSignature)
  const [applySignature, setApplySignature] = useState(draft?.applySignature ?? true)
  const [scheduled, setScheduled] = useState(Boolean(draft?.scheduledAt))
  const [scheduledAt, setScheduledAt] = useState<ISODateTimeString | undefined>(initialScheduledAt)
  const [initialFingerprint, setInitialFingerprint] = useState(() => composeFingerprint({
    accountId: defaultAccount?.id || "",
    to: recipientItems(draft?.to || ""),
    cc: recipientItems(draft?.cc || ""),
    bcc: recipientItems(draft?.bcc || ""),
    subject: draft?.subject || "",
    bodyText: draft?.body || "",
    attachments: draft?.attachments || [],
    signatureId: initialSignature,
    applySignature: draft?.applySignature ?? true,
    scheduled: Boolean(draft?.scheduledAt),
    scheduledAt: initialScheduledAt,
  }))
  const [busy, setBusy] = useState<"send" | "draft" | "ai" | null>(null)
  const attachmentInputRef = useRef<HTMLInputElement | null>(null)
  const selectedAccount = accounts.find((account) => account.id === accountId)
  const selectedSignature = signatures.find((signature) => signature.id === signatureId) || null
  const currentFingerprint = useMemo(() => composeFingerprint({
    accountId,
    to,
    cc,
    bcc,
    subject,
    bodyText,
    attachments: composeAttachments,
    signatureId,
    applySignature,
    scheduled,
    scheduledAt,
  }), [accountId, applySignature, bcc, bodyText, cc, composeAttachments, scheduled, scheduledAt, signatureId, subject, to])
  const hasContent = Boolean(to.length || cc.length || bcc.length || subject || bodyText.trim() || composeAttachments.length)
  const dirty = draft?.id ? currentFingerprint !== initialFingerprint : hasContent
  const editorStyle = {
    "--compose-font-size": `${preferences.composeFontSize}px`,
    "--compose-font-color": preferences.composeFontColor,
  } as CSSProperties
  const recipientSource = useMemo(
    () => createStaticSource(contactItems(contacts), {
      keywords: (item) => [
        item.auxiliaryData?.email || "",
        item.auxiliaryData?.name || "",
        ...(item.auxiliaryData?.aliases || []),
      ],
    }),
    [contacts],
  )
  const initialContent = useMemo(
    () => draft?.htmlBody || paragraphsToHtml(draft?.body || ""),
    [bodyRevision, draft?.body, draft?.htmlBody],
  )

  useEffect(() => {
    setAccountId(defaultAccount?.id || "")
    setTo(recipientItems(draft?.to || ""))
    setCc(recipientItems(draft?.cc || ""))
    setBcc(recipientItems(draft?.bcc || ""))
    setSubject(draft?.subject || "")
    setBodyText(draft?.body || "")
    setComposeAttachments(draft?.attachments || [])
    setBodyRevision((value) => value + 1)
    const nextSignatureId = initialSignatureId(defaultAccount, draft)
    const nextScheduled = Boolean(draft?.scheduledAt)
    const nextScheduledAt = draft?.scheduledAt ? toLocalDateTimeInput(new Date(draft.scheduledAt * 1000)) : undefined
    const nextApplySignature = draft?.applySignature ?? true
    setSignatureId(nextSignatureId)
    setScheduled(nextScheduled)
    setScheduledAt(nextScheduledAt)
    setApplySignature(nextApplySignature)
    setInitialFingerprint(composeFingerprint({
      accountId: defaultAccount?.id || "",
      to: recipientItems(draft?.to || ""),
      cc: recipientItems(draft?.cc || ""),
      bcc: recipientItems(draft?.bcc || ""),
      subject: draft?.subject || "",
      bodyText: draft?.body || "",
      attachments: draft?.attachments || [],
      signatureId: nextSignatureId,
      applySignature: nextApplySignature,
      scheduled: nextScheduled,
      scheduledAt: nextScheduledAt,
    }))
  }, [defaultAccount?.id, draft])

  const requestClose = useCallback(async (restoreFocus = true) => {
    if (busyRef.current) return false
    if (!dirty) {
      onClose(restoreFocus)
      return true
    }
    const confirmed = await discardDialog.confirm({
      title: t("discardDraftTitle"),
      description: t("discardDraftConfirm"),
      cancelLabel: t("keepEditing"),
      actionLabel: t("discard"),
      actionVariant: "destructive",
    })
    if (confirmed) onClose(restoreFocus)
    return confirmed
  }, [dirty, discardDialog, onClose, t])

  useImperativeHandle(ref, () => ({
    requestClose: (options) => requestClose(options?.restoreFocus ?? true),
  }), [requestClose])

  useEffect(() => {
    api.signatures().then(setSignatures).catch(() => setSignatures([]))
    api.contacts(new URLSearchParams({ limit: "100" })).then(setContacts).catch(() => setContacts([]))
  }, [])

  useEffect(() => {
    if (draft?.id) return
    const next = draft?.signatureId || selectedAccount?.signatureId || "none"
    setSignatureId((current) => current === "none" || !signatures.some((signature) => signature.id === current) ? next : current)
  }, [draft?.id, draft?.signatureId, selectedAccount?.signatureId, signatures])

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape" || event.defaultPrevented || busyRef.current) return
      event.preventDefault()
      void requestClose()
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [requestClose])

  async function submit(event: FormEvent) {
    event.preventDefault()
    await sendNow()
  }

  async function sendNow() {
    if (!accountId || !to.length) return
    busyRef.current = true
    setBusy("send")
    try {
      const email = await exportEmail()
      const input = {
        accountId,
        to: to.map(addressOf),
        cc: cc.map(addressOf),
        bcc: bcc.map(addressOf),
        subject,
        textBody: email.text,
        htmlBody: email.html,
        attachments: composeAttachments,
        signatureId: signatureId === "none" ? null : signatureId,
        applySignature,
      }
      if (draft?.id) {
        await api.updateDraft(draft.id, { ...input, scheduledAt: null })
        await api.sendDraft(draft.id)
      } else {
        await api.sendMessage(input)
      }
      onSent()
    } catch {
      showToast({ body: t("genericError"), type: "error", uniqueID: "compose-send-error", collisionBehavior: "overwrite" })
    } finally {
      busyRef.current = false
      setBusy(null)
    }
  }

  async function saveDraft() {
    if (!accountId) return
    busyRef.current = true
    setBusy("draft")
    try {
      const email = await exportEmail()
      const input = {
        accountId,
        to: to.map(addressOf),
        cc: cc.map(addressOf),
        bcc: bcc.map(addressOf),
        subject,
        textBody: email.text,
        htmlBody: email.html,
        attachments: composeAttachments,
        signatureId: signatureId === "none" ? null : signatureId,
        applySignature,
        scheduledAt: scheduled ? scheduledTimestamp(scheduledAt) : null,
      }
      if (draft?.id) await api.updateDraft(draft.id, input)
      else await api.createDraft(input)
      onDraftSaved?.(scheduled)
      onClose()
    } catch {
      showToast({ body: t("genericError"), type: "error", uniqueID: "compose-draft-error", collisionBehavior: "overwrite" })
    } finally {
      busyRef.current = false
      setBusy(null)
    }
  }

  async function exportEmail() {
    const fallbackText = bodyText.trim()
    if (!editorRef.current) {
      return { html: paragraphsToHtml(fallbackText), text: fallbackText }
    }
    const email = await editorRef.current.getEmail()
    return {
      html: email.html,
      text: email.text.trim() || fallbackText,
    }
  }

  async function addAttachments(event: ChangeEvent<HTMLInputElement>) {
    const files = [...(event.target.files || [])]
    event.target.value = ""
    if (!files.length) return
    try {
      const next = await attachmentsFromFiles([...composeAttachments], files)
      setComposeAttachments(next)
    } catch {
      showToast({ body: t("attachmentTooLarge"), type: "error", uniqueID: "compose-attachment-error", collisionBehavior: "overwrite" })
    }
  }

  async function polishBody() {
    if (!aiEnabled || !bodyText.trim()) return
    busyRef.current = true
    setBusy("ai")
    try {
      const result = await api.polishText({ text: bodyText })
      editorRef.current?.editor?.commands.setContent(paragraphsToHtml(result.text))
      setBodyText(result.text)
      showToast({ body: t("emailPolished"), type: "info", uniqueID: "compose-polish-success", collisionBehavior: "overwrite" })
    } catch {
      showToast({ body: t("genericError"), type: "error", uniqueID: "compose-polish-error", collisionBehavior: "overwrite" })
    } finally {
      busyRef.current = false
      setBusy(null)
    }
  }

  return (
    <>
      <section className="compose-workspace" aria-labelledby="compose-workspace-title">
        <form className="compose-form" onSubmit={submit}>
          <Layout
            className="compose-workspace-layout"
            height="fill"
            padding={0}
            header={
              <LayoutHeader className="compose-workspace-header" padding={0} hasDivider>
                <div className="compose-workspace-header-inner">
                  <IconButton
                    label={t("back")}
                    icon={<ArrowLeft aria-hidden="true" />}
                    variant="ghost"
                    isDisabled={Boolean(busy)}
                    onClick={() => void requestClose()}
                  />
                  <span className="compose-workspace-icon"><MailPlus aria-hidden="true" /></span>
                  <div className="compose-workspace-heading">
                    <h1 id="compose-workspace-title">{t("compose")}</h1>
                    <p title={selectedAccount?.email || t("from")}>
                      {draft?.id ? `${t("drafts")} · ` : ""}{selectedAccount?.email || t("from")}
                    </p>
                  </div>
                </div>
              </LayoutHeader>
            }
            content={
              <LayoutContent className="compose-workspace-content" padding={0} isScrollable>
                <div className="compose-fields">
                  <Selector
                    label={t("from")}
                    value={accountId}
                    onChange={setAccountId}
                    options={accounts.map((account) => ({ value: account.id, label: `${account.displayName} · ${account.email}` }))}
                    width="100%"
                  />

                  <RecipientRow label={t("to")} value={to} onChange={setTo} source={recipientSource} placeholder={t("recipientPlaceholder")} required />
                  <RecipientRow label={t("cc")} value={cc} onChange={setCc} source={recipientSource} placeholder={t("recipientPlaceholder")} />
                  <RecipientRow label={t("bcc")} value={bcc} onChange={setBcc} source={recipientSource} placeholder={t("recipientPlaceholder")} />

                  <div className="compose-subject-grid">
                    <label htmlFor="compose-subject">{t("subject")}</label>
                    <input
                      id="compose-subject"
                      className="compose-native-input"
                      value={subject}
                      onChange={(event) => setSubject(event.target.value)}
                      placeholder={t("subjectPlaceholder")}
                    />
                  </div>

                  <div className="compose-options-row">
                    <Selector
                      label={t("emailSignature")}
                      value={signatureId}
                      onChange={setSignatureId}
                      options={[{ value: "none", label: t("noSignature") }, ...signatures.map((signature) => ({ value: signature.id, label: signature.name }))]}
                      width="100%"
                    />
                    <CheckboxInput
                      label={t("applySignature")}
                      value={applySignature}
                      onChange={setApplySignature}
                      isDisabled={signatureId === "none"}
                    />
                  </div>

                  {selectedSignature && applySignature && (
                    <div className="compose-signature-preview">
                      <small>{t("selectedSignature")}</small>
                      <p>{selectedSignature.bodyText}</p>
                    </div>
                  )}

                  <section className="compose-editor-shell" style={editorStyle} aria-label={t("message")}>
                    <div className="compose-editor-label">
                      <span>{t("message")} · {t("required")}</span>
                      <span className="compose-editor-tools">
                        <small>{t("richTextEditorHint")}</small>
                        {aiEnabled && (
                          <Button
                            label={busy === "ai" ? t("polishingEmail") : t("polishEmail")}
                            icon={<WandSparkles aria-hidden="true" />}
                            variant="ghost"
                            size="sm"
                            isLoading={busy === "ai"}
                            isDisabled={Boolean(busy) || !bodyText.trim()}
                            onClick={() => void polishBody()}
                          />
                        )}
                      </span>
                    </div>
                    <EmailEditor
                      key={bodyRevision}
                      ref={editorRef}
                      content={initialContent}
                      placeholder={t("messagePlaceholder")}
                      className={`compose-email-editor font-${preferences.composeFontFamily}`}
                      onReady={(ref) => configureEditorDom(ref, t("message"))}
                      onUpdate={(ref) => {
                        configureEditorDom(ref, t("message"))
                        setBodyText(ref.editor?.getText() || "")
                      }}
                    />
                  </section>

                  <div className="compose-schedule-panel">
                    <CheckboxInput
                      label={t("scheduleSend")}
                      description={t("scheduleSendDescription")}
                      value={scheduled}
                      onChange={setScheduled}
                      labelIcon={<Clock3 aria-hidden="true" />}
                    />
                    {scheduled && (
                      <DateTimeInput
                        label={t("scheduledSendTime")}
                        value={scheduledAt}
                        onChange={setScheduledAt}
                        min={minScheduleDateTime()}
                        hasClear
                        hourFormat="24h"
                        timeIncrement={5}
                        width="100%"
                      />
                    )}
                  </div>

                  {composeAttachments.length > 0 && (
                    <section className="compose-attachment-list" aria-label={t("attachmentFiles")}>
                      <header>
                        <span><Paperclip aria-hidden="true" />{t("attachmentCount", { count: composeAttachments.length })}</span>
                        <small>{formatAttachmentTotal(composeAttachments, locale)}</small>
                      </header>
                      <ul>
                        {composeAttachments.map((attachment, index) => (
                          <li key={`${attachment.filename}:${attachment.size}:${index}`}>
                            <span><FileText aria-hidden="true" /><strong>{attachment.filename}</strong><small>{formatFileSize(attachment.size, locale)}</small></span>
                            <IconButton
                              label={`${t("removeAttachment")}: ${attachment.filename}`}
                              icon={<Trash2 aria-hidden="true" />}
                              variant="ghost"
                              size="sm"
                              isDisabled={Boolean(busy)}
                              onClick={() => setComposeAttachments((items) => items.filter((item) => item !== attachment))}
                            />
                          </li>
                        ))}
                      </ul>
                    </section>
                  )}
                </div>
              </LayoutContent>
            }
            footer={
              <LayoutFooter className="compose-workspace-footer" padding={0} hasDivider>
                <div className="compose-workspace-footer-inner">
                  <span className="compose-attachment-action">
                    <input ref={attachmentInputRef} type="file" multiple onChange={(event) => void addAttachments(event)} />
                    <IconButton
                      label={t("addAttachment")}
                      icon={<Paperclip aria-hidden="true" />}
                      variant="ghost"
                      isDisabled={Boolean(busy)}
                      tooltip={t("addAttachment")}
                      onClick={() => attachmentInputRef.current?.click()}
                    />
                  </span>
                  <span className="compose-workspace-actions">
                    <Button
                      label={busy === "draft" ? t("saving") : scheduled ? t("saveScheduledDraft") : t("saveDraft")}
                      icon={<FilePenLine aria-hidden="true" />}
                      variant="secondary"
                      isLoading={busy === "draft"}
                      isDisabled={Boolean(busy) || !accountId || (scheduled && !scheduledAt)}
                      onClick={() => void saveDraft()}
                    />
                    <Button
                      label={busy === "send" ? t("sending") : t("send")}
                      icon={<Send aria-hidden="true" />}
                      variant="primary"
                      type="submit"
                      isLoading={busy === "send"}
                      isDisabled={Boolean(busy) || !accountId || !to.length || !bodyText.trim()}
                    />
                  </span>
                </div>
              </LayoutFooter>
            }
          />
        </form>
      </section>
      {discardDialog.element}
    </>
  )
})

function configureEditorDom(ref: EmailEditorRef, label: string) {
  const element = ref.editor?.view.dom
  if (!element) return
  element.setAttribute("role", "textbox")
  element.setAttribute("aria-label", label)
  element.setAttribute("aria-multiline", "true")
  element.setAttribute("data-testid", "compose-rich-editor")
}

function initialSignatureId(account: MailAccount | undefined, draft?: ComposeDraft | null) {
  if (draft?.id) return draft.signatureId || "none"
  return draft?.signatureId || account?.signatureId || "none"
}

function composeFingerprint(value: {
  accountId: string
  to: RecipientItem[]
  cc: RecipientItem[]
  bcc: RecipientItem[]
  subject: string
  bodyText: string
  attachments: ComposeAttachmentInput[]
  signatureId: string
  applySignature: boolean
  scheduled: boolean
  scheduledAt?: ISODateTimeString
}) {
  return JSON.stringify({
    accountId: value.accountId,
    to: value.to.map(addressOf).map((address) => address.trim().toLowerCase()).filter(Boolean),
    cc: value.cc.map(addressOf).map((address) => address.trim().toLowerCase()).filter(Boolean),
    bcc: value.bcc.map(addressOf).map((address) => address.trim().toLowerCase()).filter(Boolean),
    subject: value.subject,
    bodyText: value.bodyText.trim(),
    attachments: value.attachments.map((attachment) => `${attachment.filename}:${attachment.size}:${attachment.contentBase64.length}`),
    signatureId: value.signatureId,
    applySignature: value.applySignature,
    scheduled: value.scheduled,
    scheduledAt: value.scheduled ? value.scheduledAt || "" : "",
  })
}

async function attachmentsFromFiles(current: ComposeAttachmentInput[], files: File[]) {
  const next = [...current]
  for (const file of files) {
    if (file.size <= 0 || file.size > MAX_ATTACHMENT_BYTES) throw new Error("attachment too large")
    const total = next.reduce((sum, attachment) => sum + attachment.size, 0) + file.size
    if (total > MAX_ATTACHMENT_TOTAL_BYTES || next.length >= MAX_ATTACHMENT_COUNT) throw new Error("attachment too large")
    next.push({
      filename: file.name || "attachment",
      contentType: file.type || "application/octet-stream",
      contentBase64: await fileToBase64(file),
      size: file.size,
    })
  }
  return next
}

function fileToBase64(file: File) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.addEventListener("load", () => {
      const value = String(reader.result || "")
      resolve(value.includes(",") ? value.split(",").pop() || "" : value)
    })
    reader.addEventListener("error", () => reject(reader.error || new Error("file read failed")))
    reader.readAsDataURL(file)
  })
}

const MAX_ATTACHMENT_COUNT = 10
const MAX_ATTACHMENT_BYTES = 8 * 1024 * 1024
const MAX_ATTACHMENT_TOTAL_BYTES = 8 * 1024 * 1024

function formatAttachmentTotal(attachments: ComposeAttachmentInput[], locale: string) {
  return formatFileSize(attachments.reduce((sum, attachment) => sum + attachment.size, 0), locale)
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

function RecipientRow({ label, value, onChange, source, placeholder, required = false }: {
  label: string
  value: RecipientItem[]
  onChange: (items: RecipientItem[]) => void
  source: ReturnType<typeof createStaticSource<RecipientItem>>
  placeholder: string
  required?: boolean
}) {
  const { t } = useI18n()
  return (
    <div className="compose-recipient-line">
      <span>{label}</span>
      <Tokenizer
        label={label}
        isLabelHidden
        value={value}
        onChange={(items, change) => {
          const normalized = uniqueRecipients(items.map((item) => normalizeRecipientItem(item)))
          if (change.type === "create") {
            onChange(normalized.filter((item) => isEmail(addressOf(item))))
          } else {
            onChange(normalized)
          }
        }}
        searchSource={source}
        placeholder={placeholder}
        hasCreate
        hasEntriesOnFocus
        maxMenuItems={8}
        debounceMs={0}
        width="100%"
        isRequired={required}
        emptySearchResultsText={t("noContactsFound")}
        tokenOverflowBehavior="unfocusedInline"
        renderToken={(item, onRemove) => (
          <Token
            label={item.label}
            icon={<UserRound aria-hidden="true" />}
            description={addressOf(item)}
            onRemove={onRemove}
            color={item.auxiliaryData?.source === "contact" ? "teal" : "gray"}
          />
        )}
      />
    </div>
  )
}

function recipientItems(value: string) {
  return uniqueRecipients(
    value
      .split(/[;,]/)
      .map((address) => address.trim())
      .filter(Boolean)
      .map(manualRecipient),
  )
}

function contactItems(contacts: Contact[]): RecipientItem[] {
  return contacts.map((contact) => ({
    id: contact.email,
    label: contact.displayName === contact.email ? contact.email : `${contact.displayName} <${contact.email}>`,
    auxiliaryData: { email: contact.email, name: contact.displayName, aliases: contact.searchAliases, source: "contact" },
  }))
}

function normalizeRecipientItem(item: RecipientItem): RecipientItem {
  const email = extractEmail(item.auxiliaryData?.email || item.label)
  if (!email) return item
  return {
    id: email.toLowerCase(),
    label: item.auxiliaryData?.name && item.auxiliaryData.name !== email ? `${item.auxiliaryData.name} <${email}>` : email,
    auxiliaryData: {
      email: email.toLowerCase(),
      name: item.auxiliaryData?.name,
      aliases: item.auxiliaryData?.aliases,
      source: item.auxiliaryData?.source || "manual",
    },
  }
}

function manualRecipient(value: string): RecipientItem {
  const email = extractEmail(value) || value
  return {
    id: email.toLowerCase(),
    label: value.includes("<") ? value : email,
    auxiliaryData: { email: email.toLowerCase(), source: "manual" },
  }
}

function uniqueRecipients(items: RecipientItem[]) {
  const seen = new Set<string>()
  return items.filter((item) => {
    const email = addressOf(item).toLowerCase()
    if (seen.has(email)) return false
    seen.add(email)
    return true
  })
}

function addressOf(item: RecipientItem) {
  return item.auxiliaryData?.email || extractEmail(item.label) || item.label
}

function extractEmail(value: string) {
  return value.match(/[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/i)?.[0] || ""
}

function isEmail(value: string) {
  return /^[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+$/.test(value)
}

function paragraphsToHtml(value: string) {
  const escaped = value
    .split(/\n{2,}/)
    .map((paragraph) => paragraph.trim())
    .filter(Boolean)
    .map((paragraph) => `<p>${escapeHtml(paragraph).replace(/\n/g, "<br>")}</p>`)
    .join("")
  return escaped || "<p></p>"
}

function escapeHtml(value: string) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;")
}

function minScheduleDateTime() {
  return toLocalDateTimeInput(new Date(Date.now() + 5 * 60 * 1000))
}

function scheduledTimestamp(value: ISODateTimeString | undefined) {
  if (!value) return null
  const date = new Date(value)
  return Number.isFinite(date.getTime()) ? Math.floor(date.getTime() / 1000) : null
}

function toLocalDateTimeInput(date: Date) {
  const pad = (value: number) => String(value).padStart(2, "0")
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}` as ISODateTimeString
}
