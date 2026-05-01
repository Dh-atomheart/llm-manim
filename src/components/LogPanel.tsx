import { useEffect, useMemo, useRef } from "react";

import type { JobLogEntry } from "../commands/types";
import styles from "./LogPanel.module.css";

const DATE_FORMATTER = new Intl.DateTimeFormat("zh-CN", {
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
});

interface LogPanelProps {
  entries: JobLogEntry[];
  open: boolean;
  onToggle: () => void;
  title?: string;
  emptyMessage?: string;
}

function formatDateTime(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return DATE_FORMATTER.format(parsed);
}

function stageLabel(stage: JobLogEntry["stage"]): string {
  switch (stage) {
    case "provider":
      return "PROVIDER";
    case "prompt":
      return "PROMPT";
    case "parse":
      return "PARSE";
    case "static_check":
      return "STATIC_CHECK";
    case "queue":
      return "QUEUE";
    case "render":
      return "RENDER";
    case "artifact":
      return "ARTIFACT";
    case "runtime":
      return "RUNTIME";
    case "user_action":
      return "USER_ACTION";
    case "security":
      return "SECURITY";
    case "workspace":
      return "WORKSPACE";
    case "llm":
      return "LLM";
  }
}

export default function LogPanel({
  entries,
  open,
  onToggle,
  title = "任务日志",
  emptyMessage = "当前任务还没有可显示的日志。",
}: LogPanelProps) {
  const bodyRef = useRef<HTMLDivElement | null>(null);
  const visibleEntries = useMemo(
    () => entries.filter((entry) => entry.level !== "debug"),
    [entries],
  );

  useEffect(() => {
    if (!open || !bodyRef.current) {
      return;
    }

    bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [open, visibleEntries.length]);

  return (
    <div className={styles.panel}>
      <div className={styles.header}>
        <div className={styles.titleRow}>
          <span className={styles.icon} aria-hidden="true">
            LOG
          </span>
          <h4 className={styles.title}>{title}</h4>
          <span className={styles.meta}>{visibleEntries.length} 条</span>
        </div>
        <button type="button" className={styles.toggle} onClick={onToggle}>
          {open ? "收起" : "展开"}
        </button>
      </div>

      {!open ? null : visibleEntries.length === 0 ? (
        <div className={styles.empty}>{emptyMessage}</div>
      ) : (
        <div ref={bodyRef} className={styles.body}>
          {visibleEntries.map((entry) => (
            <div key={entry.id} className={styles.row}>
              <div className={styles.rowMeta}>
                <span className={styles.stage}>{stageLabel(entry.stage)}</span>
                <span className={styles[entry.level]}>
                  {entry.level.toUpperCase()}
                </span>
                <time dateTime={entry.timestamp}>
                  {formatDateTime(entry.timestamp)}
                </time>
              </div>
              <div className={styles.message}>{entry.message}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
