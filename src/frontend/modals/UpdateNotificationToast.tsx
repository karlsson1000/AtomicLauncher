import { Download, X, Clock, Loader2 } from "lucide-react"

interface UpdateNotificationToastProps {
  currentVersion: string
  newVersion: string
  isInstalling?: boolean
  onInstall: () => void
  onLater: () => void
}

export function UpdateNotificationToast({
  newVersion,
  isInstalling = false,
  onInstall,
  onLater 
}: UpdateNotificationToastProps) {
  return (
    <div className="fixed top-14 right-4 z-40 w-80 bg-[#141414] border border-[#2a2a2a] rounded-md shadow-lg overflow-hidden animate-in slide-in-from-right-4 duration-300">
      <div className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex-shrink-0 w-10 h-10 rounded-full bg-gradient-to-br from-[#238636] to-[#2ea043] flex items-center justify-center">
            {isInstalling ? (
              <Loader2 size={20} className="text-white animate-spin" />
            ) : (
              <Download size={20} className="text-white" />
            )}
          </div>
          
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-2">
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold text-[#e6edf3]">
                  {isInstalling ? "Installing Update..." : "Update Available"}
                </p>
                <p className="text-xs text-[#7d8590] mt-0.5">
                  {isInstalling 
                    ? "The app will restart automatically" 
                    : `Version ${newVersion} is ready to install`
                  }
                </p>
              </div>
              
              {!isInstalling && (
                <button
                  onClick={onLater}
                  className="flex-shrink-0 p-1 hover:bg-[#1a1a1a] rounded transition-colors text-[#7d8590] hover:text-[#e6edf3] cursor-pointer"
                >
                  <X size={14} />
                </button>
              )}
            </div>

            {!isInstalling && (
              <div className="flex items-center gap-2 mt-3">
                <button
                  onClick={onInstall}
                  className="flex-1 px-3 py-1.5 bg-gradient-to-r from-[#238636] to-[#2ea043] hover:from-[#2ea043] hover:to-[#238636] text-white text-xs font-medium rounded transition-all cursor-pointer flex items-center justify-center gap-1.5"
                >
                  <Download size={14} />
                  Install Now
                </button>
                
                <button
                  onClick={onLater}
                  className="flex-1 px-3 py-1.5 bg-[#2a2a2a] hover:bg-[#3a3a3a] text-[#e6edf3] text-xs font-medium rounded transition-colors cursor-pointer flex items-center justify-center gap-1.5"
                >
                  <Clock size={14} />
                  Ask Later
                </button>
              </div>
            )}

            {isInstalling && (
              <div className="relative h-1.5 bg-[#2a2a2a] rounded-full overflow-hidden mt-3">
                <div className="absolute inset-y-0 left-0 w-full bg-gradient-to-r from-[#238636] to-[#2ea043] animate-pulse rounded-full" />
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}