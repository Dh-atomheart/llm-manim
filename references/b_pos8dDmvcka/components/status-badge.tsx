'use client'

import { Loader2 } from 'lucide-react'
import { cn } from '@/lib/utils'

export type JobStatus = 'idle' | 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled'

const STATUS_CONFIG: Record<
  Exclude<JobStatus, 'idle'>,
  { label: string; className: string; dotClass: string; spin?: boolean }
> = {
  queued: {
    label: '排队中',
    className:
      'text-status-queued bg-status-queued-bg border border-status-queued-border',
    dotClass: 'bg-status-queued',
  },
  running: {
    label: '生成中',
    className:
      'text-status-running bg-status-running-bg border border-status-running-border',
    dotClass: 'bg-status-running',
    spin: true,
  },
  succeeded: {
    label: '已完成',
    className:
      'text-status-success bg-status-success-bg border border-status-success-border',
    dotClass: 'bg-status-success',
  },
  failed: {
    label: '失败',
    className:
      'text-status-error bg-status-error-bg border border-status-error-border',
    dotClass: 'bg-status-error',
  },
  cancelled: {
    label: '已取消',
    className:
      'text-status-cancelled bg-status-cancelled-bg border border-status-cancelled-border',
    dotClass: 'bg-status-cancelled',
  },
}

interface StatusBadgeProps {
  status: Exclude<JobStatus, 'idle'>
  className?: string
  size?: 'sm' | 'md'
}

export default function StatusBadge({ status, className, size = 'md' }: StatusBadgeProps) {
  const config = STATUS_CONFIG[status]
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded font-mono',
        size === 'sm' ? 'px-1.5 py-0.5 text-[10px]' : 'px-2 py-0.5 text-xs',
        config.className,
        className,
      )}
      aria-label={`状态: ${config.label}`}
    >
      {status === 'running' ? (
        <Loader2 className="h-3 w-3 animate-spin shrink-0" aria-hidden />
      ) : (
        <span className={cn('h-1.5 w-1.5 rounded-full shrink-0', config.dotClass)} aria-hidden />
      )}
      {config.label}
    </span>
  )
}
