import { useEffect, useState } from "react";

import {
  getGenerationSettings,
  updateGenerationSettings,
} from "../commands/settings";
import { checkRuntime } from "../commands/workspace";
import type {
  GenerationSettings,
  RuntimeComponentStatus,
  RuntimeStatus,
  WorkspaceStatus,
} from "../commands/types";
import styles from "./BasicSettingsView.module.css";

interface BasicSettingsViewProps {
  workspacePath?: string;
  workspaceRuntimeStatus: WorkspaceStatus["runtimeStatus"];
}

type DisplayStatus = RuntimeComponentStatus | "checking" | "idle";
type LogLevel = "error" | "warn" | "info" | "debug";

const RUNTIME_ITEMS: Array<{
  key: keyof Pick<
    RuntimeStatus,
    | "python"
    | "uv"
    | "manim"
    | "uvManim"
    | "ffmpeg"
    | "ffprobe"
    | "latex"
    | "dvisvgm"
  >;
  label: string;
}> = [
  { key: "python", label: "Python 3.10+" },
  { key: "uv", label: "uv" },
  { key: "manim", label: "Manim CE（可选，全局）" },
  { key: "uvManim", label: "uv 托管 Manim" },
  { key: "ffmpeg", label: "FFmpeg" },
  { key: "ffprobe", label: "FFprobe" },
  { key: "latex", label: "LaTeX / MiKTeX" },
  { key: "dvisvgm", label: "dvisvgm" },
];

const LOG_LEVEL_STORAGE_KEY = "manim4learn.logLevel";

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function labelForStatus(status: DisplayStatus): string {
  switch (status) {
    case "ok":
      return "可用";
    case "missing":
      return "缺失";
    case "checking":
      return "检查中";
    default:
      return "未检查";
  }
}

function indicatorForStatus(status: DisplayStatus): string {
  switch (status) {
    case "ok":
      return "OK";
    case "missing":
      return "NO";
    case "checking":
      return "..";
    default:
      return "--";
  }
}

function statusClassName(status: DisplayStatus): string {
  switch (status) {
    case "ok":
      return styles.statusOk;
    case "missing":
      return styles.statusMissing;
    case "checking":
      return styles.statusChecking;
    default:
      return styles.statusIdle;
  }
}

function defaultMessage(
  runtimeStatus: WorkspaceStatus["runtimeStatus"],
): string {
  switch (runtimeStatus) {
    case "ready":
      return "当前运行环境状态为 ready，可手动复检依赖版本。";
    case "broken":
      return "检测到运行环境不完整，建议先检查缺失依赖。";
    case "missing":
      return "尚未检测到本地运行环境，请先执行环境检查。";
    default:
      return "环境状态未知。";
  }
}

