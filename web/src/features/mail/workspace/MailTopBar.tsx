import { Avatar } from "@astryxdesign/core/Avatar"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Kbd } from "@astryxdesign/core/Kbd"
import { TextInput } from "@astryxdesign/core/TextInput"
import { TopNav, TopNavHeading } from "@astryxdesign/core/TopNav"
import { Bell, Languages, Moon, Search, Sun } from "lucide-react"
import type { RefObject } from "react"

import type { SessionResponse } from "../../../app/types"
import { useI18n } from "../../../i18n/I18nProvider"
import { useTheme } from "../../../theme/ThemeProvider"

export function MailTopBar({ session, search, searchRef, onSearchChange, onOpenSettings }: {
  session: SessionResponse
  search: string
  searchRef: RefObject<HTMLInputElement | null>
  onSearchChange: (value: string) => void
  onOpenSettings: () => void
}) {
  const { locale, setLocale, t } = useI18n()
  const { resolved, setMode } = useTheme()
  const user = session.user

  return (
    <TopNav
      className="mail-topbar"
      label={t("mailNavigation")}
      heading={
        <TopNavHeading
          logo={<img className="mail-topbar-logo" src="/meowmail-logo.png" alt="" />}
          heading={t("brandName")}
        />
      }
      centerContent={
        <div className="mail-search-field">
          <TextInput
            ref={searchRef}
            label={t("search")}
            isLabelHidden
            startIcon={<Search aria-hidden="true" />}
            value={search}
            onChange={onSearchChange}
            placeholder={t("search")}
            hasClear
            width="100%"
          />
          <Kbd keys="mod+k" />
        </div>
      }
      endContent={
        <div className="mail-topbar-actions">
          <IconButton
            label={locale === "zh-CN" ? t("switchToEnglish") : t("switchToChinese")}
            icon={<Languages aria-hidden="true" />}
            variant="ghost"
            size="sm"
            onClick={() => setLocale(locale === "zh-CN" ? "en" : "zh-CN")}
          />
          <IconButton
            label={resolved === "dark" ? t("switchToLight") : t("switchToDark")}
            icon={resolved === "dark" ? <Sun aria-hidden="true" /> : <Moon aria-hidden="true" />}
            variant="ghost"
            size="sm"
            onClick={() => setMode(resolved === "dark" ? "light" : "dark")}
          />
          <IconButton
            label={t("notifications")}
            icon={<Bell aria-hidden="true" />}
            variant="ghost"
            size="sm"
            onClick={onOpenSettings}
          />
          <IconButton
            className="profile-menu-button"
            label={t("profileAndSettings")}
            icon={
              <Avatar
                size="sm"
                name={user.nickname}
                src={user.hasAvatar ? `/api/v1/users/me/avatar?v=${user.updatedAt}` : undefined}
              />
            }
            variant="ghost"
            size="sm"
            onClick={onOpenSettings}
          />
        </div>
      }
    />
  )
}
