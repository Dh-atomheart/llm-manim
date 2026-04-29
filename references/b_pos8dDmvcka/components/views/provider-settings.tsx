'use client'

import { useState } from 'react'
import {
  Plus,
  Edit3,
  Trash2,
  Eye,
  EyeOff,
  Wifi,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Loader2,
  ChevronDown,
} from 'lucide-react'
import { cn } from '@/lib/utils'

type ProviderType = 'openai-compatible' | 'anthropic'
type TestStatus = 'idle' | 'testing' | 'ok' | 'failed'

export interface ProviderConfig {
  id: string
  name: string
  type: ProviderType
  baseUrl: string
  model: string
  apiKey: string
}

const INITIAL_PROVIDERS: ProviderConfig[] = [
  {
    id: 'prov1',
    name: 'OpenAI',
    type: 'openai-compatible',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-4o',
    apiKey: 'sk-proj-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
  },
  {
    id: 'prov2',
    name: 'Anthropic',
    type: 'anthropic',
    baseUrl: 'https://api.anthropic.com',
    model: 'claude-3-5-sonnet-20241022',
    apiKey: 'sk-ant-api03-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
  },
]

const EMPTY_FORM: Omit<ProviderConfig, 'id'> = {
  name: '',
  type: 'openai-compatible',
  baseUrl: '',
  model: '',
  apiKey: '',
}

function maskKey(key: string) {
  if (key.length <= 8) return '•'.repeat(key.length)
  return key.slice(0, 6) + '•'.repeat(Math.min(key.length - 8, 20)) + key.slice(-4)
}

