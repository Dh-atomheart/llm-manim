use std::{
    io,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use tokio::{
    fs,
    io::AsyncWriteExt,
    process::Command,
    time::{timeout, Duration},
};

use crate::services::manim_compat;
use crate::types::{
    error_codes::{E_DEP_MISSING, E_IO, E_STATIC_CHECK_FAILED},
    response::AppError,
};

const CHECKER_SCRIPT: &str = include_str!("../static_checker.py");
const STATIC_CHECK_TIMEOUT_SECS: u64 = 30;

#[derive(Debug)]
pub struct StaticCheckResult {
    pub scene_name: String,
    pub normalized_code: String,
}

#[derive(Debug, Deserialize)]
struct CheckerResponse {
    ok: bool,
    scene_name: Option<String>,
    normalized_code: Option<String>,
    error_code: Option<String>,
    reason: Option<String>,
}

pub async fn ensure_checker_script(workspace_root: &Path) -> Result<PathBuf, AppError> {
    let checks_dir = workspace_root.join(".runtime").join("checks");
    fs::create_dir_all(&checks_dir).await.map_err(io_error)?;

    let script_path = checks_dir.join("static_checker.py");
    fs::write(&script_path, CHECKER_SCRIPT)
        .await
        .map_err(io_error)?;
    let compat_dir = checks_dir
        .join("manimce")
        .join(manim_compat::MANIM_CE_TARGET_VERSION);
    fs::create_dir_all(&compat_dir).await.map_err(io_error)?;
    fs::write(
        compat_dir.join("api_manifest.json"),
        manim_compat::API_MANIFEST_JSON,
    )
    .await
    .map_err(io_error)?;
    fs::write(
        compat_dir.join("denylist.json"),
        manim_compat::DENYLIST_JSON,
    )
    .await
    .map_err(io_error)?;

    Ok(script_path)
}

pub async fn run_static_check(
    workspace_root: &Path,
    code: &str,
    strict_api_name_validation: bool,
) -> Result<StaticCheckResult, AppError> {
    let script_path = ensure_checker_script(workspace_root).await?;
    let mut child = Command::new("python")
        .arg(&script_path)
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env(
            "MANIM4LEARN_STRICT_API_NAMES",
            if strict_api_name_validation { "1" } else { "0" },
        )
        .env(
            "MANIM4LEARN_MANIMCE_COMPAT_DIR",
            script_path
                .parent()
                .expect("checker script has a parent directory")
                .join("manimce")
                .join(manim_compat::MANIM_CE_TARGET_VERSION),
        )
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(map_spawn_error)?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(code.as_bytes()).await.map_err(io_error)?;
    }

    let output = timeout(
        Duration::from_secs(STATIC_CHECK_TIMEOUT_SECS),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| AppError::new(E_STATIC_CHECK_FAILED, "静态校验超时", false))
    .and_then(|result| {
        result.map_err(|error| {
            AppError::new(
                E_STATIC_CHECK_FAILED,
                format!("静态校验进程执行失败: {error}"),
                false,
            )
        })
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "静态校验进程返回非零状态".to_string()
        } else {
            format!("静态校验进程失败: {stderr}")
        };
        return Err(AppError::new(E_STATIC_CHECK_FAILED, message, false));
    }

    let payload: CheckerResponse = serde_json::from_slice(&output.stdout).map_err(|error| {
        AppError::new(
            E_STATIC_CHECK_FAILED,
            format!("静态校验返回了无效结果: {error}"),
            false,
        )
    })?;

    if !payload.ok {
        return Err(AppError::new(
            payload
                .error_code
                .unwrap_or_else(|| E_STATIC_CHECK_FAILED.to_string()),
            payload
                .reason
                .unwrap_or_else(|| "静态校验未通过".to_string()),
            false,
        ));
    }

    let scene_name = payload
        .scene_name
        .ok_or_else(|| AppError::new(E_STATIC_CHECK_FAILED, "静态校验缺少 Scene 名称", false))?;
    let normalized_code = payload
        .normalized_code
        .ok_or_else(|| AppError::new(E_STATIC_CHECK_FAILED, "静态校验缺少规范化代码", false))?;

    Ok(StaticCheckResult {
        scene_name,
        normalized_code,
    })
}

fn io_error(error: io::Error) -> AppError {
    AppError::new(E_IO, format!("无法准备静态校验环境: {error}"), false)
}

