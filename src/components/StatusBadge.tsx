import type { JobState } from "../commands/types";
import styles from "./StatusBadge.module.css";

interface StatusBadgeProps {
  status: JobState;
  size?: "sm" | "md";
}

function labelForStatus(status: JobState): string {
  switch (status) {
    case "queued":
      return "排队中";
    case "running":
      return "运行中";
    case "succeeded":
      return "已完成";
    case "failed":
      return "失败";
    case "cancelled":
      return "已取消";
  }
}

export default function StatusBadge({ status, size = "md" }: StatusBadgeProps) {
  return (
    <span className={`${styles.badge} ${styles[size]} ${styles[status]}`}>
      {status === "running" ? (
        <span className={styles.spinner} aria-hidden="true" />
      ) : (
        <span className={styles.dot} aria-hidden="true" />
      )}
      <span>{labelForStatus(status)}</span>
    </span>
  );
}
