# References 打包说明

`references/` 是 LLM-Manim 的构建期知识库，包含 ManimCE API manifest、denylist、prompt skill、规则和模板。它对开发者和 CI 很重要，但 Release 安装包用户不需要单独下载或配置它。

## 当前引用方式

Rust 代码通过 `include_str!` 在编译时读取 `references/` 文件，例如：

```rust
include_str!("../../../references/manimce/0.20.1/api_manifest.json")
include_str!("../../../references/manimce/0.20.1/denylist.json")
include_str!("../../../references/skills/manim-composer/SKILL.md")
include_str!("../../../references/skills/manimce-best-practices/SKILL.md")
```

prompt 规则和模板也通过同样方式嵌入。

## 对 Release 用户意味着什么

用户下载并安装 MSI/NSIS 后：

- 不需要 `references/` 目录。
- 不需要自己构建或复制 `references/`。
- 程序仍可以使用这些规则、模板、manifest 和 denylist，因为它们已经在打包前编译进二进制。

## 对开发者意味着什么

从源码运行、测试、CI 或重新打包时必须保留 `references/`。如果缺失，常见错误包括：

```text
couldn't read references/manimce/0.20.1/api_manifest.json
couldn't read references/skills/manimce-best-practices/SKILL.md
```

这类错误会发生在编译期，不是运行期。

## 静态检查如何使用 manifest 和 denylist

后端编译时嵌入 `api_manifest.json` 和 `denylist.json`。运行静态检查时，Rust 后端会把这些已嵌入内容写入 workspace 的：

```text
.runtime/checks/manimce/0.20.1/
```

然后 Python 静态检查脚本读取该目录中的 JSON。也就是说，安装后的程序不是从安装目录读取源码 `references/`，而是使用二进制内嵌内容生成运行时检查文件。

## 修改 references 的流程

如果需要更新 ManimCE 规则、API manifest 或 prompt skill：

1. 修改 `references/` 中对应文件。
2. 运行 `cargo check`，确认 `include_str!` 路径仍正确。
3. 运行相关 Rust 测试或至少执行一次生成/静态检查流程。
4. 重新构建安装包。
5. 在 Release Notes 中说明规则或兼容性变化。

## 不要忽略 references

不要把整个 `references/` 加入 `.gitignore`。它不是临时文件，也不是用户数据，而是源码构建输入。

可以忽略的是本地生成目录，例如：

```gitignore
node_modules/
dist/
src-tauri/target/
.runtime/
```
