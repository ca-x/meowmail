import { Spinner } from "@astryxdesign/core/Spinner"
import { useEffect, useState } from "react"

import { LoginPage } from "../features/auth/LoginPage"
import { LockScreen } from "../features/auth/LockScreen"
import { publishAuthStateChange, startSessionAutoLock, subscribeAuthStateChanges } from "../features/auth/sessionActivity"
import { MailWorkspace } from "../features/mail/MailWorkspace"
import { useI18n } from "../i18n/I18nProvider"
import type { AuthConfig, SessionResponse } from "./types"
import { ApiError, api } from "./api"

type AuthState =
  | { status: "loading" }
  | { status: "guest"; config: AuthConfig }
  | { status: "locked"; session: SessionResponse }
  | { status: "ready"; session: SessionResponse }

export function App() {
  const [auth, setAuth] = useState<AuthState>({ status: "loading" })
  const [pathname, setPathname] = useState(() => window.location.pathname)
  const { t } = useI18n()

  async function loadAuthState() {
    const config = await api.authConfig().catch(() => ({ localEnabled: true, oidcEnabled: false }))
    try {
      const session = await api.session()
      setAuth(session.locked ? { status: "locked", session } : { status: "ready", session })
    } catch {
      setAuth({ status: "guest", config })
    }
  }

  function showReady(session: SessionResponse) {
    setAuth({ status: "ready", session })
    publishAuthStateChange()
  }

  function showLocked(session: SessionResponse) {
    setAuth({ status: "locked", session })
    publishAuthStateChange()
  }

  async function showLoggedOut() {
    const config = await api.authConfig().catch(() => ({ localEnabled: true, oidcEnabled: false }))
    setAuth({ status: "guest", config })
    publishAuthStateChange()
  }

  useEffect(() => {
    const update = () => setPathname(window.location.pathname)
    window.addEventListener("popstate", update)
    return () => window.removeEventListener("popstate", update)
  }, [])

  useEffect(() => subscribeAuthStateChanges(() => void loadAuthState()), [])

  useEffect(() => {
    if (auth.status !== "ready") return
    return startSessionAutoLock(auth.session, showLocked)
  }, [auth])

  useEffect(() => {
    let active = true
    void (async () => {
      const config = await api.authConfig().catch(() => ({ localEnabled: true, oidcEnabled: false }))
      try {
        const session = await api.session()
        if (active) setAuth(session.locked ? { status: "locked", session } : { status: "ready", session })
      } catch (error) {
        if (active) {
          setAuth(error instanceof ApiError && error.status === 401
            ? { status: "guest", config }
            : { status: "guest", config })
        }
      }
    })()
    return () => { active = false }
  }, [])

  useEffect(() => {
    const target = auth.status === "guest"
      ? "/login"
      : (auth.status === "ready" || auth.status === "locked")
          && !pathname.startsWith("/mail")
          && !pathname.startsWith("/calendar")
        ? "/mail/inbox"
        : null
    if (target && pathname !== target) {
      window.history.replaceState(null, "", target)
      setPathname(target)
    }
  }, [auth.status, pathname])

  if (auth.status === "loading") {
    return (
      <main className="boot-screen" aria-live="polite">
        <img src="/meowmail-logo.png" alt="" />
        <Spinner size="lg" label={t("loading")} />
      </main>
    )
  }

  if (auth.status === "guest") {
    return (
      <LoginPage
        config={auth.config}
        onAuthenticated={(session) => {
          showReady(session)
          window.history.replaceState(null, "", "/mail/inbox")
          setPathname("/mail/inbox")
        }}
      />
    )
  }

  if (auth.status === "locked") {
    return (
      <LockScreen
        session={auth.session}
        onUnlocked={showReady}
        onLoggedOut={async () => {
          await api.logout()
          await showLoggedOut()
        }}
      />
    )
  }

  return (
    <MailWorkspace
      session={auth.session}
      onSessionChanged={showReady}
      onLocked={showLocked}
      onLoggedOut={showLoggedOut}
    />
  )
}
