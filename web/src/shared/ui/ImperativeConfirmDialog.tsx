import { useImperativeAlertDialog } from "@astryxdesign/core/AlertDialog"
import { useCallback, useEffect, useRef } from "react"

export function useImperativeConfirmDialog() {
  const alert = useImperativeAlertDialog()
  const pendingRef = useRef<{
    opened: boolean
    resolve: (value: boolean) => void
  } | null>(null)

  useEffect(() => {
    const pending = pendingRef.current
    if (!pending) return
    if (alert.isOpen) {
      pending.opened = true
    } else if (pending.opened) {
      pendingRef.current = null
      pending.resolve(false)
    }
  }, [alert.isOpen])

  const confirm = useCallback((options: {
    title: string
    description: string
    cancelLabel: string
    actionLabel: string
    actionVariant?: "primary" | "secondary" | "ghost" | "destructive"
  }) => new Promise<boolean>((resolve) => {
    const close = (value: boolean) => {
      const pending = pendingRef.current
      if (!pending) return
      pendingRef.current = null
      alert.hide()
      pending.resolve(value)
    }
    pendingRef.current = { opened: false, resolve }
    alert.show({
      title: options.title,
      description: options.description,
      cancelLabel: options.cancelLabel,
      actionLabel: options.actionLabel,
      actionVariant: options.actionVariant || "destructive",
      onAction: () => close(true),
    })
  }), [alert])

  return { confirm, element: alert.element, isOpen: alert.isOpen }
}
