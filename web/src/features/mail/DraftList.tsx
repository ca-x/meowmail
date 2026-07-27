import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { EmptyState } from "@astryxdesign/core/EmptyState"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Item } from "@astryxdesign/core/Item"
import { List } from "@astryxdesign/core/List"
import { Skeleton } from "@astryxdesign/core/Skeleton"
import { Clock3, FilePenLine, Inbox, SendHorizontal, Trash2 } from "lucide-react"

import type { EmailDraft, MailAccount } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function DraftList({ drafts, accounts, loading, busyId, onOpenDraft, onDeleteDraft, onSendDraft }: {
  drafts: EmailDraft[]
  accounts: MailAccount[]
  loading: boolean
  busyId: string | null
  onOpenDraft: (draft: EmailDraft) => void
  onDeleteDraft: (draft: EmailDraft) => void
  onSendDraft: (draft: EmailDraft) => void
}) {
  const { locale, t } = useI18n()

  if (loading) return <DraftListSkeleton label={t("loading")} />
  if (!drafts.length) {
    return (
      <div className="message-list-empty">
        <EmptyState
          isCompact
          icon={<FilePenLine aria-hidden="true" />}
          title={t("noDrafts")}
          description={t("noDraftsDescription")}
        />
      </div>
    )
  }

  return (
    <div className="message-list-scroll" data-testid="draft-list-scroll">
      <List className="draft-list" density="balanced" hasDividers>
        {drafts.map((draft) => {
          const account = accounts.find((item) => item.id === draft.accountId)
          const isBusy = busyId === draft.id
          return (
            <Item
              key={draft.id}
              as="li"
              align="start"
              className="draft-item"
              startContent={<span className="draft-item-icon"><FilePenLine aria-hidden="true" /></span>}
              label={
                <span className="draft-item-heading">
                  <strong>{draft.subject || t("noSubject")}</strong>
                  {draft.scheduledAt
                    ? <Badge label={formatDateTime(draft.scheduledAt, locale)} icon={<Clock3 aria-hidden="true" />} variant="warning" />
                    : <Badge label={t(draftStatusKey(draft.status))} variant={draft.status === "ambiguous" ? "error" : "neutral"} />}
                </span>
              }
              description={
                <span className="draft-item-copy">
                  <span>{t("to")}: {draft.to.length ? draft.to.join(", ") : t("draftNoRecipients")}</span>
                  <small>{account?.displayName || t("unknownAccount")} · {formatDateTime(draft.updatedAt, locale)}</small>
                  {draft.lastError && <small className="draft-error">{draft.lastError}</small>}
                </span>
              }
              endContent={
                <span className="draft-actions">
                  <Button label={t("editDraft")} icon={<FilePenLine aria-hidden="true" />} variant="ghost" size="sm" isDisabled={isBusy || draft.status !== "draft"} onClick={() => onOpenDraft(draft)} />
                  <IconButton label={t("sendNow")} icon={<SendHorizontal aria-hidden="true" />} variant="ghost" size="sm" isDisabled={isBusy || draft.status !== "draft" || !draft.to.length} onClick={() => onSendDraft(draft)} />
                  <IconButton label={t("deleteDraft")} icon={<Trash2 aria-hidden="true" />} variant="ghost" size="sm" className="danger-text" isDisabled={isBusy || draft.status === "sending"} onClick={() => onDeleteDraft(draft)} />
                </span>
              }
            />
          )
        })}
      </List>
    </div>
  )
}

function DraftListSkeleton({ label }: { label: string }) {
  return (
    <div className="message-list-skeleton" aria-label={label} aria-busy="true">
      {Array.from({ length: 5 }, (_, index) => (
        <div className="message-skeleton-row" key={index}>
          <Skeleton width={36} height={36} radius={3} index={index} />
          <span>
            <Skeleton width="54%" height={12} index={index} />
            <Skeleton width="84%" height={11} index={index + 1} />
            <Skeleton width="48%" height={10} index={index + 2} />
          </span>
        </div>
      ))}
    </div>
  )
}

function draftStatusKey(status: EmailDraft["status"]) {
  if (status === "sending") return "draftStatusSending"
  if (status === "ambiguous") return "draftStatusAmbiguous"
  if (status === "sent") return "draftStatusSent"
  return "draftStatusDraft"
}

function formatDateTime(timestamp: number, locale: string) {
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(timestamp * 1000))
}
