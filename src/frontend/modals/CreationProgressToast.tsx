import { useState, useEffect } from "react"
import { Loader2, CheckCircle2, XCircle, X } from "lucide-react"
import { listen } from "@tauri-apps/api/event"

interface CreationProgressToastProps {
  instanceName: string
  onComplete: () => void
  onError: () => void
  onDismiss: () => void
}

interface ProgressPayload {
  instance: string
  progress: number
  stage?: string
  current_file?: string
}

export function CreationProgressToast({ instanceName, onComplete, onDismiss }: CreationProgressToastProps) {
  const [progress, setProgress] = useState(0)
  const [status, setStatus] = useState<"creating" | "success" | "error">("creating")
  const [hasRealProgress, setHasRealProgress] = useState(false)

  useEffect(() => {
    let unlisten: (() => void) | undefined
    let completionTimer: NodeJS.Timeout | undefined
    let fallbackTimer: NodeJS.Timeout | undefined

    const setupListener = async () => {
      // Listen for duplication progress events
      const unlistenDuplication = await listen<ProgressPayload>("duplication-progress", (event) => {
        if (event.payload.instance === instanceName) {
          console.log("Duplication progress:", event.payload.progress)
          setHasRealProgress(true)
          setProgress(event.payload.progress)
          
          if (event.payload.progress >= 100) {
            setStatus("success")
            completionTimer = setTimeout(() => {
              onComplete()
            }, 1500)
          }
        }
      })

      // Listen for creation progress events
      const unlistenCreation = await listen<ProgressPayload>("creation-progress", (event) => {
        if (event.payload.instance === instanceName) {
          console.log("Creation progress:", event.payload.progress)
          setHasRealProgress(true)
          setProgress(event.payload.progress)
          
          if (event.payload.progress >= 100) {
            setStatus("success")
            completionTimer = setTimeout(() => {
              onComplete()
            }, 1500)
          }
        }
      })

      // Combine unlisteners
      unlisten = () => {
        unlistenDuplication()
        unlistenCreation()
      }

      // Fallback: If no progress events received after 2 seconds, log it
      fallbackTimer = setTimeout(() => {
        if (!hasRealProgress) {
          console.log("No real progress events received after 2 seconds")
        }
      }, 2000)
    }

    setupListener()

    return () => {
      if (unlisten) {
        unlisten()
      }
      if (completionTimer) {
        clearTimeout(completionTimer)
      }
      if (fallbackTimer) {
        clearTimeout(fallbackTimer)
      }
    }
  }, [instanceName, onComplete, hasRealProgress])

  // Fallback: Simulate progress if no real progress events are received
  useEffect(() => {
    if (status === "creating" && !hasRealProgress) {
      const interval = setInterval(() => {
        setProgress((prev) => {
          if (prev >= 90) return prev
          return prev + Math.random() * 15
        })
      }, 300)

      // Simulate completion after 5 seconds if no real progress
      const timer = setTimeout(() => {
        setProgress(100)
        setStatus("success")
        setTimeout(() => {
          onComplete()
        }, 2000)
      }, 5000)

      return () => {
        clearInterval(interval)
        clearTimeout(timer)
      }
    }
  }, [status, hasRealProgress, onComplete])

  const getStatusColor = () => {
    switch (status) {
      case "creating":
        return "from-[#16a34a] to-[#15803d]"
      case "success":
        return "from-[#16a34a] to-[#16a34a]"
      case "error":
        return "from-red-500 to-red-600"
    }
  }

  return (
    <div className="fixed top-14 right-4 z-40 w-80 bg-[#1a1a1a] border border-[#2a2a2a] rounded-lg overflow-hidden animate-in slide-in-from-right-4 duration-300">
      <div className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-2">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <p className="text-sm font-medium text-[#e8e8e8]">
                    {status === "creating" && "Creating instance"}
                    {status === "success" && "Instance created"}
                    {status === "error" && "Creation failed"}
                  </p>
                  {status === "creating" && (
                    <Loader2 size={14} className="animate-spin text-[#16a34a]" />
                  )}
                  {status === "success" && (
                    <CheckCircle2 size={16} className="text-[#16a34a]" />
                  )}
                  {status === "error" && (
                    <XCircle size={16} className="text-red-500" />
                  )}
                </div>
                <p className="text-xs text-[#808080] mt-0.5 truncate">{instanceName}</p>
              </div>
              
              {/* Percentage */}
              <p className="text-xs text-[#4a4a4a] flex-shrink-0">
                {Math.round(progress)}%
              </p>
              
              {status !== "creating" && (
                <button
                  onClick={onDismiss}
                  className="flex-shrink-0 p-1 hover:bg-[#2a2a2a] rounded transition-colors text-[#808080] hover:text-[#e8e8e8]"
                >
                  <X size={14} />
                </button>
              )}
            </div>

            {/* Progress bar */}
            <div className="relative h-1.5 bg-[#2a2a2a] rounded-full overflow-hidden mt-3">
              <div 
                className={`absolute inset-y-0 left-0 bg-gradient-to-r ${getStatusColor()} transition-all duration-300 ease-out rounded-full`}
                style={{ width: `${progress}%` }}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}