export default function ProviderSettings() {
  const [providers, setProviders] = useState<ProviderConfig[]>(INITIAL_PROVIDERS)
  const [editingId, setEditingId] = useState<string | null>(null)
  const [isAdding, setIsAdding] = useState(false)
  const [form, setForm] = useState<Omit<ProviderConfig, 'id'>>(EMPTY_FORM)
  const [showKey, setShowKey] = useState(false)
  const [testStatus, setTestStatus] = useState<Record<string, TestStatus>>({})
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null)

  function startEdit(provider: ProviderConfig) {
    setEditingId(provider.id)
    setForm({ name: provider.name, type: provider.type, baseUrl: provider.baseUrl, model: provider.model, apiKey: provider.apiKey })
    setIsAdding(false)
    setShowKey(false)
  }

  function startAdd() {
    setIsAdding(true)
    setEditingId(null)
    setForm(EMPTY_FORM)
    setShowKey(false)
  }

  function cancelEdit() {
    setEditingId(null)
    setIsAdding(false)
    setForm(EMPTY_FORM)
    setShowKey(false)
  }

  function saveEdit() {
    if (editingId) {
      setProviders((prev) =>
        prev.map((p) => (p.id === editingId ? { ...p, ...form } : p)),
      )
    } else {
      setProviders((prev) => [
        ...prev,
        { id: `prov_${Date.now()}`, ...form },
      ])
    }
    cancelEdit()
  }

  function handleDelete(id: string) {
    if (deleteConfirm === id) {
      setProviders((prev) => prev.filter((p) => p.id !== id))
      setDeleteConfirm(null)
      if (editingId === id) cancelEdit()
    } else {
      setDeleteConfirm(id)
    }
  }

  function handleTest(id: string) {
    setTestStatus((prev) => ({ ...prev, [id]: 'testing' }))
    setTimeout(() => {
      setTestStatus((prev) => ({ ...prev, [id]: Math.random() > 0.3 ? 'ok' : 'failed' }))
    }, 1800)
  }

  const activeForm = isAdding || editingId !== null

  return (
    <div className="flex h-full flex-col overflow-y-auto">
      <div className="max-w-2xl w-full mx-auto px-5 py-5 space-y-6">
        {/* Section header */}
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-sm font-semibold text-foreground">模型 Provider</h2>
            <p className="text-xs text-muted-foreground mt-0.5">
              配置 OpenAI-compatible 或 Anthropic 服务。API Key 明文存储于本地。
            </p>
          </div>
          <button
            type="button"
            onClick={startAdd}
            disabled={activeForm}
            className="flex items-center gap-1.5 h-8 px-3 rounded border border-border bg-secondary text-xs text-foreground hover:bg-accent transition-colors disabled:opacity-40"
            aria-label="添加新 Provider"
          >
            <Plus className="h-3.5 w-3.5" aria-hidden />
            添加
          </button>
        </div>

        {/* API key warning */}
        <div className="flex items-start gap-2.5 rounded border border-status-warning-border bg-status-warning-bg px-3 py-2.5" role="alert">
          <AlertTriangle className="h-3.5 w-3.5 shrink-0 text-status-warning mt-0.5" aria-hidden />
          <p className="text-xs text-status-warning leading-relaxed">
            <span className="font-medium">安全提示：</span>
            API Key 以明文存储于本地配置文件。请勿在共享设备上配置高权限密钥，勿将配置目录同步至公开云服务。
          </p>
        </div>

        {/* Provider list */}
        {providers.length === 0 && !isAdding && (
          <div className="flex items-center justify-center h-24 rounded border border-dashed border-border text-sm text-muted-foreground">
            暂无 Provider 配置。点击"添加"新增。
          </div>
        )}

        <div className="space-y-0 rounded border border-border overflow-hidden divide-y divide-border">
          {providers.map((p) => {
            const ts = testStatus[p.id] || 'idle'
            const isEditing = editingId === p.id
            return (
              <div key={p.id}>
                {/* Row */}
                <div className={cn('px-3 py-3', isEditing && 'bg-accent')}>
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0 space-y-0.5">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-foreground">{p.name}</span>
                        <span className="text-xs text-muted-foreground border border-border rounded px-1.5 py-0.5 font-mono">
                          {p.type === 'openai-compatible' ? 'OpenAI-compatible' : 'Anthropic'}
                        </span>
                      </div>
                      <p className="text-xs text-muted-foreground font-mono truncate">{p.baseUrl}</p>
                      <p className="text-xs text-muted-foreground font-mono">{p.model}</p>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      {/* Test result */}
                      {ts === 'testing' && (
                        <span className="flex items-center gap-1 text-xs text-muted-foreground">
                          <Loader2 className="h-3 w-3 animate-spin" aria-hidden />
                          测试中
                        </span>
                      )}
                      {ts === 'ok' && (
                        <span className="flex items-center gap-1 text-xs text-status-success" role="status">
                          <CheckCircle2 className="h-3 w-3" aria-hidden />
                          连接正常
                        </span>
                      )}
                      {ts === 'failed' && (
                        <span className="flex items-center gap-1 text-xs text-status-error" role="status">
                          <XCircle className="h-3 w-3" aria-hidden />
                          连接失败
                        </span>
                      )}
                      <button
                        type="button"
                        onClick={() => handleTest(p.id)}
                        disabled={ts === 'testing' || activeForm}
                        className="flex items-center gap-1 h-6 px-2 rounded border border-border bg-secondary text-xs text-muted-foreground hover:text-foreground hover:bg-accent transition-colors disabled:opacity-40"
                        aria-label={`测试 ${p.name} 连接`}
                      >
                        <Wifi className="h-3 w-3" aria-hidden />
                        测试
                      </button>
                      <button
                        type="button"
                        onClick={() => startEdit(p)}
                        disabled={activeForm && !isEditing}
                        className="flex items-center justify-center h-6 w-6 rounded border border-border bg-secondary text-muted-foreground hover:text-foreground hover:bg-accent transition-colors disabled:opacity-40"
                        aria-label={`编辑 ${p.name}`}
                      >
                        <Edit3 className="h-3 w-3" aria-hidden />
                      </button>
                      <button
                        type="button"
                        onClick={() => handleDelete(p.id)}
                        className={cn(
                          'flex items-center justify-center h-6 w-6 rounded border transition-colors',
                          deleteConfirm === p.id
                            ? 'border-status-error-border bg-status-error-bg text-status-error'
                            : 'border-border bg-secondary text-muted-foreground hover:text-status-error hover:border-status-error-border hover:bg-status-error-bg',
                        )}
                        aria-label={deleteConfirm === p.id ? `确认删除 ${p.name}` : `删除 ${p.name}`}
                        title={deleteConfirm === p.id ? '再次点击确认' : '删除'}
                      >
                        <Trash2 className="h-3 w-3" aria-hidden />
                      </button>
                    </div>
                  </div>
                </div>

                {/* Inline edit form */}
                {isEditing && (
                  <ProviderForm
                    form={form}
                    onChange={setForm}
                    showKey={showKey}
                    onToggleKey={() => setShowKey((v) => !v)}
                    onSave={saveEdit}
                    onCancel={cancelEdit}
                    saveLabel="保存更改"
                  />
                )}
              </div>
            )
          })}
        </div>

        {/* Add form */}
        {isAdding && (
          <div className="rounded border border-border overflow-hidden">
            <div className="px-3 py-2 border-b border-border bg-accent">
              <span className="text-xs font-medium text-foreground">新增 Provider</span>
            </div>
            <ProviderForm
              form={form}
              onChange={setForm}
              showKey={showKey}
              onToggleKey={() => setShowKey((v) => !v)}
              onSave={saveEdit}
              onCancel={cancelEdit}
              saveLabel="添加"
            />
          </div>
        )}
      </div>
    </div>
  )
}

