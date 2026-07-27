import { Avatar } from "@astryxdesign/core/Avatar"
import { Button } from "@astryxdesign/core/Button"
import { Dialog, DialogHeader, useImperativeDialog } from "@astryxdesign/core/Dialog"
import { EmptyState } from "@astryxdesign/core/EmptyState"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Item } from "@astryxdesign/core/Item"
import { Layout, LayoutContent, LayoutFooter } from "@astryxdesign/core/Layout"
import { List } from "@astryxdesign/core/List"
import { Skeleton } from "@astryxdesign/core/Skeleton"
import { TextArea } from "@astryxdesign/core/TextArea"
import { TextInput } from "@astryxdesign/core/TextInput"
import { useToast } from "@astryxdesign/core/Toast"
import { AtSign, ContactRound, PencilLine, Search, Trash2, UserRoundPlus, UsersRound } from "lucide-react"
import { useEffect, useMemo, useState, type FormEvent } from "react"

import { api } from "../../app/api"
import type { Contact, ContactInput } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"

const emptyContact: ContactInput = { displayName: "", email: "", notes: "" }

export function ContactsDialog({ isOpen, onClose }: {
  isOpen: boolean
  onClose: () => void
}) {
  const { t } = useI18n()
  const showToast = useToast()
  const formDialog = useImperativeDialog({ purpose: "form", width: 520, padding: 0 })
  const deleteDialog = useImperativeConfirmDialog()
  const [contacts, setContacts] = useState<Contact[]>([])
  const [query, setQuery] = useState("")
  const [loading, setLoading] = useState(false)
  const [busyId, setBusyId] = useState<string | null>(null)
  const filtered = useMemo(() => contacts.filter((contact) => {
    const haystack = `${contact.displayName} ${contact.email} ${contact.notes}`.toLowerCase()
    return haystack.includes(query.trim().toLowerCase())
  }), [contacts, query])

  async function loadContacts(search = query) {
    setLoading(true)
    try {
      const params = new URLSearchParams({ limit: "100" })
      if (search.trim()) params.set("q", search.trim())
      setContacts(await api.contacts(params))
    } catch {
      showToast({ body: t("genericError"), type: "error", uniqueID: "contacts-load-error", collisionBehavior: "overwrite" })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    if (!isOpen) return
    void loadContacts("")
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen])

  useEffect(() => {
    if (!isOpen) return
    const timer = window.setTimeout(() => void loadContacts(query), 220)
    return () => window.clearTimeout(timer)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, isOpen])

  const openForm = (contact: Contact | null) => {
    formDialog.show(
      <ContactForm
        contact={contact}
        onCancel={formDialog.hide}
        onSubmit={async (input) => {
          try {
            if (contact) await api.updateContact(contact.id, input)
            else await api.createContact(input)
            formDialog.hide()
            await loadContacts(query)
            showToast({ body: t(contact ? "contactUpdated" : "contactCreated"), type: "info", uniqueID: "contact-saved", collisionBehavior: "overwrite" })
          } catch {
            showToast({ body: t("contactInvalid"), type: "error", uniqueID: "contact-save-error", collisionBehavior: "overwrite" })
          }
        }}
      />,
      { "aria-label": contact ? t("editContact") : t("newContact") },
    )
  }

  async function deleteContact(contact: Contact) {
    const confirmed = await deleteDialog.confirm({
      title: t("deleteContactTitle"),
      description: t("deleteContactConfirm"),
      cancelLabel: t("cancel"),
      actionLabel: t("delete"),
      actionVariant: "destructive",
    })
    if (!confirmed) return
    setBusyId(contact.id)
    try {
      await api.deleteContact(contact.id)
      setContacts((items) => items.filter((item) => item.id !== contact.id))
      showToast({ body: t("contactDeleted"), type: "info", uniqueID: "contact-deleted", collisionBehavior: "overwrite" })
    } catch {
      showToast({ body: t("genericError"), type: "error", uniqueID: "contact-delete-error", collisionBehavior: "overwrite" })
    } finally {
      setBusyId(null)
    }
  }

  return (
    <>
      <Dialog
        className="contacts-dialog"
        isOpen={isOpen}
        onOpenChange={(open) => { if (!open) onClose() }}
        purpose="info"
        width={680}
        maxHeight="86dvh"
        padding={0}
        aria-label={t("contacts")}
      >
        <Layout
          className="contacts-dialog-layout"
          height="fill"
          padding={4}
          header={
            <DialogHeader
              title={t("contacts")}
              subtitle={t("contactsDescription")}
              startContent={<span className="contacts-dialog-icon"><UsersRound aria-hidden="true" /></span>}
              onOpenChange={(open) => { if (!open) onClose() }}
              hasDivider
            />
          }
          content={
            <LayoutContent className="contacts-dialog-content" padding={0} isScrollable>
              <div className="contacts-toolbar">
                <TextInput
                  label={t("searchContacts")}
                  isLabelHidden
                  startIcon={<Search aria-hidden="true" />}
                  value={query}
                  onChange={setQuery}
                  placeholder={t("searchContacts")}
                  hasClear
                  width="100%"
                />
                <Button label={t("newContact")} icon={<UserRoundPlus aria-hidden="true" />} variant="primary" onClick={() => openForm(null)} />
              </div>

              {loading ? (
                <ContactSkeleton label={t("loading")} />
              ) : filtered.length ? (
                <List className="contacts-list" density="balanced" hasDividers>
                  {filtered.map((contact) => (
                    <Item
                      key={contact.id}
                      as="li"
                      align="start"
                      startContent={<Avatar name={contact.displayName || contact.email} size="md" />}
                      label={<span className="contact-primary">{contact.displayName || contact.email}</span>}
                      description={<span className="contact-secondary"><AtSign aria-hidden="true" />{contact.email}{contact.notes && <small>{contact.notes}</small>}</span>}
                      endContent={
                        <span className="contact-actions">
                          <IconButton label={`${t("editContact")}: ${contact.displayName || contact.email}`} icon={<PencilLine aria-hidden="true" />} variant="ghost" size="sm" onClick={() => openForm(contact)} />
                          <IconButton label={`${t("deleteContact")}: ${contact.displayName || contact.email}`} icon={<Trash2 aria-hidden="true" />} variant="ghost" size="sm" className="danger-text" isDisabled={busyId === contact.id} onClick={() => void deleteContact(contact)} />
                        </span>
                      }
                    />
                  ))}
                </List>
              ) : (
                <div className="contacts-empty">
                  <EmptyState
                    isCompact
                    icon={<ContactRound aria-hidden="true" />}
                    title={query.trim() ? t("noContactsFound") : t("noContacts")}
                    description={query.trim() ? t("noContactsFoundDescription") : t("noContactsDescription")}
                    actions={!query.trim() && <Button label={t("newContact")} icon={<UserRoundPlus aria-hidden="true" />} variant="primary" onClick={() => openForm(null)} />}
                  />
                </div>
              )}
            </LayoutContent>
          }
          footer={
            <LayoutFooter className="contacts-dialog-footer" padding={3} hasDivider>
              <span>{t("contactsCount", { count: filtered.length })}</span>
              <Button label={t("done")} variant="secondary" onClick={onClose} />
            </LayoutFooter>
          }
        />
      </Dialog>
      {formDialog.element}
      {deleteDialog.element}
    </>
  )
}

