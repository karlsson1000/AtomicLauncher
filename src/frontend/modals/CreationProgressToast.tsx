import { useState, useEffect, useRef } from "react"
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

export function CreationProgressToast({ 
  instanceName,
  onError,
  onDismiss 
}: CreationProgressToastProps) {
  const [progress, setProgress] = useState(0)
  const [stage, setStage] = useState<string>("")
  const [status, setStatus] = useState<"creating" | "success" | "error">("creating")
  const [hasReceivedProgress, setHasReceivedProgress] = useState(false)
  
  // Use refs to prevent issues with closures and timers
  const isCompletingRef = useRef<boolean>(false)
  const completionTimerRef = useRef<NodeJS.Timeout | undefined>(undefined)
  const errorTimerRef = useRef<NodeJS.Timeout | undefined>(undefined)
  const unlistenFunctionsRef = useRef<Array<() => void>>([])
  const onDismissRef = useRef(onDismiss)

  useEffect(() => {
    onDismissRef.current = onDismiss
  }, [onDismiss])

  // Auto-dismiss
  useEffect(() => {
    if (status === "success" || status === "error") {
      console.log(`[ProgressToast] Setting up auto-dismiss timer for status: ${status}`)
      
      const timer = setTimeout(() => {
        console.log(`[ProgressToast] Auto-dismissing after 5 seconds`)
        onDismissRef.current()
      }, 5000)

      return () => {
        console.log(`[ProgressToast] Clearing auto-dismiss timer`)
        clearTimeout(timer)
      }
    }
  }, [status])

  useEffect(() => {
    const setupListeners = async () => {
      console.log(`[ProgressToast] Setting up listeners for: ${instanceName}`)
      
      try {
        // Listen for duplication progress events
        const unlistenDuplication = await listen<ProgressPayload>("duplication-progress", (event) => {
          if (event.payload.instance === instanceName) {
            console.log(`[ProgressToast] Duplication progress: ${event.payload.progress}%`, event.payload.stage)
            
            setHasReceivedProgress(true)
            setProgress(event.payload.progress)
            
            if (event.payload.stage) {
              setStage(event.payload.stage)
            }
            
            if (event.payload.progress >= 100 && !isCompletingRef.current) {
              isCompletingRef.current = true
              console.log(`[ProgressToast] Duplication complete!`)
              setStatus("success")
            }
          }
        })

        // Listen for creation progress events
        const unlistenCreation = await listen<ProgressPayload>("creation-progress", (event) => {
          if (event.payload.instance === instanceName) {
            console.log(`[ProgressToast] Creation progress: ${event.payload.progress}%`, event.payload.stage)
            
            setHasReceivedProgress(true)
            setProgress(event.payload.progress)
            
            if (event.payload.stage) {
              setStage(event.payload.stage)
            }
            
            if (event.payload.progress >= 100 && !isCompletingRef.current) {
              isCompletingRef.current = true
              console.log(`[ProgressToast] Creation complete!`)
              setStatus("success")
            }
          }
        })

        // Listen for modpack installation progress events
        const unlistenModpack = await listen<ProgressPayload>("modpack-install-progress", (event) => {
          if (event.payload.instance === instanceName) {
            console.log(`[ProgressToast] Modpack install progress: ${event.payload.progress}%`, event.payload.stage)
            
            setHasReceivedProgress(true)
            setProgress(event.payload.progress)
            
            if (event.payload.stage) {
              setStage(event.payload.stage)
            }
            
            if (event.payload.progress >= 100 && !isCompletingRef.current) {
              isCompletingRef.current = true
              console.log(`[ProgressToast] Modpack install complete!`)
              setStatus("success")
            }
          }
        })

        // Store unlisten functions
        unlistenFunctionsRef.current = [
          unlistenDuplication,
          unlistenCreation,
          unlistenModpack
        ]

        console.log(`[ProgressToast] All listeners set up successfully`)

        // Set up error timeout - if no progress after 15 seconds, show error
        errorTimerRef.current = setTimeout(() => {
          if (!isCompletingRef.current) {
            console.error(`[ProgressToast] No progress events received after 15 seconds for ${instanceName}`)
            setStatus("error")
            setStage("Operation timed out - no progress received")
            onError()
          }
        }, 15000)

      } catch (error) {
        console.error(`[ProgressToast] Failed to set up listeners:`, error)
        setStatus("error")
        setStage("Failed to initialize listeners")
        onError()
      }
    }

    setupListeners()

    // Cleanup function
    return () => {
      console.log(`[ProgressToast] Cleaning up listeners for ${instanceName}`)
      
      // Clear all timers
      if (completionTimerRef.current) {
        clearTimeout(completionTimerRef.current)
      }
      if (errorTimerRef.current) {
        clearTimeout(errorTimerRef.current)
      }
      
      // Unlisten from all events
      unlistenFunctionsRef.current.forEach(unlisten => {
        try {
          unlisten()
        } catch (error) {
          console.error(`[ProgressToast] Error unlistening:`, error)
        }
      })
      unlistenFunctionsRef.current = []
    }
  }, [instanceName, onError])

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

  const getStatusText = () => {
    // If we have a stage, always show it
    if (stage) return stage
    
    // Otherwise show status-based text
    switch (status) {
      case "creating":
        return hasReceivedProgress ? "Processing..." : "Starting..."
      case "success":
        return "Complete!"
      case "error":
        return "Failed"
    }
  }

  const handleDismiss = () => {
    onDismiss()
  }

  return (
    <div className="fixed top-14 right-4 z-40 w-80 bg-[#1a1a1a] rounded-lg overflow-hidden animate-in slide-in-from-right-4 duration-300">
      <div className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-2">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <p className="text-sm font-medium text-[#e8e8e8] truncate">
                    {getStatusText()}
                  </p>
                  {status === "creating" && (
                    <Loader2 size={14} className="animate-spin text-[#16a34a] flex-shrink-0" />
                  )}
                  {status === "success" && (
                    <CheckCircle2 size={16} className="text-[#16a34a] flex-shrink-0" />
                  )}
                  {status === "error" && (
                    <XCircle size={16} className="text-red-500 flex-shrink-0" />
                  )}
                </div>
                <p className="text-xs text-[#808080] mt-0.5 truncate">{instanceName}</p>
              </div>
              
              <div className="flex items-center gap-2">
                {/* Percentage */}
                <p className="text-xs text-[#4a4a4a] flex-shrink-0">
                  {Math.round(progress)}%
                </p>
                
                {status !== "creating" && (
                  <button
                    onClick={handleDismiss}
                    className="flex-shrink-0 p-1 hover:bg-[#2a2a2a] rounded transition-colors text-[#808080] hover:text-[#e8e8e8] cursor-pointer"
                  >
                    <X size={14} />
                  </button>
                )}
              </div>
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