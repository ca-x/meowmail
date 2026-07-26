export function readStoredValue(key: string): string | null {
  try {
    return window.localStorage?.getItem(key) ?? null
  } catch {
    return null
  }
}

export function writeStoredValue(key: string, value: string): void {
  try {
    window.localStorage?.setItem(key, value)
  } catch {
    // Appearance and navigation preferences remain usable without persistence.
  }
}

export function removeStoredValue(key: string): void {
  try {
    window.localStorage?.removeItem(key)
  } catch {
    // Appearance and navigation preferences remain usable without persistence.
  }
}
