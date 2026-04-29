import { useEffect, useState } from "react";
import { createProject, deleteProject, listProjects } from "./commands/project";
import { getWorkspaceStatus } from "./commands/workspace";
import type { Project, WorkspaceStatus } from "./commands/types";
import styles from "./App.module.css";
import { useProjectStore } from "./store/project";
import { useWorkspaceStore } from "./store/workspace";
import FirstLaunch from "./views/FirstLaunch";
import ProviderSettings from "./views/ProviderSettings";

type MainView =
  | "workbench"
  | "history"
  | "provider-settings"
  | "basic-settings";

const VIEW_LABELS: Record<MainView, string> = {
  workbench: "工作台",
  history: "历史记录",
  "provider-settings": "Provider 设置",
  "basic-settings": "基础设置",
};

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function runtimeLabel(status: WorkspaceStatus["runtimeStatus"]): string {
  switch (status) {
    case "ready":
      return "已就绪";
    case "broken":
      return "部分缺失";
    case "missing":
      return "未检测到";
    default:
      return "未知";
  }
}

function runtimeTone(status: WorkspaceStatus["runtimeStatus"]): string {
  switch (status) {
    case "ready":
      return styles.success;
    case "broken":
      return styles.warning;
    case "missing":
      return styles.error;
    default:
      return styles.neutral;
  }
}

function boolTone(value: boolean): string {
  return value ? styles.success : styles.error;
}

function boolLabel(value: boolean, positive: string, negative: string): string {
  return value ? positive : negative;
}

