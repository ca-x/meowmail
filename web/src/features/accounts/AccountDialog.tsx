import { Banner } from "@astryxdesign/core/Banner"
import { Button } from "@astryxdesign/core/Button"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { Dialog, DialogHeader } from "@astryxdesign/core/Dialog"
import { Layout, LayoutContent, LayoutFooter } from "@astryxdesign/core/Layout"
import { MailPlus, Server, Trash2 } from "lucide-react"
import { useEffect, useState, type FormEvent } from "react"

import { api } from "../../app/api"
import type { AccountInput, MailAccount } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"
import { AccountIdentityFields, AccountProxySettings, AccountServerSettings } from "./AccountFormFields"

interface Props {
  isOpen?: boolean
  account: MailAccount | null
  onClose: () => void
  onSaved: (account: MailAccount) => void
  onDeleted: () => void
}

type AccountPreset = "gmail" | "outlook" | "qq" | "netease163" | "tencentExmail" | "aliyunEnterprise" | "custom"

const serverPresets: Record<Exclude<AccountPreset, "custom">, Pick<AccountInput, "imap" | "smtp">> = {
  gmail: {
    imap: { host: "imap.gmail.com", port: 993, security: "tls" },
    smtp: { host: "smtp.gmail.com", port: 465, security: "tls" },
  },
  outlook: {
    imap: { host: "outlook.office365.com", port: 993, security: "tls" },
    smtp: { host: "smtp.office365.com", port: 587, security: "starttls" },
  },
  qq: {
    imap: { host: "imap.qq.com", port: 993, security: "tls" },
    smtp: { host: "smtp.qq.com", port: 465, security: "tls" },
  },
  netease163: {
    imap: { host: "imap.163.com", port: 993, security: "tls" },
    smtp: { host: "smtp.163.com", port: 465, security: "tls" },
  },
  tencentExmail: {
    imap: { host: "imap.exmail.qq.com", port: 993, security: "tls" },
    smtp: { host: "smtp.exmail.qq.com", port: 465, security: "tls" },
  },
  aliyunEnterprise: {
    imap: { host: "imap.qiye.aliyun.com", port: 993, security: "tls" },
    smtp: { host: "smtp.qiye.aliyun.com", port: 465, security: "tls" },
  },
}

function emptyInput(): AccountInput {
  return {
    displayName: "",
    email: "",
    username: "",
    password: "",
    imap: { host: "", port: 993, security: "tls" },
    smtp: { host: "", port: 465, security: "tls" },
    proxy: { kind: "direct" },
    isDefault: false,
  }
}

function fromAccount(account: MailAccount): AccountInput {
  return {
    displayName: account.displayName,
    email: account.email,
    username: account.username,
    password: "",
    imap: { ...account.imap },
    smtp: { ...account.smtp },
    proxy: {
      kind: account.proxy.kind,
      host: account.proxy.host || "",
      port: account.proxy.port || undefined,
      username: account.proxy.username || "",
      password: "",
    },
    isDefault: account.isDefault,
  }
}

