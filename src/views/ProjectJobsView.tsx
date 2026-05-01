import { useEffect, useEffectEvent, useMemo, useState } from "react";

import {
  cancelJob,
  deleteJob,
  getJob,
  getJobLogs,
  getRenderArtifact,
  getVideoFileUrl,
  listProjectJobs,
  openRenderArtifact,
  retryJob,
  submitPromptJob,
} from "../commands/job";
import LogPanel from "../components/LogPanel";
import StatusBadge from "../components/StatusBadge";
import VideoPreview from "../components/VideoPreview";
import { listProviderConfigs } from "../commands/provider";
import type {
  JobLogEntry,
  JobState,
  PromptJob,
  ProviderSummary,
  RenderArtifact,
  WorkspaceStatus,
} from "../commands/types";
import styles from "./ProjectJobsView.module.css";

interface ProjectJobsViewProps {
  mode: "workbench" | "history";
  projectId: string;
  projectName: string;
  workspacePath?: string;
  runtimeStatus: WorkspaceStatus["runtimeStatus"];
  preferredProviderId?: string | null;
  onProviderChange: (providerId: string) => void;
  onOpenProviderSettings: () => void;
}

const DATE_FORMATTER = new Intl.DateTimeFormat("zh-CN", {
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
});

const JOBS_PER_PAGE = 5;

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function sortJobs(jobs: PromptJob[]): PromptJob[] {
  return [...jobs].sort(
    (left, right) =>
      new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime(),
  );
}

function upsertJob(current: PromptJob[], nextJob: PromptJob): PromptJob[] {
  const exists = current.some((job) => job.id === nextJob.id);
  return sortJobs(
    exists
      ? current.map((job) => (job.id === nextJob.id ? nextJob : job))
      : [nextJob, ...current],
  );
}

function resolveSelectedJobId(
  jobs: PromptJob[],
  preferredJobId: string | null | undefined,
  currentSelectedJobId: string | null,
): string | null {
  if (preferredJobId && jobs.some((job) => job.id === preferredJobId)) {
    return preferredJobId;
  }

  if (
    currentSelectedJobId &&
    jobs.some((job) => job.id === currentSelectedJobId)
  ) {
    return currentSelectedJobId;
  }

  return jobs[0]?.id ?? null;
}

function isLiveJobState(state: JobState): boolean {
  return state === "queued" || state === "running";
}

function isRetryableJobState(state: JobState): boolean {
  return state === "failed" || state === "cancelled";
}

function isDeletableJobState(state: JobState): boolean {
  return !isLiveJobState(state);
}

