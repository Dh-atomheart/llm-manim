'use client'

import { useState, useEffect, useRef } from 'react'
import {
  Play,
  Square,
  RefreshCw,
  ChevronDown,
  ChevronRight,
  Terminal,
  Film,
  Copy,
  ExternalLink,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import StatusBadge, { type JobStatus } from '@/components/status-badge'

const MOCK_LOG_LINES = [
  '[12:04:01] INFO  Sending prompt to gpt-4o...',
  '[12:04:03] INFO  Received Manim script (84 lines)',
  '[12:04:03] INFO  Writing script to /tmp/scene_abc123.py',
  '[12:04:03] INFO  Running: uv run manim scene_abc123.py FourierScene -ql',
  '[12:04:05] INFO  Manim initialization complete',
  '[12:04:07] INFO  Rendering frame 1/240...',
  '[12:04:09] INFO  Rendering frame 60/240...',
  '[12:04:11] INFO  Rendering frame 120/240...',
  '[12:04:14] INFO  Rendering frame 180/240...',
  '[12:04:17] INFO  Rendering frame 240/240',
  '[12:04:17] INFO  Combining frames with ffmpeg...',
  '[12:04:18] SUCCESS  Output: /workspace/videos/FourierScene.mp4 (12s, 4.2 MB)',
]

const MOCK_ERROR = {
  code: 'MANIM_RENDER_ERROR',
  summary: 'Manim 渲染失败：场景类未找到。',
  detail: 'NameError: name "FourierScene" is not defined. The generated script references a class that was not created in the output.',
  suggestion: '重试通常可解决此问题。如果持续失败，请简化提示词或明确指定动画类型。',
}

interface WorkbenchProps {
  projectName: string
  provider: string
  model: string
}

export default function Workbench({ projectName, provider, model }: WorkbenchProps) {
  const [prompt, setPrompt] = useState('')
  const [jobStatus, setJobStatus] = useState<JobStatus>('idle')
  const [elapsed, setElapsed] = useState(0)
  const [logsOpen, setLogsOpen] = useState(false)
  const [visibleLogs, setVisibleLogs] = useState<string[]>([])
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const logTimersRef = useRef<ReturnType<typeof setTimeout>[]>([])

  useEffect(() => {
    if (jobStatus === 'running') {
      setElapsed(0)
      intervalRef.current = setInterval(() => setElapsed((s) => s + 1), 1000)
    } else {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
  }, [jobStatus])

  function clearLogTimers() {
    logTimersRef.current.forEach(clearTimeout)
    logTimersRef.current = []
  }

  function startGeneration(shouldSucceed: boolean) {
    setJobStatus('queued')
    setVisibleLogs([])
    clearLogTimers()

    setTimeout(() => {
      setJobStatus('running')
      setLogsOpen(true)
      MOCK_LOG_LINES.forEach((line, i) => {
        const t = setTimeout(
          () => setVisibleLogs((prev) => [...prev, line]),
          800 + i * 600,
        )
        logTimersRef.current.push(t)
      })

      const doneDelay = 800 + MOCK_LOG_LINES.length * 600 + 400
      const doneTimer = setTimeout(() => {
        setJobStatus(shouldSucceed ? 'succeeded' : 'failed')
      }, doneDelay)
      logTimersRef.current.push(doneTimer)
    }, 1200)
  }

  function handleCancel() {
    clearLogTimers()
    if (intervalRef.current) clearInterval(intervalRef.current)
    setJobStatus('cancelled')
  }

  function handleRetry() {
    if (prompt.trim()) startGeneration(true)
  }

  function handleReset() {
    setJobStatus('idle')
    setVisibleLogs([])
    setElapsed(0)
    clearLogTimers()
  }

  const isActive = jobStatus === 'queued' || jobStatus === 'running'
  const charCount = prompt.length

  return (
    <div className="flex h-full flex-col">
      {/* Prompt area */}
      <div className="flex-1 min-h-0 flex flex-col p-5 gap-4 overflow-y-auto">
        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider" htmlFor="prompt-input">
            提示词
          </label>
          <div className="relative">
            <textarea
              id="prompt-input"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="描述你想要生成的数学动画...&#10;&#10;例如：绘制一个单位圆，展示正弦函数从圆上各点的投影关系，动画时长 10 秒。"
              rows={6}
              disabled={isActive}
              className={cn(
                'w-full resize-none rounded border border-border bg-input px-3 py-2.5 text-sm text-foreground',
                'placeholder:text-muted-foreground leading-relaxed',
                'focus:outline-none focus:ring-1 focus:ring-ring',
                'disabled:opacity-60 disabled:cursor-not-allowed',
              )}
              aria-label="动画提示词输入"
            />
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground font-mono">
              {charCount > 0 ? `${charCount} 字符` : ''}
            </span>
            <div className="flex items-center gap-2">
              {(jobStatus === 'succeeded' || jobStatus === 'failed' || jobStatus === 'cancelled') && (
                <button
                  type="button"
                  onClick={handleReset}
                  className="flex items-center gap-1.5 h-8 px-3 rounded border border-border bg-secondary text-xs text-foreground hover:bg-accent transition-colors"
                  aria-label="清除当前结果，重新输入"
                >
                  <RefreshCw className="h-3 w-3" aria-hidden />
                  重置
                </button>
              )}

              {/* Demo controls */}
              {!isActive && jobStatus === 'idle' && (
                <div className="flex items-center gap-1">
                  <button
                    type="button"
                    onClick={() => startGeneration(true)}
                    disabled={prompt.trim().length === 0}
                    className="flex items-center gap-2 h-8 px-4 rounded bg-primary text-sm font-medium text-primary-foreground hover:opacity-90 transition-opacity disabled:opacity-30"
                    aria-label="生成视频"
                  >
                    <Play className="h-3.5 w-3.5 fill-current" aria-hidden />
                    生成视频
                  </button>
                  <button
                    type="button"
                    onClick={() => startGeneration(false)}
                    disabled={prompt.trim().length === 0}
                    title="模拟失败（演示用）"
                    className="h-8 px-2 rounded border border-border bg-secondary text-xs text-muted-foreground hover:bg-accent transition-colors disabled:opacity-30"
                    aria-label="模拟失败状态（演示）"
                  >
                    失败演示
                  </button>
                </div>
              )}

              {jobStatus === 'running' && (
                <button
                  type="button"
                  onClick={handleCancel}
                  className="flex items-center gap-2 h-8 px-3 rounded border border-status-error-border bg-status-error-bg text-xs text-status-error hover:opacity-80 transition-opacity"
                  aria-label="取消生成"
                >
                  <Square className="h-3 w-3 fill-current" aria-hidden />
                  取消
                </button>
              )}

              {jobStatus === 'queued' && (
                <button
                  type="button"
                  onClick={handleCancel}
                  className="flex items-center gap-2 h-8 px-3 rounded border border-border bg-secondary text-xs text-muted-foreground hover:bg-accent transition-colors"
                  aria-label="取消排队"
                >
                  <Square className="h-3 w-3" aria-hidden />
                  取消
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Status / Output area */}
        {jobStatus !== 'idle' && (
          <div className="space-y-3">
            {/* Status bar */}
            <div className="flex items-center gap-3 rounded border border-border bg-card px-3 py-2">
              <StatusBadge status={jobStatus === 'idle' ? 'queued' : jobStatus as Exclude<JobStatus,'idle'>} />
              {jobStatus === 'running' && (
                <span className="text-xs text-muted-foreground font-mono">
                  已用时 {elapsed}s
                </span>
              )}
              {jobStatus === 'queued' && (
                <span className="text-xs text-muted-foreground">等待调度...</span>
              )}
              {jobStatus === 'succeeded' && (
                <span className="text-xs text-muted-foreground font-mono">
                  {`用时 ${elapsed}s · ${provider} / ${model}`}
                </span>
              )}
              {jobStatus === 'failed' && (
                <span className="text-xs text-muted-foreground font-mono">
                  {`用时 ${elapsed}s · ${provider} / ${model}`}
                </span>
              )}
              {jobStatus === 'cancelled' && (
                <span className="text-xs text-muted-foreground">已手动取消</span>
              )}
            </div>

            {/* Video preview */}
            {jobStatus === 'succeeded' && (
              <div className="rounded border border-border overflow-hidden">
                <div
                  className="relative flex items-center justify-center bg-card"
                  style={{ aspectRatio: '16/9' }}
                  role="img"
                  aria-label="生成的动画视频预览"
                >
                  <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-muted-foreground">
                    <Film className="h-8 w-8" aria-hidden />
                    <span className="text-sm">FourierScene.mp4 · 12s · 4.2 MB</span>
                    <div className="flex items-center gap-2">
                      <button
                        type="button"
                        className="flex items-center gap-1.5 h-7 px-3 rounded bg-primary text-xs font-medium text-primary-foreground hover:opacity-90 transition-opacity"
                        aria-label="播放视频"
                      >
                        <Play className="h-3 w-3 fill-current" aria-hidden />
                        播放
                      </button>
                      <button
                        type="button"
                        className="flex items-center gap-1.5 h-7 px-2 rounded border border-border bg-secondary text-xs text-foreground hover:bg-accent transition-colors"
                        aria-label="在文件管理器中打开"
                      >
                        <ExternalLink className="h-3 w-3" aria-hidden />
                        在资源管理器中打开
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Error block */}
            {jobStatus === 'failed' && (
              <div className="rounded border border-status-error-border bg-status-error-bg space-y-0 overflow-hidden">
                <div className="px-3 py-2.5 border-b border-status-error-border">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-medium text-status-error">
                      {MOCK_ERROR.code}
                    </span>
                    <button
                      type="button"
                      className="flex items-center gap-1 text-xs text-status-error hover:opacity-70 transition-opacity"
                      aria-label="复制错误信息"
                    >
                      <Copy className="h-3 w-3" aria-hidden />
                      复制
                    </button>
                  </div>
                  <p className="mt-1 text-sm text-status-error font-medium">{MOCK_ERROR.summary}</p>
                </div>
                <div className="px-3 py-2 space-y-2">
                  <p className="text-xs text-status-error/80 font-mono leading-relaxed break-all">
                    {MOCK_ERROR.detail}
                  </p>
                  <div className="border-t border-status-error-border pt-2 flex items-start justify-between gap-4">
                    <p className="text-xs text-muted-foreground leading-relaxed">
                      <span className="font-medium text-foreground">建议：</span>{' '}
                      {MOCK_ERROR.suggestion}
                    </p>
                    <button
                      type="button"
                      onClick={handleRetry}
                      className="flex shrink-0 items-center gap-1.5 h-7 px-3 rounded border border-status-error-border bg-status-error-bg text-xs text-status-error hover:opacity-80 transition-opacity"
                      aria-label="重试生成"
                    >
                      <RefreshCw className="h-3 w-3" aria-hidden />
                      重试
                    </button>
                  </div>
                </div>
              </div>
            )}

            {/* Cancelled */}
            {jobStatus === 'cancelled' && (
              <div className="rounded border border-status-cancelled-border bg-status-cancelled-bg px-3 py-2.5">
                <p className="text-sm text-status-cancelled">
                  生成已取消。修改提示词后可重新发起。
                </p>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Log panel */}
      <div className="shrink-0 border-t border-border">
        <button
          type="button"
          onClick={() => setLogsOpen((o) => !o)}
          className="flex w-full items-center gap-2 px-4 py-2 text-xs text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
          aria-expanded={logsOpen}
          aria-controls="log-panel"
        >
          {logsOpen ? (
            <ChevronDown className="h-3 w-3" aria-hidden />
          ) : (
            <ChevronRight className="h-3 w-3" aria-hidden />
          )}
          <Terminal className="h-3 w-3" aria-hidden />
          <span>日志</span>
          {visibleLogs.length > 0 && (
            <span className="ml-auto font-mono">{visibleLogs.length} 行</span>
          )}
        </button>

        {logsOpen && (
          <div
            id="log-panel"
            className="h-40 overflow-y-auto bg-card border-t border-border px-3 py-2 space-y-0.5"
            role="log"
            aria-label="生成日志"
            aria-live="polite"
          >
            {visibleLogs.length === 0 ? (
              <p className="text-xs text-muted-foreground italic">暂无日志。</p>
            ) : (
              visibleLogs.map((line, i) => (
                <p key={i} className="text-xs font-mono text-muted-foreground leading-5 whitespace-pre-wrap break-all">
                  {line}
                </p>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  )
}
