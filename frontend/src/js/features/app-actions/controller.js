import { $ } from "../../dom.js";
import { buildJobsEndpoint } from "../../network.js";
import { getOcrProviderDefinition, normalizeOcrProvider } from "../../provider-config.js";

export function mountAppActionsFeature({
  state,
  apiBase,
  apiPrefix,
  buildApiEndpoint,
  isMockMode,
  openSetupDialog,
  renderJob,
  setText,
  submitJson,
  submitJobRequest,
  saveDesktopConfig,
  setDesktopBusy,
  openDesktopOutputDirectory,
  resetUploadedFile,
  currentWorkflow,
  workflowNeedsCredentials,
  workflowNeedsUpload,
  currentRenderSourceJobId,
  collectRunPayload,
  getBrowserCredentialsFeature,
  getJobRuntimeFeature,
  onDesktopConfigSaved,
}) {
  function currentDesktopSetupProvider() {
    return normalizeOcrProvider($("setup-ocr-provider")?.value || $("ocr_provider")?.value || "mineru");
  }

  function syncDesktopSetupProviderUi() {
    const provider = currentDesktopSetupProvider();
    const select = $("setup-ocr-provider");
    if (!select) {
      return;
    }
    select.value = provider;
    document.querySelectorAll("[data-setup-provider-panel]").forEach((panel) => {
      panel.hidden = panel.dataset.setupProviderPanel !== provider;
    });
  }

  function isMissingUploadError(error) {
    const message = `${error?.message || error || ""}`;
    return message.includes("upload not found");
  }

  function handleMissingUploadError() {
    state.uploadId = "";
    state.uploadedFileName = "";
    state.uploadedPageCount = 0;
    state.uploadedBytes = 0;
    resetUploadedFile?.();
    setText("error-box", "当前上传文件已失效，请重新上传 PDF 后再提交。");
  }

  async function submitForm(event) {
    event.preventDefault();
    const workflow = currentWorkflow();
    if (isMockMode()) {
      $("submit-btn").disabled = true;
      setText("error-box", "-");
      try {
        const payload = await submitJobRequest(apiPrefix, { workflow, source: {}, mock: true });
        state.currentJobStartedAt = new Date().toISOString();
        state.currentJobFinishedAt = "";
        renderJob(payload);
        getJobRuntimeFeature()?.startPolling(payload.job_id);
      } catch (err) {
        setText("error-box", err.message);
      } finally {
        $("submit-btn").disabled = false;
      }
      return;
    }
    if (state.desktopMode && !state.desktopConfigured && workflowNeedsCredentials(workflow)) {
      openSetupDialog();
      setText("error-box", "请先完成首次配置。");
      return;
    }
    if (workflowNeedsUpload(workflow) && !state.uploadId) {
      setText("error-box", "请先选择并上传 PDF 文件");
      return;
    }
    if (!workflowNeedsUpload(workflow) && !currentRenderSourceJobId()) {
      setText("error-box", "请先在开发者设置里填写 Render 源任务 ID。");
      return;
    }
    if (workflowNeedsCredentials(workflow) && !(await getBrowserCredentialsFeature()?.ensureOcrCredentialsReady({
      onMissingToken: () => {
        setText("error-box", "请先填写当前 OCR Provider 凭证。");
        if (!state.desktopMode) {
          getBrowserCredentialsFeature()?.openBrowserCredentialsDialog();
        }
      },
      onInvalidToken: (result) => {
        setText("error-box", result.summary || "OCR Provider 凭证校验未通过。");
        if (!state.desktopMode) {
          getBrowserCredentialsFeature()?.openBrowserCredentialsDialog();
        }
      },
    }))) {
      return;
    }

    $("submit-btn").disabled = true;
    setText("error-box", "-");

    try {
      const runPayload = collectRunPayload();
      const payload = await submitJobRequest(apiPrefix, runPayload);
      state.currentJobStartedAt = new Date().toISOString();
      state.currentJobFinishedAt = "";
      renderJob(payload);
      getJobRuntimeFeature()?.startPolling(payload.job_id);
    } catch (err) {
      if (isMissingUploadError(err)) {
        handleMissingUploadError();
        return;
      }
      setText("error-box", err.message);
    } finally {
      $("submit-btn").disabled = false;
    }
  }

  async function checkApiConnectivity() {
    try {
      const resp = await fetch(buildApiEndpoint("", "health"));
      if (!resp.ok) {
        throw new Error(`health ${resp.status}`);
      }
      return true;
    } catch (_err) {
      const message = `当前前端无法连接后端。API Base: ${apiBase()}。请确认本地服务已经启动，然后重试。`;
      setText("error-box", message);
      throw new Error(message);
    }
  }

  async function handleDesktopSetupSave() {
    const provider = currentDesktopSetupProvider();
    const definition = getOcrProviderDefinition(provider);
    const mineruToken = $("setup-mineru-token")?.value?.trim() || "";
    const paddleToken = $("setup-paddle-token")?.value?.trim() || "";
    const providerToken = definition.id === "paddle" ? paddleToken : mineruToken;
    const modelApiKey = $("setup-model-api-key").value.trim();
    if (!providerToken || !modelApiKey) {
      setDesktopBusy(`请先填写 ${definition.tokenLabel} 和 DeepSeek Key。`);
      return;
    }
    setDesktopBusy("正在保存配置并启动服务…");
    try {
      await saveDesktopConfig(
        {
          browserConfig: {
            ocrProvider: provider,
            mineruToken,
            paddleToken,
            modelApiKey,
          },
          markConfigured: true,
        },
        checkApiConnectivity,
      );
      onDesktopConfigSaved?.();
      setDesktopBusy("");
    } catch (err) {
      setDesktopBusy(err.message || String(err));
    }
  }

  async function handleOpenOutputDir() {
    try {
      await openDesktopOutputDirectory();
    } catch (err) {
      setText("error-box", err.message || String(err));
    }
  }

  $("setup-ocr-provider")?.addEventListener("change", syncDesktopSetupProviderUi);
  syncDesktopSetupProviderUi();

  return {
    checkApiConnectivity,
    handleDesktopSetupSave,
    handleOpenOutputDir,
    submitForm,
  };
}
