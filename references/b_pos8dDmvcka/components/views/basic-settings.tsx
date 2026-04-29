'use client'

import { useState } from 'react'
import {
  FolderOpen,
  RefreshCw,
  CheckCircle2,
  XCircle,
  Loader2,
  ChevronDown,
  Terminal,
  Info,
} from 'lucide-react'
import { cn } from '@/lib/utils'

type EnvStatus = 'unchecked' | 'checking' | 'ok' | 'missing'
type LogLevel = 'error' | 'warn' | 'info' | 'debug'

interface EnvItem {
  key: string
  label: string
  status: EnvStatus
  version?: string
  path?: string
}

const INITIAL_ENV: EnvItem[] = [
  { key: 'python', label: 'Python 3.10+', status: 'unchecked' },
  { key: 'uv', label: 'uv (package manager)', status: 'unchecked' },
  { key: 'manim', label: 'Manim CE', status: 'unchecked' },
  { key: 'ffmpeg', label: 'FFmpeg', status: 'unchecked' },
]

const MOCK_RESULT: EnvItem[] = [
  { key: 'python', label: 'Python 3.10+', status: 'ok', version: '3.12.3', path: 'C:\\Python312\\python.exe' },
  { key: 'uv', label: 'uv (package manager)', status: 'ok', version: '0.4.27', path: 'C:\\Users\\User\\.cargo\\bin\\uv.exe' },
  { key: 'manim', label: 'Manim CE', status: 'ok', version: '0.18.1', path: 'C:\\Users\\User\\.venv\\Scripts\\manim.exe' },
  { key: 'ffmpeg', label: 'FFmpeg', status: 'missing' },
]

