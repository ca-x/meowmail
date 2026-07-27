import { NumberInput } from "@astryxdesign/core/NumberInput"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { Selector } from "@astryxdesign/core/Selector"
import { TextInput } from "@astryxdesign/core/TextInput"
import { Server, ShieldCheck } from "lucide-react"

import type { AccountInput, ConnectionSecurity, ProxyKind } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

interface AccountIdentityFieldsProps {
  input: AccountInput
  isEditing: boolean
  onChange: (input: AccountInput) => void
}

export function AccountIdentityFields({ input, isEditing, onChange }: AccountIdentityFieldsProps) {
  const { t } = useI18n()

  return (
    <section className="account-form-section account-form-grid" aria-label={t("accountInformation")}>
      <TextInput
        label={`${t("displayName")} · ${t("required")}`}
        placeholder={t("displayNamePlaceholder")}
        value={input.displayName}
        onChange={(displayName) => onChange({ ...input, displayName })}
        hasAutoFocus
        width="100%"
      />
      <TextInput
        label={`${t("email")} · ${t("required")}`}
        placeholder={t("emailPlaceholder")}
        value={input.email}
        onChange={(email) => onChange({
          ...input,
          email,
          username: !input.username || input.username === input.email ? email : input.username,
        })}
        type="email"
        width="100%"
      />
      <TextInput
        label={`${t("username")} · ${t("required")}`}
        placeholder={t("usernamePlaceholder")}
        value={input.username}
        onChange={(username) => onChange({ ...input, username })}
        width="100%"
      />
      <PasswordField
        label={t("password")}
        description={isEditing ? t("passwordKeep") : undefined}
        placeholder={isEditing ? t("passwordKeep") : t("passwordPlaceholder")}
        value={input.password || ""}
        onChange={(password) => onChange({ ...input, password })}
        required={!isEditing}
        autoComplete="new-password"
      />
    </section>
  )
}

export function AccountServerSettings({ input, onChange }: Pick<AccountIdentityFieldsProps, "input" | "onChange">) {
  const { t } = useI18n()

  return (
    <section className="account-form-section account-server-grid" aria-label={t("serverSettings")}>
      <ServerFields title={t("imapServer")} value={input.imap} onChange={(imap) => onChange({ ...input, imap })} />
      <ServerFields title={t("smtpServer")} value={input.smtp} onChange={(smtp) => onChange({ ...input, smtp })} />
    </section>
  )
}

export function AccountProxySettings({ input, onChange }: Pick<AccountIdentityFieldsProps, "input" | "onChange">) {
  const { t } = useI18n()

  return (
    <section className="account-form-section account-proxy-section" aria-label={t("proxy")}>
      <div className="account-section-heading">
        <ShieldCheck aria-hidden="true" />
        <div><strong>{t("proxy")}</strong><small>{t("proxyDescription")}</small></div>
      </div>
      <SegmentedControl
        value={input.proxy.kind}
        onChange={(kind) => onChange({ ...input, proxy: { ...input.proxy, kind: kind as ProxyKind } })}
        label={t("proxy")}
        size="sm"
      >
        <SegmentedControlItem value="direct" label={t("direct")} />
        <SegmentedControlItem value="http" label={t("http")} />
        <SegmentedControlItem value="socks5" label={t("socks5")} />
      </SegmentedControl>
      {input.proxy.kind !== "direct" && (
        <div className="account-form-grid account-proxy-fields">
          <TextInput label={`${t("host")} · ${t("required")}`} placeholder={t("proxyHostPlaceholder")} value={input.proxy.host || ""} onChange={(host) => onChange({ ...input, proxy: { ...input.proxy, host } })} width="100%" />
          <NumberInput label={`${t("port")} · ${t("required")}`} placeholder={t("proxyPortPlaceholder")} value={input.proxy.port} onChange={(port) => onChange({ ...input, proxy: { ...input.proxy, port } })} min={1} max={65_535} isIntegerOnly hasClear width="100%" />
          <TextInput label={t("proxyUsername")} placeholder={t("proxyUsernamePlaceholder")} value={input.proxy.username || ""} onChange={(username) => onChange({ ...input, proxy: { ...input.proxy, username } })} width="100%" />
          <PasswordField label={t("proxyPassword")} placeholder={t("proxyPasswordPlaceholder")} value={input.proxy.password || ""} onChange={(password) => onChange({ ...input, proxy: { ...input.proxy, password } })} autoComplete="new-password" />
        </div>
      )}
    </section>
  )
}

function PasswordField({ label, description, placeholder, value, onChange, required = false, autoComplete }: {
  label: string
  description?: string
  placeholder?: string
  value: string
  onChange: (value: string) => void
  required?: boolean
  autoComplete?: string
}) {
  return (
    <label className="account-native-field">
      <span>{label}</span>
      {description && <small>{description}</small>}
      <input
        type="password"
        value={value}
        placeholder={placeholder}
        autoComplete={autoComplete}
        onChange={(event) => onChange(event.target.value)}
        required={required}
      />
    </label>
  )
}

function ServerFields({ title, value, onChange }: {
  title: string
  value: AccountInput["imap"]
  onChange: (value: AccountInput["imap"]) => void
}) {
  const { t } = useI18n()

  return (
    <section className="account-server-card" aria-label={title}>
      <div className="account-section-heading"><Server aria-hidden="true" /><strong>{title}</strong></div>
      <TextInput label={`${t("host")} · ${t("required")}`} placeholder={t("hostPlaceholder")} value={value.host} onChange={(host) => onChange({ ...value, host })} width="100%" />
      <div className="account-server-row">
        <NumberInput label={`${t("port")} · ${t("required")}`} placeholder={t("portPlaceholder")} value={value.port} onChange={(port) => onChange({ ...value, port })} min={1} max={65_535} isIntegerOnly hasClear width="100%" />
        <Selector
          label={t("security")}
          value={value.security}
          onChange={(security) => onChange({ ...value, security: security as ConnectionSecurity })}
          options={[
            { value: "tls", label: t("tls") },
            { value: "starttls", label: t("starttls") },
          ]}
          width="100%"
        />
      </div>
    </section>
  )
}