function App() {
  const workspaceStatus = useWorkspaceStore((state) => state.status);
  const setWorkspaceStatus = useWorkspaceStore((state) => state.setStatus);
  const clearWorkspaceStatus = useWorkspaceStore((state) => state.clear);

  const projects = useProjectStore((state) => state.projects);
  const selectedProjectId = useProjectStore((state) => state.selectedProjectId);
  const setProjects = useProjectStore((state) => state.setProjects);
  const selectProject = useProjectStore((state) => state.selectProject);
  const clearProjects = useProjectStore((state) => state.clear);

  const [view, setView] = useState<MainView>("workbench");
  const [isBootstrapping, setIsBootstrapping] = useState(true);
  const [isRefreshingProjects, setIsRefreshingProjects] = useState(false);
  const [isCreateInputVisible, setIsCreateInputVisible] = useState(false);
  const [newProjectName, setNewProjectName] = useState("");
  const [isSubmittingProject, setIsSubmittingProject] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Project | null>(null);
  const [isDeletingProject, setIsDeletingProject] = useState(false);
  const [appError, setAppError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);

  const selectedProject =
    projects.find((project) => project.id === selectedProjectId) ?? null;

  async function reloadProjects(nextSelectedProjectId?: string | null) {
    setIsRefreshingProjects(true);

    try {
      const response = await listProjects();
      if (!response.ok) {
        setAppError(response.error.message);
        return;
      }

      setProjects(response.data);
      if (nextSelectedProjectId !== undefined) {
        const resolvedProjectId = response.data.some(
          (project) => project.id === nextSelectedProjectId,
        )
          ? nextSelectedProjectId
          : (response.data[0]?.id ?? null);
        selectProject(resolvedProjectId);
      }
    } catch (error) {
      setAppError(`无法读取项目列表：${toMessage(error)}`);
    } finally {
      setIsRefreshingProjects(false);
    }
  }

  async function bootstrap() {
    setIsBootstrapping(true);
    setAppError(null);

    try {
      const response = await getWorkspaceStatus();
      if (!response.ok) {
        clearWorkspaceStatus();
        clearProjects();
        setAppError(response.error.message);
        return;
      }

      setWorkspaceStatus(response.data);
      if (response.data.configured) {
        await reloadProjects();
      } else {
        clearProjects();
      }
    } catch (error) {
      clearWorkspaceStatus();
      clearProjects();
      setAppError(`无法获取工作区状态：${toMessage(error)}`);
    } finally {
      setIsBootstrapping(false);
    }
  }

  useEffect(() => {
    void bootstrap();
  }, []);

  async function handleCreateProject() {
    const projectName = newProjectName.trim();
    if (!projectName) {
      setAppError("请输入项目名称");
      return;
    }

    setIsSubmittingProject(true);
    setAppError(null);

    try {
      const response = await createProject(projectName);
      if (!response.ok) {
        setAppError(response.error.message);
        return;
      }

      await reloadProjects(response.data.id);
      setView("workbench");
      setActionMessage(`已创建项目「${response.data.name}」`);
      setNewProjectName("");
      setIsCreateInputVisible(false);
    } catch (error) {
      setAppError(`创建项目失败：${toMessage(error)}`);
    } finally {
      setIsSubmittingProject(false);
    }
  }

  async function handleDeleteProject() {
    if (!deleteTarget) {
      return;
    }

    setIsDeletingProject(true);
    setAppError(null);

    try {
      const response = await deleteProject(deleteTarget.id);
      if (!response.ok) {
        setAppError(response.error.message);
        return;
      }

      const deletedName = deleteTarget.name;
      setDeleteTarget(null);
      await reloadProjects();
      setActionMessage(`已删除项目「${deletedName}」`);
    } catch (error) {
      setAppError(`删除项目失败：${toMessage(error)}`);
    } finally {
      setIsDeletingProject(false);
    }
  }

  function renderWorkbenchView() {
    if (!workspaceStatus) {
      return null;
    }

    if (!selectedProject) {
      return (
        <div className={styles.emptyState}>
          <p>请在左侧选择或新建项目以继续。</p>
          <button
            type="button"
            className={styles.actionButton}
            onClick={() => {
              setIsCreateInputVisible(true);
              setNewProjectName("");
              setAppError(null);
            }}
          >
            新建第一个项目
          </button>
        </div>
      );
    }

    return (
      <div className={styles.workbenchPanel}>
        <dl className={styles.definitionList}>
          <div>
            <dt>工作区</dt>
            <dd className={styles.pathValue}>{workspaceStatus.workspacePath ?? "未配置"}</dd>
          </div>
          <div>
            <dt>可写</dt>
            <dd>
              <span className={`${styles.statusBadge} ${boolTone(workspaceStatus.writable)}`}>
                {boolLabel(workspaceStatus.writable, "可写", "不可写")}
              </span>
            </dd>
          </div>
          <div>
            <dt>数据库</dt>
            <dd>
              <span
                className={`${styles.statusBadge} ${boolTone(workspaceStatus.databaseReady)}`}
              >
                {boolLabel(workspaceStatus.databaseReady, "已初始化", "不可用")}
              </span>
            </dd>
          </div>
          <div>
            <dt>Runtime</dt>
            <dd>
              <span
                className={`${styles.statusBadge} ${runtimeTone(workspaceStatus.runtimeStatus)}`}
              >
                {runtimeLabel(workspaceStatus.runtimeStatus)}
              </span>
            </dd>
          </div>
        </dl>
        <p className={styles.hint}>
          工作台将在 M4 接入；当前 M2 已完成工作区初始化与项目管理。
        </p>
      </div>
    );
  }

  function renderPlaceholderView(label: string, description: string) {
    return (
      <div className={styles.placeholderView}>
        <h2>{label}</h2>
        <p>{description}</p>
      </div>
    );
  }

  if (isBootstrapping) {
    return (
      <div className={styles.loadingScreen}>
        <h1 className={styles.loadingTitle}>LLM-Manim</h1>
        <p className={styles.loadingHint}>正在检查工作区与 SQLite 状态…</p>
      </div>
    );
  }

  if (!workspaceStatus && appError) {
    return (
      <div className={styles.loadingScreen}>
        <h1 className={styles.loadingTitle}>启动失败</h1>
        <p className={styles.loadingHint}>{appError}</p>
        <button type="button" className={styles.actionButton} onClick={() => void bootstrap()}>
          重试
        </button>
      </div>
    );
  }

  if (!workspaceStatus || !workspaceStatus.configured) {
    return (
      <FirstLaunch
        initialWorkspacePath={workspaceStatus?.workspacePath ?? null}
        onComplete={() => bootstrap()}
      />
    );
  }

  const topbarLabel =
    view === "workbench"
      ? selectedProject?.name ?? VIEW_LABELS[view]
      : VIEW_LABELS[view];

  return (
    <div className={styles.layout}>
      <header className={styles.topbar}>
        <div className={styles.topbarLeft}>
          <span className={styles.logo} aria-hidden="true">
            M
          </span>
          <span className={styles.appName}>LLM-Manim</span>
          <span className={styles.topbarSep} aria-hidden="true">
            /
          </span>
          <span className={styles.topbarPage}>{topbarLabel}</span>
        </div>

        <div className={styles.topbarRight}>
          <button
            type="button"
            className={`${styles.providerBtn} ${
              view === "provider-settings" ? styles.providerBtnActive : ""
            }`}
            onClick={() => setView("provider-settings")}
          >
            <svg
              className={styles.providerBtnIcon}
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              aria-hidden="true"
            >
              <rect x="1" y="3" width="14" height="10" rx="1" />
              <path d="M1 6h14" />
            </svg>
            <span>Provider</span>
            <span className={styles.topbarSep} aria-hidden="true">
              /
            </span>
            <span className={styles.providerModel}>设置</span>
          </button>

          <button
            type="button"
            className={`${styles.topbarIconBtn} ${
              view === "history" ? styles.topbarIconBtnActive : ""
            }`}
            onClick={() => setView("history")}
            aria-label="历史记录"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              aria-hidden="true"
            >
              <circle cx="8" cy="8" r="6.5" />
              <path d="M8 4.5V8l2.5 1.5" />
            </svg>
          </button>

          <button
            type="button"
            className={`${styles.topbarIconBtn} ${
              view === "basic-settings" ? styles.topbarIconBtnActive : ""
            }`}
            onClick={() => setView("basic-settings")}
            aria-label="基础设置"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              aria-hidden="true"
            >
              <circle cx="8" cy="8" r="2.5" />
              <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.2 3.2l1.4 1.4M11.4 11.4l1.4 1.4M3.2 12.8l1.4-1.4M11.4 4.6l1.4-1.4" />
            </svg>
          </button>
        </div>
      </header>

      <div className={styles.body}>
        <aside className={styles.sidebar} aria-label="项目列表">
          <div className={styles.sidebarTopSection}>
            {!isCreateInputVisible ? (
              <button
                type="button"
                className={styles.newProjectBtn}
                onClick={() => {
                  setIsCreateInputVisible(true);
                  setNewProjectName("");
                  setAppError(null);
                }}
              >
                <svg
                  className={styles.sidebarIcon}
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  aria-hidden="true"
                >
                  <path d="M8 3v10M3 8h10" />
                </svg>
                新建项目
              </button>
            ) : (
              <div className={styles.newProjectComposer}>
                <input
                  type="text"
                  value={newProjectName}
                  className={styles.sidebarInput}
                  placeholder="输入项目名称"
                  autoFocus
                  onChange={(event) => setNewProjectName(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      void handleCreateProject();
                    }
                    if (event.key === "Escape") {
                      setIsCreateInputVisible(false);
                      setNewProjectName("");
                    }
                  }}
                />
                <div className={styles.sidebarInputActions}>
                  <button
                    type="button"
                    className={styles.sidebarInputButton}
                    onClick={() => void handleCreateProject()}
                    disabled={isSubmittingProject}
                  >
                    {isSubmittingProject ? "保存中" : "保存"}
                  </button>
                  <button
                    type="button"
                    className={styles.sidebarInputButton}
                    onClick={() => {
                      setIsCreateInputVisible(false);
                      setNewProjectName("");
                    }}
                    disabled={isSubmittingProject}
                  >
                    取消
                  </button>
                </div>
              </div>
            )}
          </div>

          <nav className={styles.sidebarNavSection} aria-label="导航">
            <button
              type="button"
              className={`${styles.sidebarNavItem} ${
                view === "history" ? styles.sidebarNavItemActive : ""
              }`}
              onClick={() => setView("history")}
            >
              <svg
                className={styles.sidebarIcon}
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                aria-hidden="true"
              >
                <rect x="2" y="4" width="12" height="9" rx="1" />
                <path d="M5 4V3a1 1 0 011-1h4a1 1 0 011 1v1" />
                <circle cx="8" cy="9" r="1.5" />
              </svg>
              全部视频
            </button>
          </nav>

          <div className={styles.sidebarProjects}>
            <span className={styles.sidebarGroupLabel}>项目</span>
            {projects.length === 0 ? (
              <p className={styles.sidebarEmpty}>（暂无项目）</p>
            ) : (
              projects.map((project) => {
                const isSelected = project.id === selectedProjectId;

                return (
                  <div
                    key={project.id}
                    className={`${styles.sidebarProjectItem} ${
                      isSelected ? styles.sidebarProjectItemActive : ""
                    }`}
                  >
                    <button
                      type="button"
                      className={styles.sidebarProjectMain}
                      onClick={() => {
                        selectProject(project.id);
                        setView("workbench");
                      }}
                    >
                      <svg
                        className={styles.sidebarIcon}
                        viewBox="0 0 16 16"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        aria-hidden="true"
                      >
                        <path d="M2.5 4.5h4l1.5 1.5h5.5v5.5a1 1 0 01-1 1h-10a1 1 0 01-1-1v-6a1 1 0 011-1z" />
                      </svg>
                      <span className={styles.sidebarProjectName}>{project.name}</span>
                    </button>
                    <button
                      type="button"
                      className={styles.sidebarItemTrash}
                      onClick={() => setDeleteTarget(project)}
                      aria-label={`删除项目 ${project.name}`}
                    >
                      <svg
                        width="14"
                        height="14"
                        viewBox="0 0 16 16"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        aria-hidden="true"
                      >
                        <path d="M3.5 4.5h9" />
                        <path d="M6 4.5V3.25h4V4.5" />
                        <path d="M5 6.5v5" />
                        <path d="M8 6.5v5" />
                        <path d="M11 6.5v5" />
                        <path d="M4.5 4.5l.5 8.5h6l.5-8.5" />
                      </svg>
                    </button>
                  </div>
                );
              })
            )}
          </div>

          <div className={styles.sidebarFooter}>
            <button
              type="button"
              className={`${styles.sidebarNavItem} ${
                view === "provider-settings" ? styles.sidebarNavItemActive : ""
              }`}
              onClick={() => setView("provider-settings")}
            >
              <svg
                className={styles.sidebarIcon}
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                aria-hidden="true"
              >
                <rect x="1" y="3" width="14" height="10" rx="1" />
                <path d="M1 6h14" />
              </svg>
              Provider 设置
            </button>
          </div>
        </aside>

        <main className={styles.main}>
          <div className={styles.mainTitleBar}>
            <div className={styles.mainTitleBlock}>
              <span className={styles.mainTitle}>{VIEW_LABELS[view]}</span>
              <span className={styles.mainSubtitle}>
                {workspaceStatus.workspacePath ?? "未配置工作区"}
              </span>
            </div>
            <div className={styles.mainTitleActions}>
              <button
                type="button"
                className={styles.textButton}
                onClick={() => void bootstrap()}
                disabled={isRefreshingProjects}
              >
                刷新状态
              </button>
            </div>
          </div>

          <div className={styles.mainInner}>
            {appError ? <div className={styles.errorBanner}>{appError}</div> : null}
            {actionMessage ? <div className={styles.infoBanner}>{actionMessage}</div> : null}

            {view === "workbench"
              ? renderWorkbenchView()
              : view === "history"
                ? renderPlaceholderView(
                    "历史记录",
                    "M2 先完成工作区与项目管理，历史任务视图会在后续里程碑接入。",
                  )
                : view === "provider-settings"
                  ? <ProviderSettings />
                  : renderPlaceholderView(
                      "基础设置",
                      "Runtime 修复与基础设置视图将在后续里程碑继续实现。",
                    )}
          </div>
        </main>
      </div>

      {deleteTarget ? (
        <div className={styles.modalOverlay} role="presentation">
          <div className={styles.modalBox} role="dialog" aria-modal="true">
            <h2 className={styles.modalTitle}>删除项目「{deleteTarget.name}」？</h2>
            <p className={styles.modalMessage}>
              该操作会移除 SQLite 中的项目记录，并删除工作区下对应的项目目录。此操作不可撤销。
            </p>
            <div className={styles.modalActions}>
              <button
                type="button"
                className={styles.secondaryButton}
                onClick={() => setDeleteTarget(null)}
                disabled={isDeletingProject}
              >
                取消
              </button>
              <button
                type="button"
                className={styles.dangerButton}
                onClick={() => void handleDeleteProject()}
                disabled={isDeletingProject}
              >
                {isDeletingProject ? "删除中" : "确认删除"}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

export default App;
