'use client'

import { useState } from 'react'
import { FolderOpen, CheckCircle2, XCircle, Loader2, RefreshCw, ArrowRight } from 'lucide-react'
import { cn } from '@/lib/utils'

interface EnvItem {
  key: string
  label: string
  status: 'checking' | 'ok' | 'missing'
  version?: string
}

const INITIAL_ENV: EnvItem[] = [
  { key: 'python', label: 'Python 3.10+', status: 'checking' },
  { key: 'uv', label: 'uv (package manager)', status: 'checking' },
  { key: 'manim', label: 'Manim CE', status: 'checking' },
  { key: 'ffmpeg', label: 'FFmpeg', status: 'checking' },
]

const MOCK_ENV_RESULT: EnvItem[] = [
  { key: 'python', label: 'Python 3.10+', status: 'ok', version: '3.12.3' },
  { key: 'uv', label: 'uv (package manager)', status: 'ok', version: '0.4.27' },
  { key: 'manim', label: 'Manim CE', status: 'ok', version: '0.18.1' },
  { key: 'ffmpeg', label: 'FFmpeg', status: 'missing' },
]

interface FirstLaunchProps {
  onComplete: () => void
}

export default function FirstLaunch({ onComplete }: FirstLaunchProps) {
  const [workspacePath, setWorkspacePath] = useState('C:\\Users\\User\\LLM-Manim-Projects')
  const [envItems, setEnvItems] = useState<EnvItem[]>(INITIAL_ENV)
  const [checking, setChecking] = useState(false)
  const [checked, setChecked] = useState(false)

  function handleCheck() {
    setChecking(true)
    setChecked(false)
    setEnvItems(INITIAL_ENV.map((i) => ({ ...i, status: 'checking' })))

    const delays = [400, 700, 1100, 1500]
    MOCK_ENV_RESULT.forEach((item, idx) => {
      setTimeout(() => {
        setEnvItems((prev) =>
          prev.map((p) => (p.key === item.key ? { ...p, ...item } : p)),
        )
        if (idx === MOCK_ENV_RESULT.length - 1) {
          setChecking(false)
          setChecked(true)
        }
      }, delays[idx])
    })
  }

  const allOk = envItems.every((i) => i.status === 'ok')
  const hasMissing = envItems.some((i) => i.status === 'missing')
  const canContinue = checked && workspacePath.trim().length > 0

  return (
    <div className="flex h-screen w-screen items-center justify-center bg-background">
      <div className="w-[480px] space-y-8">
        {/* Header */}
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="flex h-6 w-6 items-center justify-center rounded bg-foreground">
              <span className="text-xs font-bold text-primary-foreground font-mono">M</span>
            </span>
            <span className="text-sm font-medium text-foreground">LLM-Manim</span>
          </div>
          <h1 className="text-xl font-semibold text-foreground leading-tight">初始化工作区</h1>
          <p className="text-sm text-muted-foreground">
            选择项目存储目录并检查本地运行环境。
          </p>
        </div>

        {/* Workspace */}
        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
            工作区目录
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={workspacePath}
              onChange={(e) => setWorkspacePath(e.target.value)}
              className="flex-1 h-8 rounded border border-border bg-input px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring font-mono"
              placeholder="选择目录..."
              aria-label="工作区目录路径"
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
          <p className="text-xs text-muted-foreground">
            生成的视频和项目文件将保存在此目录。
          </p>
        </div>

        {/* Env Check */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
              环境检查
            </label>
            <button
              type="button"
              onClick={handleCheck}
              disabled={checking}
              className="flex items-center gap-1.5 h-6 px-2 rounded border border-border bg-secondary text-xs text-foreground hover:bg-accent transition-colors disabled:opacity-50"
              aria-label="重新检查环境"
            >
              <RefreshCw className={cn('h-3 w-3', checking && 'animate-spin')} aria-hidden />
              检查
            </button>
          </div>

          <div className="rounded border border-border divide-y divide-border overflow-hidden">
            {envItems.map((item) => (
              <div key={item.key} className="flex items-center justify-between px-3 py-2">
                <span className="text-sm text-foreground">{item.label}</span>
                <div className="flex items-center gap-2">
                  {item.version && (
                    <span className="text-xs text-muted-foreground font-mono">{item.version}</span>
                  )}
                  {item.status === 'checking' && (
                    <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" aria-label="检查中" />
                  )}
                  {item.status === 'ok' && (
                    <CheckCircle2 className="h-3.5 w-3.5 text-status-success" aria-label="已就绪" />
                  )}
                  {item.status === 'missing' && (
                    <XCircle className="h-3.5 w-3.5 text-status-error" aria-label="未找到" />
                  )}
                </div>
              </div>
            ))}
          </div>

          {checked && hasMissing && (
            <div className="flex gap-2 rounded border border-status-warning-border bg-status-warning-bg px-3 py-2">
              <span className="text-xs text-status-warning leading-relaxed">
                <span className="font-medium">部分依赖缺失。</span>{' '}
                缺少的组件可能导致某些功能不可用。可先继续，稍后在设置中修复。
              </span>
            </div>
          )}
          {checked && allOk && (
            <div className="flex gap-2 rounded border border-status-success-border bg-status-success-bg px-3 py-2">
              <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-status-success mt-0.5" aria-hidden />
              <span className="text-xs text-status-success">所有依赖已就绪。</span>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center justify-between pt-2 border-t border-border">
          <span className="text-xs text-muted-foreground">
            {!checked ? '请先运行环境检查。' : hasMissing ? '存在缺失依赖，可继续但功能受限。' : ''}
          </span>
          <button
            type="button"
            onClick={onComplete}
            disabled={!canContinue}
            className="flex items-center gap-2 h-8 px-4 rounded bg-primary text-sm font-medium text-primary-foreground hover:opacity-90 transition-opacity disabled:opacity-30"
            aria-label="继续进入主界面"
          >
            继续
            <ArrowRight className="h-3.5 w-3.5" aria-hidden />
          </button>
        </div>
      </div>
    </div>
  )
}
