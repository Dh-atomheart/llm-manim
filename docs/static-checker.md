# LLM-Manim V1 静态校验规格

## 1. 目标
本文定义 LLM Markdown 代码块提取后的静态校验。V1 校验目标是阻止明显不兼容、不安全或不可渲染的代码进入 Manim 执行阶段。

## 2. 输入与输出
输入：

- 已从 LLM 响应中提取的单个 Python 代码块字符串。

输出：

```json
{
  "ok": true,
  "scene_name": "GeneratedScene",
  "normalized_code": "..."
}
```

失败：

- 格式或代码块问题：`E_LLM_OUTPUT_INVALID`
- 代码结构、安全或 ManimCE 约束问题：`E_STATIC_CHECK_FAILED`

## 3. 校验流程
```text
code block text
-> size check
-> Python ast.parse
-> import validation
-> class validation
-> call/name validation
-> denylist scan
-> scene_name extraction
```

## 4. Python AST 校验
实现应使用受控 Python `ast` 解析脚本完成结构检查。Rust 后端调用该 checker 时：

- checker 脚本来自应用代码或 runtime 管理目录，不来自模型。
- 输入通过 stdin 或临时非执行文件传入。
- 输出 JSON。
- checker 失败不得进入 Manim 渲染。

## 5. 必须满足
- 代码可被 `ast.parse` 解析。
- 必须导入 ManimCE：推荐 `from manim import *`。
- 必须存在且只存在一个可渲染 Scene 类。
- Scene 类必须继承：
  - `Scene`
  - `MovingCameraScene`
  - `ThreeDScene`
- 必须能提取唯一 `SceneName` 给渲染命令使用。
- 代码长度不得超过配置阈值。

## 6. 必须拒绝
ManimGL：

- `from manimlib import *`
- `import manimlib`
- `InteractiveScene`
- `manimgl`

本地命令和子进程：

- `subprocess`
- `os.system`
- `os.popen`
- `shutil`
- `signal`

文件系统：

- `open`
- `Path`
- `pathlib`
- `glob`
- `tempfile`
- `os.remove`
- `os.unlink`
- `os.rmdir`
- `os.walk`

网络：

- `socket`
- `requests`
- `urllib`
- `httpx`

动态执行：

- `eval`
- `exec`
- `compile`
- `__import__`
- `importlib`
- `input`

其他：

- 绝对路径字符串，例如 Windows 盘符路径或以 `/` 开头的路径。
- shell 命令片段，例如 `manim `、`pip install`、`uv run`。

## 7. Denylist 与 AST 的关系
- AST 是主校验，用于识别 import、call、name、attribute 和 class inheritance。
- denylist 是补充，用于拦截字符串中隐藏的 shell、路径或 ManimGL 片段。
- denylist 命中时不得继续渲染。

## 8. 错误建议
静态校验失败时，普通用户提示应包含：

- 原因：生成代码包含不兼容或受限能力。
- 影响：无法安全渲染视频。
- 建议：改写提示词，要求“只使用 Manim Community Edition，生成单个 Scene，不使用文件/网络/命令”。

不得展示完整源码。

## 9. 测试用例
必须覆盖：

- 有效 ManimCE Scene。
- 无法解析的 Python。
- 无 Scene 类。
- 多个 Scene 类。
- ManimGL import。
- `InteractiveScene`。
- `subprocess`。
- `open(...)`。
- `requests`。
- `eval/exec`。
- 绝对路径字符串。
- shell 命令字符串。
