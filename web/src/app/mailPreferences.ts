import type { MailPreferences } from "./types"

export const defaultMailPreferences: MailPreferences = {
  readingMode: "preview",
  listDensity: "default",
  conversationMode: false,
  aggregatePromotions: true,
  showSummary: true,
  showMessageSize: false,
  showAttachmentPreview: true,
  afterAction: "nextMessage",
  plainTextReading: false,
  attachOriginalOnReply: true,
  subjectPrefixLanguage: "english",
  emptySubjectFromBody: false,
  composeFontFamily: "default",
  composeFontSize: 14,
  composeFontColor: "#1A1A1A",
  autoForwardEnabled: false,
  autoForwardAddress: null,
  autoReplyEnabled: false,
  autoReplyText: "",
}
