import { Button } from "@astryxdesign/core/Button"
import { EmptyState } from "@astryxdesign/core/EmptyState"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { Inbox, Plus, RefreshCw } from "lucide-react"

import type { MailAccount, MailPreferences, MessageSummary } from "../../../app/types"
import { useI18n } from "../../../i18n/I18nProvider"
import { MessageList } from "../MessageList"
import type { MailFilter } from "./types"

const filters: MailFilter[] = ["inbox", "unread", "starred", "attachments"]

export function MessageColumn({ accounts, activeAccount, filter, messages, selectedId, loading, syncing, preferences, onChooseFilter, onSync, onAddAccount, onSelect, onToggleStar }: {
  accounts: MailAccount[]
  activeAccount: MailAccount | null
  filter: MailFilter
  messages: MessageSummary[]
  selectedId: string | null
  loading: boolean
  syncing: boolean
  preferences: MailPreferences
  onChooseFilter: (filter: MailFilter) => void
  onSync: () => void
  onAddAccount: () => void
  onSelect: (message: MessageSummary) => void
  onToggleStar: (message: MessageSummary) => void
}) {
  const { t } = useI18n()

  return (
    <section className="message-column" aria-labelledby="message-column-title">
      <header className="message-column-header">
        <div>
          <p>{activeAccount?.displayName || t("allAccounts")}</p>
          <h1 id="message-column-title">{t(filter)}</h1>
        </div>
        <Button
          label={syncing ? t("syncing") : t("sync")}
          icon={<RefreshCw className={syncing ? "rotating" : undefined} aria-hidden="true" />}
          variant="ghost"
          size="sm"
          isDisabled={!accounts.length || syncing}
          onClick={onSync}
        />
      </header>

      <div className="message-filter-bar">
        <SegmentedControl value={filter} onChange={(value) => onChooseFilter(value as MailFilter)} label={t("mailFilters")} size="sm">
          {filters.map((value) => <SegmentedControlItem key={value} value={value} label={t(value)} />)}
        </SegmentedControl>
        <span className="message-count" aria-label={t("messageCount", { count: messages.length })}>{messages.length}</span>
      </div>

      {!accounts.length && !loading ? (
        <div className="message-column-empty">
          <EmptyState
            icon={<Inbox aria-hidden="true" />}
            title={t("noAccounts")}
            description={t("noAccountsDescription")}
            actions={<Button label={t("addFirstAccount")} icon={<Plus aria-hidden="true" />} variant="primary" onClick={onAddAccount} />}
          />
        </div>
      ) : (
        <MessageList
          messages={messages}
          selectedId={selectedId}
          loading={loading}
          preferences={preferences}
          onSelect={onSelect}
          onToggleStar={onToggleStar}
        />
      )}
    </section>
  )
}
