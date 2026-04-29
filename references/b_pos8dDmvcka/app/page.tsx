'use client'

import { useState } from 'react'
import {
  Plus,
  Clock,
  Settings,
  Server,
  ChevronDown,
  Film,
  FolderOpen,
  MoreHorizontal,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import FirstLaunch from '@/components/views/first-launch'
import Workbench from '@/components/views/workbench'
import HistoryView from '@/components/views/history'
import ProviderSettings from '@/components/views/provider-settings'
import BasicSettings from '@/components/views/basic-settings'

type View = 'workbench' | 'history' | 'provider-settings' | 'basic-settings'

interface Project {
  id: string
  name: string
  lastModified: string
}

const MOCK_PROJECTS: Project[] = [
  { id: 'p1', name: 'Fourier Series', lastModified: '2025-04-28' },
  { id: 'p2', name: 'Circle Animation', lastModified: '2025-04-27' },
  { id: 'p3', name: 'Mandelbrot Set', lastModified: '2025-04-25' },
  { id: 'p4', name: 'Graph Theory', lastModified: '2025-04-20' },
]

const MOCK_PROVIDERS = [
  { id: 'prov1', name: 'OpenAI', model: 'gpt-4o' },
  { id: 'prov2', name: 'Anthropic', model: 'claude-3-5-sonnet' },
]

export default function App() {
  const [isFirstLaunch, setIsFirstLaunch] = useState(true)
  const [view, setView] = useState<View>('workbench')
  const [projects, setProjects] = useState<Project[]>(MOCK_PROJECTS)
  const [selectedProjectId, setSelectedProjectId] = useState('p1')
  const [selectedProviderId, setSelectedProviderId] = useState('prov1')
  const [providerMenuOpen, setProviderMenuOpen] = useState(false)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [renameValue, setRenameValue] = useState('')

  if (isFirstLaunch) {
    return <FirstLaunch onComplete={() => setIsFirstLaunch(false)} />
  }

  const selectedProject = projects.find((p) => p.id === selectedProjectId) ?? projects[0]
  const selectedProvider = MOCK_PROVIDERS.find((p) => p.id === selectedProviderId) ?? MOCK_PROVIDERS[0]

  function addProject() {
    const newId = `p_${Date.now()}`
    const newProject: Project = {
      id: newId,
      name: `新项目 ${projects.length + 1}`,
      lastModified: new Date().toISOString().slice(0, 10),
    }
    setProjects((prev) => [newProject, ...prev])
    setSelectedProjectId(newId)
    setView('workbench')
  }

  function startRename(id: string, currentName: string) {
    setRenaming(id)
    setRenameValue(currentName)
  }

  function commitRename(id: string) {
    if (renameValue.trim()) {
      setProjects((prev) =>
        prev.map((p) => (p.id === id ? { ...p, name: renameValue.trim() } : p)),
      )
    }
    setRenaming(null)
  }

  const mainViewLabel: Record<View, string> = {
    workbench: selectedProject?.name ?? '工作台',
    history: '历史记录',
    'provider-settings': 'Provider 设置',
    'basic-settings': '基础设置',
  }

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
      {/* ── Top bar ─────────────────────────────────────────── */}
      <header
        className="flex h-10 shrink-0 items-center border-b border-topbar-border bg-topbar px-3 gap-3"
        role="banner"
      >
        {/* App identity */}
        <div className="flex items-center gap-2 shrink-0">
          <span
            className="flex h-5 w-5 items-center justify-center rounded bg-foreground"
            aria-hidden
          >
            <span className="text-[10px] font-bold text-primary-foreground font-mono leading-none">M</span>
          </span>
          <span className="text-xs font-semibold text-foreground tracking-tight">LLM-Manim</span>
        </div>

        {/* Separator + project name */}
        <span className="text-border text-sm" aria-hidden>/</span>
        {view === 'workbench' ? (
          renaming === selectedProject?.id ? (
            <input
              autoFocus
              type="text"
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onBlur={() => commitRename(selectedProject.id)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') commitRename(selectedProject.id)
                if (e.key === 'Escape') setRenaming(null)
              }}
              className="h-6 w-40 rounded border border-ring bg-input px-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-ring font-medium"
              aria-label="重命名项目"
            />
          ) : (
            <button
              type="button"
              onDoubleClick={() => startRename(selectedProject.id, selectedProject.name)}
              className="text-xs font-medium text-foreground hover:text-muted-foreground transition-colors"
              title="双击重命名"
              aria-label={`当前项目: ${selectedProject.name}，双击重命名`}
            >
              {selectedProject.name}
            </button>
          )
        ) : (
          <span className="text-xs font-medium text-muted-foreground">
            {mainViewLabel[view]}
          </span>
        )}

        <div className="flex-1" />

        {/* Provider selector */}
        <div className="relative">
          <button
            type="button"
            onClick={() => setProviderMenuOpen((v) => !v)}
            className="flex items-center gap-1.5 h-7 px-2.5 rounded border border-border bg-secondary text-xs text-foreground hover:bg-accent transition-colors"
            aria-haspopup="listbox"
            aria-expanded={providerMenuOpen}
            aria-label={`当前 Provider: ${selectedProvider.name} / ${selectedProvider.model}`}
          >
            <Server className="h-3 w-3 text-muted-foreground" aria-hidden />
            <span>{selectedProvider.name}</span>
            <span className="text-muted-foreground">/</span>
            <span className="font-mono">{selectedProvider.model}</span>
            <ChevronDown className="h-3 w-3 text-muted-foreground" aria-hidden />
          </button>
          {providerMenuOpen && (
            <>
              <div
                className="fixed inset-0 z-10"
                onClick={() => setProviderMenuOpen(false)}
                aria-hidden
              />
              <div
                className="absolute right-0 top-full mt-1 z-20 min-w-[200px] rounded border border-border bg-popover shadow-lg overflow-hidden"
                role="listbox"
                aria-label="选择 Provider"
              >
                {MOCK_PROVIDERS.map((p) => (
                  <button
                    key={p.id}
                    type="button"
                    role="option"
                    aria-selected={p.id === selectedProviderId}
                    onClick={() => {
                      setSelectedProviderId(p.id)
                      setProviderMenuOpen(false)
                    }}
                    className={cn(
                      'flex w-full items-center justify-between px-3 py-2 text-xs hover:bg-accent transition-colors',
                      p.id === selectedProviderId
                        ? 'text-foreground'
                        : 'text-muted-foreground',
                    )}
                  >
                    <span>{p.name}</span>
                    <span className="font-mono text-muted-foreground">{p.model}</span>
                  </button>
                ))}
                <div className="border-t border-border">
                  <button
                    type="button"
                    onClick={() => {
                      setView('provider-settings')
                      setProviderMenuOpen(false)
                    }}
                    className="flex w-full items-center gap-2 px-3 py-2 text-xs text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                  >
                    <Settings className="h-3 w-3" aria-hidden />
                    管理 Provider...
                  </button>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Nav icons */}
        <div className="flex items-center gap-0.5">
          <TopBarIconButton
            label="历史记录"
            active={view === 'history'}
            onClick={() => setView('history')}
          >
            <Clock className="h-3.5 w-3.5" />
          </TopBarIconButton>
          <TopBarIconButton
            label="基础设置"
            active={view === 'basic-settings'}
            onClick={() => setView('basic-settings')}
          >
            <Settings className="h-3.5 w-3.5" />
          </TopBarIconButton>
        </div>
      </header>

      {/* ── Body ────────────────────────────────────────────── */}
      <div className="flex flex-1 min-h-0">
        {/* Sidebar */}
        <aside
          className="flex w-52 shrink-0 flex-col border-r border-sidebar-border bg-sidebar overflow-hidden"
          aria-label="项目列表"
        >
          {/* New project */}
          <div className="px-3 py-2.5 border-b border-sidebar-border shrink-0">
            <button
              type="button"
              onClick={addProject}
              className="flex w-full items-center gap-2 h-7 px-2 rounded border border-border text-xs text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
              aria-label="新建项目"
            >
              <Plus className="h-3 w-3" aria-hidden />
              新建项目
            </button>
          </div>

          {/* Sidebar nav items */}
          <nav className="px-1.5 py-1.5 space-y-0.5 border-b border-sidebar-border shrink-0">
            <SidebarNavItem
              label="全部视频"
              icon={<Film className="h-3.5 w-3.5" aria-hidden />}
              active={view === 'history'}
              onClick={() => setView('history')}
            />
          </nav>

          {/* Project list */}
          <div className="flex-1 min-h-0 overflow-y-auto">
            <div className="px-3 pt-3 pb-1">
              <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                项目
              </span>
            </div>
            <ul className="px-1.5 pb-2 space-y-0.5" role="listbox" aria-label="项目列表">
              {projects.map((project) => {
                const isSelected = project.id === selectedProjectId && view === 'workbench'
                return (
                  <li key={project.id} role="option" aria-selected={isSelected}>
                    <button
                      type="button"
                      onClick={() => {
                        setSelectedProjectId(project.id)
                        setView('workbench')
                      }}
                      className={cn(
                        'flex w-full items-center gap-2 rounded px-2 py-1.5 text-xs transition-colors text-left group',
                        isSelected
                          ? 'bg-accent text-foreground'
                          : 'text-muted-foreground hover:text-foreground hover:bg-accent',
                      )}
                      aria-label={`打开项目: ${project.name}`}
                    >
                      <FolderOpen className="h-3 w-3 shrink-0" aria-hidden />
                      <span className="flex-1 truncate">{project.name}</span>
                    </button>
                  </li>
                )
              })}
            </ul>
          </div>

          {/* Provider settings shortcut */}
          <div className="border-t border-sidebar-border px-1.5 py-1.5 shrink-0">
            <SidebarNavItem
              label="Provider 设置"
              icon={<Server className="h-3.5 w-3.5" aria-hidden />}
              active={view === 'provider-settings'}
              onClick={() => setView('provider-settings')}
            />
          </div>
        </aside>

        {/* Main content */}
        <main className="flex flex-1 min-w-0 flex-col" role="main">
          {/* Content header */}
          <div className="flex items-center justify-between border-b border-border px-5 py-2.5 shrink-0">
            <div className="flex items-center gap-3">
              <h1 className="text-sm font-semibold text-foreground">
                {mainViewLabel[view]}
              </h1>
              {view === 'workbench' && (
                <span className="text-xs text-muted-foreground">
                  {selectedProvider.name} / {selectedProvider.model}
                </span>
              )}
            </div>
            {view === 'workbench' && (
              <button
                type="button"
                onClick={() => startRename(selectedProject.id, selectedProject.name)}
                className="flex items-center justify-center h-6 w-6 rounded border border-border bg-secondary text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                aria-label="重命名项目"
                title="重命名"
              >
                <MoreHorizontal className="h-3.5 w-3.5" aria-hidden />
              </button>
            )}
          </div>

          {/* View content */}
          <div className="flex-1 min-h-0">
            {view === 'workbench' && (
              <Workbench
                key={selectedProjectId}
                projectName={selectedProject.name}
                provider={selectedProvider.name}
                model={selectedProvider.model}
              />
            )}
            {view === 'history' && <HistoryView />}
            {view === 'provider-settings' && <ProviderSettings />}
            {view === 'basic-settings' && <BasicSettings />}
          </div>
        </main>
      </div>
    </div>
  )
}

/* ── Small helpers ─────────────────────────────────────────── */

function TopBarIconButton({
  label,
  active,
  onClick,
  children,
}: {
  label: string
  active: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'flex items-center justify-center h-7 w-7 rounded transition-colors',
        active
          ? 'bg-accent text-foreground'
          : 'text-muted-foreground hover:text-foreground hover:bg-accent',
      )}
      aria-label={label}
      aria-pressed={active}
      title={label}
    >
      {children}
    </button>
  )
}

function SidebarNavItem({
  label,
  icon,
  active,
  onClick,
}: {
  label: string
  icon: React.ReactNode
  active: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'flex w-full items-center gap-2 rounded px-2 py-1.5 text-xs transition-colors',
        active
          ? 'bg-accent text-foreground'
          : 'text-muted-foreground hover:text-foreground hover:bg-accent',
      )}
      aria-current={active ? 'page' : undefined}
    >
      {icon}
      {label}
    </button>
  )
}
