use std::path::{Path, PathBuf};

use serde_json::Value;
use tokio::{fs, process::Command};

use crate::{
    services::redact,
    types::{
        error_codes::{E_ARTIFACT_INVALID, E_DEP_MISSING, E_IO},
        response::AppError,
    },
};

#[derive(Debug)]
pub struct ArtifactInfo {
    pub relative_path: String,
    pub duration_secs: f64,
    pub file_size_bytes: i64,
}

pub async fn check_artifact(
    workspace_root: &Path,
    project_id: &str,
    job_id: &str,
) -> Result<ArtifactInfo, AppError> {
    let media_root = workspace_root
        .join("artifacts")
        .join(project_id)
        .join(job_id)
        .join("media");
    let Some(source_mp4) = find_output_mp4(&media_root).await? else {
        return Err(AppError::new(
            E_ARTIFACT_INVALID,
            "渲染未生成 MP4 产物",
            false,
        ));
    };

    let relative_path = format!("artifacts/{project_id}/{job_id}/output.mp4");
    let target_mp4 = workspace_root.join(&relative_path);
    if let Some(parent) = target_mp4.parent() {
        fs::create_dir_all(parent).await.map_err(io_error)?;
    }

    if source_mp4 != target_mp4 {
        fs::copy(&source_mp4, &target_mp4).await.map_err(io_error)?;
    }

    let metadata = fs::metadata(&target_mp4).await.map_err(io_error)?;
    if metadata.len() < 1024 {
        return Err(AppError::new(
            E_ARTIFACT_INVALID,
            "生成的 MP4 文件为空或过小",
            false,
        ));
    }

    ensure_render_log_is_valid(workspace_root, job_id).await?;
    let duration_secs = check_duration_with_ffprobe(&target_mp4).await?;
    if duration_secs <= 0.0 {
        return Err(AppError::new(
            E_ARTIFACT_INVALID,
            "生成的 MP4 时长无效",
            false,
        ));
    }

    Ok(ArtifactInfo {
        relative_path,
        duration_secs,
        file_size_bytes: i64::try_from(metadata.len()).unwrap_or(i64::MAX),
    })
}

async fn find_output_mp4(root: &Path) -> Result<Option<PathBuf>, AppError> {
    if !fs::try_exists(root).await.map_err(io_error)? {
        return Ok(None);
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries = fs::read_dir(&dir).await.map_err(io_error)?;
        while let Some(entry) = entries.next_entry().await.map_err(io_error)? {
            let file_type = entry.file_type().await.map_err(io_error)?;
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            let is_mp4 = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("mp4"))
                .unwrap_or(false);
            if is_mp4 {
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

async fn ensure_render_log_is_valid(workspace_root: &Path, job_id: &str) -> Result<(), AppError> {
    let stderr_path = workspace_root
        .join("jobs")
        .join(job_id)
        .join("render_stderr.log");
    if !fs::try_exists(&stderr_path).await.map_err(io_error)? {
        return Ok(());
    }

    let stderr_bytes = fs::read(stderr_path).await.map_err(io_error)?;
    let stderr = String::from_utf8_lossy(&stderr_bytes);
    if stderr.to_ascii_lowercase().contains("fatal") {
        return Err(AppError::new(
            E_ARTIFACT_INVALID,
            "渲染日志包含 fatal 错误，产物校验失败",
            false,
        ));
    }

    Ok(())
}

async fn check_duration_with_ffprobe(mp4_path: &Path) -> Result<f64, AppError> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("json")
        .arg(mp4_path)
        .output()
        .await
        .map_err(map_ffprobe_error)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AppError::new(
            E_ARTIFACT_INVALID,
            format!(
                "无法读取 MP4 时长: {}",
                redact::truncate(&stderr.replace('\n', " "), 200)
            ),
            false,
        ));
    }

    let payload: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        AppError::new(
            E_ARTIFACT_INVALID,
            format!("ffprobe 输出不是有效 JSON: {error}"),
            false,
        )
    })?;

    let Some(duration_value) = payload
        .get("format")
        .and_then(|format| format.get("duration"))
    else {
        return Err(AppError::new(
            E_ARTIFACT_INVALID,
            "ffprobe 未返回可用时长",
            false,
        ));
    };

    if let Some(duration_text) = duration_value.as_str() {
        return duration_text.parse::<f64>().map_err(|error| {
            AppError::new(
                E_ARTIFACT_INVALID,
                format!("MP4 时长格式无效: {error}"),
                false,
            )
        });
    }

    duration_value.as_f64().ok_or_else(|| {
        AppError::new(
            E_ARTIFACT_INVALID,
            "ffprobe 返回的时长字段既不是字符串也不是数字",
            false,
        )
    })
}

