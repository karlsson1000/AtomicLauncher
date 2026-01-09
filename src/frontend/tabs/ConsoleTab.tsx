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
        setTimeout(() => setUploadState({ loading: false, url: null, error: null }), 3000)
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
        // Auto-copy the URL to clipboard
        await navigator.clipboard.writeText(data.url)
        // Open the link immediately
        window.open(data.url, '_blank')
        setUploadState({ loading: false, url: data.url, error: null })
        // Clear success message after 3 seconds
        setTimeout(() => setUploadState({ loading: false, url: null, error: null }), 3000)
      } else {
        setUploadState({ loading: false, url: null, error: data.error || 'Upload failed' })
        setTimeout(() => setUploadState({ loading: false, url: null, error: null }), 3000)
      }
    } catch (err) {
      console.error('Failed to upload logs:', err)
      setUploadState({ loading: false, url: null, error: 'Network error occurred' })
      setTimeout(() => setUploadState({ loading: false, url: null, error: null }), 3000)
    }
  }

  return (
    <div className="p-6 space-y-4">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-4">
          <h1 className="text-2xl font-semibold text-[#e6edf3] tracking-tight">Console</h1>
          <p className="text-sm text-[#7d8590] mt-0.5">View game output and logs</p>
        </div>

        {/* Console Display */}
        <div className="bg-[#0f0f0f] border border-[#2a2a2a] rounded-md overflow-hidden" style={{ height: 'calc(100vh - 225px)' }}>
          {consoleLogs.length === 0 ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-center">
                <Terminal size={48} className="text-[#16a34a] mx-auto mb-3" strokeWidth={1.5} />
                <p className="text-base text-[#e6edf3] mb-1">No console output yet</p>
                <p className="text-sm text-[#7d8590]">Launch an instance to see logs</p>
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
                        : "text-[#e6edf3]"
                    }`}
                  >
                    <span className="text-[#7d8590] mr-2">{new Date().toLocaleTimeString()}</span>
                    <span className="text-[#16a34a] mr-2">[{log.instance}]</span>
                    <span>{log.message}</span>
                  </div>
                );
              })}
              <div ref={consoleEndRef} />
            </div>
          )}
        </div>

        {/* Action Buttons */}
        <div className="flex items-center justify-end gap-2 mt-4">
          <button
            onClick={handleUploadToMcLogs}
            disabled={consoleLogs.length === 0 || uploadState.loading}
            className={`px-4 h-8 rounded-md font-medium text-sm flex items-center gap-2 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed border ${
              uploadState.url
                ? "bg-[#16a34a]/10 border-[#16a34a]/30 text-[#16a34a]"
                : uploadState.error
                ? "bg-red-500/10 border-red-400/30 text-red-400"
                : "bg-[#141414] hover:bg-[#1a1a1a] text-[#e6edf3] border-[#2a2a2a]"
            }`}
            title="Upload logs to mclo.gs"
          >
            {uploadState.loading ? (
              <Loader2 size={16} strokeWidth={2} className="animate-spin" />
            ) : uploadState.url ? (
              <ExternalLink size={16} strokeWidth={2} />
            ) : uploadState.error ? (
              <X size={16} strokeWidth={2} />
            ) : (
              <Upload size={16} strokeWidth={2} />
            )}
            <span>
              {uploadState.loading 
                ? 'Uploading...' 
                : uploadState.url
                ? 'Uploaded & Copied!'
                : uploadState.error
                ? uploadState.error
                : 'Upload to mclo.gs'}
            </span>
          </button>
          <button
            onClick={onClearConsole}
            disabled={consoleLogs.length === 0}
            className="px-4 h-8 bg-[#141414] hover:bg-[#1a1a1a] text-[#e6edf3] rounded-md font-medium text-sm flex items-center gap-2 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed border border-[#2a2a2a]"
          >
            <Trash2 size={16} strokeWidth={2} />
            <span>Clear</span>
          </button>
        </div>
      </div>
    </div>
  )
}