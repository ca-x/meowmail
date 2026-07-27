import { EmailEditor, type EmailEditorRef } from "@react-email/editor"
import { Button } from "@astryxdesign/core/Button"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { DateTimeInput, type ISODateTimeString } from "@astryxdesign/core/DateTimeInput"
import { Dialog, DialogHeader } from "@astryxdesign/core/Dialog"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Layout, LayoutContent, LayoutFooter } from "@astryxdesign/core/Layout"
import { Selector } from "@astryxdesign/core/Selector"
import { Token } from "@astryxdesign/core/Token"
import { Tokenizer } from "@astryxdesign/core/Tokenizer"
import { useToast } from "@astryxdesign/core/Toast"
import { createStaticSource, type SearchableItem } from "@astryxdesign/core/Typeahead"
import { AtSign, Clock3, FilePenLine, Paperclip, Send, UserRound } from "lucide-react"
import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type FormEvent } from "react"

import { api } from "../../app/api"
import type { Contact, MailAccount, MailPreferences, Signature } from "../../app/types"
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
  signatureId?: string | null
  applySignature?: boolean
  scheduledAt?: number | null
}

interface RecipientItem extends SearchableItem<{ email: string; name?: string; source: "contact" | "manual" }> {}

export function ComposeDialog({ isOpen = true, accounts, activeAccountId, preferences, draft, onClose, onSent, onDraftSaved }: {
  isOpen?: boolean
  accounts: MailAccount[]
  activeAccountId: string | null
  preferences: MailPreferences
  draft?: ComposeDraft | null
  onClose: () => void
  onSent: () => void
  onDraftSaved?: (scheduled: boolean) => void
}) {
  const { t } = useI18n()
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
    signatureId: initialSignature,
    applySignature: draft?.applySignature ?? true,
    scheduled: Boolean(draft?.scheduledAt),
    scheduledAt: initialScheduledAt,
  }))
  const [busy, setBusy] = useState<"send" | "draft" | null>(null)
  const selectedAccount = accounts.find((account) => account.id === accountId)
  const selectedSignature = signatures.find((signature) => signature.id === signatureId) || null
  const currentFingerprint = useMemo(() => composeFingerprint({
    accountId,
    to,
    cc,
    bcc,
    subject,
    bodyText,
    signatureId,
    applySignature,
    scheduled,
    scheduledAt,
  }), [accountId, applySignature, bcc, bodyText, cc, scheduled, scheduledAt, signatureId, subject, to])
  const hasContent = Boolean(to.length || cc.length || bcc.length || subject || bodyText.trim())
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
      ],
    }),
    [contacts],
  )
  const initialContent = useMemo(
    () => draft?.htmlBody || paragraphsToHtml(draft?.body || ""),
    [bodyRevision, draft?.body, draft?.htmlBody],
  )

  useEffect(() => {
    if (!isOpen) return
    setAccountId(defaultAccount?.id || "")
    setTo(recipientItems(draft?.to || ""))
    setCc(recipientItems(draft?.cc || ""))
    setBcc(recipientItems(draft?.bcc || ""))
    setSubject(draft?.subject || "")
    setBodyText(draft?.body || "")
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
      signatureId: nextSignatureId,
      applySignature: nextApplySignature,
      scheduled: nextScheduled,
      scheduledAt: nextScheduledAt,
    }))
  }, [defaultAccount?.id, draft, isOpen])

  const requestClose = useCallback(async () => {
    if (busyRef.current) return
    if (!dirty) {
      onClose()
      return
    }
    const confirmed = await discardDialog.confirm({
      title: t("discardDraftTitle"),
      description: t("discardDraftConfirm"),
      cancelLabel: t("keepEditing"),
      actionLabel: t("discard"),
      actionVariant: "destructive",
    })
    if (confirmed) onClose()
  }, [dirty, discardDialog, onClose, t])

  useEffect(() => {
    if (!isOpen) return
    api.signatures().then(setSignatures).catch(() => setSignatures([]))
    api.contacts(new URLSearchParams({ limit: "100" })).then(setContacts).catch(() => setContacts([]))
  }, [isOpen])

  useEffect(() => {
    if (!isOpen || draft?.id) return
    const next = draft?.signatureId || selectedAccount?.signatureId || "none"
    setSignatureId((current) => current === "none" || !signatures.some((signature) => signature.id === current) ? next : current)
  }, [draft?.id, draft?.signatureId, isOpen, selectedAccount?.signatureId, signatures])

  useEffect(() => {
    if (!isOpen) return
    function onKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape" || event.defaultPrevented || busyRef.current) return
      event.preventDefault()
      void requestClose()
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [isOpen, requestClose])

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

  return (
    <>
      <Dialog
        className="compose-dialog"
        isOpen={isOpen}
        onOpenChange={(open) => { if (!open) void requestClose() }}
        purpose="form"
        width={860}
        maxHeight="92dvh"
        padding={0}
        aria-label={t("compose")}
      >
        <form className="compose-form" onSubmit={submit}>
          <Layout
            className="compose-dialog-layout"
            height="fill"
            padding={4}
            header={
              <DialogHeader
                title={t("compose")}
                startContent={<span className="compose-dialog-icon"><AtSign aria-hidden="true" /></span>}
                onOpenChange={busy ? undefined : (open) => { if (!open) void requestClose() }}
                hasDivider
              />
            }
            content={
              <LayoutContent className="compose-dialog-content" padding={0} isScrollable>
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
                      <small>{t("richTextEditorHint")}</small>
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
                </div>
              </LayoutContent>
            }
            footer={
              <LayoutFooter className="compose-dialog-footer" padding={3} hasDivider>
                <IconButton label={t("attachmentsComingSoon")} icon={<Paperclip aria-hidden="true" />} variant="ghost" isDisabled tooltip={t("attachmentsComingSoon")} />
                <span className="compose-dialog-actions">
                  <Button label={t("cancel")} variant="secondary" isDisabled={Boolean(busy)} onClick={() => void requestClose()} />
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
              </LayoutFooter>
            }
          />
        </form>
      </Dialog>
      {discardDialog.element}
    </>
  )
}

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
    signatureId: value.signatureId,
    applySignature: value.applySignature,
    scheduled: value.scheduled,
    scheduledAt: value.scheduled ? value.scheduledAt || "" : "",
  })
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
    auxiliaryData: { email: contact.email, name: contact.displayName, source: "contact" },
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