function ContactForm({ contact, onCancel, onSubmit }: {
  contact: Contact | null
  onCancel: () => void
  onSubmit: (input: ContactInput) => Promise<void>
}) {
  const { t } = useI18n()
  const [input, setInput] = useState<ContactInput>(contact
    ? { displayName: contact.displayName, email: contact.email, notes: contact.notes }
    : emptyContact)
  const [busy, setBusy] = useState(false)
  const canSubmit = Boolean(input.email.trim()) && looksLikeEmail(input.email)

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!canSubmit || busy) return
    setBusy(true)
    try {
      await onSubmit({
        displayName: input.displayName.trim(),
        email: input.email.trim(),
        notes: input.notes?.trim() || "",
      })
    } finally {
      setBusy(false)
    }
  }

  return (
    <form className="contact-form" onSubmit={submit}>
      <Layout
        className="contact-form-layout"
        padding={4}
        header={<DialogHeader title={contact ? t("editContact") : t("newContact")} startContent={<span className="contacts-dialog-icon"><ContactRound aria-hidden="true" /></span>} hasDivider onOpenChange={(open) => { if (!open) onCancel() }} />}
        content={
          <LayoutContent className="contact-form-content" padding={4}>
            <TextInput
              label={t("contactName")}
              value={input.displayName}
              onChange={(displayName) => setInput({ ...input, displayName })}
              placeholder={t("contactNamePlaceholder")}
              width="100%"
            />
            <TextInput
              label={t("email")}
              type="email"
              value={input.email}
              onChange={(email) => setInput({ ...input, email })}
              placeholder={t("emailPlaceholder")}
              width="100%"
              isRequired
            />
            <TextArea
              label={t("contactNotes")}
              value={input.notes || ""}
              onChange={(notes) => setInput({ ...input, notes })}
              placeholder={t("contactNotesPlaceholder")}
              rows={4}
              width="100%"
            />
          </LayoutContent>
        }
        footer={
          <LayoutFooter className="contact-form-footer" padding={3} hasDivider>
            <Button label={t("cancel")} variant="secondary" isDisabled={busy} onClick={onCancel} />
            <Button label={busy ? t("saving") : t("save")} variant="primary" type="submit" isLoading={busy} isDisabled={!canSubmit || busy} />
          </LayoutFooter>
        }
      />
    </form>
  )
}

function ContactSkeleton({ label }: { label: string }) {
  return (
    <div className="contacts-skeleton" aria-label={label} aria-busy="true">
      {Array.from({ length: 5 }, (_, index) => (
        <div className="contacts-skeleton-row" key={index}>
          <Skeleton width={36} height={36} radius="rounded" index={index} />
          <span>
            <Skeleton width="42%" height={12} index={index} />
            <Skeleton width="64%" height={11} index={index + 1} />
          </span>
        </div>
      ))}
    </div>
  )
}

function looksLikeEmail(value: string) {
  return /^[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+$/.test(value.trim())
}