export default function BasicSettings() {
  const [workspacePath, setWorkspacePath] = useState('C:\\Users\\User\\LLM-Manim-Projects')
  const [logLevel, setLogLevel] = useState<LogLevel>('info')
  const [envItems, setEnvItems] = useState<EnvItem[]>(INITIAL_ENV)
  const [checking, setChecking] = useState(false)
  const [hasChecked, setHasChecked] = useState(false)
  const [saved, setSaved] = useState(false)
  const [diagOpen, setDiagOpen] = useState(false)

  function handleCheck() {
    setChecking(true)
    setHasChecked(false)
    setEnvItems(INITIAL_ENV.map((i) => ({ ...i, status: 'checking' })))

    const delays = [450, 800, 1200, 1650]
    MOCK_RESULT.forEach((item, idx) => {
      setTimeout(() => {
        setEnvItems((prev) =>
          prev.map((p) => (p.key === item.key ? { ...p, ...item } : p)),
        )
        if (idx === MOCK_RESULT.length - 1) {
          setChecking(false)
          setHasChecked(true)
        }
      }, delays[idx])
    })
  }

  function handleSave() {
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
  }

  const hasMissing = hasChecked && envItems.some((i) => i.status === 'missing')
  const allOk = hasChecked && envItems.every((i) => i.status === 'ok')

  return (
    <div className="flex h-full flex-col overflow-y-auto">
      <div className="max-w-2xl w-full mx-auto px-5 py-5 space-y-8">

        {/* Workspace */}
        <section className="space-y-3" aria-labelledby="workspace-heading">
          <div className="pb-2 border-b border-border">
            <h2 id="workspace-heading" className="text-sm font-semibold text-foreground">工作区</h2>
            <p className="text-xs text-muted-foreground mt-0.5">项目和生成视频的存储路径。</p>
          </div>
          <div className="space-y-1.5">
            <label className="text-xs text-muted-foreground" htmlFor="workspace-path">
              目录路径
            </label>
            <div className="flex gap-2">
              <input
                id="workspace-path"
                type="text"
                value={workspacePath}
                onChange={(e) => setWorkspacePath(e.target.value)}
                className="flex-1 h-8 rounded border border-border bg-input px-3 text-sm text-foreground font-mono placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              />
              <button
                type="button"
                className="flex items-center gap-1.5 h-8 px-3 rounded border border-border bg-secondary text-sm text-foreground hover:bg-accent transition-colors"
                aria-label="浏览目录"
              >
                <FolderOpen className="h-3.5 w-3.5" aria-hidden />
                浏览
              </button>
            </div>
          </div>
        </section>

        {/* Runtime */}
        <section className="space-y-3" aria-labelledby="runtime-heading">
          <div className="pb-2 border-b border-border flex items-center justify-between">
            <div>
              <h2 id="runtime-heading" className="text-sm font-semibold text-foreground">本地运行环境</h2>
              <p className="text-xs text-muted-foreground mt-0.5">检查 Manim 和相关依赖的安装状态。</p>
            </div>
            <button
              type="button"
              onClick={handleCheck}
              disabled={checking}
              className="flex items-center gap-1.5 h-7 px-3 rounded border border-border bg-secondary text-xs text-foreground hover:bg-accent transition-colors disabled:opacity-50"
              aria-label="重新检查运行环境"
            >
              <RefreshCw className={cn('h-3 w-3', checking && 'animate-spin')} aria-hidden />
              检查环境
            </button>
          </div>

          <div className="rounded border border-border overflow-hidden divide-y divide-border">
            {envItems.map((item) => (
              <div key={item.key} className="px-3 py-2.5">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-foreground">{item.label}</span>
                  <div className="flex items-center gap-2">
                    {item.version && (
                      <span className="text-xs text-muted-foreground font-mono">{item.version}</span>
                    )}
                    {item.status === 'unchecked' && (
                      <span className="text-xs text-muted-foreground">—</span>
                    )}
                    {item.status === 'checking' && (
                      <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" aria-label="检查中" />
                    )}
                    {item.status === 'ok' && (
                      <CheckCircle2 className="h-3.5 w-3.5 text-status-success" aria-label="已就绪" />
                    )}
                    {item.status === 'missing' && (
                      <div className="flex items-center gap-1.5">
                        <span className="text-xs text-status-error">未找到</span>
                        <XCircle className="h-3.5 w-3.5 text-status-error" aria-label="缺失" />
                      </div>
                    )}
                  </div>
                </div>
                {item.path && (
                  <p className="text-xs text-muted-foreground font-mono mt-0.5 truncate" title={item.path}>
                    {item.path}
                  </p>
                )}
              </div>
            ))}
          </div>

          {hasMissing && (
            <div className="flex items-start gap-2 rounded border border-status-warning-border bg-status-warning-bg px-3 py-2.5" role="alert">
              <Info className="h-3.5 w-3.5 shrink-0 text-status-warning mt-0.5" aria-hidden />
              <p className="text-xs text-status-warning leading-relaxed">
                <span className="font-medium">FFmpeg 未找到。</span>{' '}
                请安装 FFmpeg 并确保其在系统 PATH 中，或通过{' '}
                <code className="font-mono">uv tool install ffmpeg</code> 安装。
              </p>
            </div>
          )}
          {allOk && (
            <div className="flex items-center gap-2 rounded border border-status-success-border bg-status-success-bg px-3 py-2" role="status">
              <CheckCircle2 className="h-3.5 w-3.5 text-status-success" aria-hidden />
              <span className="text-xs text-status-success">所有运行环境依赖已就绪。</span>
            </div>
          )}
        </section>

        {/* Log level */}
        <section className="space-y-3" aria-labelledby="log-heading">
          <div className="pb-2 border-b border-border">
            <h2 id="log-heading" className="text-sm font-semibold text-foreground">日志</h2>
            <p className="text-xs text-muted-foreground mt-0.5">控制生成过程中输出的日志详细程度。</p>
          </div>
          <div className="flex items-center gap-4">
            <label className="text-sm text-foreground shrink-0" htmlFor="log-level">
              日志级别
            </label>
            <div className="relative">
              <select
                id="log-level"
                value={logLevel}
                onChange={(e) => setLogLevel(e.target.value as LogLevel)}
                className="h-8 rounded border border-border bg-input pl-3 pr-8 text-sm text-foreground appearance-none focus:outline-none focus:ring-1 focus:ring-ring"
              >
                <option value="error">ERROR — 仅错误</option>
                <option value="warn">WARN — 警告及以上</option>
                <option value="info">INFO — 标准输出（推荐）</option>
                <option value="debug">DEBUG — 详细调试信息</option>
              </select>
              <ChevronDown className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" aria-hidden />
            </div>
          </div>
        </section>

        {/* Diagnostics */}
        <section className="space-y-2" aria-labelledby="diag-heading">
          <button
            type="button"
            onClick={() => setDiagOpen((v) => !v)}
            className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors"
            aria-expanded={diagOpen}
            aria-controls="diag-panel"
          >
            <Terminal className="h-3 w-3" aria-hidden />
            <span id="diag-heading">诊断信息</span>
            <ChevronDown className={cn('h-3 w-3 transition-transform', diagOpen && 'rotate-180')} aria-hidden />
          </button>
          {diagOpen && (
            <div
              id="diag-panel"
              className="rounded border border-border bg-card p-3 font-mono text-xs text-muted-foreground space-y-1 leading-5"
            >
              <p>App Version: 0.1.0-alpha</p>
              <p>Platform: Windows 11 (10.0.22631)</p>
              <p>Workspace: {workspacePath}</p>
              <p>Log Level: {logLevel.toUpperCase()}</p>
              <p>Config Dir: %APPDATA%\LLM-Manim\config.json</p>
              <p>Cache Dir: %LOCALAPPDATA%\LLM-Manim\cache</p>
            </div>
          )}
        </section>

        {/* Save */}
        <div className="flex items-center justify-end gap-3 pt-2 border-t border-border">
          {saved && (
            <span className="flex items-center gap-1.5 text-xs text-status-success" role="status">
              <CheckCircle2 className="h-3.5 w-3.5" aria-hidden />
              已保存
            </span>
          )}
          <button
            type="button"
            onClick={handleSave}
            className="h-8 px-4 rounded bg-primary text-sm font-medium text-primary-foreground hover:opacity-90 transition-opacity"
            aria-label="保存设置"
          >
            保存
          </button>
        </div>
      </div>
    </div>
  )
}