fn io_error(error: std::io::Error) -> AppError {
    AppError::new(E_IO, format!("无法读取或写入产物文件: {error}"), false)
}

fn map_ffprobe_error(error: std::io::Error) -> AppError {
    if error.kind() == std::io::ErrorKind::NotFound {
        return AppError::new(E_DEP_MISSING, "未检测到 ffprobe，无法校验 MP4 时长", false);
    }

    AppError::new(
        E_ARTIFACT_INVALID,
        format!("无法启动 ffprobe: {error}"),
        false,
    )
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn check_artifact_rejects_missing_mp4_output() {
        let workspace_root = setup_workspace_root().await;

        let error = check_artifact(&workspace_root, "project_a", "job_a")
            .await
            .unwrap_err();

        assert_eq!(error.code, E_ARTIFACT_INVALID);
        assert!(error.message.contains("未生成 MP4"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn ensure_render_log_is_valid_accepts_non_utf8_log() {
        let workspace_root = setup_workspace_root().await;
        let job_dir = workspace_root.join("jobs").join("job_non_utf8");

        fs::create_dir_all(&job_dir).await.unwrap();
        fs::write(
            job_dir.join("render_stderr.log"),
            [b'W', b'a', b'r', b'n', b'i', b'n', b'g', b':', 0xFF, 0xFE],
        )
        .await
        .unwrap();

        ensure_render_log_is_valid(&workspace_root, "job_non_utf8")
            .await
            .unwrap();

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn ensure_render_log_is_valid_rejects_non_utf8_log_with_fatal_marker() {
        let workspace_root = setup_workspace_root().await;
        let job_dir = workspace_root.join("jobs").join("job_non_utf8_fatal");

        fs::create_dir_all(&job_dir).await.unwrap();
        fs::write(
            job_dir.join("render_stderr.log"),
            [
                b'F', b'A', b'T', b'A', b'L', b':', b' ', 0xFF, b'r', b'e', b'n', b'd', b'e', b'r',
            ],
        )
        .await
        .unwrap();

        let error = ensure_render_log_is_valid(&workspace_root, "job_non_utf8_fatal")
            .await
            .unwrap_err();

        assert_eq!(error.code, E_ARTIFACT_INVALID);
        assert!(error.message.contains("fatal"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn ensure_render_log_is_valid_rejects_fatal_marker() {
        let workspace_root = setup_workspace_root().await;
        let job_dir = workspace_root.join("jobs").join("job_fatal");

        fs::create_dir_all(&job_dir).await.unwrap();
        fs::write(job_dir.join("render_stderr.log"), "FATAL: renderer crashed")
            .await
            .unwrap();

        let error = ensure_render_log_is_valid(&workspace_root, "job_fatal")
            .await
            .unwrap_err();

        assert_eq!(error.code, E_ARTIFACT_INVALID);
        assert!(error.message.contains("fatal"));

        cleanup(workspace_root).await;
    }

    async fn setup_workspace_root() -> PathBuf {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-artifact-tests-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace_root).await.unwrap();
        workspace_root
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = fs::remove_dir_all(workspace_root).await;
    }
}
