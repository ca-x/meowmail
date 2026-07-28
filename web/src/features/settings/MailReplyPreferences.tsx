import { Banner } from "@astryxdesign/core/Banner"
import { Card } from "@astryxdesign/core/Card"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { DateTimeInput, type ISODateTimeString } from "@astryxdesign/core/DateTimeInput"
import { MultiSelector } from "@astryxdesign/core/MultiSelector"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { Switch } from "@astryxdesign/core/Switch"
import { TextArea } from "@astryxdesign/core/TextArea"
import { TextInput } from "@astryxdesign/core/TextInput"
import { CalendarClock, MessageSquareReply, MonitorUp } from "lucide-react"

import type { MailAccount, MailPreferences } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

const MIN_MAIL_DATE_TIME = "2000-01-01T00:00" as ISODateTimeString
const MIN_MAIL_TIMESTAMP = 946_684_800

export function MailReplyPreferences({ preferences, accounts, onChange }: {
  preferences: MailPreferences
  accounts: MailAccount[]
  onChange: (preferences: MailPreferences) => void
}) {
  const { t } = useI18n()
  const availableAccountIds = new Set(accounts.map((account) => account.id))
  const selectedAutoReplyAccounts = preferences.autoReplyAccountIds.filter((id) => availableAccountIds.has(id))
  const accountOptions = accounts.map((account) => ({
    value: account.id,
    label: account.displayName,
    description: account.email,
  }))

  return (
    <section className="mail-preference-section" aria-labelledby="reply-settings-title">
      <div className="mail-preference-heading">
        <MessageSquareReply aria-hidden="true" />
        <div><h3 id="reply-settings-title">{t("replyAndForward")}</h3><p>{t("replyAndForwardDescription")}</p></div>
      </div>
      <Card className="mail-preference-card" padding={4}>
        <Switch label={t("attachOriginalOnReply")} description={t("attachOriginalOnReplyDescription")} value={preferences.attachOriginalOnReply} onChange={(attachOriginalOnReply) => onChange({ ...preferences, attachOriginalOnReply })} labelPosition="start" labelSpacing="spread" />
        <div className="mail-preference-choice-row">
          <strong>{t("replySubjectPrefix")}</strong>
          <SegmentedControl value={preferences.subjectPrefixLanguage} onChange={(subjectPrefixLanguage) => onChange({ ...preferences, subjectPrefixLanguage: subjectPrefixLanguage as MailPreferences["subjectPrefixLanguage"] })} label={t("replySubjectPrefix")} size="sm">
            <SegmentedControlItem value="chinese" label={t("useChinesePrefix")} />
            <SegmentedControlItem value="english" label={t("useEnglishPrefix")} />
          </SegmentedControl>
        </div>
        <Switch label={t("automaticForwarding")} description={t("automaticForwardingDescription")} value={preferences.autoForwardEnabled} onChange={(autoForwardEnabled) => onChange({ ...preferences, autoForwardEnabled })} labelPosition="start" labelSpacing="spread" />
        {preferences.autoForwardEnabled && (
          <div className="mail-preference-inset">
            <TextInput label={`${t("forwardToAddress")} · ${t("required")}`} type="email" value={preferences.autoForwardAddress || ""} onChange={(autoForwardAddress) => onChange({ ...preferences, autoForwardAddress })} placeholder={t("forwardToAddressPlaceholder")} width="100%" />
          </div>
        )}
        <Switch label={t("automaticReply")} description={t("automaticReplyDescription")} value={preferences.autoReplyEnabled} onChange={(autoReplyEnabled) => onChange({ ...preferences, autoReplyEnabled })} labelPosition="start" labelSpacing="spread" />
        {preferences.autoReplyEnabled && (
          <div className="mail-preference-inset vacation-reply-panel">
            <div className="vacation-reply-heading">
              <CalendarClock aria-hidden="true" />
              <div><strong>{t("vacationReply")}</strong><small>{t("vacationReplyDescription")}</small></div>
            </div>
            <MultiSelector
              label={t("automaticReplyAccountScope")}
              description={t("automaticReplyAccountScopeDescription")}
              value={selectedAutoReplyAccounts}
              onChange={(autoReplyAccountIds) => onChange({ ...preferences, autoReplyAccountIds })}
              options={accountOptions}
              placeholder={t("allAccounts")}
              hasClear
              hasSelectAll
              selectAllLabel={t("allAccounts")}
              triggerDisplay="labels"
              width="100%"
              isDisabled={!accountOptions.length}
              disabledMessage={t("automaticReplyNoAccounts")}
            />
            <div className="vacation-reply-grid">
              <DateTimeInput
                label={t("vacationReplyStart")}
                value={epochToLocalDateTime(preferences.autoReplyStartAt)}
                onChange={(value) => onChange({ ...preferences, autoReplyStartAt: localDateTimeToEpoch(value) })}
                min={MIN_MAIL_DATE_TIME}
                hasClear
                hourFormat="24h"
                timeIncrement={30}
                width="100%"
              />
              <DateTimeInput
                label={t("vacationReplyEnd")}
                value={epochToLocalDateTime(preferences.autoReplyEndAt)}
                onChange={(value) => onChange({ ...preferences, autoReplyEndAt: localDateTimeToEpoch(value) })}
                min={epochToLocalDateTime(preferences.autoReplyStartAt) || MIN_MAIL_DATE_TIME}
                hasClear
                hourFormat="24h"
                timeIncrement={30}
                width="100%"
              />
            </div>
            <TextInput
              label={t("automaticReplySubject")}
              value={preferences.autoReplySubject}
              onChange={(autoReplySubject) => onChange({ ...preferences, autoReplySubject })}
              placeholder={t("automaticReplySubjectPlaceholder")}
              width="100%"
            />
            <TextArea label={`${t("automaticReplyContent")} · ${t("required")}`} value={preferences.autoReplyText} onChange={(autoReplyText) => onChange({ ...preferences, autoReplyText })} placeholder={t("automaticReplyContentPlaceholder")} rows={5} width="100%" />
            <CheckboxInput
              label={t("automaticReplyContactsOnly")}
              description={t("automaticReplyContactsOnlyDescription")}
              value={preferences.autoReplyContactsOnly}
              onChange={(autoReplyContactsOnly) => onChange({ ...preferences, autoReplyContactsOnly })}
            />
          </div>
        )}
        <Banner status="info" title={t("automaticMailSafetyNote")} icon={<MonitorUp aria-hidden="true" />} />
      </Card>
    </section>
  )
}

function epochToLocalDateTime(value?: number | null): ISODateTimeString | undefined {
  if (!value) return undefined
  return toLocalDateTimeInput(new Date(value * 1000))
}

function localDateTimeToEpoch(value: ISODateTimeString | undefined) {
  if (!value) return null
  const timestamp = new Date(value).getTime()
  if (!Number.isFinite(timestamp)) return null
  const seconds = Math.floor(timestamp / 1000)
  return seconds >= MIN_MAIL_TIMESTAMP ? seconds : null
}

function toLocalDateTimeInput(date: Date) {
  const pad = (value: number) => String(value).padStart(2, "0")
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}` as ISODateTimeString
}
