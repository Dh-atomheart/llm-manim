import type { RenderArtifact } from "../commands/types";
import styles from "./VideoPreview.module.css";

interface VideoPreviewProps {
  src: string | null;
  artifact: RenderArtifact | null;
  onOpenInExplorer: () => void;
  openDisabled?: boolean;
  emptyMessage?: string;
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

function fileName(filePath: string): string {
  return filePath.split("/").pop() ?? filePath;
}

export default function VideoPreview({
  src,
  artifact,
  onOpenInExplorer,
  openDisabled = false,
  emptyMessage = "任务完成后，可在此直接预览生成视频。",
}: VideoPreviewProps) {
  return (
    <div className={styles.panel}>
      {src ? (
        <div className={styles.frame}>
          <video
            className={styles.video}
            src={src}
            controls
            preload="metadata"
          />
        </div>
      ) : (
        <div className={styles.empty}>
          <span className={styles.icon} aria-hidden="true">
            VIDEO
          </span>
          <span>{emptyMessage}</span>
        </div>
      )}

      {artifact ? (
        <div className={styles.meta}>
          <div className={styles.metaList}>
            <span>{fileName(artifact.filePath)}</span>
            <span>{artifact.durationSecs.toFixed(1)}s</span>
            <span>{formatBytes(artifact.fileSizeBytes)}</span>
          </div>
          <button
            type="button"
            className={styles.action}
            onClick={onOpenInExplorer}
            disabled={openDisabled}
          >
            在文件管理器中打开
          </button>
        </div>
      ) : null}
    </div>
  );
}
