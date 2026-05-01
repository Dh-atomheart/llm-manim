import type { Page } from "@playwright/test";

type RuntimeStatus = "ready" | "broken" | "missing";
type ProviderType = "openai_compatible" | "anthropic_compatible";

interface SeedProvider {
  id?: string;
  name: string;
  providerType?: ProviderType;
  baseUrl?: string;
  model?: string;
  apiKey?: string;
}

interface SeedProject {
  id?: string;
  name: string;
}

export interface TauriMockOptions {
  workspaceConfigured?: boolean;
  workspacePath?: string;
  runtimeStatus?: RuntimeStatus;
  seedProviders?: SeedProvider[];
  seedProjects?: SeedProject[];
}

export async function installTauriMock(
  page: Page,
  options: TauriMockOptions = {},
): Promise<void> {
  await page.addInitScript((rawOptions: TauriMockOptions) => {
    type JobOutcome = "success" | "static-fail" | "render-fail" | "cancel";
    type JobState = "queued" | "running" | "succeeded" | "failed" | "cancelled";
    type LogLevel = "info" | "warn" | "error";

    interface ProviderRecord {
      id: string;
      name: string;
      providerType: ProviderType;
      baseUrl: string;
      model: string;
      apiKey: string;
      createdAt: string;
      updatedAt: string;
      deletedAt: string | null;
    }

    interface ProjectRecord {
      id: string;
      name: string;
      createdAt: string;
      updatedAt: string;
      deletedAt: string | null;
    }

    interface ArtifactRecord {
      id: string;
      jobId: string;
      projectId: string;
      filePath: string;
      durationSecs: number;
      fileSizeBytes: number;
      createdAt: string;
    }

    interface JobLogRecord {
      id: string;
      stage:
        | "queue"
        | "provider"
        | "render"
        | "artifact"
        | "static_check"
        | "user_action";
      level: LogLevel;
      message: string;
      timestamp: string;
    }

    interface JobRecord {
      id: string;
      projectId: string;
      providerId: string;
      promptText: string;
      state: JobState;
      createdAt: string;
      startedAt?: string;
      finishedAt?: string;
      errorCode?: string;
      errorSummary?: string;
      suggestion?: string;
      retryOfJobId?: string;
      outcome: JobOutcome;
      pollsUntilTerminal: number | null;
      logs: JobLogRecord[];
      artifact?: ArtifactRecord;
      deletedAt: string | null;
    }

    const options = rawOptions ?? {};
    const workspacePath = options.workspacePath ?? "F:/mock-workspace";
    const runtimeStatus = options.runtimeStatus ?? "ready";
    const runtimeMessage =
      runtimeStatus === "ready"
        ? "运行环境可用，可在当前工作区启动 Manim 渲染"
        : runtimeStatus === "broken"
          ? "部分运行依赖缺失，无法在当前工作区启动 Manim 渲染"
          : "未检测到所需运行依赖";

    let providerCount = 0;
    let projectCount = 0;
    let jobCount = 0;
    let artifactCount = 0;
    let logCount = 0;

    const now = () => new Date().toISOString();
    const redact = (value: string) => value.replace(/sk-[\w-]+/g, "[REDACTED]");
    const ok = <T>(data: T) => ({ ok: true, data });
    const err = (
      code: string,
      message: string,
      retryable = false,
      details?: Record<string, unknown>,
    ) => ({
      ok: false,
      error: {
        code,
        message: redact(message),
        retryable,
        ...(details ? { details } : {}),
      },
    });

    const state = {
      workspaceConfigured: options.workspaceConfigured ?? false,
      runtimeStatus,
      providers: [] as ProviderRecord[],
      projects: [] as ProjectRecord[],
      jobs: [] as JobRecord[],
      openedArtifacts: [] as string[],
    };

    const runtimeComponent = (status: "ok" | "missing", version?: string) => ({
      status,
      ...(version ? { version } : {}),
    });

    const runtimePayload = () => {
      if (state.runtimeStatus === "ready") {
        return {
          status: "ready",
          python: runtimeComponent("ok", "3.11.9"),
          uv: runtimeComponent("ok", "0.8.0"),
          manim: runtimeComponent("missing"),
          uvManim: runtimeComponent("ok", "0.19.0"),
          ffmpeg: runtimeComponent("ok", "7.0"),
          ffprobe: runtimeComponent("ok", "7.0"),
          latex: runtimeComponent("ok", "MiKTeX"),
          dvisvgm: runtimeComponent("ok", "3.4"),
          message: runtimeMessage,
        };
      }

      if (state.runtimeStatus === "broken") {
        return {
          status: "broken",
          python: runtimeComponent("ok", "3.11.9"),
          uv: runtimeComponent("ok", "0.8.0"),
          manim: runtimeComponent("ok", "0.19.0"),
          uvManim: runtimeComponent("missing"),
          ffmpeg: runtimeComponent("ok", "7.0"),
          ffprobe: runtimeComponent("missing"),
          latex: runtimeComponent("missing"),
          dvisvgm: runtimeComponent("missing"),
          errorCode: "E_RUNTIME_INVALID",
          message: runtimeMessage,
        };
      }

      return {
        status: "missing",
        python: runtimeComponent("missing"),
        uv: runtimeComponent("missing"),
        manim: runtimeComponent("missing"),
        uvManim: runtimeComponent("missing"),
        ffmpeg: runtimeComponent("missing"),
        ffprobe: runtimeComponent("missing"),
        latex: runtimeComponent("missing"),
        dvisvgm: runtimeComponent("missing"),
        errorCode: "E_RUNTIME_INVALID",
        message: runtimeMessage,
      };
    };

    const nextProviderId = () => `provider_${++providerCount}`;
    const nextProjectId = () => `project_${++projectCount}`;
    const nextJobId = () => `job_${++jobCount}`;
    const nextArtifactId = () => `artifact_${++artifactCount}`;
    const nextLogId = () => `log_${++logCount}`;

    const toProviderSummary = (provider: ProviderRecord) => ({
      id: provider.id,
      name: provider.name,
      providerType: provider.providerType,
      baseUrl: provider.baseUrl,
      model: provider.model,
      createdAt: provider.createdAt,
      updatedAt: provider.updatedAt,
    });

    const toProject = (project: ProjectRecord) => ({
      id: project.id,
      name: project.name,
      createdAt: project.createdAt,
      updatedAt: project.updatedAt,
    });

    const toJobView = (job: JobRecord) => ({
      id: job.id,
      projectId: job.projectId,
      providerId: job.providerId,
      promptText: job.promptText,
      state: job.state,
      ...(job.errorCode ? { errorCode: job.errorCode } : {}),
      ...(job.errorSummary ? { errorSummary: job.errorSummary } : {}),
      ...(job.suggestion ? { suggestion: job.suggestion } : {}),
      ...(job.retryOfJobId ? { retryOfJobId: job.retryOfJobId } : {}),
      createdAt: job.createdAt,
      ...(job.startedAt ? { startedAt: job.startedAt } : {}),
      ...(job.finishedAt ? { finishedAt: job.finishedAt } : {}),
    });

    const appendJobLog = (
      job: JobRecord,
      stage: JobLogRecord["stage"],
      level: LogLevel,
      message: string,
    ) => {
      job.logs.push({
        id: nextLogId(),
        stage,
        level,
        message: redact(message),
        timestamp: now(),
      });
    };

    const createArtifact = (job: JobRecord): ArtifactRecord => ({
      id: nextArtifactId(),
      jobId: job.id,
      projectId: job.projectId,
      filePath: `artifacts/${job.projectId}/${job.id}/output.mp4`,
      durationSecs: 4.2,
      fileSizeBytes: 131072,
      createdAt: now(),
    });

    const setRunning = (job: JobRecord) => {
      if (job.state !== "queued") {
        return;
      }

      job.state = "running";
      job.startedAt = now();
      appendJobLog(
        job,
        "queue",
        "info",
        "job dequeued and entered running state",
      );
      appendJobLog(job, "render", "info", "render process started");
    };

    const finalizeJob = (job: JobRecord) => {
      if (job.state !== "running") {
        return;
      }

      if (job.outcome === "success") {
        job.state = "succeeded";
        job.finishedAt = now();
        job.artifact = createArtifact(job);
        appendJobLog(
          job,
          "artifact",
          "info",
          "artifact check finished and job succeeded",
        );
        return;
      }

      if (job.outcome === "static-fail") {
        job.state = "failed";
        job.finishedAt = now();
        job.errorCode = "E_STATIC_CHECK_FAILED";
        job.errorSummary = "生成代码调用了受限能力";
        job.suggestion =
          "请改写提示词，要求只使用 Manim Community Edition 并避免文件、网络或命令调用。";
        appendJobLog(
          job,
          "static_check",
          "error",
          "static check failed: 生成代码调用了受限能力",
        );
        return;
      }

      if (job.outcome === "render-fail") {
        job.state = "failed";
        job.finishedAt = now();
        job.errorCode = "E_RENDER_FAIL";
        job.errorSummary = "Manim 渲染失败";
        job.suggestion =
          "请检查运行环境与渲染日志后重试；若使用 MathTex 或坐标轴公式标签，请确认 LaTeX/MiKTeX 与 dvisvgm 已安装并在 PATH 中。";
        appendJobLog(
          job,
          "render",
          "error",
          "render process exited with failure: render failure",
        );
      }
    };

    const maybeAdvanceJob = (job: JobRecord) => {
      if (job.state === "queued") {
        setRunning(job);
        return;
      }

      if (job.state !== "running" || job.pollsUntilTerminal === null) {
        return;
      }

      job.pollsUntilTerminal -= 1;
      if (job.pollsUntilTerminal <= 0) {
        finalizeJob(job);
      }
    };

    const classifyPrompt = (
      promptText: string,
      retryOfJobId?: string,
    ): JobOutcome => {
      if (retryOfJobId) {
        return "success";
      }

      const text = promptText.toLowerCase();
      if (text.includes("[cancel]")) {
        return "cancel";
      }
      if (text.includes("[static-fail]")) {
        return "static-fail";
      }
      if (text.includes("[render-fail]")) {
        return "render-fail";
      }
      return "success";
    };

    const createJob = (
      projectId: string,
      providerId: string,
      promptText: string,
      retryOfJobId?: string,
    ): JobRecord => {
      const createdAt = now();
      const outcome = classifyPrompt(promptText, retryOfJobId);
      const job: JobRecord = {
        id: nextJobId(),
        projectId,
        providerId,
        promptText,
        state: "queued",
        createdAt,
        ...(retryOfJobId ? { retryOfJobId } : {}),
        outcome,
        pollsUntilTerminal: outcome === "cancel" ? null : 1,
        logs: [],
        deletedAt: null,
      };

      appendJobLog(
        job,
        "queue",
        "info",
        retryOfJobId
          ? `retry job created from ${retryOfJobId}`
          : "job created and queued",
      );
      state.jobs.unshift(job);
      return job;
    };

    const activeProviders = () =>
      [...state.providers]
        .filter((provider) => provider.deletedAt === null)
        .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));

    const activeProjects = () =>
      [...state.projects]
        .filter((project) => project.deletedAt === null)
        .sort((left, right) => right.createdAt.localeCompare(left.createdAt));

    const findProvider = (providerId: string) =>
      state.providers.find((provider) => provider.id === providerId) ?? null;
    const findProject = (projectId: string) =>
      state.projects.find((project) => project.id === projectId) ?? null;
    const findJob = (jobId: string) =>
      state.jobs.find((job) => job.id === jobId && job.deletedAt === null) ??
      null;
    const findArtifact = (artifactId: string) =>
      state.jobs.find((job) => job.artifact?.id === artifactId)?.artifact ??
      null;

    const seedProvider = (seed: SeedProvider) => {
      const createdAt = now();
      state.providers.push({
        id: seed.id ?? nextProviderId(),
        name: seed.name,
        providerType: seed.providerType ?? "openai_compatible",
        baseUrl: seed.baseUrl ?? "https://api.example.com",
        model: seed.model ?? "mock-model-v1",
        apiKey: seed.apiKey ?? "sk-mock-provider",
        createdAt,
        updatedAt: createdAt,
        deletedAt: null,
      });
    };

    const seedProject = (seed: SeedProject) => {
      const createdAt = now();
      state.projects.push({
        id: seed.id ?? nextProjectId(),
        name: seed.name,
        createdAt,
        updatedAt: createdAt,
        deletedAt: null,
      });
    };

    for (const provider of options.seedProviders ?? []) {
      seedProvider(provider);
    }
    for (const project of options.seedProjects ?? []) {
      seedProject(project);
    }

    const handleInvoke = async (
      command: string,
      args: Record<string, unknown> = {},
    ) => {
      if (command === "plugin:dialog|open") {
        return workspacePath;
      }

      if (command === "ping_backend") {
        return ok({ message: "pong" });
      }

      if (command === "get_workspace_status") {
        return ok({
          configured: state.workspaceConfigured,
          ...(state.workspaceConfigured ? { workspacePath } : {}),
          writable: state.workspaceConfigured,
          databaseReady: state.workspaceConfigured,
          runtimeStatus: state.runtimeStatus,
        });
      }

      if (command === "check_runtime") {
        return ok(runtimePayload());
      }

      if (command === "initialize_workspace") {
        const created = !state.workspaceConfigured;
        state.workspaceConfigured = true;
        return ok({
          workspacePath,
          created,
          databaseReady: true,
        });
      }

      if (!state.workspaceConfigured) {
        return err("E_WORKSPACE_INVALID", "工作区尚未初始化");
      }

      if (command === "list_provider_configs") {
        return ok(activeProviders().map(toProviderSummary));
      }

      if (command === "save_provider_config") {
        const input = (args.input ?? {}) as Partial<SeedProvider> & {
          id?: string;
        };
        const existing = input.id ? findProvider(input.id) : null;
        const createdAt = existing?.createdAt ?? now();
        const updatedAt = now();
        const provider: ProviderRecord = {
          id: existing?.id ?? nextProviderId(),
          name: input.name ?? existing?.name ?? "Mock Provider",
          providerType:
            input.providerType ?? existing?.providerType ?? "openai_compatible",
          baseUrl:
            input.baseUrl ?? existing?.baseUrl ?? "https://api.example.com",
          model: input.model ?? existing?.model ?? "mock-model-v1",
          apiKey: input.apiKey ?? existing?.apiKey ?? "sk-mock-provider",
          createdAt,
          updatedAt,
          deletedAt: null,
        };

        if (existing) {
          Object.assign(existing, provider);
        } else {
          state.providers.unshift(provider);
        }

        return ok({ id: provider.id });
      }

      if (command === "test_provider_config") {
        const input = (args.input ?? {}) as Partial<SeedProvider>;
        const providerType = input.providerType ?? "openai_compatible";
        const baseUrl = input.baseUrl ?? "https://api.example.com";
        const model = input.model ?? "mock-model-v1";
        const apiKey = input.apiKey ?? "";

        if (apiKey.toLowerCase().includes("bad")) {
          return err("E_AUTH_401", "Provider 鉴权失败，请检查 API Key");
        }
        if (
          baseUrl.toLowerCase().includes("timeout") ||
          model.toLowerCase().includes("timeout")
        ) {
          return err("E_NET_TIMEOUT", "连接测试超时，请稍后重试", true);
        }

        return ok({
          reachable: true,
          modelAccepted: true,
          message: `${providerType} / ${baseUrl} / ${model} 连接测试成功`,
        });
      }

      if (command === "delete_provider_config") {
        const providerId = String(args.id ?? "");
        const provider = findProvider(providerId);
        if (!provider || provider.deletedAt !== null) {
          return err("E_NOT_FOUND", "Provider 不存在");
        }
        provider.deletedAt = now();
        provider.updatedAt = now();
        return ok({ deleted: true });
      }

      if (command === "list_projects") {
        return ok(activeProjects().map(toProject));
      }

      if (command === "create_project") {
        const name = String(args.name ?? "").trim();
        if (!name) {
          return err("E_VALIDATION", "项目名称不能为空");
        }
        const createdAt = now();
        const project: ProjectRecord = {
          id: nextProjectId(),
          name,
          createdAt,
          updatedAt: createdAt,
          deletedAt: null,
        };
        state.projects.unshift(project);
        return ok(toProject(project));
      }

      if (command === "delete_project") {
        const projectId = String(args.id ?? "");
        const project = findProject(projectId);
        if (!project || project.deletedAt !== null) {
          return err("E_NOT_FOUND", "项目不存在");
        }
        project.deletedAt = now();
        project.updatedAt = now();
        return ok({});
      }

      if (command === "submit_prompt_job") {
        const projectId = String(args.projectId ?? "");
        const providerId = String(args.providerId ?? "");
        const promptText = String(args.promptText ?? "").trim();
        if (!promptText) {
          return err("E_VALIDATION", "提示词不能为空");
        }
        const job = createJob(projectId, providerId, promptText);
        return ok({ jobId: job.id, state: "queued" });
      }

      if (command === "list_project_jobs") {
        const projectId = String(args.projectId ?? "");
        return ok(
          state.jobs
            .filter((job) => job.projectId === projectId && job.deletedAt === null)
            .map((job) => {
              if (job.state === "queued") {
                setRunning(job);
              }
              return toJobView(job);
            }),
        );
      }

      if (command === "get_job") {
        const job = findJob(String(args.jobId ?? ""));
        if (!job) {
          return err("E_NOT_FOUND", "任务不存在");
        }
        maybeAdvanceJob(job);
        return ok(toJobView(job));
      }

      if (command === "cancel_job") {
        const job = findJob(String(args.jobId ?? ""));
        if (!job) {
          return err("E_NOT_FOUND", "任务不存在");
        }
        if (job.state === "queued") {
          setRunning(job);
        }
        if (job.state !== "running") {
          return err("E_JOB_NOT_CANCELLABLE", "当前任务状态不允许取消");
        }
        job.state = "cancelled";
        job.finishedAt = now();
        job.errorCode = "E_CANCELLED";
        job.errorSummary = "任务已取消";
        job.suggestion = "如需继续生成，请手动重试任务。";
        appendJobLog(job, "user_action", "info", "job marked as cancelled");
        return ok({ jobId: job.id, state: job.state });
      }

      if (command === "delete_job") {
        const job = findJob(String(args.jobId ?? ""));
        if (!job) {
          return err("E_NOT_FOUND", "ä»»åŠ¡ä¸å­˜åœ¨");
        }
        if (["queued", "running"].includes(job.state)) {
          return err(
            "E_JOB_NOT_DELETABLE",
            "queued or running jobs cannot be deleted",
          );
        }
        job.deletedAt = now();
        appendJobLog(job, "user_action", "info", "job hidden by user");
        return ok({ deleted: true });
      }

      if (command === "retry_job") {
        const sourceJob = findJob(String(args.jobId ?? ""));
        if (!sourceJob) {
          return err("E_NOT_FOUND", "任务不存在");
        }
        if (!["failed", "cancelled"].includes(sourceJob.state)) {
          return err(
            "E_JOB_NOT_RETRYABLE",
            "只有 failed 或 cancelled 任务允许手动重试",
          );
        }
        const retryJob = createJob(
          sourceJob.projectId,
          sourceJob.providerId,
          sourceJob.promptText,
          sourceJob.id,
        );
        return ok({
          jobId: retryJob.id,
          state: "queued",
          retryOfJobId: sourceJob.id,
        });
      }

      if (command === "get_job_logs") {
        const job = findJob(String(args.jobId ?? ""));
        if (!job) {
          return ok([]);
        }
        return ok(
          job.logs.map((entry) => ({
            id: entry.id,
            stage: entry.stage,
            level: entry.level,
            message: entry.message,
            timestamp: entry.timestamp,
          })),
        );
      }

      if (command === "get_render_artifact") {
        const job = findJob(String(args.jobId ?? ""));
        if (!job || !job.artifact) {
          return err("E_NOT_FOUND", "当前任务尚未生成可用产物");
        }
        return ok(job.artifact);
      }

      if (command === "get_video_file_url") {
        const artifact = findArtifact(String(args.artifactId ?? ""));
        if (!artifact) {
          return err("E_ARTIFACT_INVALID", "当前任务尚未生成可预览的视频");
        }
        return ok({ url: "https://example.com/mock-output.mp4" });
      }

      if (command === "open_render_artifact") {
        const artifact = findArtifact(String(args.artifactId ?? ""));
        if (!artifact) {
          return err("E_ARTIFACT_INVALID", "渲染产物不存在或已失效");
        }
        state.openedArtifacts.push(artifact.id);
        return ok({ opened: true });
      }

      return err("E_NOT_FOUND", `未实现的 mock command: ${command}`);
    };

    Object.defineProperty(window, "__MANIM4LEARN_TAURI_MOCK__", {
      value: state,
      configurable: true,
      writable: false,
    });

    let callbackId = 0;
    window.__TAURI_INTERNALS__ = {
      invoke: (command: string, args?: Record<string, unknown>) =>
        handleInvoke(command, args),
      transformCallback: (callback: unknown) => {
        callbackId += 1;
        return callbackId;
      },
      unregisterCallback: (_callback: unknown) => undefined,
      metadata: {
        currentWindow: {
          label: "main",
        },
      },
    } as typeof window.__TAURI_INTERNALS__;
  }, options);
}
