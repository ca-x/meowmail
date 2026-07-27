import type { ReactNode } from "react"

export function SettingsPanelHeading({ icon, title, description }: {
  icon: ReactNode
  title: string
  description: string
}) {
  return (
    <header className="settings-panel-heading">
      <span className="settings-panel-icon" aria-hidden="true">{icon}</span>
      <div>
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
    </header>
  )
}

