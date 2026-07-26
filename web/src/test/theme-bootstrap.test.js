import { readFileSync } from "node:fs"

import { expect, test } from "vitest"

import { messages } from "../i18n/messages"

const bootstrap = readFileSync("public/theme-bootstrap.js", "utf8")

function runBootstrap({ language, storedLocale = null, storageDenied = false }) {
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
