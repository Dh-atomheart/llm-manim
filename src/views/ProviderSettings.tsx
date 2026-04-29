import { useEffect, useMemo, useState } from "react";

import {
  deleteProviderConfig,
  listProviderConfigs,
  saveProviderConfig,
  testProviderConfig,
} from "../commands/provider";
import type {
  ProviderSummary,
  ProviderType,
  SaveProviderConfigInput,
  TestProviderConfigInput,
} from "../commands/types";
import styles from "./ProviderSettings.module.css";

type TestState =
  | { status: "idle" }
  | { status: "testing"; message: string }
  | { status: "ok"; message: string }
  | { status: "failed"; message: string };

type ProviderFormState = {
  name: string;
  providerType: ProviderType;
  baseUrl: string;
  model: string;
  apiKey: string;
};

const EMPTY_FORM: ProviderFormState = {
  name: "",
  providerType: "openai_compatible",
  baseUrl: "",
  model: "",
  apiKey: "",
};

const IDLE_TEST_STATE: TestState = { status: "idle" };

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function providerTypeLabel(providerType: ProviderType): string {
  return providerType === "openai_compatible" ? "OpenAI-compatible" : "Anthropic-compatible";
}

function isFilled(value: string): boolean {
  return value.trim().length > 0;
}

function statusClassName(status: TestState["status"]): string {
  switch (status) {
    case "testing":
      return styles.statusTesting;
    case "ok":
      return styles.statusOk;
    case "failed":
      return styles.statusFailed;
    default:
      return "";
  }
}

