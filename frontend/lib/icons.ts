import { convertFileSrc } from "@tauri-apps/api/core"

export function instanceIconSrc(iconPath?: string | null): string | null {
  return iconPath ? convertFileSrc(iconPath) : null
}
