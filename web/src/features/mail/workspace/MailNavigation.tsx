import { Avatar } from "@astryxdesign/core/Avatar"
import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { IconButton } from "@astryxdesign/core/IconButton"
import { TreeList, type TreeListItemData } from "@astryxdesign/core/TreeList"
import { ChevronDown, FilePenLine, FileText, Inbox, LogOut, MailPlus, NotebookTabs, Paperclip, Plus, Send, Settings, Star, Trash2 } from "lucide-react"
import { useMemo, useState } from "react"

import type { MailAccount } from "../../../app/types"
import { useI18n } from "../../../i18n/I18nProvider"
import type { MailFilter } from "./types"

export function MailNavigation({ accounts, activeAccount, activeAccountId, filter, unreadCount, draftCount, onChooseAccount, onChooseFilter, onCompose, onOpenContacts, onEditAccount, onAddAccount, onOpenSettings, onLogout }: {
  accounts: MailAccount[]
  activeAccount: MailAccount | null
  activeAccountId: string | null
  filter: MailFilter
  unreadCount: number
  draftCount: number
  onChooseAccount: (id: string | null) => void
  onChooseFilter: (filter: MailFilter) => void
  onCompose: () => void
  onOpenContacts: () => void
  onEditAccount: (account: MailAccount | null) => void
  onAddAccount: () => void
  onOpenSettings: () => void
  onLogout: () => void
}) {
  const { t } = useI18n()
  const [accountsExpanded, setAccountsExpanded] = useState(true)
  const folderItems = useMemo<TreeListItemData[]>(() => [
    { id: "inbox", label: t("inbox"), startContent: <Inbox aria-hidden="true" />, endContent: unreadCount > 0 ? <Badge label={unreadCount} variant="info" /> : undefined, isSelected: filter === "inbox", onClick: () => onChooseFilter("inbox") },
    { id: "starred", label: t("starred"), startContent: <Star aria-hidden="true" />, isSelected: filter === "starred", onClick: () => onChooseFilter("starred") },
    { id: "unread", label: t("unread"), startContent: <FileText aria-hidden="true" />, isSelected: filter === "unread", onClick: () => onChooseFilter("unread") },
    { id: "attachments", label: t("attachments"), startContent: <Paperclip aria-hidden="true" />, isSelected: filter === "attachments", onClick: () => onChooseFilter("attachments") },
    { id: "drafts", label: t("drafts"), startContent: <FilePenLine aria-hidden="true" />, endContent: draftCount > 0 ? <Badge label={draftCount} variant="neutral" /> : undefined, isSelected: filter === "drafts", onClick: () => onChooseFilter("drafts") },
    { id: "sent", label: t("sent"), startContent: <Send aria-hidden="true" />, isDisabled: true },
    { id: "trash", label: t("trash"), startContent: <Trash2 aria-hidden="true" />, isDisabled: true },
  ], [draftCount, filter, onChooseFilter, t, unreadCount])

  const accountItems = useMemo<TreeListItemData[]>(() => [
    {
      id: "all-accounts",
      label: t("allAccounts"),
      description: `${accounts.length} ${t("accounts")}`,
      startContent: <Avatar size="xsm" name={t("allAccounts")} />,
      isSelected: activeAccountId === null,
      onClick: () => onChooseAccount(null),
    },
    ...accounts.map((account) => ({
      id: account.id,
      label: account.displayName,
      description: account.email,
      startContent: <Avatar size="xsm" name={account.displayName} />,
      endContent: account.isDefault ? <span className="default-account-dot" aria-label={t("defaultAccount")} /> : undefined,
      isSelected: account.id === activeAccountId,
      onClick: () => onChooseAccount(account.id),
    })),
  ], [accounts, activeAccountId, onChooseAccount, t])

  return (
    <div className="mail-navigation">
      <div className="mail-navigation-account">
        <Button
          label={activeAccount ? t("editAccount") : t("addAccount")}
          variant="ghost"
          width="100%"
          onClick={() => onEditAccount(activeAccount)}
        >
          <span className="active-account-summary">
            <Avatar size="md" name={activeAccount?.displayName || t("allAccounts")} />
            <span>
              <strong>{activeAccount?.displayName || t("allAccounts")}</strong>
              <small>{activeAccount?.email || `${accounts.length} ${t("accounts")}`}</small>
            </span>
            <Settings aria-hidden="true" />
          </span>
        </Button>
      </div>

      <div className="mail-navigation-compose">
        <Button
          label={t("compose")}
          icon={<MailPlus aria-hidden="true" />}
          variant="primary"
          width="100%"
          isDisabled={!accounts.length}
          onClick={onCompose}
        />
        <Button
          label={t("contacts")}
          icon={<NotebookTabs aria-hidden="true" />}
          variant="secondary"
          width="100%"
          onClick={onOpenContacts}
        />
      </div>

      <nav className="mail-navigation-folders" aria-label={t("mailFolders")}>
        <TreeList items={folderItems} density="compact" />
      </nav>

      <section className={`mail-navigation-accounts${accountsExpanded ? "" : " is-collapsed"}`} aria-labelledby="mail-account-heading">
        <div className="mail-navigation-section-heading">
          <button
            type="button"
            className="mail-navigation-section-toggle"
            aria-expanded={accountsExpanded}
            aria-controls="mail-account-list"
            onClick={() => setAccountsExpanded((value) => !value)}
          >
            <ChevronDown aria-hidden="true" />
            <span id="mail-account-heading">{t("accounts")}</span>
            <span className="visually-hidden">{accountsExpanded ? t("collapseAccounts") : t("expandAccounts")}</span>
          </button>
          <IconButton label={t("addAccount")} icon={<Plus aria-hidden="true" />} variant="ghost" size="sm" onClick={onAddAccount} />
        </div>
        {accountsExpanded && <div id="mail-account-list"><TreeList items={accountItems} density="compact" /></div>}
      </section>

      <footer className="mail-navigation-footer">
        <Button label={t("settings")} icon={<Settings aria-hidden="true" />} variant="ghost" size="sm" onClick={onOpenSettings} />
        <Button label={t("logout")} icon={<LogOut aria-hidden="true" />} variant="ghost" size="sm" onClick={onLogout} />
      </footer>
    </div>
  )
}