export function AccountDialog({ isOpen = true, account, onClose, onSaved, onDeleted }: Props) {
  const { t } = useI18n()
  const deleteDialog = useImperativeConfirmDialog()
  const [input, setInput] = useState<AccountInput>(() => account ? fromAccount(account) : emptyInput())
  const [busy, setBusy] = useState<"save" | "test" | "delete" | null>(null)
  const [message, setMessage] = useState<MessageKey | null>(null)

  useEffect(() => {
    if (!isOpen) return
    setInput(account ? fromAccount(account) : emptyInput())
    setMessage(null)
  }, [account, isOpen])

  function preset(kind: AccountPreset, displayName: string) {
    if (kind === "custom") {
      setInput((value) => ({
        ...value,
        imap: { ...value.imap, host: "" },
        smtp: { ...value.smtp, host: "" },
      }))
      return
    }
    const servers = serverPresets[kind]
    setInput((value) => ({
      ...value,
      displayName: value.displayName || displayName,
      imap: { ...servers.imap },
      smtp: { ...servers.smtp },
    }))
  }

  async function submit(event: FormEvent) {
    event.preventDefault()
    setBusy("save")
    setMessage(null)
    try {
      const payload = normalize(input, Boolean(account))
      const saved = account
        ? await api.updateAccount(account.id, payload)
        : await api.createAccount(payload)
      onSaved(saved)
    } catch {
      setMessage("genericError")
    } finally {
      setBusy(null)
    }
  }

  async function testConnection() {
    setBusy("test")
    setMessage(null)
    try {
      if (account && !input.password && !input.proxy.password) await api.testSavedAccount(account.id)
      else await api.testAccount(normalize(input, false))
      setMessage("connectionOk")
    } catch {
      setMessage("genericError")
    } finally {
      setBusy(null)
    }
  }

  async function requestDelete() {
    if (!account) return
    const confirmed = await deleteDialog.confirm({
      title: t("deleteAccountConfirm", { account: account.displayName }),
      description: account.email,
      cancelLabel: t("cancel"),
      actionLabel: t("delete"),
      actionVariant: "destructive",
    })
    if (!confirmed) return
    setBusy("delete")
    try {
      await api.deleteAccount(account.id)
      onDeleted()
    } catch {
      setMessage("genericError")
      setBusy(null)
    }
  }

  const isBusy = Boolean(busy)
  const isComplete = isAccountInputComplete(input, Boolean(account))

  return (
    <>
      <Dialog
        className="account-dialog"
        isOpen={isOpen}
        onOpenChange={(open) => { if (!open && !isBusy) onClose() }}
        purpose="form"
        width={820}
        maxHeight="calc(100dvh - 24px)"
        padding={0}
        aria-label={account ? t("editAccount") : t("addAccount")}
      >
        <form className="account-dialog-form" onSubmit={submit}>
          <Layout
            className="account-dialog-layout"
            height="fill"
            padding={4}
            header={
              <DialogHeader
                title={account ? t("editAccount") : t("addAccount")}
                subtitle={t("accounts")}
                startContent={<span className="account-dialog-icon"><MailPlus aria-hidden="true" /></span>}
                onOpenChange={isBusy ? undefined : (open) => { if (!open) onClose() }}
                hasDivider
              />
            }
            content={
              <LayoutContent className="account-dialog-content" padding={0} isScrollable>
                <div className="account-dialog-sections">
                  <section className="account-form-section account-preset-section" aria-label={t("preset")}>
                    <div className="account-preset-heading">
                      <strong>{t("preset")}</strong>
                      <small>{t("presetDescription")}</small>
                    </div>
                    <div className="account-preset-row">
                      <Button label="Gmail" variant="secondary" size="sm" onClick={() => preset("gmail", "Gmail")} />
                      <Button label="Outlook" variant="secondary" size="sm" onClick={() => preset("outlook", "Outlook")} />
                      <Button label={t("providerQqMail")} variant="secondary" size="sm" onClick={() => preset("qq", t("providerQqMail"))} />
                      <Button label={t("provider163Mail")} variant="secondary" size="sm" onClick={() => preset("netease163", t("provider163Mail"))} />
                      <Button label={t("providerTencentExmail")} variant="secondary" size="sm" onClick={() => preset("tencentExmail", t("providerTencentExmail"))} />
                      <Button label={t("providerAliyunMail")} variant="secondary" size="sm" onClick={() => preset("aliyunEnterprise", t("providerAliyunMail"))} />
                      <Button label={t("custom")} icon={<Server aria-hidden="true" />} variant="secondary" size="sm" onClick={() => preset("custom", t("custom"))} />
                    </div>
                  </section>

                  <AccountIdentityFields input={input} isEditing={Boolean(account)} onChange={setInput} />
                  <AccountServerSettings input={input} onChange={setInput} />
                  <AccountProxySettings input={input} onChange={setInput} />

                  <section className="account-form-section account-default-section">
                    <CheckboxInput label={t("defaultAccount")} value={input.isDefault} onChange={(isDefault) => setInput({ ...input, isDefault })} />
                  </section>

                  {message && <Banner status={message === "connectionOk" ? "success" : "error"} title={t(message)} container="section" />}
                </div>
              </LayoutContent>
            }
            footer={
              <LayoutFooter className="account-dialog-footer" padding={3} hasDivider>
                <span>
                  {account && <Button label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="destructive" isDisabled={isBusy} onClick={() => void requestDelete()} />}
                </span>
                <span className="account-dialog-actions">
                  <Button label={busy === "test" ? t("testing") : t("testConnection")} variant="secondary" size="lg" isLoading={busy === "test"} isDisabled={isBusy || !isComplete} onClick={() => void testConnection()} />
                  <Button label={busy === "save" ? t("saving") : t("save")} variant="primary" size="lg" type="submit" isLoading={busy === "save"} isDisabled={isBusy || !isComplete} />
                </span>
              </LayoutFooter>
            }
          />
        </form>
      </Dialog>
      {deleteDialog.element}
    </>
  )
}

function normalize(input: AccountInput, editing: boolean): AccountInput {
  const next: AccountInput = structuredClone(input)
  if (editing && !next.password) delete next.password
  if (!next.proxy.password) delete next.proxy.password
  if (!next.proxy.username) delete next.proxy.username
  if (next.proxy.kind === "direct") next.proxy = { kind: "direct" }
  return next
}

function isAccountInputComplete(input: AccountInput, editing: boolean) {
  const requiredText = [input.displayName, input.email, input.username, input.imap.host, input.smtp.host]
  if (requiredText.some((value) => !value.trim())) return false
  if (!editing && !input.password?.trim()) return false
  if (!isValidPort(input.imap.port) || !isValidPort(input.smtp.port)) return false
  if (input.proxy.kind === "direct") return true
  return Boolean(input.proxy.host?.trim() && isValidPort(input.proxy.port))
}

function isValidPort(port: number | null | undefined) {
  return typeof port === "number" && Number.isInteger(port) && port >= 1 && port <= 65_535
}
