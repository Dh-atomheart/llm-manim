# LLM-Manim V1 Provider 协议规格

## 1. 目标
本文定义 V1 Provider 调用、连接测试、错误映射和日志脱敏。Provider 调用只发生在 Rust 后端，统一使用 `reqwest`。

## 2. Provider 类型
V1 只支持：

```text
openai_compatible | anthropic_compatible
```

DeepSeek 使用 `openai_compatible`，不新增 `deepseek` 类型。

## 3. 通用配置
ProviderConfig 至少包含：

- `id`
- `provider_type`
- `base_url`
- `api_key`
- `model`
- `created_at`
- `updated_at`

规则：

- `api_key` V1 明文存储，但不得通过列表接口返回。
- `base_url` 必须是 `https://` 或用户明确选择的本地测试地址。
- `model` 不由应用自动猜测，用户必须显式填写或选择。
- Provider 请求不得包含本地工具执行能力。

## 4. OpenAI-compatible
### 4.1 生成请求
默认 endpoint：

```text
POST {base_url}/chat/completions
```

请求体：

```json
{
  "model": "deepseek-v4-pro",
  "messages": [
    { "role": "system", "content": "..." },
    { "role": "user", "content": "..." }
  ],
  "temperature": 0.2,
  "stream": false
}
```

响应解析：

- 优先读取 `choices[0].message.content`。
- 若字段缺失、为空或不是字符串，返回 `E_PROVIDER_RESPONSE_INVALID`。
- 返回内容不在 Provider 层解析代码块，代码块解析属于 Prompt/Parse 阶段。

### 4.2 DeepSeek 示例
DeepSeek 首个验收配置：

```json
{
  "provider_type": "openai_compatible",
  "base_url": "https://api.deepseek.com",
  "model": "deepseek-v4-pro"
}
```

实际模型名以用户账号可用模型为准；连接测试失败时不得自动替换模型。

## 5. Anthropic-compatible
### 5.1 生成请求
默认 endpoint：

```text
POST {base_url}/messages
```

请求体：

```json
{
  "model": "claude-compatible-model",
  "system": "...",
  "messages": [
    { "role": "user", "content": "..." }
  ],
  "max_tokens": 4096,
  "temperature": 0.2
}
```

响应解析：

- 优先读取第一个文本 content block。
- 若没有文本 block，返回 `E_PROVIDER_RESPONSE_INVALID`。

## 6. 连接测试
连接测试使用最小请求，不保存配置：

- OpenAI-compatible：发送一条要求返回短文本的 chat completion。
- Anthropic-compatible：发送一条要求返回短文本的 messages 请求。

成功条件：

- HTTP 状态为成功。
- 响应能解析出非空文本。
- 文本内容不需要符合 Manim 输出协议。

失败条件：

- 401/403：`E_AUTH_401`
- 网络连接失败或 DNS 失败：`E_PROVIDER_ERROR`
- 超时：`E_NET_TIMEOUT`
- 响应结构不符合预期：`E_PROVIDER_RESPONSE_INVALID`

## 7. 超时与重试
- 连接测试默认超时：30 秒。
- 生成请求默认超时：180 秒。
- V1 不做自动重试。
- 用户可在失败后手动重试任务或重新测试连接。

## 8. 错误映射
```text
401/403 -> E_AUTH_401
timeout -> E_NET_TIMEOUT
non-2xx provider response -> E_PROVIDER_ERROR
invalid response schema -> E_PROVIDER_RESPONSE_INVALID
empty model content -> E_PROVIDER_RESPONSE_INVALID
```

错误 `details` 可包含：

- provider_type
- http_status
- provider_error_type
- request_id
- sanitized_body_excerpt

错误 `details` 不得包含：

- API Key
- Authorization header
- 完整请求体中的 secret 字段

## 9. 日志脱敏
写入日志前必须脱敏：

- `Authorization: Bearer ...`
- `api_key`
- `x-api-key`
- `secret`
- 用户保存的明文 Key

Provider 原始响应只允许记录摘要或截断后的脱敏片段。
