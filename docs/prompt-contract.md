# LLM-Manim V1 Prompt 合同

## 1. 目标
本文定义 V1 Prompt 拼装结构和 LLM 输出协议。实现者不得在每个调用点临时拼 Prompt。

## 2. Prompt 结构
V1 使用固定结构：

```text
System Rules
ManimCE Rules
Output Contract
User Prompt
```

变量：

- `{user_prompt}`：用户输入的自然语言需求。
- `{target_resolution}`：固定为 `1280x720`。
- `{target_fps}`：固定为 `30`。
- `{scene_name}`：推荐为 `GeneratedScene`。

## 3. System Rules
系统规则必须表达：

- 你生成的是 Manim Community Edition Python 代码。
- 目标是教学动画，不是交互式应用。
- 只生成一个可渲染 Scene。
- 不要使用 ManimGL。
- 不要输出 shell 命令、文件路径、安装命令或解释性文字。
- 不要读写本地文件、访问网络、启动子进程或使用动态执行。

## 4. ManimCE Rules
默认注入精选 ManimCE 规则：

- 使用 `from manim import *`。
- 定义一个继承 `Scene`、`MovingCameraScene` 或 `ThreeDScene` 的类。
- 优先使用稳定的基础对象：`Text`、`MathTex`、`VGroup`、`Line`、`Arrow`、`Circle`、`Square`、`Axes`。
- 布局居中，控制对象数量，避免遮挡。
- 使用逐步揭示，避免一次性塞满画面。
- 每个关键步骤后使用短 `wait`。
- 使用 720p/30fps 目标，不在代码中设置输出目录。

可参考 `manim_skill` 中的 ManimCE 相关 best practices，但不得注入 ManimGL 规则。

## 5. Output Contract
模型必须只返回一个 Markdown Python 代码块：

````markdown
```python
from manim import *

class GeneratedScene(Scene):
    def construct(self):
        title = Text("示例")
        self.play(Write(title))
        self.wait(1)
```
````

规则：

- 代码块语言推荐 `python`。
- 不允许代码块外解释。
- 不允许多个代码块。
- 不允许 JSON 包裹代码。
- 不允许返回 shell 命令。
- 不允许指定输出路径、媒体目录或文件名。

## 6. User Prompt 包装
用户输入必须作为需求文本注入，不得直接作为系统规则拼接。

推荐包装：

```text
User animation request:
{user_prompt}

Create one concise ManimCE scene for this request.
```

若用户提示词包含“执行命令”“读取文件”“保存到某路径”等内容，Prompt 可以要求模型忽略这些执行指令；最终仍由静态校验兜底。

## 7. 有效输出
有效输出必须满足：

- 存在唯一 Markdown 代码块。
- 代码块内是 Python。
- 导入 ManimCE。
- 定义唯一 Scene 类。
- 不包含危险本地副作用。

## 8. 无效输出示例
无代码块：

```text
下面是代码：from manim import *
```

多代码块：

````markdown
```python
from manim import *
```

```bash
manim scene.py GeneratedScene
```
````

ManimGL：

```python
from manimlib import *

class Demo(InteractiveScene):
    pass
```

危险 API：

```python
import subprocess
subprocess.run(["dir"], shell=True)
```

这些输出必须在解析或静态校验阶段失败。
