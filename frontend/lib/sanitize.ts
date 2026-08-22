import DOMPurify from "dompurify"

export function sanitizeHtml(dirty: string): string {
  return DOMPurify.sanitize(dirty, {
    FORBID_TAGS: ["style", "form", "input", "button"],
    FORBID_ATTR: ["srcset"],
  })
}