export default function ProviderSettings() {
  const [providers, setProviders] = useState<ProviderSummary[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isAdding, setIsAdding] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [deleteConfirmId, setDeleteConfirmId] = useState<string | null>(null);
  const [form, setForm] = useState<ProviderFormState>(EMPTY_FORM);
  const [showKey, setShowKey] = useState(false);
  const [confirmPlaintextRisk, setConfirmPlaintextRisk] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [infoMessage, setInfoMessage] = useState<string | null>(null);
  const [testStates, setTestStates] = useState<Record<string, TestState>>({});

  const activeForm = isAdding || editingId !== null;
  const canReuseStoredKey = editingId !== null;
  const hasApiKeyInput = isFilled(form.apiKey);
  const requiresRiskConfirmation = hasApiKeyInput;
  const canSave =
    isFilled(form.name) &&
    isFilled(form.baseUrl) &&
    isFilled(form.model) &&
    (hasApiKeyInput || canReuseStoredKey) &&
    (!requiresRiskConfirmation || confirmPlaintextRisk);

  const activeTestState = useMemo(() => {
    if (editingId) {
      return testStates[editingId] ?? IDLE_TEST_STATE;
    }

    if (isAdding) {
      return testStates.draft ?? IDLE_TEST_STATE;
    }

    return IDLE_TEST_STATE;
  }, [editingId, isAdding, testStates]);

  useEffect(() => {
    void reloadProviders();
  }, []);

  async function reloadProviders() {
    setIsLoading(true);

    try {
      const response = await listProviderConfigs();
      if (!response.ok) {
        setErrorMessage(response.error.message);
        return;
      }

      setProviders(response.data);
    } catch (error) {
      setErrorMessage(`无法读取 Provider 列表：${toMessage(error)}`);
    } finally {
      setIsLoading(false);
    }
  }

  function resetForm() {
    setForm(EMPTY_FORM);
    setShowKey(false);
    setConfirmPlaintextRisk(false);
  }

  function startAdd() {
    setIsAdding(true);
    setEditingId(null);
    setDeleteConfirmId(null);
    setErrorMessage(null);
    setInfoMessage(null);
    resetForm();
  }

  function startEdit(provider: ProviderSummary) {
    setEditingId(provider.id);
    setIsAdding(false);
    setDeleteConfirmId(null);
    setErrorMessage(null);
    setInfoMessage(null);
    setShowKey(false);
    setConfirmPlaintextRisk(false);
    setForm({
      name: provider.name,
      providerType: provider.providerType,
      baseUrl: provider.baseUrl,
      model: provider.model,
      apiKey: "",
    });
  }

  function cancelForm() {
    setIsAdding(false);
    setEditingId(null);
    setErrorMessage(null);
    resetForm();
  }

  function patchForm<Key extends keyof ProviderFormState>(key: Key, value: ProviderFormState[Key]) {
    if (key === "apiKey") {
      setConfirmPlaintextRisk(false);
    }

    setForm((current) => ({ ...current, [key]: value }));
  }

  async function handleSave() {
    if (!canSave) {
      return;
    }

    setIsSaving(true);
    setErrorMessage(null);
    setInfoMessage(null);

    const payload: SaveProviderConfigInput = {
      id: editingId ?? undefined,
      name: form.name.trim(),
      providerType: form.providerType,
      baseUrl: form.baseUrl.trim(),
      model: form.model.trim(),
      apiKey: form.apiKey.trim() || undefined,
    };

    try {
      const response = await saveProviderConfig(payload);
      if (!response.ok) {
        setErrorMessage(response.error.message);
        return;
      }

      await reloadProviders();
      setInfoMessage(editingId ? "Provider 已更新。" : "Provider 已添加。")
      ;
      cancelForm();
    } catch (error) {
      setErrorMessage(`保存 Provider 失败：${toMessage(error)}`);
    } finally {
      setIsSaving(false);
    }
  }

  async function runTest(key: string, input: TestProviderConfigInput) {
    setTestStates((current) => ({
      ...current,
      [key]: { status: "testing", message: "连接测试中" },
    }));
    setErrorMessage(null);

    try {
      const response = await testProviderConfig(input);
      if (!response.ok) {
        const message = `${response.error.message} (${response.error.code})`;
        setTestStates((current) => ({
          ...current,
          [key]: { status: "failed", message },
        }));
        return;
      }

      setTestStates((current) => ({
        ...current,
        [key]: { status: "ok", message: response.data.message },
      }));
    } catch (error) {
      setTestStates((current) => ({
        ...current,
        [key]: { status: "failed", message: `连接测试失败：${toMessage(error)}` },
      }));
    }
  }

  async function handleFormTest() {
    const key = editingId ?? "draft";

    await runTest(key, {
      id: editingId ?? undefined,
      providerType: form.providerType,
      baseUrl: form.baseUrl.trim() || undefined,
      model: form.model.trim() || undefined,
      apiKey: form.apiKey.trim() || undefined,
    });
  }

  async function handleRowTest(provider: ProviderSummary) {
    await runTest(provider.id, { id: provider.id });
  }

  async function handleDelete(id: string) {
    if (deleteConfirmId !== id) {
      setDeleteConfirmId(id);
      return;
    }

    setDeletingId(id);
    setErrorMessage(null);
    setInfoMessage(null);

    try {
      const response = await deleteProviderConfig(id);
      if (!response.ok) {
        setErrorMessage(response.error.message);
        return;
      }

      if (editingId === id) {
        cancelForm();
      }

      await reloadProviders();
      setDeleteConfirmId(null);
      setInfoMessage("Provider 已删除。")
      ;
    } catch (error) {
      setErrorMessage(`删除 Provider 失败：${toMessage(error)}`);
    } finally {
      setDeletingId(null);
    }
  }

  return (
    <div className={styles.page}>
      <div className={styles.container}>
        <div className={styles.stack}>
          <div className={styles.headerRow}>
            <div className={styles.headerCopy}>
              <h2>模型 Provider</h2>
              <p>配置 OpenAI-compatible 或 Anthropic-compatible 服务，并在保存前完成本地连接验证。</p>
            </div>

            <button
              type="button"
              className={styles.actionButton}
              onClick={startAdd}
              disabled={activeForm}
            >
              <svg className={styles.buttonIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                <path d="M8 3v10M3 8h10" />
              </svg>
              添加
            </button>
          </div>

          <div className={styles.warningBanner} role="alert">
            <svg className={styles.warningIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
              <path d="M8 2.5l5.5 10H2.5L8 2.5z" />
              <path d="M8 6v3.25" />
              <circle cx="8" cy="11.5" r=".5" fill="currentColor" stroke="none" />
            </svg>
            <div className={styles.warningCopy}>
              <strong>安全提示：</strong> API Key 会以明文存储在本机工作区数据库中。请避免在共享设备上配置高权限密钥，也不要把工作区同步到公开云盘。
            </div>
          </div>

          {errorMessage ? <div className={styles.errorBanner}>{errorMessage}</div> : null}
          {infoMessage ? <div className={styles.infoBanner}>{infoMessage}</div> : null}

          {isLoading ? (
            <div className={styles.emptyBox}>正在读取 Provider 配置…</div>
          ) : providers.length === 0 && !isAdding ? (
            <div className={styles.emptyBox}>
              <div className={styles.emptyState}>暂无 Provider 配置。点击“添加”开始配置。</div>
            </div>
          ) : null}

          {providers.length > 0 ? (
            <div className={styles.list}>
              {providers.map((provider) => {
                const isEditing = editingId === provider.id;
                const testState = testStates[provider.id] ?? IDLE_TEST_STATE;

                return (
                  <div key={provider.id} className={styles.rowWrap}>
                    <div className={`${styles.row} ${isEditing ? styles.rowActive : ""}`}>
                      <div className={styles.rowHeader}>
                        <div className={styles.rowMeta}>
                          <div className={styles.rowTitleLine}>
                            <span className={styles.rowTitle}>{provider.name}</span>
                            <span className={styles.typeTag}>{providerTypeLabel(provider.providerType)}</span>
                          </div>
                          <p className={styles.rowCode}>{provider.baseUrl}</p>
                          <p className={styles.rowCode}>{provider.model}</p>
                        </div>

                        <div className={styles.rowActions}>
                          {testState.status !== "idle" ? (
                            <span className={`${styles.statusInline} ${statusClassName(testState.status)}`}>
                              {testState.status === "testing" ? (
                                <svg className={`${styles.statusIcon} ${styles.spin}`} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                                  <path d="M13 8A5 5 0 103.5 10.3" />
                                </svg>
                              ) : testState.status === "ok" ? (
                                <svg className={styles.statusIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                                  <path d="M3 8.5l3 3 7-7" />
                                </svg>
                              ) : (
                                <svg className={styles.statusIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                                  <path d="M4 4l8 8M12 4l-8 8" />
                                </svg>
                              )}
                              <span className={styles.statusText}>{testState.message}</span>
                            </span>
                          ) : null}

                          <button
                            type="button"
                            className={styles.testButton}
                            onClick={() => void handleRowTest(provider)}
                            disabled={activeForm || deletingId === provider.id || testState.status === "testing"}
                          >
                            <svg className={styles.buttonIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                              <path d="M2.5 8h11" />
                              <path d="M10 5.5L13.5 8 10 10.5" />
                            </svg>
                            测试
                          </button>
                          <button
                            type="button"
                            className={styles.iconButton}
                            onClick={() => startEdit(provider)}
                            disabled={activeForm && !isEditing}
                            aria-label={`编辑 ${provider.name}`}
                          >
                            <svg className={styles.buttonIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                              <path d="M3 11.75l.5-2.5L10.75 2l2.25 2.25L5.75 11.5 3 11.75z" />
                            </svg>
                          </button>
                          <button
                            type="button"
                            className={`${styles.dangerButton} ${deleteConfirmId === provider.id ? styles.dangerButtonConfirm : ""}`}
                            onClick={() => void handleDelete(provider.id)}
                            disabled={deletingId === provider.id || (activeForm && !isEditing)}
                          >
                            <svg className={styles.buttonIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                              <path d="M3.5 4.5h9" />
                              <path d="M6 4.5V3.25h4V4.5" />
                              <path d="M5 6.5v5" />
                              <path d="M8 6.5v5" />
                              <path d="M11 6.5v5" />
                              <path d="M4.5 4.5l.5 8.5h6l.5-8.5" />
                            </svg>
                            {deleteConfirmId === provider.id ? "确认删除" : "删除"}
                          </button>
                        </div>
                      </div>
                    </div>

                    {isEditing ? (
                      <ProviderForm
                        form={form}
                        setForm={patchForm}
                        showKey={showKey}
                        onToggleKey={() => setShowKey((current) => !current)}
                        onCancel={cancelForm}
                        onSave={() => void handleSave()}
                        onTest={() => void handleFormTest()}
                        saveLabel="保存更改"
                        isSaving={isSaving}
                        testState={activeTestState}
                        canSave={canSave}
                        confirmPlaintextRisk={confirmPlaintextRisk}
                        onConfirmPlaintextRiskChange={setConfirmPlaintextRisk}
                        canReuseStoredKey={true}
                      />
                    ) : null}
                  </div>
                );
              })}
            </div>
          ) : null}

          {isAdding ? (
            <div className={styles.list}>
              <div className={styles.row}>
                <div className={styles.rowTitle}>新增 Provider</div>
              </div>
              <ProviderForm
                form={form}
                setForm={patchForm}
                showKey={showKey}
                onToggleKey={() => setShowKey((current) => !current)}
                onCancel={cancelForm}
                onSave={() => void handleSave()}
                onTest={() => void handleFormTest()}
                saveLabel="添加 Provider"
                isSaving={isSaving}
                testState={activeTestState}
                canSave={canSave}
                confirmPlaintextRisk={confirmPlaintextRisk}
                onConfirmPlaintextRiskChange={setConfirmPlaintextRisk}
                canReuseStoredKey={false}
              />
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

interface ProviderFormProps {
  form: ProviderFormState;
  setForm: <Key extends keyof ProviderFormState>(key: Key, value: ProviderFormState[Key]) => void;
  showKey: boolean;
  onToggleKey: () => void;
  onCancel: () => void;
  onSave: () => void;
  onTest: () => void;
  saveLabel: string;
  isSaving: boolean;
  testState: TestState;
  canSave: boolean;
  confirmPlaintextRisk: boolean;
  onConfirmPlaintextRiskChange: (value: boolean) => void;
  canReuseStoredKey: boolean;
}

function ProviderForm({
  form,
  setForm,
  showKey,
  onToggleKey,
  onCancel,
  onSave,
  onTest,
  saveLabel,
  isSaving,
  testState,
  canSave,
  confirmPlaintextRisk,
  onConfirmPlaintextRiskChange,
  canReuseStoredKey,
}: ProviderFormProps) {
  const showRiskConfirmation = isFilled(form.apiKey);

  return (
    <div className={styles.formShell}>
      <div className={styles.formGrid}>
        <div className={styles.field}>
          <label className={styles.label}>名称</label>
          <input
            type="text"
            className={styles.input}
            value={form.name}
            onChange={(event) => setForm("name", event.target.value)}
            placeholder="DeepSeek / Anthropic / OpenAI"
          />
        </div>

        <div className={styles.field}>
          <label className={styles.label}>类型</label>
          <div className={styles.selectWrap}>
            <select
              className={styles.select}
              value={form.providerType}
              onChange={(event) => setForm("providerType", event.target.value as ProviderType)}
            >
              <option value="openai_compatible">OpenAI-compatible</option>
              <option value="anthropic_compatible">Anthropic-compatible</option>
            </select>
            <svg className={styles.selectIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
              <path d="M4 6l4 4 4-4" />
            </svg>
          </div>
        </div>

        <div className={styles.fieldFull}>
          <label className={styles.label}>Base URL</label>
          <input
            type="url"
            className={`${styles.input} ${styles.monoInput}`}
            value={form.baseUrl}
            onChange={(event) => setForm("baseUrl", event.target.value)}
            placeholder={form.providerType === "openai_compatible" ? "https://api.deepseek.com 或 https://api.openai.com/v1" : "https://api.anthropic.com"}
          />
          <div className={styles.hintText}>
            {form.providerType === "openai_compatible"
              ? "DeepSeek 请使用 OpenAI-compatible；连接测试会发送最小 chat completion 请求。"
              : "Anthropic-compatible 会发送最小 messages 请求。"}
          </div>
        </div>

        <div className={styles.fieldFull}>
          <label className={styles.label}>模型 ID</label>
          <input
            type="text"
            className={`${styles.input} ${styles.monoInput}`}
            value={form.model}
            onChange={(event) => setForm("model", event.target.value)}
            placeholder={form.providerType === "openai_compatible" ? "deepseek-v3 / gpt-4o" : "claude-3-5-sonnet-20241022"}
          />
        </div>

        <div className={styles.fieldFull}>
          <label className={styles.label}>API Key</label>
          <div className={styles.keyWrap}>
            <input
              type={showKey ? "text" : "password"}
              className={`${styles.keyInput} ${styles.monoInput}`}
              value={form.apiKey}
              onChange={(event) => setForm("apiKey", event.target.value)}
              placeholder={canReuseStoredKey ? "留空则继续使用已保存的 API Key" : "sk-..."}
              aria-label="API Key"
            />
            <button type="button" className={styles.keyToggle} onClick={onToggleKey} aria-label={showKey ? "隐藏 API Key" : "显示 API Key"}>
              {showKey ? (
                <svg className={styles.buttonIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                  <path d="M2 2l12 12" />
                  <path d="M6.1 6.1A3 3 0 019.9 9.9" />
                  <path d="M3.3 6.2C4.7 4.6 6.3 3.8 8 3.8c2.5 0 4.8 1.8 6.7 4.2-.7.9-1.4 1.6-2.1 2.2" />
                  <path d="M6.4 12.1c.5.1 1 .1 1.6.1 2.5 0 4.8-1.8 6.7-4.2-.5-.7-1-1.3-1.6-1.8" />
                </svg>
              ) : (
                <svg className={styles.buttonIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                  <path d="M1.8 8s2.4-4.2 6.2-4.2S14.2 8 14.2 8 11.8 12.2 8 12.2 1.8 8 1.8 8z" />
                  <circle cx="8" cy="8" r="2" />
                </svg>
              )}
            </button>
          </div>
          {canReuseStoredKey ? (
            <div className={styles.hintText}>列表接口不会返回已保存的 Key。若无需更新，可留空。</div>
          ) : null}
        </div>
      </div>

      {showRiskConfirmation ? (
        <label className={styles.confirmRow}>
          <input
            type="checkbox"
            className={styles.checkbox}
            checked={confirmPlaintextRisk}
            onChange={(event) => onConfirmPlaintextRiskChange(event.target.checked)}
          />
          <span>我已知晓 API Key 将以明文保存到本机工作区数据库中，并确认当前设备适合存放该密钥。</span>
        </label>
      ) : null}

      <div className={styles.formActions}>
        <div>
          {testState.status !== "idle" ? (
            <span className={`${styles.statusInline} ${statusClassName(testState.status)}`}>
              {testState.status === "testing" ? (
                <svg className={`${styles.statusIcon} ${styles.spin}`} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                  <path d="M13 8A5 5 0 103.5 10.3" />
                </svg>
              ) : testState.status === "ok" ? (
                <svg className={styles.statusIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                  <path d="M3 8.5l3 3 7-7" />
                </svg>
              ) : (
                <svg className={styles.statusIcon} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
                  <path d="M4 4l8 8M12 4l-8 8" />
                </svg>
              )}
              <span className={styles.statusText}>{testState.message}</span>
            </span>
          ) : null}
        </div>

        <div className={styles.formActionsRight}>
          <button type="button" className={styles.secondaryButton} onClick={onTest} disabled={testState.status === "testing" || isSaving}>
            测试连接
          </button>
          <button type="button" className={styles.ghostButton} onClick={onCancel} disabled={isSaving}>
            取消
          </button>
          <button type="button" className={styles.actionButton} onClick={onSave} disabled={!canSave || isSaving}>
            {isSaving ? "保存中" : saveLabel}
          </button>
        </div>
      </div>
    </div>
  );
}