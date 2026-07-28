import { Avatar } from "@astryxdesign/core/Avatar"
import { Badge } from "@astryxdesign/core/Badge"
import { Button } from "@astryxdesign/core/Button"
import { FileInput } from "@astryxdesign/core/FileInput"
import { IconButton } from "@astryxdesign/core/IconButton"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { TextInput } from "@astryxdesign/core/TextInput"
import { Languages, Mail, Moon, Pencil, Save, Sun, UserRound, X } from "lucide-react"
import { useEffect, useState, type FormEvent } from "react"

import { ApiError, api } from "../../app/api"
import type { MailAccount, PublicUser, SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { AstryxThemeName } from "../../theme/astryxThemes"
import { useTheme, type ThemeMode } from "../../theme/ThemeProvider"
import { SettingsPanelHeading } from "./SettingsPanelHeading"
import type { SettingsNotice } from "./settingsTypes"
import { ThemePicker } from "./ThemePicker"

export function GeneralSettingsPanel({ session, accounts, onSessionChanged, onOpenAccounts, onNotice }: {
  session: SessionResponse
  accounts: MailAccount[]
  onSessionChanged: (session: SessionResponse) => void
  onOpenAccounts: () => void
  onNotice: (notice: SettingsNotice) => void
}) {
  const { locale, setLocale, t } = useI18n()
  const { mode, setMode, themeName, setThemeName } = useTheme()
  const [user, setUser] = useState(session.user)
  const [username, setUsername] = useState(session.user.username)
  const [nickname, setNickname] = useState(session.user.nickname)
  const [isEditingProfile, setIsEditingProfile] = useState(false)
  const [busy, setBusy] = useState<"profile" | "avatar" | null>(null)
  const themeLabels: Record<AstryxThemeName, string> = {
    neutral: t("themeNeutral"),
    stone: t("themeStone"),
    butter: t("themeButter"),
    matcha: t("themeMatcha"),
    chocolate: t("themeChocolate"),
    gothic: t("themeGothic"),
    y2k: t("themeY2k"),
  }

  useEffect(() => {
    setUser(session.user)
    setUsername(session.user.username)
    setNickname(session.user.nickname)
    setIsEditingProfile(false)
  }, [session.user])
  const themeDescriptions: Record<AstryxThemeName, string> = {
    neutral: t("themeNeutralDescription"),
    stone: t("themeStoneDescription"),
    butter: t("themeButterDescription"),
    matcha: t("themeMatchaDescription"),
    chocolate: t("themeChocolateDescription"),
    gothic: t("themeGothicDescription"),
    y2k: t("themeY2kDescription"),
  }

  function publishUser(next: PublicUser) {
    setUser(next)
    setUsername(next.username)
    setNickname(next.nickname)
    onSessionChanged({ ...session, user: next })
  }

  async function saveProfile(event: FormEvent) {
    event.preventDefault()
    setBusy("profile")
    try {
      publishUser(await api.updateProfile(username === user.username ? null : username, nickname))
      setIsEditingProfile(false)
      onNotice({ key: "profileSaved" })
    } catch (error) {
      const key = error instanceof ApiError && error.status === 409
        ? "usernameUnavailable"
        : error instanceof ApiError && error.status === 422
          ? "profileInvalid"
          : "genericError"
      onNotice({ key, error: true })
    } finally {
      setBusy(null)
    }
  }

  async function updateAvatar(file: File | File[] | null) {
    if (!(file instanceof File)) return
    setBusy("avatar")
    try {
      publishUser(await api.updateAvatar(file))
      onNotice({ key: "avatarSaved" })
    } catch {
      onNotice({ key: "avatarInvalid", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function removeAvatar() {
    setBusy("avatar")
    try {
      publishUser(await api.removeAvatar())
      onNotice({ key: "avatarRemoved" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  function cancelProfileEdit() {
    setUsername(user.username)
    setNickname(user.nickname)
    setIsEditingProfile(false)
  }

  return (
    <div className="settings-panel-stack">
      <SettingsPanelHeading icon={<UserRound />} title={t("profile")} description={t("profileDescription")} />
      <section className="settings-profile-block" aria-label={t("profile")}>
        <div className="settings-profile-view">
          <div className="settings-profile-summary">
            <Avatar
              size={64}
              name={user.nickname}
              src={user.hasAvatar ? `/api/v1/users/me/avatar?v=${user.updatedAt}` : undefined}
            />
            <div>
              <strong>{user.nickname}</strong>
              <span>@{user.username}</span>
              <Badge variant={user.role === "admin" ? "blue" : "neutral"} label={user.role === "admin" ? t("administrator") : t("standardUser")} />
            </div>
          </div>
          <IconButton
            label={t("edit")}
            icon={<Pencil aria-hidden="true" />}
            variant="ghost"
            onClick={() => setIsEditingProfile(true)}
          />
        </div>
        {isEditingProfile && (
          <form className="settings-profile-form" onSubmit={saveProfile}>
            <div className="settings-avatar-actions">
              <FileInput
                label={t("changeAvatar")}
                value={null}
                onChange={(file) => void updateAvatar(file)}
                accept="image/png,image/jpeg,image/webp"
                maxSize={512 * 1024}
                isLoading={busy === "avatar"}
                isDisabled={busy === "avatar"}
                placeholder={t("changeAvatar")}
                width="100%"
              />
              {user.hasAvatar && <Button label={t("remove")} variant="ghost" isDisabled={busy === "avatar"} onClick={() => void removeAvatar()} />}
            </div>
            <div className="settings-profile-fields">
              <TextInput
                label={t("profileUsername")}
                value={username}
                onChange={setUsername}
                placeholder={t("profileUsernamePlaceholder")}
                labelTooltip={t("usernameRequirements")}
                width="100%"
              />
              <TextInput
                label={t("nickname")}
                value={nickname}
                onChange={setNickname}
                placeholder={t("nicknamePlaceholder")}
                width="100%"
              />
            </div>
            <div className="settings-profile-actions">
              <Button label={t("cancel")} icon={<X aria-hidden="true" />} variant="ghost" isDisabled={busy === "profile"} onClick={cancelProfileEdit} />
              <Button
                label={t("save")}
                icon={<Save aria-hidden="true" />}
                type="submit"
                variant="secondary"
                isLoading={busy === "profile"}
                isDisabled={username.trim().length < 2 || !nickname.trim() || busy === "profile"}
              />
            </div>
          </form>
        )}
      </section>

      <div className="settings-subsection-divider" />
      <SettingsPanelHeading icon={<Languages />} title={t("appearance")} description={`${t("language")} · ${t("themeStyle")} · ${t("themeMode")}`} />
      <section className="settings-choice-list" aria-label={t("appearance")}>
        <div className="settings-choice-row">
          <div><strong>{t("language")}</strong></div>
          <SegmentedControl value={locale} onChange={(value) => setLocale(value as "zh-CN" | "en")} label={t("language")} size="sm">
            <SegmentedControlItem value="zh-CN" label="中文" />
            <SegmentedControlItem value="en" label="English" />
          </SegmentedControl>
        </div>
        <div className="settings-choice-row settings-theme-choice-row">
          <div><strong>{t("themeStyle")}</strong><small>{themeDescriptions[themeName]}</small></div>
          <ThemePicker labels={themeLabels} />
        </div>
        <div className="settings-choice-row">
          <div><strong>{t("themeMode")}</strong></div>
          <SegmentedControl value={mode} onChange={(value) => setMode(value as ThemeMode)} label={t("themeMode")} size="sm">
            <SegmentedControlItem value="system" label={t("themeSystem")} />
            <SegmentedControlItem value="light" label={t("themeLight")} icon={<Sun aria-hidden="true" />} />
            <SegmentedControlItem value="dark" label={t("themeDark")} icon={<Moon aria-hidden="true" />} />
          </SegmentedControl>
        </div>
        <div className="settings-choice-row account-entry">
          <div><strong>{t("accounts")}</strong><small>{accounts.length ? t("accountsConfigured", { count: accounts.length }) : t("noAccountsDescription")}</small></div>
          <Button label={t("manageAccounts")} icon={<Mail aria-hidden="true" />} variant="secondary" onClick={onOpenAccounts} />
        </div>
      </section>
    </div>
  )
}
