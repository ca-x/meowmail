import { ApiError, api } from "../../app/api"
import type { SessionResponse } from "../../app/types"

const AUTH_STATE_KEY = "meowmail-auth-state-change"
const ACTIVITY_KEY_PREFIX = "meowmail-session-activity"

export function publishAuthStateChange() {
  try {
    window.localStorage.setItem(AUTH_STATE_KEY, `${Date.now()}:${Math.random()}`)
  } catch {
    // Cross-tab coordination is a progressive enhancement.
  }
}

export function subscribeAuthStateChanges(listener: () => void) {
  const handleStorage = (event: StorageEvent) => {
    if (event.key === AUTH_STATE_KEY) listener()
  }
  window.addEventListener("storage", handleStorage)
  return () => window.removeEventListener("storage", handleStorage)
}

export function startSessionAutoLock(
  session: SessionResponse,
  onLocked: (session: SessionResponse) => void,
) {
  const minutes = session.user.autoLockMinutes
  if (!session.user.hasPin || minutes === null) return () => undefined

  const timeout = minutes * 60_000
  const activityKey = `${ACTIVITY_KEY_PREFIX}:${session.user.id}`
  let timer = 0
  let locking = false
  let lastWheelActivity = 0
  let lastActivity = Date.now()

  const readLastActivity = () => {
    try {
      const value = Number(window.localStorage.getItem(activityKey))
      if (Number.isFinite(value) && value > 0) lastActivity = Math.max(lastActivity, value)
    } catch {
      // Fall back to the in-memory timestamp for this tab.
    }
    return lastActivity
  }

  const schedule = () => {
    window.clearTimeout(timer)
    const remaining = Math.max(0, readLastActivity() + timeout - Date.now())
    timer = window.setTimeout(() => void checkIdle(), remaining)
  }

  const recordActivity = () => {
    lastActivity = Date.now()
    try {
      window.localStorage.setItem(activityKey, String(lastActivity))
    } catch {
      // The current tab still receives a fresh timer when storage is unavailable.
    }
    schedule()
  }

  const checkIdle = async () => {
    if (Date.now() - readLastActivity() < timeout) {
      schedule()
      return
    }
    if (locking) return
    locking = true
    try {
      onLocked(await api.lock())
    } catch (cause) {
      if (cause instanceof ApiError && cause.status === 423) {
        const current = await api.session().catch(() => null)
        if (current?.locked) onLocked(current)
      }
    } finally {
      locking = false
    }
  }

  const handleWheel = () => {
    const now = Date.now()
    if (now - lastWheelActivity < 1_000) return
    lastWheelActivity = now
    recordActivity()
  }
  const handleVisibility = () => {
    if (document.visibilityState !== "visible") return
    if (Date.now() - readLastActivity() >= timeout) void checkIdle()
    else recordActivity()
  }
  const handleStorage = (event: StorageEvent) => {
    if (event.key !== activityKey) return
    const value = Number(event.newValue)
    if (Number.isFinite(value) && value > 0) lastActivity = Math.max(lastActivity, value)
    schedule()
  }

  recordActivity()
  window.addEventListener("pointerdown", recordActivity, { passive: true })
  window.addEventListener("keydown", recordActivity)
  window.addEventListener("wheel", handleWheel, { passive: true })
  window.addEventListener("focus", handleVisibility)
  window.addEventListener("storage", handleStorage)
  document.addEventListener("visibilitychange", handleVisibility)

  return () => {
    window.clearTimeout(timer)
    window.removeEventListener("pointerdown", recordActivity)
    window.removeEventListener("keydown", recordActivity)
    window.removeEventListener("wheel", handleWheel)
    window.removeEventListener("focus", handleVisibility)
    window.removeEventListener("storage", handleStorage)
    document.removeEventListener("visibilitychange", handleVisibility)
  }
}
