'use client'

import { useState } from 'react'
import { Play, Trash2, ExternalLink, Search } from 'lucide-react'
import { cn } from '@/lib/utils'
import StatusBadge from '@/components/status-badge'

export interface HistoryItem {
  id: string
  projectId: string
  projectName: string
  prompt: string
  status: 'succeeded' | 'failed' | 'cancelled'
  provider: string
  model: string
  createdAt: string
  elapsedSeconds: number
  videoDurationSeconds?: number
}

const MOCK_HISTORY: HistoryItem[] = [
  {
    id: 'h1',
    projectId: 'p1',
    projectName: 'Fourier Series',
    prompt: 'Show a circle decomposing into Fourier sine waves step by step with labels',
    status: 'succeeded',
    provider: 'OpenAI',
    model: 'gpt-4o',
    createdAt: '2025-04-28T10:12:00Z',
    elapsedSeconds: 18,
    videoDurationSeconds: 12,
  },
  {
    id: 'h2',
    projectId: 'p1',
    projectName: 'Fourier Series',
    prompt: 'Animate a square wave approximation with 10 harmonics',
    status: 'failed',
    provider: 'OpenAI',
    model: 'gpt-4o',
    createdAt: '2025-04-28T09:50:00Z',
    elapsedSeconds: 7,
  },
  {
    id: 'h3',
    projectId: 'p2',
    projectName: 'Circle Animation',
    prompt: 'Draw a circle that smoothly transforms into a square over 5 seconds',
    status: 'succeeded',
    provider: 'Anthropic',
    model: 'claude-3-5-sonnet',
    createdAt: '2025-04-27T15:42:00Z',
    elapsedSeconds: 22,
    videoDurationSeconds: 8,
  },
  {
    id: 'h4',
    projectId: 'p2',
    projectName: 'Circle Animation',
    prompt: 'Animate pi approximation using inscribed polygons with increasing sides',
    status: 'cancelled',
    provider: 'OpenAI',
    model: 'gpt-4o-mini',
    createdAt: '2025-04-27T14:20:00Z',
    elapsedSeconds: 3,
  },
  {
    id: 'h5',
    projectId: 'p3',
    projectName: 'Mandelbrot Set',
    prompt: 'Zoom into Mandelbrot set around coordinate (-0.75, 0.1) over 20 seconds',
    status: 'succeeded',
    provider: 'OpenAI',
    model: 'gpt-4o',
    createdAt: '2025-04-25T09:15:00Z',
    elapsedSeconds: 34,
    videoDurationSeconds: 20,
  },
  {
    id: 'h6',
    projectId: 'p3',
    projectName: 'Mandelbrot Set',
    prompt: 'Show Julia set iteration for c = -0.7 + 0.27i',
    status: 'succeeded',
    provider: 'OpenAI',
    model: 'gpt-4o',
    createdAt: '2025-04-24T18:05:00Z',
    elapsedSeconds: 29,
    videoDurationSeconds: 15,
  },
  {
    id: 'h7',
    projectId: 'p4',
    projectName: 'Graph Theory',
    prompt: 'Animate Dijkstra shortest path algorithm on a 6-node weighted graph',
    status: 'failed',
    provider: 'Anthropic',
    model: 'claude-3-5-sonnet',
    createdAt: '2025-04-20T12:30:00Z',
    elapsedSeconds: 11,
  },
]

