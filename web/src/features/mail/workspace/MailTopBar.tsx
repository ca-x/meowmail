import { Avatar } from "@astryxdesign/core/Avatar"
import { DropdownMenu, type DropdownMenuOption } from "@astryxdesign/core/DropdownMenu"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Kbd } from "@astryxdesign/core/Kbd"
import { TextInput } from "@astryxdesign/core/TextInput"
import { TopNav, TopNavHeading } from "@astryxdesign/core/TopNav"
import { CalendarDays, Languages, LockKeyhole, LogOut, Moon, Search, Settings, Sun } from "lucide-react"
import type { RefObject } from "react"

import type { SessionResponse } from "../../../app/types"
import { useI18n } from "../../../i18n/I18nProvider"
import { useTheme } from "../../../theme/ThemeProvider"

export function MailTopBar({ session, activeView, search, searchRef, onSearchChange, onOpenSettings, onLock, onLogout }: {
  session: SessionResponse
  activeView: "mail" | "calendar"
  search: string
  searchRef: RefObject<HTMLInputElement | null>
  onSearchChange: (value: string) => void
  onOpenSettings: () => void
  onLock: () => void
  onLogout: () => void
}) {
  const { locale, setLocale, t } = useI18n()
  const { resolved, setMode } = useTheme()
  const user = session.user
  const accountMenuItems: DropdownMenuOption[] = [
    { label: t("profileAndSettings"), icon: <Settings aria-hidden="true" />, onClick: onOpenSettings },
    { label: t("lockCurrentSession"), icon: <LockKeyhole aria-hidden="true" />, onClick: onLock },
    { type: "divider" },
    { label: t("logout"), icon: <LogOut aria-hidden="true" />, onClick: onLogout },
  ]

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
      centerContent={activeView === "mail" ? (
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
      ) : <div className="mail-topbar-current-view"><CalendarDays aria-hidden="true" /><span>{t("calendarView")}</span></div>}
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
          <DropdownMenu
            hasChevron={false}
            placement="below"
            menuWidth={220}
            button={{
              className: "profile-menu-button",
              label: t("profileAndSettings"),
              icon: <Avatar size="sm" name={user.nickname} src={user.hasAvatar ? `/api/v1/users/me/avatar?v=${user.updatedAt}` : undefined} />,
              isIconOnly: true,
              variant: "ghost",
              size: "sm",
            }}
            items={accountMenuItems}
          />
        </div>
      }
    />
  )
}