fn map_spawn_error(error: io::Error) -> AppError {
    if error.kind() == io::ErrorKind::NotFound {
        return AppError::new(E_DEP_MISSING, "未检测到 Python，无法执行静态校验", false);
    }

    AppError::new(
        E_STATIC_CHECK_FAILED,
        format!("无法启动静态校验进程: {error}"),
        false,
    )
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn run_static_check_accepts_valid_manim_scene() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        title = Text('Demo')\n",
            "        self.play(Write(title))\n",
            "        self.wait()\n",
        );

        let result = run_static_check(&workspace_root, code, false)
            .await
            .unwrap();

        assert_eq!(result.scene_name, "Demo");
        assert!(result.normalized_code.contains("class Demo(Scene):"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_accepts_non_ascii_source() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        title = Text('姝ｅ鸡鍑芥暟')\n",
            "        self.play(Write(title))\n",
            "        self.wait()\n",
        );

        let result = run_static_check(&workspace_root, code, false)
            .await
            .unwrap();

        assert_eq!(result.scene_name, "Demo");
        assert!(result.normalized_code.contains("姝ｅ鸡鍑芥暟"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_rejects_manimgl_imports() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "from manimlib import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        self.wait()\n",
        );

        let error = run_static_check(&workspace_root, code, false)
            .await
            .unwrap_err();

        assert_eq!(error.code, E_STATIC_CHECK_FAILED);
        assert!(error
            .message
            .contains("denied text fragment `from manimlib import`"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_reports_denied_shell_command_fragment() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        self.wait()\n",
            "manim -pql generated_scene.py Demo\n",
        );

        let error = run_static_check(&workspace_root, code, false)
            .await
            .unwrap_err();

        assert_eq!(error.code, E_STATIC_CHECK_FAILED);
        assert!(error.message.contains("denied shell command line `manim`"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_allows_lowercase_path_helper_for_trajectory() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        def path(t):\n",
            "            return RIGHT * t + UP * t\n",
            "        dot = Dot(path(0))\n",
            "        self.add(dot)\n",
            "        self.wait()\n",
        );

        let result = run_static_check(&workspace_root, code, false)
            .await
            .unwrap();

        assert_eq!(result.scene_name, "Demo");

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_accepts_official_manifest_examples() {
        let cases = [
            (
                "graphing",
                concat!(
                    "from manim import *\n",
                    "import numpy as np\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        axes = Axes(x_range=[-3, 3, 1], y_range=[-2, 2, 1])\n",
                    "        graph = axes.plot(lambda x: np.sin(x), color=BLUE)\n",
                    "        label = MathTex(r'y = \\sin x')\n",
                    "        self.play(Create(axes), Create(graph), Write(label))\n",
                ),
            ),
            (
                "parametric function with tracker",
                concat!(
                    "from manim import *\n",
                    "import numpy as np\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        tracker = ValueTracker(0)\n",
                    "        curve = ParametricFunction(lambda t: np.array([np.cos(t), np.sin(t), 0]), t_range=[0, TAU], color=YELLOW)\n",
                    "        dot = always_redraw(lambda: Dot(curve.point_from_proportion(tracker.get_value()), color=RED))\n",
                    "        self.add(curve, dot)\n",
                    "        self.play(tracker.animate.set_value(1), run_time=2)\n",
                ),
            ),
            (
                "surface",
                concat!(
                    "from manim import *\n",
                    "import numpy as np\n",
                    "class Demo(ThreeDScene):\n",
                    "    def construct(self):\n",
                    "        surface = Surface(lambda u, v: np.array([u, v, np.sin(u) * np.cos(v)]), u_range=[-2, 2], v_range=[-2, 2])\n",
                    "        self.set_camera_orientation(phi=75 * DEGREES, theta=-45 * DEGREES)\n",
                    "        self.play(Create(surface))\n",
                ),
            ),
            (
                "lagged start",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        dots = VGroup(*[Dot() for _ in range(3)]).arrange(RIGHT)\n",
                    "        self.play(LaggedStart(*[GrowFromCenter(dot) for dot in dots], lag_ratio=0.1))\n",
                ),
            ),
        ];

        for (name, code) in cases {
            let workspace_root = setup_workspace_root().await;
            let result = run_static_check(&workspace_root, code, false).await;

            assert!(result.is_ok(), "case: {name}, result: {result:?}");

            cleanup(workspace_root).await;
        }
    }

    #[tokio::test]
    async fn run_static_check_allows_tip_length_on_arrow() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        arrow = Arrow(LEFT, RIGHT, tip_length=0.4)\n",
            "        self.add(arrow)\n",
        );

        let result = run_static_check(&workspace_root, code, false).await;

        assert!(result.is_ok(), "result: {result:?}");

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_allows_background_line_style_on_number_plane() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        plane = NumberPlane(background_line_style={'stroke_opacity': 0.2})\n",
            "        self.add(plane)\n",
        );

        let result = run_static_check(&workspace_root, code, false).await;

        assert!(result.is_ok(), "result: {result:?}");

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_allows_tex_math_with_explicit_delimiters() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        formula = Tex(r'$\\lambda = 2L$')\n",
            "        self.add(formula)\n",
        );

        let result = run_static_check(&workspace_root, code, false).await;

        assert!(result.is_ok(), "result: {result:?}");

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_only_applies_manifest_name_check_in_strict_mode() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        item = DefinitelyNotOfficialManimApi()\n",
            "        self.add(item)\n",
        );

        let relaxed = run_static_check(&workspace_root, code, false).await;
        let strict = run_static_check(&workspace_root, code, true)
            .await
            .unwrap_err();

        assert!(relaxed.is_ok(), "relaxed result: {relaxed:?}");
        assert_eq!(strict.code, E_STATIC_CHECK_FAILED);
        assert!(strict.message.contains("MANIM_API_UNSUPPORTED"));
        assert!(strict.message.contains("DefinitelyNotOfficialManimApi"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_allows_sorted_builtin_in_strict_mode() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        values = sorted([3, 1, 2])\n",
            "        dots = VGroup(*[Dot(RIGHT * value) for value in values])\n",
            "        self.add(dots)\n",
        );

        let result = run_static_check(&workspace_root, code, true).await;

        assert!(result.is_ok(), "strict result: {result:?}");

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_rejects_dangerous_api_calls() {
        let workspace_root = setup_workspace_root().await;
        let code = concat!(
            "from manim import *\n",
            "class Demo(Scene):\n",
            "    def construct(self):\n",
            "        open('secret.txt')\n",
            "        self.wait()\n",
        );

        let error = run_static_check(&workspace_root, code, false)
            .await
            .unwrap_err();

        assert_eq!(error.code, E_STATIC_CHECK_FAILED);
        assert!(error.message.contains("denied text fragment `open(`"));

        cleanup(workspace_root).await;
    }

    #[tokio::test]
    async fn run_static_check_rejects_denied_import_roots() {
        let cases = [
            (
                "import manimlib",
                concat!(
                    "from manim import *\n",
                    "import manimlib\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "import subprocess",
                concat!(
                    "from manim import *\n",
                    "import subprocess\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "import socket",
                concat!(
                    "from manim import *\n",
                    "import socket\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "import requests",
                concat!(
                    "from manim import *\n",
                    "import requests\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "import urllib",
                concat!(
                    "from manim import *\n",
                    "import urllib.request\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        self.wait()\n",
                ),
            ),
        ];

        for (name, code) in cases {
            let workspace_root = setup_workspace_root().await;
            let error = run_static_check(&workspace_root, code, false)
                .await
                .unwrap_err();

            assert_eq!(error.code, E_STATIC_CHECK_FAILED, "case: {name}");
            assert!(!error.message.is_empty(), "case: {name}");

            cleanup(workspace_root).await;
        }
    }

    #[tokio::test]
    async fn run_static_check_rejects_denied_identifiers_and_bases() {
        let cases = [
            (
                "InteractiveScene base class",
                concat!(
                    "from manim import *\n",
                    "class Demo(InteractiveScene):\n",
                    "    def construct(self):\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "Path call",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        Path('artifact.mp4')\n",
                    "        self.wait()\n",
                ),
            ),
        ];

        for (name, code) in cases {
            let workspace_root = setup_workspace_root().await;
            let error = run_static_check(&workspace_root, code, false)
                .await
                .unwrap_err();

            assert_eq!(error.code, E_STATIC_CHECK_FAILED, "case: {name}");
            assert!(!error.message.is_empty(), "case: {name}");

            cleanup(workspace_root).await;
        }
    }

    #[tokio::test]
    async fn run_static_check_rejects_known_manimce_compatibility_failures() {
        let cases = [
            (
                "Color",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        color = Color('#FF0000')\n",
                    "        dot = Dot(color=color)\n",
                    "        self.add(dot)\n",
                ),
                "Color",
            ),
            (
                "ParametricSurface",
                concat!(
                    "from manim import *\n",
                    "class Demo(ThreeDScene):\n",
                    "    def construct(self):\n",
                    "        surface = ParametricSurface(lambda u, v: [u, v, 0])\n",
                    "        self.add(surface)\n",
                ),
                "ParametricSurface",
            ),
            (
                "CYAN",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        dot = Dot(color=CYAN)\n",
                    "        self.add(dot)\n",
                ),
                "CYAN",
            ),
            (
                "MAGENTA",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        dot = Dot(color=MAGENTA)\n",
                    "        self.add(dot)\n",
                ),
                "MAGENTA",
            ),
            (
                "Sequence",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        values = Sequence()\n",
                    "        self.wait()\n",
                ),
                "Sequence",
            ),
            (
                "OpenGL-only class",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        group = OpenGLVGroup()\n",
                    "        self.add(group)\n",
                ),
                "OpenGLVGroup",
            ),
            (
                "unsupported import",
                concat!(
                    "from manim import *\n",
                    "import pandas\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        self.wait()\n",
                ),
                "MANIM_IMPORT_UNSUPPORTED",
            ),
            (
                "output config assignment",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        config.media_dir = 'custom'\n",
                    "        self.wait()\n",
                ),
                "CONFIG_OUTPUT_DENIED",
            ),
            (
                "Tex undelimited math",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        formula = Tex(r'\\lambda = 2L')\n",
                    "        self.add(formula)\n",
                ),
                "math delimiters",
            ),
            (
                "risky latex prose",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        formula = MathTex('涓枃璇存槑')\n",
                    "        self.add(formula)\n",
                ),
                "LATEX_RISKY_TEXT",
            ),
            (
                "Arrow3D tip_length",
                concat!(
                    "from manim import *\n",
                    "class Demo(ThreeDScene):\n",
                    "    def construct(self):\n",
                    "        arrow = Arrow3D(ORIGIN, OUT, tip_length=0.4)\n",
                    "        self.add(arrow)\n",
                ),
                "tip_length",
            ),
            (
                "Axes background_line_style",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        axes = Axes(background_line_style={'stroke_opacity': 0.2})\n",
                    "        self.add(axes)\n",
                ),
                "background_line_style",
            ),
            (
                "tip_length on non-tip mobject",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        item = Mobject(tip_length=0.4)\n",
                    "        self.add(item)\n",
                ),
                "tip_length",
            ),
            (
                "self.play list of animation builders",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        dot = Dot()\n",
                    "        self.play([dot.animate.shift(RIGHT)])\n",
                ),
                "self.play",
            ),
            (
                "AnimationGroup list of animation builders",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        dot = Dot()\n",
                    "        self.play(AnimationGroup([dot.animate.shift(RIGHT)]))\n",
                ),
                "AnimationGroup",
            ),
            (
                "len animation builder",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        dot = Dot()\n",
                    "        count = len(dot.animate.shift(RIGHT))\n",
                    "        self.wait()\n",
                ),
                "len",
            ),
        ];

        for (name, code, expected) in cases {
            let workspace_root = setup_workspace_root().await;
            let error = run_static_check(&workspace_root, code, false)
                .await
                .unwrap_err();

            assert_eq!(error.code, E_STATIC_CHECK_FAILED, "case: {name}");
            assert!(
                error.message.contains(expected),
                "case: {name}, message: {}",
                error.message
            );

            cleanup(workspace_root).await;
        }
    }

    #[tokio::test]
    async fn run_static_check_rejects_denied_calls() {
        let cases = [
            (
                "os.system",
                concat!(
                    "from manim import *\n",
                    "import os\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        os.system('dir')\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "eval",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        eval('1 + 1')\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "exec",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        exec('print(1)')\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "compile",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        compile('1 + 1', '<string>', 'eval')\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "__import__",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        __import__('math')\n",
                    "        self.wait()\n",
                ),
            ),
            (
                "input",
                concat!(
                    "from manim import *\n",
                    "class Demo(Scene):\n",
                    "    def construct(self):\n",
                    "        input('secret')\n",
                    "        self.wait()\n",
                ),
            ),
        ];

        for (name, code) in cases {
            let workspace_root = setup_workspace_root().await;
            let error = run_static_check(&workspace_root, code, false)
                .await
                .unwrap_err();

            assert_eq!(error.code, E_STATIC_CHECK_FAILED, "case: {name}");
            assert!(!error.message.is_empty(), "case: {name}");

            cleanup(workspace_root).await;
        }
    }

    async fn setup_workspace_root() -> PathBuf {
        let workspace_root =
            env::temp_dir().join(format!("manim4learn-static-check-{}", Uuid::new_v4()));
        fs::create_dir_all(&workspace_root).await.unwrap();
        workspace_root
    }

    async fn cleanup(workspace_root: PathBuf) {
        let _ = fs::remove_dir_all(workspace_root).await;
    }
}