function formatDate(iso: string) {
  const d = new Date(iso)
  return d.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function formatDuration(s: number) {
  if (s < 60) return `${s}s`
  return `${Math.floor(s / 60)}m${s % 60}s`
}

export default function HistoryView() {
  const [items, setItems] = useState<HistoryItem[]>(MOCK_HISTORY)
  const [filter, setFilter] = useState<'all' | 'succeeded' | 'failed' | 'cancelled'>('all')
  const [search, setSearch] = useState('')
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null)

  const filtered = items.filter((i) => {
    const matchesFilter = filter === 'all' || i.status === filter
    const matchesSearch =
      search.trim() === '' ||
      i.prompt.toLowerCase().includes(search.toLowerCase()) ||
      i.projectName.toLowerCase().includes(search.toLowerCase())
    return matchesFilter && matchesSearch
  })

  function handleDelete(id: string) {
    if (deleteConfirm === id) {
      setItems((prev) => prev.filter((i) => i.id !== id))
      setDeleteConfirm(null)
    } else {
      setDeleteConfirm(id)
    }
  }

  return (
    <div className="flex h-full flex-col">
      {/* Toolbar */}
      <div className="flex items-center gap-3 border-b border-border px-4 py-2.5 shrink-0">
        <div className="relative flex-1 max-w-xs">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" aria-hidden />
          <input
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索提示词或项目..."
            className="w-full h-7 rounded border border-border bg-input pl-7 pr-3 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
            aria-label="搜索历史记录"
          />
        </div>
        <div className="flex items-center gap-1 ml-auto" role="group" aria-label="按状态筛选">
          {(['all', 'succeeded', 'failed', 'cancelled'] as const).map((f) => (
            <button
              key={f}
              type="button"
              onClick={() => setFilter(f)}
              className={cn(
                'h-7 px-2.5 rounded text-xs transition-colors border',
                filter === f
                  ? 'bg-secondary border-border text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-accent',
              )}
              aria-pressed={filter === f}
            >
              {f === 'all' ? '全部' : f === 'succeeded' ? '已完成' : f === 'failed' ? '失败' : '已取消'}
            </button>
          ))}
        </div>
        <span className="text-xs text-muted-foreground font-mono shrink-0">
          {filtered.length} 条记录
        </span>
      </div>

      {/* Table header */}
      <div className="grid grid-cols-[1fr_120px_140px_100px_80px_100px] gap-0 border-b border-border px-4 py-2 shrink-0">
        {['提示词 / 项目', '状态', '时间', 'Provider / 模型', '用时', '操作'].map((col, i) => (
          <span key={col} className={cn('text-xs font-medium text-muted-foreground', i > 0 && 'text-center')}>
            {col}
          </span>
        ))}
      </div>

      {/* Table body */}
      <div className="flex-1 min-h-0 overflow-y-auto divide-y divide-border" role="table" aria-label="历史记录列表">
        {filtered.length === 0 ? (
          <div className="flex items-center justify-center h-32 text-sm text-muted-foreground">
            暂无匹配记录
          </div>
        ) : (
          filtered.map((item) => (
            <div
              key={item.id}
              className="grid grid-cols-[1fr_120px_140px_100px_80px_100px] gap-0 px-4 py-2.5 hover:bg-accent transition-colors"
              role="row"
            >
              {/* Prompt + project */}
              <div className="min-w-0 pr-3" role="cell">
                <p className="text-sm text-foreground leading-snug truncate" title={item.prompt}>
                  {item.prompt}
                </p>
                <p className="text-xs text-muted-foreground mt-0.5">{item.projectName}</p>
              </div>

              {/* Status */}
              <div className="flex items-center justify-center" role="cell">
                <StatusBadge status={item.status} size="sm" />
              </div>

              {/* Created at */}
              <div className="flex items-center justify-center" role="cell">
                <span className="text-xs text-muted-foreground font-mono">
                  {formatDate(item.createdAt)}
                </span>
              </div>

              {/* Provider / model */}
              <div className="flex flex-col items-center justify-center" role="cell">
                <span className="text-xs text-foreground">{item.provider}</span>
                <span className="text-xs text-muted-foreground font-mono">{item.model}</span>
              </div>

              {/* Elapsed */}
              <div className="flex items-center justify-center" role="cell">
                <span className="text-xs text-muted-foreground font-mono">
                  {formatDuration(item.elapsedSeconds)}
                  {item.videoDurationSeconds ? (
                    <>
                      <br />
                      <span className="text-status-success">{item.videoDurationSeconds}s 视频</span>
                    </>
                  ) : null}
                </span>
              </div>

              {/* Actions */}
              <div className="flex items-center justify-center gap-1" role="cell">
                {item.status === 'succeeded' && (
                  <button
                    type="button"
                    className="flex items-center justify-center h-6 w-6 rounded border border-border bg-secondary text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                    aria-label={`播放 ${item.prompt.slice(0, 20)} 的视频`}
                    title="播放视频"
                  >
                    <Play className="h-3 w-3 fill-current" aria-hidden />
                  </button>
                )}
                {item.status === 'succeeded' && (
                  <button
                    type="button"
                    className="flex items-center justify-center h-6 w-6 rounded border border-border bg-secondary text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                    aria-label="在资源管理器中打开"
                    title="在资源管理器中打开"
                  >
                    <ExternalLink className="h-3 w-3" aria-hidden />
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => handleDelete(item.id)}
                  className={cn(
                    'flex items-center justify-center h-6 w-6 rounded border transition-colors',
                    deleteConfirm === item.id
                      ? 'border-status-error-border bg-status-error-bg text-status-error'
                      : 'border-border bg-secondary text-muted-foreground hover:text-status-error hover:border-status-error-border hover:bg-status-error-bg',
                  )}
                  aria-label={deleteConfirm === item.id ? '确认删除' : '删除记录'}
                  title={deleteConfirm === item.id ? '再次点击确认删除' : '删除'}
                >
                  <Trash2 className="h-3 w-3" aria-hidden />
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