interface ProviderFormProps {
  form: Omit<ProviderConfig, 'id'>
  onChange: (f: Omit<ProviderConfig, 'id'>) => void
  showKey: boolean
  onToggleKey: () => void
  onSave: () => void
  onCancel: () => void
  saveLabel: string
}

function ProviderForm({ form, onChange, showKey, onToggleKey, onSave, onCancel, saveLabel }: ProviderFormProps) {
  const isValid = form.name.trim() && form.baseUrl.trim() && form.model.trim() && form.apiKey.trim()

  function set(key: keyof Omit<ProviderConfig, 'id'>, value: string) {
    onChange({ ...form, [key]: value })
  }

  return (
    <div className="px-3 py-3 bg-card space-y-3">
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">名称 *</label>
          <input
            type="text"
            value={form.name}
            onChange={(e) => set('name', e.target.value)}
            placeholder="My OpenAI"
            className="w-full h-7 rounded border border-border bg-input px-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">类型 *</label>
          <div className="relative">
            <select
              value={form.type}
              onChange={(e) => set('type', e.target.value as ProviderType)}
              className="w-full h-7 rounded border border-border bg-input pl-2 pr-6 text-xs text-foreground appearance-none focus:outline-none focus:ring-1 focus:ring-ring"
            >
              <option value="openai-compatible">OpenAI-compatible</option>
              <option value="anthropic">Anthropic</option>
            </select>
            <ChevronDown className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" aria-hidden />
          </div>
        </div>
      </div>

      <div className="space-y-1">
        <label className="text-xs text-muted-foreground">Base URL *</label>
        <input
          type="url"
          value={form.baseUrl}
          onChange={(e) => set('baseUrl', e.target.value)}
          placeholder="https://api.openai.com/v1"
          className="w-full h-7 rounded border border-border bg-input px-2 text-xs text-foreground font-mono placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
        />
      </div>

      <div className="space-y-1">
        <label className="text-xs text-muted-foreground">模型 ID *</label>
        <input
          type="text"
          value={form.model}
          onChange={(e) => set('model', e.target.value)}
          placeholder="gpt-4o"
          className="w-full h-7 rounded border border-border bg-input px-2 text-xs text-foreground font-mono placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
        />
      </div>

      <div className="space-y-1">
        <label className="text-xs text-muted-foreground">API Key *</label>
        <div className="relative">
          <input
            type={showKey ? 'text' : 'password'}
            value={form.apiKey}
            onChange={(e) => set('apiKey', e.target.value)}
            placeholder="sk-..."
            className="w-full h-7 rounded border border-border bg-input pl-2 pr-8 text-xs text-foreground font-mono placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
            aria-label="API Key"
          />
          <button
            type="button"
            onClick={onToggleKey}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
            aria-label={showKey ? '隐藏 API Key' : '显示 API Key'}
          >
            {showKey ? <EyeOff className="h-3 w-3" aria-hidden /> : <Eye className="h-3 w-3" aria-hidden />}
          </button>
        </div>
      </div>

      <div className="flex items-center justify-end gap-2 pt-1">
        <button
          type="button"
          onClick={onCancel}
          className="h-7 px-3 rounded border border-border bg-secondary text-xs text-foreground hover:bg-accent transition-colors"
        >
          取消
        </button>
        <button
          type="button"
          onClick={onSave}
          disabled={!isValid}
          className="h-7 px-3 rounded bg-primary text-xs font-medium text-primary-foreground hover:opacity-90 transition-opacity disabled:opacity-30"
        >
          {saveLabel}
        </button>
      </div>
    </div>
  )
}