export default function BasicSettingsView({
  workspacePath,
  workspaceRuntimeStatus,
}: BasicSettingsViewProps) {
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus | null>(
    null,
  );
  const [isChecking, setIsChecking] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string>(
    defaultMessage(workspaceRuntimeStatus),
  );
  const [logLevel, setLogLevel] = useState<LogLevel>(() => {
    if (typeof window === "undefined") {
      return "info";
    }

    const stored = window.localStorage.getItem(LOG_LEVEL_STORAGE_KEY);
    if (
      stored === "error" ||
      stored === "warn" ||
      stored === "info" ||
      stored === "debug"
    ) {
      return stored;
    }

    return "info";
  });
  const [generationSettings, setGenerationSettings] =
    useState<GenerationSettings>({
      strictApiNameValidation: false,
    });
  const [isSavingGenerationSettings, setIsSavingGenerationSettings] =
    useState(false);
  const [generationSettingsMessage, setGenerationSettingsMessage] = useState<
    string | null
  >(null);

  useEffect(() => {
    setFeedback(defaultMessage(workspaceRuntimeStatus));
  }, [workspaceRuntimeStatus]);

  useEffect(() => {
    void handleCheckRuntime();
    void loadGenerationSettings();
  }, []);

  async function loadGenerationSettings() {
    try {
      const response = await getGenerationSettings();
      if (!response.ok) {
        setGenerationSettingsMessage(response.error.message);
        return;
      }

      setGenerationSettings(response.data);
      setGenerationSettingsMessage(null);
    } catch (error) {
      setGenerationSettingsMessage(`读取生成设置失败：${toMessage(error)}`);
    }
  }

  async function handleCheckRuntime() {
    setIsChecking(true);
    setErrorMessage(null);

    try {
      const response = await checkRuntime(workspacePath);
      if (!response.ok) {
        setRuntimeStatus(null);
        setErrorMessage(response.error.message);
        return;
      }

      setRuntimeStatus(response.data);
      setFeedback(response.data.message);
    } catch (error) {
      setRuntimeStatus(null);
      setErrorMessage(`环境检查失败：${toMessage(error)}`);
    } finally {
      setIsChecking(false);
    }
  }

  function handleLogLevelChange(nextLevel: LogLevel) {
    setLogLevel(nextLevel);
    window.localStorage.setItem(LOG_LEVEL_STORAGE_KEY, nextLevel);
  }

  async function handleStrictApiNameValidationChange(enabled: boolean) {
    const previous = generationSettings;
    const next = {
      ...generationSettings,
      strictApiNameValidation: enabled,
    };
    setGenerationSettings(next);
    setIsSavingGenerationSettings(true);
    setGenerationSettingsMessage(null);

    try {
      const response = await updateGenerationSettings(next);
      if (!response.ok) {
        setGenerationSettings(previous);
        setGenerationSettingsMessage(response.error.message);
        return;
      }

      setGenerationSettings(response.data);
      setGenerationSettingsMessage("生成设置已保存，后续新任务生效。");
    } catch (error) {
      setGenerationSettings(previous);
      setGenerationSettingsMessage(`保存生成设置失败：${toMessage(error)}`);
    } finally {
      setIsSavingGenerationSettings(false);
    }
  }

  const hasMissingDependency =
    runtimeStatus !== null &&
    [
      runtimeStatus.python,
      runtimeStatus.uv,
      runtimeStatus.uvManim,
      runtimeStatus.ffmpeg,
      runtimeStatus.ffprobe,
      runtimeStatus.latex,
      runtimeStatus.dvisvgm,
    ].some((info) => info.status === "missing");

  return (
    <div className={styles.page}>
      <section className={styles.section}>
        <div className={styles.titleBlock}>
          <h2 className={styles.title}>工作区</h2>
          <p className={styles.copy}>当前应用绑定的本地工作区目录。</p>
        </div>
        <label className={styles.pathField}>
          <span className={styles.label}>Workspace Path</span>
          <input
            type="text"
            readOnly
            value={workspacePath ?? "未配置工作区"}
            className={styles.pathInput}
          />
        </label>
      </section>

      <section className={styles.section}>
        <div className={styles.titleBlock}>
          <h2 className={styles.title}>生成校验</h2>
          <p className={styles.copy}>
            控制 ManimCE 代码生成后的静态校验强度。安全检查和已知错误拦截始终开启。
          </p>
        </div>
        <label className={styles.toggleField}>
          <input
            type="checkbox"
            checked={generationSettings.strictApiNameValidation}
            disabled={isSavingGenerationSettings}
            onChange={(event) =>
              void handleStrictApiNameValidationChange(event.target.checked)
            }
          />
          <span className={styles.toggleText}>
            严格 ManimCE API 名称校验（实验）
          </span>
        </label>
        <p className={styles.copy}>
          开启后会用本地 manifest 提前发现旧版、ManimGL 或不存在的 API
          名称，但可能误拒官方 API。关闭后仍会拦截文件、网络、子进程、ManimGL
          和已知错误写法。
        </p>
        {generationSettingsMessage ? (
          <div className={styles.callout}>{generationSettingsMessage}</div>
        ) : null}
      </section>

      <section className={styles.section}>
        <div className={styles.header}>
          <div className={styles.titleBlock}>
            <h2 className={styles.title}>运行环境</h2>
            <p className={styles.copy}>
              检查 Python、uv、uv 托管的 Manim、FFmpeg 和 FFprobe 是否可用；全局
              Manim 仅作参考。
            </p>
          </div>
          <button
            type="button"
            className={styles.secondaryButton}
            onClick={() => void handleCheckRuntime()}
            disabled={isChecking}
          >
            {isChecking ? "检查中" : "重新检查"}
          </button>
        </div>

        <div className={styles.runtimeList} role="list">
          {RUNTIME_ITEMS.map((item) => {
            const componentInfo = isChecking ? null : runtimeStatus?.[item.key];
            const status: DisplayStatus = isChecking
              ? "checking"
              : (componentInfo?.status ?? "idle");

            return (
              <div
                key={item.key}
                className={styles.runtimeItem}
                role="listitem"
              >
                <div className={styles.runtimeLabelBlock}>
                  <span className={styles.runtimeLabel}>{item.label}</span>
                  <span className={styles.runtimeVersion}>
                    {componentInfo?.version ?? "version unknown"}
                  </span>
                </div>
                <div className={styles.runtimeMeta}>
                  <span>{labelForStatus(status)}</span>
                  <span
                    className={`${styles.indicator} ${statusClassName(status)}`}
                  >
                    {indicatorForStatus(status)}
                  </span>
                </div>
              </div>
            );
          })}
        </div>

        {errorMessage ? (
          <div className={`${styles.callout} ${styles.calloutError}`}>
            {errorMessage}
          </div>
        ) : (
          <div
            className={`${styles.callout} ${
              hasMissingDependency ? styles.calloutWarn : styles.calloutSuccess
            }`}
          >
            {feedback}
          </div>
        )}
      </section>

      <section className={styles.section}>
        <div className={styles.titleBlock}>
          <h2 className={styles.title}>日志级别</h2>
          <p className={styles.copy}>
            控制前端保留的界面调试级别，供后续日志面板收敛展示使用。
          </p>
        </div>
        <label className={styles.selectField}>
          <span className={styles.label}>Frontend Log Level</span>
          <select
            className={styles.select}
            value={logLevel}
            onChange={(event) =>
              handleLogLevelChange(event.target.value as LogLevel)
            }
          >
            <option value="error">error</option>
            <option value="warn">warn</option>
            <option value="info">info</option>
            <option value="debug">debug</option>
          </select>
        </label>
      </section>
    </div>
  );
}
