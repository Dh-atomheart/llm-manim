# Provider 配置手册

LLM-Manim 通过 Provider 调用外部 LLM 服务。当前支持两类协议：OpenAI-compatible 和 Anthropic-compatible。

## 配置字段

新增 Provider 时需要填写：

- 名称：本地显示名，例如 `OpenAI`、`DeepSeek`、`Claude`、`Local Proxy`。
- 类型：`OpenAI-compatible` 或 `Anthropic-compatible`。
- Base URL：API 服务基础地址。
- Model：模型名称。
- API Key：服务密钥。

API Key 当前存储在本地配置中。不要把 workspace、数据库或日志公开上传。

## OpenAI-compatible

适用于 OpenAI API 兼容服务。常见格式：

```text
Base URL: https://api.openai.com/v1
Model: gpt-4.1-mini
```

也可用于兼容 OpenAI Chat Completions 风格的第三方服务，例如某些代理、网关或本地模型服务。具体 `base_url` 和 `model` 以服务商文档为准。

## Anthropic-compatible

适用于 Anthropic 风格服务。常见字段仍是 base URL、model 和 API Key，但请求协议与 OpenAI-compatible 不同。选择类型时要与服务商协议一致。

## 测试连接

保存前或保存后可以测试 Provider。测试成功表示：

- Base URL 能访问。
- API Key 有效。
- Model 名称可用。
- 协议类型与服务端匹配。

测试失败时，优先检查错误摘要。

## 常见错误

- 401 / unauthorized：API Key 错误、过期，或没有权限访问对应模型。
- 404 / model not found：Model 名称错误，或 base URL 不属于该服务。
- timeout：网络不可达、代理配置问题、服务端响应太慢。
- invalid response：Provider 返回格式不符合当前协议类型，常见原因是类型选错。
- quota / rate limit：账号额度不足或请求过快。

## 使用建议

- 先用小模型或低成本模型测试配置是否可用。
- 为不同服务分别创建 Provider，不要频繁覆盖同一个配置。
- 如果编辑 Provider 时不输入新的 API Key，应用会尝试复用已保存的密钥。
- 删除 Provider 是软删除；历史任务仍可能显示“已删除 Provider”的引用信息。
