import { useEffect, useState } from "react"

import { LoginPage } from "../features/auth/LoginPage"
import { LockScreen } from "../features/auth/LockScreen"
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

  useEffect(() => {
    const update = () => setPathname(window.location.pathname)
    window.addEventListener("popstate", update)
    return () => window.removeEventListener("popstate", update)
  }, [])

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
      : (auth.status === "ready" || auth.status === "locked") && !pathname.startsWith("/mail")
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
        <span className="spinner" aria-hidden="true" />
        <p>{t("loading")}</p>
      </main>
    )
  }

  if (auth.status === "guest") {
    return (
      <LoginPage
        config={auth.config}
        onAuthenticated={(session) => {
          setAuth({ status: "ready", session })
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
        onUnlocked={(session) => setAuth({ status: "ready", session })}
        onLoggedOut={async () => {
          await api.logout().catch(() => undefined)
          const config = await api.authConfig().catch(() => ({ localEnabled: true, oidcEnabled: false }))
          setAuth({ status: "guest", config })
        }}
      />
    )
  }

  return (
    <MailWorkspace
      session={auth.session}
      onSessionChanged={(session) => setAuth({ status: "ready", session })}
      onLocked={(session) => setAuth({ status: "locked", session })}
      onLoggedOut={async () => {
        const config = await api.authConfig().catch(() => ({ localEnabled: true, oidcEnabled: false }))
        setAuth({ status: "guest", config })
      }}
    />
  )
}