function formatDateTime(value?: string): string {
  if (!value) {
    return "-";
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return DATE_FORMATTER.format(parsed);
}

function formatBytes(size: number): string {
  if (size >= 1024 * 1024) {
    return `${(size / (1024 * 1024)).toFixed(1)} MB`;
  }

  if (size >= 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }

  return `${size} B`;
}

function stateLabel(state: JobState): string {
  switch (state) {
    case "queued":
      return "排队中";
    case "running":
      return "渲染中";
    case "succeeded":
      return "已完成";
    case "failed":
      return "失败";
    case "cancelled":
      return "已取消";
    default:
      return state;
  }
}

function providerLabel(
  providerId: string,
  providers: ProviderSummary[],
): string {
  const provider = providers.find((item) => item.id === providerId);
  if (provider) {
    return provider.name;
  }

  return `已删除 Provider (${providerId.slice(0, 8)})`;
}

export default function ProjectJobsView({
  mode,
  projectId,
  projectName,
  workspacePath,
  runtimeStatus,
  preferredProviderId,
  onProviderChange,
  onOpenProviderSettings,
}: ProjectJobsViewProps) {
  const [providers, setProviders] = useState<ProviderSummary[]>([]);
  const [selectedProviderId, setSelectedProviderId] = useState("");
  const [promptText, setPromptText] = useState("");
  const [jobs, setJobs] = useState<PromptJob[]>([]);
  const [currentPage, setCurrentPage] = useState(1);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [jobLogs, setJobLogs] = useState<JobLogEntry[]>([]);
  const [artifact, setArtifact] = useState<RenderArtifact | null>(null);
  const [videoSrc, setVideoSrc] = useState<string | null>(null);
  const [pageError, setPageError] = useState<string | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [infoMessage, setInfoMessage] = useState<string | null>(null);
  const [isLoadingProviders, setIsLoadingProviders] = useState(true);
  const [isLoadingJobs, setIsLoadingJobs] = useState(true);
  const [isLoadingDetails, setIsLoadingDetails] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isMutatingJob, setIsMutatingJob] = useState(false);
  const [isOpeningArtifact, setIsOpeningArtifact] = useState(false);
  const [logsOpen, setLogsOpen] = useState(false);

  const selectedJob = useMemo(
    () => jobs.find((job) => job.id === selectedJobId) ?? null,
    [jobs, selectedJobId],
  );

  const totalPages = Math.max(1, Math.ceil(jobs.length / JOBS_PER_PAGE));
  const paginatedJobs = useMemo(() => {
    const startIndex = (currentPage - 1) * JOBS_PER_PAGE;
    return jobs.slice(startIndex, startIndex + JOBS_PER_PAGE);
  }, [currentPage, jobs]);

  const canSubmit =
    mode === "workbench" &&
    runtimeStatus === "ready" &&
    selectedProviderId.length > 0 &&
    promptText.trim().length > 0 &&
    !isSubmitting;

  async function loadProviders() {
    setIsLoadingProviders(true);

    try {
      const response = await listProviderConfigs();
      if (!response.ok) {
        setPageError(response.error.message);
        return;
      }

      setProviders(response.data);
      const nextSelectedProviderId =
        (preferredProviderId &&
        response.data.some((provider) => provider.id === preferredProviderId)
          ? preferredProviderId
          : null) ??
        (selectedProviderId &&
        response.data.some((provider) => provider.id === selectedProviderId)
          ? selectedProviderId
          : null) ??
        response.data[0]?.id ??
        "";

      setSelectedProviderId(nextSelectedProviderId);
      if (
        nextSelectedProviderId &&
        nextSelectedProviderId !== preferredProviderId
      ) {
        onProviderChange(nextSelectedProviderId);
      }
    } catch (error) {
      setPageError(`无法读取 Provider 列表：${toMessage(error)}`);
    } finally {
      setIsLoadingProviders(false);
    }
  }

  async function loadJobs(preferredJobId?: string | null) {
    setIsLoadingJobs(true);

    try {
      const response = await listProjectJobs(projectId);
      if (!response.ok) {
        setPageError(response.error.message);
        return;
      }

      const nextJobs = sortJobs(response.data);
      setJobs(nextJobs);
      setSelectedJobId((current) =>
        resolveSelectedJobId(nextJobs, preferredJobId, current),
      );
    } catch (error) {
      setPageError(`无法读取任务列表：${toMessage(error)}`);
    } finally {
      setIsLoadingJobs(false);
    }
  }

  const loadSelectedJobDetails = useEffectEvent(
    async (job: PromptJob | null) => {
      if (!job) {
        setJobLogs([]);
        setArtifact(null);
        setVideoSrc(null);
        setDetailError(null);
        return;
      }

      setIsLoadingDetails(true);
      setDetailError(null);

      try {
        const logsResponse = await getJobLogs(job.id);
        if (!logsResponse.ok) {
          setJobLogs([]);
          setDetailError(logsResponse.error.message);
          return;
        }

        setJobLogs(logsResponse.data);

        if (job.state !== "succeeded") {
          setArtifact(null);
          setVideoSrc(null);
          return;
        }

        const artifactResponse = await getRenderArtifact(job.id);

        if (!artifactResponse.ok) {
          setArtifact(null);
          setVideoSrc(null);
          setDetailError(artifactResponse.error.message);
          return;
        }

        const videoResponse = await getVideoFileUrl(artifactResponse.data.id);

        if (!videoResponse.ok) {
          setArtifact(artifactResponse.data);
          setVideoSrc(null);
          setDetailError(videoResponse.error.message);
          return;
        }

        setArtifact(artifactResponse.data);
        setVideoSrc(videoResponse.data.url);
      } catch (error) {
        setArtifact(null);
        setVideoSrc(null);
        setDetailError(`无法读取任务详情：${toMessage(error)}`);
      } finally {
        setIsLoadingDetails(false);
      }
    },
  );

  const refreshSelectedJob = useEffectEvent(async (jobId: string) => {
    try {
      const response = await getJob(jobId);
      if (!response.ok) {
        setDetailError(response.error.message);
        return;
      }

      setJobs((current) => upsertJob(current, response.data));

      if (selectedJobId === jobId) {
        await loadSelectedJobDetails(response.data);
      }
    } catch (error) {
      setDetailError(`无法刷新任务状态：${toMessage(error)}`);
    }
  });

  useEffect(() => {
    setPageError(null);
    setDetailError(null);
    setInfoMessage(null);
    setPromptText("");
    setJobs([]);
    setCurrentPage(1);
    setSelectedJobId(null);
    setJobLogs([]);
    setArtifact(null);
    setVideoSrc(null);
    setLogsOpen(false);
    void loadProviders();
    void loadJobs();
  }, [projectId]);

  useEffect(() => {
    setCurrentPage((page) => Math.min(Math.max(page, 1), totalPages));
  }, [totalPages]);

  useEffect(() => {
    if (!selectedJob) {
      setJobLogs([]);
      setArtifact(null);
      setVideoSrc(null);
      setDetailError(null);
      return;
    }

    void loadSelectedJobDetails(selectedJob);

    if (!isLiveJobState(selectedJob.state)) {
      return;
    }

    const intervalId = window.setInterval(() => {
      void refreshSelectedJob(selectedJob.id);
    }, 2000);

    return () => window.clearInterval(intervalId);
  }, [selectedJob?.id, selectedJob?.state]);

  useEffect(() => {
    if (
      preferredProviderId &&
      preferredProviderId !== selectedProviderId &&
      providers.some((provider) => provider.id === preferredProviderId)
    ) {
      setSelectedProviderId(preferredProviderId);
    }
  }, [preferredProviderId, providers, selectedProviderId]);

  async function handleSubmit() {
    if (!canSubmit) {
      return;
    }

    setIsSubmitting(true);
    setPageError(null);
    setInfoMessage(null);

    try {
      const response = await submitPromptJob(
        projectId,
        selectedProviderId,
        promptText.trim(),
      );
      if (!response.ok) {
        setPageError(response.error.message);
        return;
      }

      setPromptText("");
      setInfoMessage(
        `任务已提交，当前状态：${stateLabel(response.data.state)}`,
      );
      await loadJobs(response.data.jobId);
    } catch (error) {
      setPageError(`提交任务失败：${toMessage(error)}`);
    } finally {
      setIsSubmitting(false);
    }
  }

  async function handleCancel() {
    if (!selectedJob || !isLiveJobState(selectedJob.state)) {
      return;
    }

    setIsMutatingJob(true);
    setPageError(null);
    setInfoMessage(null);

    try {
      const response = await cancelJob(selectedJob.id);
      if (!response.ok) {
        setPageError(response.error.message);
        return;
      }

      setInfoMessage("已发送取消请求。");
      await refreshSelectedJob(response.data.jobId);
    } catch (error) {
      setPageError(`取消任务失败：${toMessage(error)}`);
    } finally {
      setIsMutatingJob(false);
    }
  }

  async function handleRetry() {
    if (!selectedJob || !isRetryableJobState(selectedJob.state)) {
      return;
    }

    setIsMutatingJob(true);
    setPageError(null);
    setInfoMessage(null);

    try {
      const response = await retryJob(selectedJob.id);
      if (!response.ok) {
        setPageError(response.error.message);
        return;
      }

      setInfoMessage("已创建重试任务并重新加入队列。");
      await loadJobs(response.data.jobId);
    } catch (error) {
      setPageError(`重试任务失败：${toMessage(error)}`);
    } finally {
      setIsMutatingJob(false);
    }
  }

  async function handleDeleteJob(job: PromptJob) {
    if (!isDeletableJobState(job.state)) {
      return;
    }

    setIsMutatingJob(true);
    setPageError(null);
    setInfoMessage(null);

    try {
      const response = await deleteJob(job.id);
      if (!response.ok) {
        setPageError(response.error.message);
        return;
      }

      const nextJobs = jobs.filter((item) => item.id !== job.id);
      setJobs(nextJobs);
      if (selectedJobId === job.id) {
        setSelectedJobId(nextJobs[0]?.id ?? null);
      }
      setInfoMessage("任务已删除。");
    } catch (error) {
      setPageError(`删除任务失败：${toMessage(error)}`);
    } finally {
      setIsMutatingJob(false);
    }
  }

  async function handleRefresh() {
    setPageError(null);
    setInfoMessage(null);
    await Promise.all([loadProviders(), loadJobs(selectedJobId)]);
  }

  async function handleOpenArtifact() {
    if (!artifact) {
      return;
    }

    setIsOpeningArtifact(true);
    setDetailError(null);

    try {
      const response = await openRenderArtifact(
        artifact.id,
        "reveal_in_folder",
      );
      if (!response.ok) {
        setDetailError(response.error.message);
      }
    } catch (error) {
      setDetailError(`无法打开渲染产物：${toMessage(error)}`);
    } finally {
      setIsOpeningArtifact(false);
    }
  }

  return (
    <div className={styles.page}>
      {pageError ? <div className={styles.errorBanner}>{pageError}</div> : null}
      {infoMessage ? (
        <div className={styles.infoBanner}>{infoMessage}</div>
      ) : null}

      {mode === "workbench" ? (
        <section className={styles.surface}>
          <div className={styles.heroHeader}>
            <div>
              <h2 className={styles.sectionTitle}>生成工作台</h2>
              <p className={styles.sectionCopy}>
                为项目「{projectName}」选择 Provider，提交
                prompt，并在同一处跟踪日志与视频产物。
              </p>
            </div>

            <button
              type="button"
              className={styles.secondaryButton}
              onClick={() => void handleRefresh()}
            >
              刷新任务
            </button>
          </div>

          <div className={styles.summaryGrid}>
            <div className={styles.summaryCard}>
              <span className={styles.summaryLabel}>工作区</span>
              <strong className={styles.summaryValue}>
                {workspacePath ?? "未配置"}
              </strong>
            </div>
            <div className={styles.summaryCard}>
              <span className={styles.summaryLabel}>Runtime</span>
              <strong className={styles.summaryValue}>
                {runtimeStatus === "ready"
                  ? "已就绪"
                  : runtimeStatus === "broken"
                    ? "部分缺失"
                    : "未检测到"}
              </strong>
            </div>
            <div className={styles.summaryCard}>
              <span className={styles.summaryLabel}>Provider</span>
              <strong className={styles.summaryValue}>
                {providers.length} 个
              </strong>
            </div>
            <div className={styles.summaryCard}>
              <span className={styles.summaryLabel}>任务总数</span>
              <strong className={styles.summaryValue}>{jobs.length}</strong>
            </div>
          </div>

          <div className={styles.composerGrid}>
            <label className={styles.field}>
              <span className={styles.fieldLabel}>Provider</span>
              <select
                className={styles.select}
                value={selectedProviderId}
                onChange={(event) => {
                  setSelectedProviderId(event.target.value);
                  onProviderChange(event.target.value);
                }}
                disabled={isLoadingProviders || providers.length === 0}
              >
                {providers.length === 0 ? (
                  <option value="">暂无可用 Provider</option>
                ) : null}
                {providers.map((provider) => (
                  <option key={provider.id} value={provider.id}>
                    {provider.name} / {provider.model}
                  </option>
                ))}
              </select>
            </label>

            <label className={styles.field}>
              <span className={styles.fieldLabel}>Prompt</span>
              <textarea
                className={styles.textarea}
                value={promptText}
                onChange={(event) => setPromptText(event.target.value)}
                placeholder="描述你希望生成的 Manim 场景，例如：生成一个展示正弦函数与切线变化的动画。"
                rows={6}
              />
            </label>

            {providers.length === 0 ? (
              <div className={styles.warningBanner}>
                <div>
                  还没有可用的 Provider。先完成 Provider 配置，工作台才能提交
                  prompt。
                </div>
                <button
                  type="button"
                  className={styles.inlineButton}
                  onClick={onOpenProviderSettings}
                >
                  打开 Provider 设置
                </button>
              </div>
            ) : null}

            {runtimeStatus !== "ready" ? (
              <div className={styles.warningBanner}>
                本地渲染依赖未就绪，当前只能查看历史任务，不能提交新生成任务。
              </div>
            ) : null}

            <div className={styles.composerActions}>
              <button
                type="button"
                className={styles.primaryButton}
                onClick={() => void handleSubmit()}
                disabled={!canSubmit}
              >
                {isSubmitting ? "提交中" : "提交任务"}
              </button>
              <button
                type="button"
                className={styles.secondaryButton}
                onClick={() => setPromptText("")}
                disabled={isSubmitting || promptText.length === 0}
              >
                清空
              </button>
            </div>
          </div>
        </section>
      ) : (
        <section className={styles.surface}>
          <div className={styles.heroHeader}>
            <div>
              <h2 className={styles.sectionTitle}>历史任务</h2>
              <p className={styles.sectionCopy}>
                查看项目「{projectName}」的生成记录、日志和渲染产物。
              </p>
            </div>

            <button
              type="button"
              className={styles.secondaryButton}
              onClick={() => void handleRefresh()}
            >
              刷新列表
            </button>
          </div>
        </section>
      )}

      <div className={styles.workspaceGrid}>
        <section className={styles.listPanel}>
          <div className={styles.panelHeader}>
            <h3 className={styles.panelTitle}>任务列表</h3>
            <span className={styles.panelMeta}>
              {isLoadingJobs ? "读取中" : `${jobs.length} 条`}
            </span>
          </div>

          {isLoadingJobs ? (
            <div className={styles.emptyState}>正在读取任务列表…</div>
          ) : jobs.length === 0 ? (
            <div className={styles.emptyState}>
              {mode === "workbench"
                ? "当前项目还没有生成任务。"
                : "当前项目还没有历史任务。"}
            </div>
          ) : (
            <div className={styles.jobList}>
              {paginatedJobs.map((job) => {
                const isSelected = job.id === selectedJobId;

                return (
                  <div
                    key={job.id}
                    role="button"
                    tabIndex={0}
                    className={`${styles.jobRow} ${isSelected ? styles.jobRowActive : ""}`}
                    onClick={() => setSelectedJobId(job.id)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        setSelectedJobId(job.id);
                      }
                    }}
                  >
                    <div className={styles.jobRowTop}>
                      <span className={styles.statusWrap}>
                        <StatusBadge status={job.state} size="sm" />
                      </span>
                      <span className={styles.jobTime}>
                        {formatDateTime(job.createdAt)}
                      </span>
                    </div>
                    <div className={styles.jobPrompt}>{job.promptText}</div>
                    <div className={styles.jobMeta}>
                      <span>{providerLabel(job.providerId, providers)}</span>
                      {job.retryOfJobId ? (
                        <span>重试自 {job.retryOfJobId.slice(0, 12)}</span>
                      ) : null}
                      {job.errorCode ? <span>{job.errorCode}</span> : null}
                    </div>
                    <div className={styles.jobRowActions}>
                      <button
                        type="button"
                        className={styles.inlineButton}
                        onClick={(event) => {
                          event.stopPropagation();
                          void handleDeleteJob(job);
                        }}
                        disabled={
                          isMutatingJob || !isDeletableJobState(job.state)
                        }
                      >
                        删除
                      </button>
                    </div>
                  </div>
                );
              })}
              <div className={styles.paginationBar}>
                <button
                  type="button"
                  className={styles.secondaryButton}
                  onClick={() => setCurrentPage((page) => Math.max(1, page - 1))}
                  disabled={currentPage <= 1}
                >
                  上一页
                </button>
                <span className={styles.panelMeta}>
                  第 {currentPage} / {totalPages} 页
                </span>
                <button
                  type="button"
                  className={styles.secondaryButton}
                  onClick={() =>
                    setCurrentPage((page) => Math.min(totalPages, page + 1))
                  }
                  disabled={currentPage >= totalPages}
                >
                  下一页
                </button>
              </div>
            </div>
          )}
        </section>

        <section className={styles.detailPanel}>
          <div className={styles.panelHeader}>
            <h3 className={styles.panelTitle}>任务详情</h3>
            <span className={styles.panelMeta}>
              {selectedJob ? selectedJob.id.slice(0, 12) : "未选择"}
            </span>
          </div>

          {!selectedJob ? (
            <div className={styles.emptyState}>
              选择左侧任务后，可查看日志、错误信息和视频预览。
            </div>
          ) : (
            <div className={styles.detailStack}>
              <div className={styles.detailCard}>
                <div className={styles.detailHeader}>
                  <span className={styles.statusWrap}>
                    <StatusBadge status={selectedJob.state} />
                  </span>
                  <div className={styles.detailActions}>
                    <button
                      type="button"
                      className={styles.secondaryButton}
                      onClick={() => void refreshSelectedJob(selectedJob.id)}
                    >
                      刷新
                    </button>
                    {isLiveJobState(selectedJob.state) ? (
                      <button
                        type="button"
                        className={styles.secondaryButton}
                        onClick={() => void handleCancel()}
                        disabled={isMutatingJob}
                      >
                        {isMutatingJob ? "处理中" : "取消"}
                      </button>
                    ) : null}
                    {isRetryableJobState(selectedJob.state) ? (
                      <button
                        type="button"
                        className={styles.primaryButton}
                        onClick={() => void handleRetry()}
                        disabled={isMutatingJob}
                      >
                        {isMutatingJob ? "处理中" : "重试"}
                      </button>
                    ) : null}
                  </div>
                </div>

                <dl className={styles.metaGrid}>
                  <div>
                    <dt>Provider</dt>
                    <dd>{providerLabel(selectedJob.providerId, providers)}</dd>
                  </div>
                  <div>
                    <dt>创建时间</dt>
                    <dd>{formatDateTime(selectedJob.createdAt)}</dd>
                  </div>
                  <div>
                    <dt>开始时间</dt>
                    <dd>{formatDateTime(selectedJob.startedAt)}</dd>
                  </div>
                  <div>
                    <dt>结束时间</dt>
                    <dd>{formatDateTime(selectedJob.finishedAt)}</dd>
                  </div>
                </dl>

                <div className={styles.promptBlock}>
                  <span className={styles.fieldLabel}>Prompt 内容</span>
                  <pre className={styles.promptText}>
                    {selectedJob.promptText}
                  </pre>
                </div>

                {selectedJob.errorCode ||
                selectedJob.errorSummary ||
                selectedJob.suggestion ? (
                  <div className={styles.failureBox}>
                    {selectedJob.errorCode ? (
                      <strong>{selectedJob.errorCode}</strong>
                    ) : null}
                    {selectedJob.errorSummary ? (
                      <p>{selectedJob.errorSummary}</p>
                    ) : null}
                    {selectedJob.suggestion ? (
                      <p>{selectedJob.suggestion}</p>
                    ) : null}
                  </div>
                ) : null}
              </div>

              <div className={styles.detailCard}>
                <div className={styles.subpanelHeader}>
                  <h4 className={styles.subpanelTitle}>视频预览</h4>
                  {artifact ? (
                    <span className={styles.panelMeta}>
                      {artifact.durationSecs.toFixed(1)}s /{" "}
                      {formatBytes(artifact.fileSizeBytes)}
                    </span>
                  ) : null}
                </div>

                {detailError ? (
                  <div className={styles.inlineError}>{detailError}</div>
                ) : null}

                {isLoadingDetails ? (
                  <div className={styles.emptyState}>正在读取日志与预览…</div>
                ) : (
                  <div className={styles.previewStack}>
                    <VideoPreview
                      src={videoSrc}
                      artifact={artifact}
                      onOpenInExplorer={() => void handleOpenArtifact()}
                      openDisabled={isOpeningArtifact}
                    />
                    {artifact ? (
                      <div className={styles.previewMeta}>
                        <span>产物已通过安全命令校验</span>
                        <span>
                          创建于: {formatDateTime(artifact.createdAt)}
                        </span>
                      </div>
                    ) : null}
                  </div>
                )}
              </div>

              <div className={styles.detailCard}>
                <LogPanel
                  entries={jobLogs}
                  open={logsOpen}
                  onToggle={() => setLogsOpen((current) => !current)}
                />
              </div>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
