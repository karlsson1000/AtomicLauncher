import { untrack } from "svelte"

export type ToastType = "success" | "error"

export interface Toast {
  id: number
  type: ToastType
  message: string
}

export const toastStore = $state<{ toasts: Toast[] }>({ toasts: [] })

let nextId = 1

export function showToast(type: ToastType, message: string, duration = 15000) {
  untrack(() => {
    const id = nextId++
    toastStore.toasts.push({ id, type, message })
    if (toastStore.toasts.length > 5) toastStore.toasts.shift()
    setTimeout(() => dismissToast(id), duration)
  })
}

export function dismissToast(id: number) {
  untrack(() => {
    const index = toastStore.toasts.findIndex(t => t.id === id)
    if (index !== -1) toastStore.toasts.splice(index, 1)
  })
}
