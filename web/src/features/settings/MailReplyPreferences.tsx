import { Banner } from "@astryxdesign/core/Banner"
import { Card } from "@astryxdesign/core/Card"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { Switch } from "@astryxdesign/core/Switch"
import { TextArea } from "@astryxdesign/core/TextArea"
import { TextInput } from "@astryxdesign/core/TextInput"
import { MessageSquareReply, MonitorUp } from "lucide-react"

import type { MailPreferences } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function MailReplyPreferences({ preferences, onChange }: {
  preferences: MailPreferences
  onChange: (preferences: MailPreferences) => void
}) {
  const { t } = useI18n()

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
          <div className="mail-preference-inset">
            <TextArea label={`${t("automaticReplyContent")} · ${t("required")}`} value={preferences.autoReplyText} onChange={(autoReplyText) => onChange({ ...preferences, autoReplyText })} placeholder={t("automaticReplyContentPlaceholder")} rows={5} width="100%" />
          </div>
        )}
        <Banner status="info" title={t("automaticMailSafetyNote")} icon={<MonitorUp aria-hidden="true" />} />
      </Card>
    </section>
  )
}
