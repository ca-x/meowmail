import { Link } from "@astryxdesign/core/Link"
import { MetadataList, MetadataListItem } from "@astryxdesign/core/MetadataList"
import { GitFork, Info, Scale } from "lucide-react"

import type { SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { SettingsPanelHeading } from "./SettingsPanelHeading"

const repositoryUrl = "https://github.com/ca-x/meowmail"

export function AboutSettingsPanel({ session }: { session: SessionResponse }) {
  const { t } = useI18n()
  return (
    <div className="settings-panel-stack">
      <SettingsPanelHeading icon={<Info />} title={t("aboutMeowmail")} description={t("aboutMeowmailDescription")} />
      <section className="settings-about-block" aria-label={t("aboutMeowmail")}>
        <div className="settings-about-brand">
          <img src="/meowmail-logo.png" alt="" />
          <div>
            <strong>{t("brandName")}</strong>
            <small>{t("metaDescription")}</small>
          </div>
        </div>
        <MetadataList className="settings-about-metadata" columns="single" label={{ position: "start", width: 128 }}>
          <MetadataListItem label={t("applicationVersion")} icon={<Info size={15} strokeWidth={2} aria-hidden="true" />}>{session.version}</MetadataListItem>
          <MetadataListItem label={t("softwareLicense")} icon={<Scale size={15} strokeWidth={2} aria-hidden="true" />}>MIT</MetadataListItem>
          <MetadataListItem label={t("sourceRepository")} icon={<GitFork size={15} strokeWidth={2} aria-hidden="true" />}>
            <Link href={repositoryUrl} isExternalLink newTabLabel={t("opensInNewTab")} maxLines={1}>{repositoryUrl}</Link>
          </MetadataListItem>
        </MetadataList>
      </section>
    </div>
  )
}
