import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { checkRuntime, initializeWorkspace } from "../commands/workspace";
import type { RuntimeComponentStatus, RuntimeStatus } from "../commands/types";
import styles from "./FirstLaunch.module.css";

interface FirstLaunchProps {
  initialWorkspacePath?: string | null;
  onComplete: () => void | Promise<void>;
}

type DisplayStatus = RuntimeComponentStatus | "checking" | "idle";

const RUNTIME_ITEMS: Array<{
  key: keyof Pick<RuntimeStatus, "python" | "uv" | "manim" | "ffmpeg">;
  label: string;
}> = [
  { key: "python", label: "Python 3.10+" },
  { key: "uv", label: "uv" },
  { key: "manim", label: "Manim CE" },
  { key: "ffmpeg", label: "FFmpeg" },
];

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function pickDirectory(selection: string | string[] | null): string | null {
  if (Array.isArray(selection)) {
    return selection[0] ?? null;
  }

  return selection;
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

export default function FirstLaunch({
  initialWorkspacePath,
  onComplete,
}: FirstLaunchProps) {
  const [workspacePath, setWorkspacePath] = useState(initialWorkspacePath ?? "");
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus | null>(null);
  const [isCheckingRuntime, setIsCheckingRuntime] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  async function browseDirectory() {
    setErrorMessage(null);

    try {
      const selection = pickDirectory(
        await open({
          directory: true,
          multiple: false,
          title: "选择工作区目录",
        }),
      );

      if (selection) {
        setWorkspacePath(selection);
        setFeedback(null);
      }
    } catch (error) {
      setErrorMessage(`无法打开目录选择器：${toMessage(error)}`);
    }
  }

  async function handleCheckRuntime() {
    setIsCheckingRuntime(true);
    setErrorMessage(null);
    setFeedback(null);

    try {
      const response = await checkRuntime();
      if (!response.ok) {
        setErrorMessage(response.error.message);
        setRuntimeStatus(null);
        return;
      }

      setRuntimeStatus(response.data);
      setFeedback(response.data.message);
    } catch (error) {
      setRuntimeStatus(null);
      setErrorMessage(`环境检查失败：${toMessage(error)}`);
    } finally {
      setIsCheckingRuntime(false);
    }
  }

  async function handleContinue() {
    if (!workspacePath.trim() || runtimeStatus === null) {
      return;
    }

    setIsInitializing(true);
    setErrorMessage(null);

    try {
      const response = await initializeWorkspace(workspacePath.trim());
      if (!response.ok) {
        setErrorMessage(response.error.message);
        return;
      }

      setFeedback(response.data.created ? "工作区创建完成" : "工作区已连接");
      await onComplete();
    } catch (error) {
      setErrorMessage(`工作区初始化失败：${toMessage(error)}`);
    } finally {
      setIsInitializing(false);
    }
  }

  const hasCheckedRuntime = runtimeStatus !== null;
  const canContinue = workspacePath.trim().length > 0 && hasCheckedRuntime;
  const hasMissingDependency =
    runtimeStatus !== null &&
    [runtimeStatus.python, runtimeStatus.uv, runtimeStatus.manim, runtimeStatus.ffmpeg].some(
      (info) => info.status === "missing",
    );

  return (
    <div className={styles.screen}>
      <div className={styles.card}>
        <div className={styles.brandRow}>
          <span className={styles.brandMark} aria-hidden="true">
            M
          </span>
          <span className={styles.brandName}>LLM-Manim</span>
        </div>

        <header className={styles.header}>
          <h1 className={styles.title}>初始化工作区</h1>
          <p className={styles.summary}>
            项目、数据库、日志和渲染产物都会保存在这个目录下。M2 先建立标准结构和 SQLite 元数据。
          </p>
        </header>

        <section className={styles.section}>
          <div className={styles.sectionHead}>
            <span className={styles.sectionTitle}>工作区目录</span>
          </div>

          <div className={styles.pathRow}>
            <input
              type="text"
              value={workspacePath}
              readOnly
              className={styles.pathInput}
              placeholder="请选择目录"
              aria-label="工作区目录"
            />
            <button
              type="button"
              className={styles.secondaryButton}
              onClick={() => void browseDirectory()}
              disabled={isCheckingRuntime || isInitializing}
            >
              浏览
            </button>
          </div>

          <p className={styles.pathHint}>
            应用会在所选目录下创建 config、db、projects、jobs、artifacts、logs、temp 和 .runtime。
          </p>
        </section>

        <section className={styles.section}>
          <div className={styles.sectionHead}>
            <span className={styles.sectionTitle}>环境检查</span>
            <button
              type="button"
              className={styles.secondaryButton}
              onClick={() => void handleCheckRuntime()}
              disabled={isCheckingRuntime || isInitializing}
            >
              {isCheckingRuntime ? "检查中" : "检查环境"}
            </button>
          </div>

          <div className={styles.statusList} role="list">
            {RUNTIME_ITEMS.map((item) => {
              const componentInfo = isCheckingRuntime ? null : runtimeStatus?.[item.key];
              const status: DisplayStatus = isCheckingRuntime
                ? "checking"
                : componentInfo?.status ?? "idle";
              const version = componentInfo?.version;

              return (
                <div key={item.key} className={styles.statusItem} role="listitem">
                  <span className={styles.statusLabel}>{item.label}</span>
                  <div className={styles.statusMeta}>
                    {version ? (
                      <span className={styles.versionText}>{version}</span>
                    ) : null}
                    <span className={styles.statusText}>{labelForStatus(status)}</span>
                    {status === "checking" ? (
                      <span className={`${styles.statusIndicator} ${styles.statusChecking}`} />
                    ) : status === "ok" ? (
                      <span className={`${styles.statusIndicator} ${styles.statusOk}`}>✓</span>
                    ) : status === "missing" ? (
                      <span className={`${styles.statusIndicator} ${styles.statusMissing}`}>!</span>
                    ) : (
                      <span className={styles.statusIndicator}>·</span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>

          {hasCheckedRuntime && runtimeStatus !== null ? (
            <div
              className={`${styles.callout} ${
                hasMissingDependency ? styles.calloutWarn : styles.calloutSuccess
              }`}
            >
              {runtimeStatus.message}
            </div>
          ) : null}
        </section>

        {errorMessage ? (
          <div className={`${styles.callout} ${styles.calloutError}`}>{errorMessage}</div>
        ) : null}
        {feedback ? <div className={styles.feedback}>{feedback}</div> : null}

        <footer className={styles.footer}>
          <p className={styles.footerHint}>
            {!workspacePath.trim()
              ? "先选择一个工作区目录。"
              : !hasCheckedRuntime
                ? "运行一次环境检查后才能继续。"
                : hasMissingDependency
                  ? "检测到缺失依赖，可以继续初始化，后续在设置中修复。"
                  : "工作区和基础依赖都已准备好。"}
          </p>
          <button
            type="button"
            className={styles.primaryButton}
            onClick={() => void handleContinue()}
            disabled={!canContinue || isInitializing}
          >
            {isInitializing ? "初始化中" : "继续"}
          </button>
        </footer>
      </div>
    </div>
  );
}