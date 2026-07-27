import { Avatar } from "@astryxdesign/core/Avatar"
import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { Dialog, DialogHeader } from "@astryxdesign/core/Dialog"
import { Layout, LayoutContent, LayoutFooter } from "@astryxdesign/core/Layout"
import { ChevronRight, Mail, MailPlus } from "lucide-react"
import { useEffect, useState } from "react"

import type { MailAccount } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { AccountDialog } from "./AccountDialog"

export function AccountManagerDialog({ isOpen, accounts, onClose, onChanged, onNotice }: {
  isOpen: boolean
  accounts: MailAccount[]
  onClose: () => void
  onChanged: () => void | Promise<void>
  onNotice: (key: MessageKey) => void
}) {
  const { t } = useI18n()
  const [editing, setEditing] = useState<MailAccount | null | undefined>(undefined)

  useEffect(() => {
    if (!isOpen) setEditing(undefined)
  }, [isOpen])

  async function changed(key: MessageKey) {
    setEditing(undefined)
    try {
      await onChanged()
      onNotice(key)
    } catch {
      onNotice("genericError")
    }
  }

  return (
    <>
      <Dialog
        className="account-manager-dialog"
        isOpen={isOpen}
        onOpenChange={(open) => { if (!open && editing === undefined) onClose() }}
        purpose="form"
        width={620}
        maxHeight="calc(100dvh - 24px)"
        padding={0}
        aria-label={t("manageAccounts")}
      >
        <Layout
          className="account-manager-layout"
          height="fill"
          padding={4}
          header={
            <DialogHeader
              title={t("manageAccounts")}
              subtitle={accounts.length ? t("accountsConfigured", { count: accounts.length }) : t("noAccountsDescription")}
              startContent={<span className="account-dialog-icon"><Mail aria-hidden="true" /></span>}
              onOpenChange={(open) => { if (!open) onClose() }}
              hasDivider
            />
          }
          content={
            <LayoutContent className="account-manager-content" padding={0} isScrollable>
              {accounts.length ? (
                <ul className="account-manager-list" aria-label={t("accounts")}>
                  {accounts.map((account) => (
                    <li key={account.id}>
                      <button type="button" className="account-manager-row" onClick={() => setEditing(account)} aria-label={`${t("editAccount")}: ${account.displayName}`}>
                        <Avatar size="sm" name={account.displayName} />
                        <span className="account-manager-copy">
                          <strong>{account.displayName}</strong>
                          <small>{account.email}</small>
                        </span>
                        {account.isDefault && <Badge className="account-manager-default" label={t("defaultAccount")} variant="info" />}
                        <ChevronRight aria-hidden="true" />
                      </button>
                    </li>
                  ))}
                </ul>
              ) : (
                <div className="account-manager-empty">
                  <span><MailPlus aria-hidden="true" /></span>
                  <h3>{t("noAccounts")}</h3>
                  <p>{t("noAccountsDescription")}</p>
                  <Button label={t("addFirstAccount")} icon={<MailPlus aria-hidden="true" />} variant="primary" onClick={() => setEditing(null)} />
                </div>
              )}
            </LayoutContent>
          }
          footer={
            <LayoutFooter className="account-manager-footer" padding={3} hasDivider>
              <Button label={t("addAccount")} icon={<MailPlus aria-hidden="true" />} variant="secondary" onClick={() => setEditing(null)} />
              <Button label={t("done")} variant="primary" onClick={onClose} />
            </LayoutFooter>
          }
        />
      </Dialog>

      <AccountDialog
        isOpen={editing !== undefined}
        account={editing ?? null}
        onClose={() => setEditing(undefined)}
        onSaved={() => { void changed("savedSuccess") }}
        onDeleted={() => { void changed("deletedSuccess") }}
      />
    </>
  )
}
