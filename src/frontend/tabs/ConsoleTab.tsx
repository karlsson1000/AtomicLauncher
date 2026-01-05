import { useRef, useEffect, useState } from "react"
import { Terminal, Trash2, Upload, ExternalLink, Loader2, X } from "lucide-react"
import type { ConsoleLog } from "../../types"

interface ConsoleTabProps {
  consoleLogs: ConsoleLog[]
  onClearConsole: () => void
}

export function ConsoleTab({ consoleLogs, onClearConsole }: ConsoleTabProps) {
  const consoleEndRef = useRef<HTMLDivElement>(null)
  const [uploadState, setUploadState] = useState<{
    loading: boolean
    url: string | null
    error: string | null
  }>({ loading: false, url: null, error: null })

  useEffect(() => {
    if (consoleEndRef.current) {
      consoleEndRef.current.scrollIntoView({ behavior: "smooth" })
    }
  }, [consoleLogs])

  const formatLogs = () => {
    return consoleLogs.map(log => 
      `${new Date().toLocaleTimeString()} [${log.instance}] ${log.message}`
    ).join('\n')
  }

  const handleUploadToMcLogs = async () => {
    setUploadState({ loading: true, url: null, error: null })
    
    try {
      const logContent = formatLogs()
      
      if (!logContent.trim()) {
        setUploadState({ loading: false, url: null, error: 'No logs to upload' })
        return
      }

      const formData = new URLSearchParams()
      formData.append('content', logContent)

      const response = await fetch('https://api.mclo.gs/1/log', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
        },
        body: formData
      })

      const data = await response.json()

      if (data.success) {
        setUploadState({ loading: false, url: data.url, error: null })
        // Auto-copy the URL to clipboard
        await navigator.clipboard.writeText(data.url)
      } else {
        setUploadState({ loading: false, url: null, error: data.error || 'Upload failed' })
      }
    } catch (err) {
      console.error('Failed to upload logs:', err)
      setUploadState({ loading: false, url: null, error: 'Network error occurred' })
    }
  }

  const handleCopyUrl = async () => {
    if (uploadState.url) {
      try {
        await navigator.clipboard.writeText(uploadState.url)
        alert('Link copied to clipboard!')
      } catch (err) {
        console.error('Failed to copy URL:', err)
      }
    }
  }

  const closeUploadNotification = () => {
    setUploadState({ loading: false, url: null, error: null })
  }

  return (
    <div className="p-6 space-y-4">
      <div className="max-w-7xl mx-auto">
        <div className="mb-4">
          <h1 className="text-2xl font-semibold text-[#e8e8e8] tracking-tight">Console</h1>
          <p className="text-sm text-[#808080] mt-0.5">View game output and logs</p>
        </div>

        <div className="bg-[#101010] rounded-md overflow-hidden" style={{ height: 'calc(100vh - 280px)' }}>
          {consoleLogs.length === 0 ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-center">
                <Terminal size={48} className="text-[#16a34a] mx-auto mb-3" strokeWidth={1.5} />
                <p className="text-base text-[#808080] mb-1">No console output yet</p>
                <p className="text-sm text-[#4a4a4a]">Launch an instance to see logs</p>
              </div>
            </div>
          ) : (
            <div className="h-full overflow-y-auto p-4 font-mono text-sm">
              {consoleLogs.map((log, index) => {
                const isError = log.type === "stderr" || log.message.toLowerCase().includes("error") || log.message.toLowerCase().includes("failed");
                const isWarning = log.message.toLowerCase().includes("warning") || log.message.toLowerCase().includes("warn");
                
                return (
                  <div
                    key={index}
                    className={`py-0.5 leading-relaxed ${
                      isError 
                        ? "text-red-400" 
                        : isWarning 
                        ? "text-yellow-400" 
                        : "text-[#e8e8e8]"
                    }`}
                  >
                    <span className="text-[#4a4a4a] mr-2">{new Date().toLocaleTimeString()}</span>
                    <span className="text-[#16a34a] mr-2">[{log.instance}]</span>
                    <span>{log.message}</span>
                  </div>
                );
              })}
              <div ref={consoleEndRef} />
            </div>
          )}
        </div>

        <div className="flex items-center justify-end mt-4">
          {/* Upload Success/Error Notification */}
          {(uploadState.url || uploadState.error) && (
            <div className={`flex-1 mr-4 p-1.5 rounded ${
              uploadState.url
            } flex items-center justify-between`}>
              <div className="flex items-center gap-3 flex-1">
                {uploadState.url ? (
                  <div className="flex items-center gap-3 flex-1">
                    <p className="text-sm font-medium text-[#16a34a]">Logs uploaded successfully!</p>
                    <a 
                      href={uploadState.url} 
                      target="_blank" 
                      rel="noopener noreferrer"
                      className="text-sm text-[#e8e8e8] hover:text-[#16a34a] underline flex items-center gap-1 transition-colors"
                    >
                      {uploadState.url}
                      <ExternalLink size={14} />
                    </a>
                    <button
                      onClick={handleCopyUrl}
                      className="text-xs px-2 py-1 bg-[#0d0d0d] hover:bg-[#16a34a]/20 text-[#e8e8e8] rounded transition-colors cursor-pointer"
                    >
                      Copy Link
                    </button>
                  </div>
                ) : (
                  <div className="flex-1">
                    <p className="text-sm font-medium text-red-400">Upload failed</p>
                    <p className="text-xs text-red-300 mt-0.5">{uploadState.error}</p>
                  </div>
                )}
              </div>
              <button
                onClick={closeUploadNotification}
                className="text-[#808080] hover:text-[#e8e8e8] transition-colors cursor-pointer"
              >
                <X size={18} />
              </button>
            </div>
          )}

          <div className="flex gap-2 flex-shrink-0">
            <button
              onClick={handleUploadToMcLogs}
              disabled={consoleLogs.length === 0 || uploadState.loading}
              className="px-4 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded font-medium text-sm flex items-center gap-2 transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
              title="Upload logs to mclo.gs"
            >
              {uploadState.loading ? (
                <Loader2 size={16} strokeWidth={2} className="animate-spin" />
              ) : (
                <Upload size={16} strokeWidth={2} />
              )}
              <span>{uploadState.loading ? 'Uploading...' : 'Upload to mclo.gs'}</span>
            </button>
            <button
              onClick={onClearConsole}
              disabled={consoleLogs.length === 0}
              className="px-4 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded font-medium text-sm flex items-center gap-2 transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Trash2 size={16} strokeWidth={2} />
              <span>Clear</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}