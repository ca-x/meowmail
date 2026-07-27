import { readFileSync } from "node:fs"

import { expect, test } from "vitest"

import { messages } from "../i18n/messages"

const bootstrap = readFileSync("public/theme-bootstrap.js", "utf8")
const mainSource = readFileSync("src/main.tsx", "utf8")
const providersSource = readFileSync("src/app/Providers.tsx", "utf8")

function runBootstrap({ language, storedLocale = null, storedTheme = null, storageDenied = false }) {
  const description = { content: "", setAttribute(_name, value) { this.content = value } }
  const documentStub = {
    documentElement: { dataset: {}, lang: "zh-CN" },
    title: "妙邮",
    querySelector: () => description,
  }
  const storage = {
    getItem(key) {
      if (storageDenied) throw new DOMException("Storage disabled", "SecurityError")
      if (key === "meowmail-locale") return storedLocale
      if (key === "meowmail-astryx-theme") return storedTheme
      return null
    },
  }

  new Function("document", "navigator", "localStorage", "matchMedia", bootstrap)(
    documentStub,
    { language },
    storage,
    () => ({ matches: false }),
  )

  return { documentStub, description }
}

test("locale bootstrap falls back to the browser language when storage is denied", () => {
  const { documentStub, description } = runBootstrap({ language: "en-US", storageDenied: true })

  expect(documentStub.documentElement.lang).toBe("en")
  expect(documentStub.title).toBe(messages.en.brandName)
  expect(description.content).toBe(messages.en.metaDescription)
})

test("locale bootstrap metadata stays aligned with the Chinese dictionary", () => {
  const { documentStub, description } = runBootstrap({ language: "en-US", storedLocale: "zh-CN" })

  expect(documentStub.documentElement.lang).toBe("zh-CN")
  expect(documentStub.title).toBe(messages["zh-CN"].brandName)
  expect(description.content).toBe(messages["zh-CN"].metaDescription)
})

test("application root mounts the Astryx theme and layer providers", () => {
  expect(mainSource).toContain('@astryxdesign/core/reset.css')
  expect(mainSource).toContain('@astryxdesign/core/astryx.css')
  for (const theme of ["neutral", "stone", "butter", "matcha", "chocolate", "gothic", "y2k"]) {
    expect(mainSource).toContain(`@astryxdesign/theme-${theme}/theme.css`)
  }
  expect(providersSource).toContain("astryxThemes[themeName]")
  expect(providersSource).toContain('<LayerProvider')
})

test("theme bootstrap restores a supported Astryx theme and rejects unknown values", () => {
  expect(runBootstrap({ language: "zh-CN", storedTheme: "gothic" }).documentStub.documentElement.dataset.astryxTheme).toBe("gothic")
  expect(runBootstrap({ language: "zh-CN", storedTheme: "unknown" }).documentStub.documentElement.dataset.astryxTheme).toBe("neutral")
})
